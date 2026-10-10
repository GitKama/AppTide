use crate::{
    catalog::{self, App, Catalog, Result, actionable, eligible, now},
    winget,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

#[derive(Clone)]
pub struct Plan {
    pub kind: String,
    pub apps: Vec<App>,
    pub created: Instant,
}
pub struct Service {
    pub catalog: Catalog,
    pub directory: PathBuf,
    pub warnings: Vec<String>,
    pub demo: bool,
    pub cancel: Arc<AtomicBool>,
    absence_verifiable: bool,
    _lock: Option<fs::File>,
}
impl Service {
    pub fn open(directory: PathBuf, demo: bool) -> Result<Self> {
        if demo {
            return Ok(Self {
                catalog: demo_catalog(),
                directory,
                warnings: vec![],
                demo,
                cancel: Arc::new(AtomicBool::new(false)),
                absence_verifiable: true,
                _lock: None,
            });
        }
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(directory.join("instance.lock"))
            .map_err(|e| e.to_string())?;
        fs2::FileExt::try_lock_exclusive(&lock).map_err(
            |_| "AppTide Beta already has this catalogue open. Close the other beta window first.",
        )?;
        let catalog = Catalog::load(&directory.join("catalog.json"))?;
        Ok(Self {
            catalog,
            directory,
            warnings: vec![],
            demo,
            cancel: Arc::new(AtomicBool::new(false)),
            absence_verifiable: false,
            _lock: Some(lock),
        })
    }
    pub fn save(&self) -> Result<()> {
        if self.demo {
            Ok(())
        } else {
            self.catalog.save(&self.directory)
        }
    }
    pub fn scan(&mut self) -> Result<bool> {
        if self.demo {
            self.catalog.scans += 1;
            self.catalog.last_scan = Some(now());
            return Ok(true);
        }
        let native = crate::inventory::read()?;
        self.warnings.clear();
        let (rows, rejected, complete) = match winget::list() {
            Ok((r, n)) => (r, n, true),
            Err(e) => {
                self.warnings.push(e);
                (vec![], 0, false)
            }
        };
        if rejected > 0 {
            self.warnings.push(format!(
                "{rejected} WinGet lines excluded (summaries or unsupported rows)."
            ));
        }
        // A table with skipped rows cannot prove that a package is absent.
        self.absence_verifiable = complete && rejected == 0;
        let observed = winget::reconcile(native, rows);
        self.catalog.observe(observed);
        self.learn_logs();
        self.save()?;
        Ok(complete)
    }
    fn learn_logs(&mut self) {
        let Some(local) = std::env::var_os("LOCALAPPDATA") else {
            return;
        };
        let ids: Vec<_> = self
            .catalog
            .apps
            .values()
            .filter_map(|a| a.winget_id.clone())
            .collect();
        for suffix in [
            "Packages/Microsoft.DesktopAppInstaller_8wekyb3d8bbwe/LocalState/DiagOutputDir",
            "Microsoft/WinGet/DiagOutputDir",
        ] {
            let Ok(entries) = fs::read_dir(PathBuf::from(&local).join(suffix)) else {
                continue;
            };
            let mut files: Vec<_> = entries
                .flatten()
                .filter_map(|e| {
                    let m = e.metadata().ok()?;
                    let n = e.file_name().to_string_lossy().to_lowercase();
                    (m.is_file()
                        && m.len() < 2 * 1024 * 1024
                        && n.starts_with("winget")
                        && n.ends_with(".log"))
                    .then(|| (m.modified().ok(), e.path()))
                })
                .collect();
            files.sort_by_key(|a| std::cmp::Reverse(a.0));
            for (_, path) in files.into_iter().take(30) {
                let Ok(text) = fs::read_to_string(path) else {
                    continue;
                };
                let hash = catalog::digest(&text);
                for id in &ids {
                    if log_mentions(&text, id)
                        && !self
                            .catalog
                            .log_signals
                            .iter()
                            .any(|s| s["hash"] == hash && s["packageId"] == *id)
                    {
                        self.catalog.log_signals.push(json!({"packageId":id,"hash":hash,"type":"mentioned-in-winget-log","observedAt":now()}));
                    }
                }
            }
        }
        let n = self.catalog.log_signals.len();
        if n > 5000 {
            self.catalog.log_signals.drain(..n - 5000);
        }
        for a in self.catalog.apps.values_mut() {
            a.log_mentions = self
                .catalog
                .log_signals
                .iter()
                .filter(|s| {
                    a.winget_id
                        .as_deref()
                        .is_some_and(|id| s["packageId"] == id)
                })
                .count();
        }
    }
    pub fn import_alpha(&mut self) -> Result<()> {
        if self.catalog.scans > 0 || !self.catalog.apps.is_empty() {
            return Err("Import is available only before the beta has its own catalogue.".into());
        }
        let path = PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA missing")?)
            .join("AppTide/catalog.json");
        if !path.is_file() {
            return Err("No alpha catalogue found in LocalAppData/AppTide.".into());
        }
        let mut imported = Catalog::load(&path)?;
        // Migration never carries automatic execution consent across applications.
        imported.settings = json!({"automatic":false});
        for app in imported.apps.values_mut() {
            app.auto_update = false;
        }
        self.catalog = imported;
        self.save()
    }
    pub fn execute(&mut self, plan: Plan, mut status: impl FnMut(&str, &Catalog)) -> Result<()> {
        if plan.created.elapsed().as_secs() > 600 {
            return Err("Preview expired. Review the operation again.".into());
        }
        self.cancel.store(false, Ordering::Relaxed);
        status(
            "Checking installed versions before proceeding…",
            &self.catalog,
        );
        if !self.scan()? {
            return Err("WinGet preflight failed. No software changed.".into());
        }
        validate_plan(&plan, &self.catalog)?;
        for before in &plan.apps {
            if self.cancel.load(Ordering::Relaxed) {
                break;
            }
            let receipt_id = format!("{}-{}", chrono::Utc::now().timestamp_millis(), before.uid);
            let mut receipt = json!({"id":receipt_id,"kind":plan.kind,"packageId":before.winget_id,"name":before.name,"before":before.version,"target":before.available,"startedAt":now(),"status":"running"});
            self.catalog.receipts.push(receipt.clone());
            self.save()?;
            status(
                &format!(
                    "{} {}",
                    if plan.kind == "update" {
                        "Updating"
                    } else {
                        "Removing"
                    },
                    before.name
                ),
                &self.catalog,
            );
            let result = (|| -> Result<(i32, bool)> {
                if self.demo {
                    std::thread::sleep(std::time::Duration::from_millis(450));
                    let a = self.catalog.apps.get_mut(&before.uid).unwrap();
                    if plan.kind == "update" {
                        a.version = a.available.clone();
                        a.available.clear();
                    } else {
                        a.present = false;
                    }
                    return Ok((0, true));
                }
                let out = winget::run("winget.exe", &winget::args(&plan.kind, before)?, None)?;
                let receipts = self.directory.join("receipts");
                fs::create_dir_all(&receipts).map_err(|e| e.to_string())?;
                fs::write(
                    receipts.join(format!("{receipt_id}.log")),
                    format!("{}\n{}", out.stdout, out.stderr),
                )
                .map_err(|e| e.to_string())?;
                status("Verifying the installed software…", &self.catalog);
                Ok((out.code, self.scan().unwrap_or(false)))
            })();
            match result {
                Ok((code, complete)) => {
                    let matches: Vec<_> = self
                        .catalog
                        .apps
                        .values()
                        .filter(|a| {
                            a.present && (a.winget_id == before.winget_id || a.key == before.key)
                        })
                        .collect();
                    let after = matches.first().copied();
                    let state = catalog::classify(
                        &plan.kind,
                        before,
                        after,
                        code,
                        complete
                            && matches.len() <= 1
                            && (plan.kind != "uninstall" || self.absence_verifiable),
                    );
                    receipt["status"] = state.into();
                    receipt["exitCode"] = code.into();
                    receipt["after"] = after.map(|a| json!(a.version)).unwrap_or(Value::Null);
                    receipt["detail"] = if state == "verified" {
                        "Confirmed by a fresh inventory."
                    } else {
                        "Requested state not confirmed. Review the local receipt before retrying."
                    }
                    .into();
                }
                Err(e) => {
                    receipt["status"] = "unverified".into();
                    receipt["detail"] = e.into();
                }
            }
            receipt["finishedAt"] = now().into();
            let verified = receipt["status"] == "verified";
            if let Some(r) = self
                .catalog
                .receipts
                .iter_mut()
                .find(|r| r["id"] == receipt_id)
            {
                *r = receipt;
            }
            if self.catalog.receipts.len() > 1000 {
                self.catalog
                    .receipts
                    .drain(..self.catalog.receipts.len() - 1000);
            }
            self.save()?;
            status("Operation recorded", &self.catalog);
            if !verified {
                break;
            }
        }
        Ok(())
    }
}
pub fn prepare(catalog: &Catalog, kind: &str, ids: &[String]) -> Result<Plan> {
    if !["update", "uninstall"].contains(&kind)
        || ids.is_empty()
        || ids.len() > 100
        || (kind == "uninstall" && ids.len() != 1)
    {
        return Err("Select between 1 and 100 updates, or one app to remove.".into());
    }
    let mut apps = vec![];
    for id in ids {
        let a = catalog.apps.get(id).ok_or("App no longer exists")?;
        if !actionable(a) || (kind == "update" && !eligible(a)) {
            return Err(
                "App is protected, ambiguous, paused, or has no supported update source.".into(),
            );
        }
        if !apps.iter().any(|old: &App| old.uid == a.uid) {
            apps.push(a.clone());
        }
    }
    Ok(Plan {
        kind: kind.into(),
        apps,
        created: Instant::now(),
    })
}
pub fn validate_plan(plan: &Plan, catalog: &Catalog) -> Result<()> {
    for before in &plan.apps {
        let matches: Vec<_> = catalog
            .apps
            .values()
            .filter(|a| a.present && a.winget_id == before.winget_id)
            .collect();
        if matches.len() != 1 {
            return Err("Package identity changed; review a fresh preview.".into());
        }
        let a = matches[0];
        if !actionable(a)
            || a.version != before.version
            || (plan.kind == "update" && (!eligible(a) || a.available != before.available))
        {
            return Err(
                "Installed software or update target changed; review a fresh preview.".into(),
            );
        }
    }
    Ok(())
}
fn log_mentions(text: &str, id: &str) -> bool {
    let pattern = format!(
        r"(?:^|[^A-Za-z0-9._+\-]){}(?:$|[^A-Za-z0-9._+\-])",
        regex::escape(id)
    );
    regex::Regex::new(&pattern).is_ok_and(|r| r.is_match(text))
}
pub fn demo_catalog() -> Catalog {
    let mut c = Catalog::default();
    let mut apps = vec![];
    for (i, (name, publisher, version, target)) in [
        ("Aster Notes", "Aster Project", "2.4.0", "2.5.1"),
        ("Cobalt Studio", "Cobalt Community", "4.2.0", "4.3.0"),
        ("Harbor Browser", "Harbor Foundation", "128.1", "129.0"),
        ("Lumen Player", "Lumen Project", "3.8.2", ""),
        ("Papertrail", "Papertrail", "1.6.0", ""),
        ("Pocket Tools", "Independent developer", "1.0", ""),
    ]
    .into_iter()
    .enumerate()
    {
        let id = format!("Demo.App{i}");
        apps.push(App {
            uid: catalog::digest(&format!("winget:{id}"))[..24].into(),
            key: format!("winget:{id}"),
            name: name.into(),
            publisher: publisher.into(),
            version: version.into(),
            available: target.into(),
            winget_id: if i == 5 { None } else { Some(id) },
            kind: "registry".into(),
            evidence: if i == 5 {
                "Windows inventory only"
            } else {
                "WinGet: unique name and installed version"
            }
            .into(),
            ..Default::default()
        });
    }
    for i in 0..180 {
        apps.push(App {
            uid: format!("fixture{i}"),
            key: format!("demo:{i}"),
            name: format!("Sample utility {i:03}"),
            version: "1.0".into(),
            kind: "registry".into(),
            publisher: "Synthetic test data".into(),
            ..Default::default()
        });
    }
    c.observe(apps);
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exclusive_catalogue_lock_is_released_on_close() {
        let dir = std::env::temp_dir().join(format!(
            "apptide-lock-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let first = Service::open(dir.clone(), false).unwrap();
        assert!(Service::open(dir.clone(), false).is_err());
        drop(first);
        drop(Service::open(dir.clone(), false).unwrap());
        fs::remove_file(dir.join("instance.lock")).unwrap();
        fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn log_sightings_require_boundaries() {
        assert!(log_mentions("found Vendor.App,", "Vendor.App"));
        assert!(!log_mentions("Vendor.App.Other", "Vendor.App"));
    }
    #[test]
    fn preflight_rejects_changed_target() {
        let mut s = Service::open(PathBuf::new(), true).unwrap();
        let id = s
            .catalog
            .apps
            .values()
            .find(|a| eligible(a))
            .unwrap()
            .uid
            .clone();
        let p = prepare(&s.catalog, "update", std::slice::from_ref(&id)).unwrap();
        s.catalog.apps.get_mut(&id).unwrap().available = "9.0".into();
        assert!(validate_plan(&p, &s.catalog).is_err());
    }
    #[test]
    fn demo_executes_and_verifies_without_processes() {
        let mut s = Service::open(PathBuf::new(), true).unwrap();
        let id = s
            .catalog
            .apps
            .values()
            .find(|a| eligible(a))
            .unwrap()
            .uid
            .clone();
        let p = prepare(&s.catalog, "update", std::slice::from_ref(&id)).unwrap();
        s.execute(p, |_, _| {}).unwrap();
        assert_eq!(s.catalog.receipts[0]["status"], "verified");
        let p = prepare(&s.catalog, "uninstall", &[id]).unwrap();
        s.execute(p, |_, _| {}).unwrap();
        assert_eq!(s.catalog.receipts[1]["status"], "verified");
    }
    #[test]
    fn corrupt_catalogue_is_not_reset() {
        let p = std::env::temp_dir().join(format!("apptide-corrupt-{}.json", std::process::id()));
        fs::write(&p, b"invalid").unwrap();
        assert!(Catalog::load(&p).is_err());
        assert_eq!(fs::read(&p).unwrap(), b"invalid");
        fs::remove_file(p).unwrap();
    }
}
