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
    pub interactive: bool,
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
        let mut catalog = Catalog::load(&directory.join("catalog.json"))?;
        if !catalog.settings.is_object() {
            catalog.settings = json!({"automatic":false});
        }
        crate::knowledge::enrich(&mut catalog);
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
    pub fn diagnostic(&mut self, event: &str, detail: &str) {
        if !self.demo
            && let Err(e) = crate::diagnostics::record(&self.directory, event, detail)
        {
            self.warnings
                .push(format!("Application log could not be written: {e}"));
        }
    }
    pub fn scan(&mut self) -> Result<bool> {
        if self.demo {
            self.catalog.scans += 1;
            self.catalog.last_scan = Some(now());
            return Ok(true);
        }
        let mut native = crate::inventory::read()?;
        let folders: Vec<String> = self.catalog.settings["portableFolders"]
            .as_array()
            .map(|v| {
                v.iter()
                    .filter_map(|s| s.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        native.extend(crate::portable::scan(&folders)?);
        native.extend(
            self.catalog
                .apps
                .values()
                .filter(|a| a.kind == "manual" && a.present)
                .cloned(),
        );
        self.warnings.clear();
        self.warnings.extend(crate::steam::enrich(&mut native));
        self.recheck_store_pending(&native);
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
        crate::knowledge::enrich(&mut self.catalog);
        self.learn_logs();
        self.refresh_releases(None);
        self.save()?;
        Ok(complete)
    }
    pub fn refresh_sources(&mut self) -> Result<()> {
        if !self.demo {
            let out = winget::run(
                "winget.exe",
                &[
                    "source",
                    "update",
                    "--name",
                    "winget",
                    "--disable-interactivity",
                ]
                .map(String::from),
                Some(std::time::Duration::from_secs(120)),
            )?;
            let message = format!(
                "Exit {} (0x{:08X})\n{}\n{}",
                out.code, out.code as u32, out.stdout, out.stderr
            );
            self.diagnostic("source-refresh", &message);
            if out.code != 0 {
                return Err(format!(
                    "WinGet source refresh failed (0x{:08X}). View local logs.",
                    out.code as u32
                ));
            }
        }
        self.scan().map(|_| ())
    }
    pub fn check_store(&mut self) -> Result<crate::store::Check> {
        if !self.demo
            && self
                .catalog
                .receipts
                .iter()
                .any(|r| r["provider"] == "Microsoft Store" && r["status"] == "pending")
        {
            self.recheck_store_pending(&crate::inventory::read()?);
        }
        let result = if self.demo {
            Ok(crate::store::demo_check())
        } else {
            crate::store::run(false).and_then(|out| {
                fs::create_dir_all(self.directory.join("logs")).map_err(|e| e.to_string())?;
                fs::write(
                    self.directory.join("logs/store-check.log"),
                    format!("Exit: {}\n{}\n{}", out.code, out.stdout, out.stderr),
                )
                .map_err(|e| e.to_string())?;
                crate::store::check_output(&out)
            })
        };
        let snapshot = match &result {
            Ok(check) => check.clone(),
            Err(error) => crate::store::Check {
                checked_at: now(),
                error: Some(error.clone()),
                ..Default::default()
            },
        };
        self.catalog
            .extra
            .insert("storeCheck".into(), serde_json::to_value(snapshot).unwrap());
        self.save()?;
        result
    }
    pub fn execute_store(&mut self, preview: crate::store::Check) -> Result<()> {
        if self
            .catalog
            .receipts
            .iter()
            .any(|r| r["provider"] == "Microsoft Store" && r["status"] == "pending")
        {
            return Err("Microsoft Store is still processing the previous batch. Check Store updates to recheck installed versions, or open Store Downloads.".into());
        }
        if self.demo {
            self.catalog.extra.insert(
                "storeCheck".into(),
                serde_json::to_value(crate::store::Check {
                    checked_at: now(),
                    ..Default::default()
                })
                .unwrap(),
            );
            self.catalog.receipts.push(json!({"name":"Microsoft Store updates","kind":"store update","status":"verified","startedAt":now(),"detail":"Simulated Store update; no installed software changed."}));
            return Ok(());
        }
        self.execute_store_with(preview, Self::check_store, crate::inventory::read, || {
            crate::store::run(true)
        })
    }
    fn execute_store_with(
        &mut self,
        preview: crate::store::Check,
        mut check: impl FnMut(&mut Self) -> Result<crate::store::Check>,
        mut inventory: impl FnMut() -> Result<Vec<App>>,
        mut apply: impl FnMut() -> Result<winget::Output>,
    ) -> Result<()> {
        self.cancel.store(false, Ordering::Relaxed);
        if preview.error.is_some() || preview.updates.is_empty() {
            return Err("Check Microsoft Store updates first.".into());
        }
        let current = check(self)?;
        if current.error.is_some()
            || !crate::store::same_updates(&preview.updates, &current.updates)
        {
            return Err(
                "Store updates changed since preview. Review the refreshed list before applying."
                    .into(),
            );
        }
        let before = inventory()?;
        if self.cancel.load(Ordering::Relaxed) {
            return Err("Store update cancelled before launch.".into());
        }
        let id = format!(
            "store-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        let file = format!("{id}.log");
        let index = self.catalog.receipts.len();
        self.catalog.receipts.push(json!({"id":id,"name":"Microsoft Store updates","kind":"store update","provider":"Microsoft Store","status":"running","startedAt":now(),"preview":preview.updates,"baseline":before.iter().filter(|a|a.kind=="appx").collect::<Vec<_>>(),"logFile":file}));
        self.save()?; // Persist intention before handing the bulk operation to Microsoft.
        let result = apply();
        let log = match &result {
            Ok(o) => format!(
                "Command: store updates --apply\nExit: {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
                o.code, o.stdout, o.stderr
            ),
            Err(e) => format!(
                "Store command result unknown: {e}\nStore may still be processing updates. Do not automatically retry."
            ),
        };
        let log_result = if self.demo {
            Ok(())
        } else {
            fs::create_dir_all(self.directory.join("receipts"))
                .and_then(|_| fs::write(self.directory.join("receipts").join(file), log))
                .map_err(|e| e.to_string())
        };
        let after = inventory();
        let (verified, review) = after
            .as_ref()
            .map(|a| crate::store::verification(&preview.updates, &before, a))
            .unwrap_or((0, preview.updates.len()));
        let succeeded = result
            .as_ref()
            .is_ok_and(|o| o.code == 0 && o.stderr.trim().is_empty());
        let state = if succeeded && review == 0 {
            "verified"
        } else if succeeded
            && result
                .as_ref()
                .is_ok_and(|o| o.stdout.to_lowercase().contains("updates started"))
        {
            "pending"
        } else if result.as_ref().is_ok_and(|o| o.code != 0) {
            "failed"
        } else {
            "unverified"
        };
        let detail = if state == "pending" {
            format!(
                "Microsoft Store accepted the updates and is still processing them. {verified} verified; {review} waiting for a newer installed version. Check Store updates or scan again to recheck. Open Store Downloads for progress; AppTide will not start a duplicate batch."
            )
        } else {
            format!(
                "Store operation finished: {verified} previewed apps have a verified newer installed version; {review} need review. Microsoft Store controls its batch, including dependencies. Review the output log for prompts or failures; no automatic retry."
            )
        };
        let receipt = &mut self.catalog.receipts[index];
        receipt["status"] = state.into();
        receipt["detail"] = detail.clone().into();
        if state != "pending" {
            receipt["finishedAt"] = now().into();
        }
        if let Ok(out) = &result {
            receipt["exitCode"] = out.code.into();
        }
        if let Err(error) = &after {
            receipt["inventoryError"] = error.clone().into();
        }
        if let Err(error) = &log_result {
            receipt["logError"] = error.clone().into();
        }
        // Update current-user AppX observations without reclassifying any WinGet identity.
        if let Ok(after) = after {
            let mut observed: Vec<_> = self
                .catalog
                .apps
                .values()
                .filter(|a| a.present && a.kind != "appx")
                .cloned()
                .collect();
            observed.extend(after.into_iter().filter(|a| a.kind == "appx").map(|mut a| {
                a.uid = catalog::digest(&a.key)[..24].into();
                a
            }));
            self.catalog.observe(observed);
            crate::knowledge::enrich(&mut self.catalog);
        }
        self.catalog.extra.remove("storeCheck"); // Old availability is no longer actionable.
        self.save()?;
        log_result?;
        if state == "verified" || state == "pending" {
            Ok(())
        } else {
            Err(detail)
        }
    }
    fn recheck_store_pending(&mut self, current: &[App]) {
        for r in self
            .catalog
            .receipts
            .iter_mut()
            .filter(|r| r["provider"] == "Microsoft Store" && r["status"] == "pending")
        {
            let baseline = serde_json::from_value::<Vec<App>>(r["baseline"].clone());
            let updates = serde_json::from_value::<Vec<crate::store::Update>>(r["preview"].clone());
            if let (Ok(before), Ok(updates)) = (baseline, updates) {
                let (verified, review) = crate::store::verification(&updates, &before, current);
                r["lastCheckedAt"] = now().into();
                if review == 0 && !updates.is_empty() {
                    r["status"] = "verified".into();
                    r["finishedAt"] = now().into();
                    r["detail"]=format!("All {verified} previewed Store updates confirmed by newer installed package versions.").into();
                } else {
                    r["detail"]=format!("Microsoft Store batch: {verified} verified, {review} still awaiting a newer installed version. Open Store Downloads for progress or errors, then check again.").into();
                    if r["startedAt"]
                        .as_str()
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                        .is_some_and(|t| {
                            chrono::Utc::now().signed_duration_since(t).num_hours() >= 24
                        })
                    {
                        r["status"] = "unverified".into();
                        r["detail"]=format!("Store verification still incomplete after 24 hours: {verified} verified, {review} need review. Inspect Store Downloads before starting another batch.").into();
                    }
                }
            }
        }
    }
    pub fn refresh_releases(&mut self, only: Option<&str>) {
        if self.demo {
            return;
        }
        let mut repos = std::collections::BTreeSet::new();
        for a in self
            .catalog
            .apps
            .values()
            .filter(|a| a.present && only.is_none_or(|id| a.uid == id))
        {
            if let Some(repo) = crate::releases::repository(a) {
                let recent = a
                    .extra
                    .get("watch")
                    .and_then(|w| {
                        w["attemptedAt"]
                            .as_str()
                            .or_else(|| w["checkedAt"].as_str())
                    })
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .is_some_and(|t| {
                        chrono::Utc::now().signed_duration_since(t).num_seconds()
                            < if only.is_some() { 0 } else { 3600 }
                    });
                if !recent
                    || a.extra
                        .get("watch")
                        .is_none_or(|w| !w["assets"].is_array() && w["attemptedAt"].is_null())
                {
                    repos.insert(repo.to_string());
                }
            }
        }
        for repo in repos.into_iter().take(20) {
            if self.cancel.load(Ordering::Relaxed) {
                break;
            }
            match crate::releases::fetch(&repo) {
                Ok(release) => {
                    for a in self.catalog.apps.values_mut().filter(|a| {
                        a.present && crate::releases::repository(a) == Some(repo.as_str())
                    }) {
                        crate::releases::apply(a, &release);
                    }
                }
                Err(e) => {
                    self.warnings.push(format!("{repo}: {e}"));
                    for a in self
                        .catalog
                        .apps
                        .values_mut()
                        .filter(|a| crate::releases::repository(a) == Some(repo.as_str()))
                    {
                        if let Some(w) = a.extra.get_mut("watch") {
                            w["error"] = e.clone().into();
                            w["attemptedAt"] = now().into();
                        }
                    }
                }
            }
        }
    }
    pub fn watch(&mut self, id: &str, repository: &str) -> Result<()> {
        self.cancel.store(false, Ordering::Relaxed);
        let a = self
            .catalog
            .apps
            .get_mut(id)
            .ok_or("App no longer exists")?;
        crate::releases::set_watch(a, repository.trim())?;
        crate::github_update::synchronize(a);
        if let Some(mut source) = crate::custom::get(a) {
            source.repository = repository.trim().into();
            a.extra
                .insert("customSource".into(), serde_json::to_value(source).unwrap());
        }
        self.save()?;
        self.refresh_releases(Some(id));
        self.save()
    }
    pub fn custom_source(
        &mut self,
        id: Option<&str>,
        name: &str,
        source: crate::custom::Source,
    ) -> Result<()> {
        let old = self.catalog.clone();
        let uid = if let Some(id) = id {
            id.to_string()
        } else {
            if name.trim().is_empty() || name.len() > 160 {
                return Err("Enter an app name (up to 160 characters).".into());
            }
            let key = format!(
                "manual-{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
            );
            let id = catalog::digest(&key)[..24].to_string();
            self.catalog.apps.insert(
                id.clone(),
                App {
                    uid: id.clone(),
                    key,
                    name: name.trim().into(),
                    kind: "manual".into(),
                    present: true,
                    first_seen: now(),
                    last_seen: now(),
                    evidence: "Manually tracked by you; installation not detected".into(),
                    ..Default::default()
                },
            );
            id
        };
        let result = (|| {
            let a = self
                .catalog
                .apps
                .get_mut(&uid)
                .ok_or("App no longer exists")?;
            if a.kind == "manual" {
                if name.trim().is_empty() || name.len() > 160 {
                    return Err("Enter an app name (up to 160 characters).".into());
                }
                a.name = name.trim().into();
            }
            crate::custom::save(a, source)?;
            crate::knowledge::enrich(&mut self.catalog);
            self.save()
        })();
        if result.is_err() {
            self.catalog = old;
        }
        result
    }
    pub fn automatic_preference(&mut self, id: Option<&str>, enabled: bool) -> Result<()> {
        let old = self.catalog.clone();
        if let Some(id) = id {
            let a = self
                .catalog
                .apps
                .get_mut(id)
                .ok_or("App no longer exists")?;
            if enabled && !actionable(a) && crate::github_update::consent_key(a).is_none() {
                return Err(
                    "Automatic updates require WinGet or a reviewed GitHub installer recipe."
                        .into(),
                );
            }
            if let Some(key) = crate::github_update::consent_key(a) {
                a.extra.insert("githubAutomaticConsent".into(), key.into());
            }
            a.auto_update = enabled;
            a.extra.insert("automaticConsent".into(), 2.into());
        } else {
            self.catalog.settings["automatic"] = enabled.into();
            self.catalog.settings["automaticConsent"] = 2.into();
            self.catalog.settings["lastAutomaticAttempt"] = now().into();
        }
        if let Err(e) = self.save() {
            self.catalog = old;
            return Err(e);
        }
        Ok(())
    }
    pub fn add_folder(&mut self, folder: &str, remove: bool) -> Result<()> {
        let mut folders: Vec<String> =
            serde_json::from_value(self.catalog.settings["portableFolders"].clone())
                .unwrap_or_default();
        if remove {
            folders.retain(|f| f != folder);
        } else {
            crate::portable::validate_folder(folder)?;
            let canonical = std::fs::canonicalize(folder)
                .map_err(|e| e.to_string())?
                .display()
                .to_string();
            // Keep a user-readable local path rather than the Windows extended-length prefix.
            let canonical = canonical
                .strip_prefix(r"\\?\")
                .unwrap_or(&canonical)
                .to_string();
            if !folders.iter().any(|f| f.eq_ignore_ascii_case(&canonical)) {
                folders.push(canonical);
            }
            if folders.len() > 10 {
                return Err("Limit portable discovery to 10 folders.".into());
            }
        }
        let old = self.catalog.settings.clone();
        self.catalog.settings["portableFolders"] = json!(folders);
        if let Err(e) = self.save() {
            self.catalog.settings = old;
            return Err(e);
        }
        Ok(())
    }
    pub fn automatic(&mut self, status: impl FnMut(&str, &Catalog)) -> Result<()> {
        if self.demo || !automatic_due(&self.catalog) {
            return Ok(());
        }
        self.cancel.store(false, Ordering::Relaxed);
        // Persist the attempt before any scan/installer. Restart cannot create a retry loop.
        self.catalog.settings["lastAutomaticAttempt"] = now().into();
        self.save()?;
        let winget_complete = self.scan()?;
        if self.cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        let ids: Vec<_> = automatic_candidates(&self.catalog)
            .into_iter()
            .filter(|id| winget_complete || self.catalog.apps[id].winget_id.is_none())
            .collect();
        if ids.is_empty() {
            return Ok(());
        }
        let plan = prepare(&self.catalog, "update", &ids)?;
        self.execute(plan, status)
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
    pub fn execute(&mut self, plan: Plan, status: impl FnMut(&str, &Catalog)) -> Result<()> {
        let interactive = plan.interactive;
        self.execute_with(plan, status, Self::scan, |service, kind, before| {
            if service.demo {
                let a = service.catalog.apps.get_mut(&before.uid).unwrap();
                if kind == "update" {
                    a.version = a.available.clone();
                    a.available.clear();
                } else {
                    a.present = false;
                }
                return Ok(winget::Output {
                    code: 0,
                    stdout: "Simulated operation".into(),
                    stderr: String::new(),
                });
            }
            if kind == "update" && before.winget_id.is_none() {
                return crate::github_update::install(before, &service.directory.join("downloads"));
            }
            winget::run(
                "winget.exe",
                &winget::args_mode(kind, before, interactive)?,
                None,
            )
        })
    }
    fn save_batch(&mut self, batch: &Value) -> Result<()> {
        self.catalog.extra.insert("lastBatch".into(), batch.clone());
        self.save()?;
        if !self.demo {
            let folder = self.directory.join("batches");
            fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
            fs::write(
                folder.join(format!("{}.json", batch["id"].as_str().unwrap())),
                serde_json::to_vec_pretty(batch).map_err(|e| e.to_string())?,
            )
            .map_err(|e| format!("Could not save batch diagnostics: {e}"))?;
        }
        Ok(())
    }
    // Injection is limited to tests; production always uses the same scan and WinGet runner.
    fn execute_with(
        &mut self,
        plan: Plan,
        mut status: impl FnMut(&str, &Catalog),
        mut scan: impl FnMut(&mut Self) -> Result<bool>,
        mut install: impl FnMut(&mut Self, &str, &App) -> Result<winget::Output>,
    ) -> Result<()> {
        let batch_id = format!(
            "{}-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(),
            std::process::id()
        );
        let mut batch = json!({"id":batch_id,"kind":plan.kind,"startedAt":now(),"status":"running",
            "requested":plan.apps.len(),"items":plan.apps.iter().map(|a| json!({"packageId":a.winget_id,"name":a.name,"before":a.version,"target":a.available})).collect::<Vec<_>>()});
        self.save_batch(&batch)?;
        let result = self.execute_inner(&plan, &batch_id, &mut status, &mut scan, &mut install);
        let receipts: Vec<_> = self
            .catalog
            .receipts
            .iter()
            .filter(|r| r["batchId"] == batch_id)
            .cloned()
            .collect();
        let verified = receipts
            .iter()
            .filter(|r| r["status"] == "verified")
            .count();
        let skipped = receipts.iter().filter(|r| r["status"] == "skipped").count();
        let attention = receipts.len() - verified - skipped;
        let not_run = plan.apps.len().saturating_sub(receipts.len());
        let cancelled = self.cancel.load(Ordering::Relaxed);
        let summary = format!(
            "{}: {verified} verified, {skipped} skipped, {attention} need review, {not_run} not run.",
            if cancelled || (result.is_err() && not_run > 0) {
                "Queue stopped"
            } else if result.is_err() || attention > 0 {
                "Queue finished with issues"
            } else if skipped > 0 {
                "Queue finished with skips"
            } else {
                "Queue complete"
            }
        );
        batch["status"] = if cancelled {
            "cancelled"
        } else if result.is_err() || attention > 0 {
            "attention"
        } else if skipped > 0 {
            "partial"
        } else {
            "complete"
        }
        .into();
        batch["summary"] = summary.clone().into();
        batch["detail"] = result.as_ref().err().cloned().unwrap_or_default().into();
        batch["finishedAt"] = now().into();
        batch["receipts"] = receipts.into();
        if self.catalog.receipts.len() > 1000 {
            self.catalog
                .receipts
                .drain(..self.catalog.receipts.len() - 1000);
        }
        self.save_batch(&batch)?;
        status(&summary, &self.catalog);
        result.map_err(|e| format!("{summary} {e} Open Activity for details and local logs."))
    }
    fn execute_inner(
        &mut self,
        plan: &Plan,
        batch_id: &str,
        status: &mut impl FnMut(&str, &Catalog),
        scan: &mut impl FnMut(&mut Self) -> Result<bool>,
        install: &mut impl FnMut(&mut Self, &str, &App) -> Result<winget::Output>,
    ) -> Result<()> {
        if plan.created.elapsed().as_secs() > 600 {
            return Err("Preview expired. Review the operation again.".into());
        }
        self.cancel.store(false, Ordering::Relaxed);
        status(
            "Checking installed versions before proceeding…",
            &self.catalog,
        );
        if !scan(self)? && plan.apps.iter().any(|a| a.winget_id.is_some()) {
            return Err(format!(
                "WinGet preflight failed. No software changed. {}",
                self.warnings.join(" ")
            ));
        }
        let mut issues = Vec::new();
        for (index, before) in plan.apps.iter().enumerate() {
            if self.cancel.load(Ordering::Relaxed) {
                break;
            }
            let receipt_id = format!("{}-{}", chrono::Utc::now().timestamp_millis(), before.uid);
            let mut receipt = json!({"id":receipt_id,"batchId":batch_id,"kind":plan.kind,"appId":before.uid,"packageId":before.winget_id,"name":before.name,"before":before.version,"target":before.available,"startedAt":now(),"provider":if before.winget_id.is_some(){"WinGet"}else{"GitHub"},"status":"running"});
            // Never silently change the reviewed target. Skip stale items without blocking unrelated apps.
            if let Err(reason) = validate_app(&plan.kind, before, &self.catalog) {
                receipt["status"] = "skipped".into();
                receipt["detail"] = reason.into();
                receipt["finishedAt"] = now().into();
                self.catalog.receipts.push(receipt);
                self.save()?;
                continue;
            }
            self.catalog.receipts.push(receipt.clone());
            self.save()?;
            status(
                &format!(
                    "({}/{}) {} {}",
                    index + 1,
                    plan.apps.len(),
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
                let args = if before.winget_id.is_none() {
                    let offer = crate::github_update::offer(before)
                        .ok_or("GitHub installer unavailable")?;
                    receipt["githubAsset"] = json!(offer);
                    vec![
                        "GitHub release installer".into(),
                        offer.repository,
                        offer.name,
                    ]
                } else {
                    winget::args_mode(&plan.kind, before, plan.interactive)?
                };
                receipt["command"] = json!(args);
                let out = install(self, &plan.kind, before)?;
                receipt["exitCode"] = out.code.into();
                if !self.demo {
                    let receipts = self.directory.join("receipts");
                    fs::create_dir_all(&receipts).map_err(|e| e.to_string())?;
                    fs::write(
                    receipts.join(format!("{receipt_id}.log")),
                    format!("AppTide {}\nBatch: {batch_id}\nOperation: {}\nExit code: {} (0x{:08X})\n--- stdout ---\n{}\n--- stderr ---\n{}", env!("CARGO_PKG_VERSION"), args.join(" "), out.code, out.code as u32, out.stdout, out.stderr),
                )
                .map_err(|e| e.to_string())?;
                    receipt["logFile"] = format!("{receipt_id}.log").into();
                }
                status("Verifying the installed software…", &self.catalog);
                let complete = scan(self)? || before.winget_id.is_none();
                if !complete {
                    receipt["scanWarnings"] = json!(self.warnings);
                }
                Ok((out.code, complete))
            })();
            let mut independent_apps_can_continue = false;
            match result {
                Ok((code, complete)) => {
                    // An individual package's failure must not block unrelated updates. Only
                    // proceed while we still have a fresh inventory and no stop/reboot request.
                    independent_apps_can_continue = complete && !winget::queue_stop_code(code);
                    let matches: Vec<_> = self
                        .catalog
                        .apps
                        .values()
                        .filter(|a| a.present && crate::github_update::same_installation(a, before))
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
                    receipt["observedVersions"] = json!(
                        matches
                            .iter()
                            .map(|a| a.version.as_str())
                            .collect::<Vec<_>>()
                    );
                    receipt["detail"] = if winget::queue_stop_code(code) {
                        format!("WinGet requested cancellation, a reboot, or reported a shared installer/source problem (0x{:08X}). Remaining apps were not started; review the log before resuming.", code as u32)
                    } else if state == "verified" {
                        "Confirmed by a fresh inventory.".to_string()
                    } else if !complete {
                        format!("Verification scan incomplete. {}", self.warnings.join(" "))
                    } else if let Some(reason) = winget::failure_hint(code) {
                        reason.to_string()
                    } else if matches.len() > 1 || after.is_some_and(|a| a.ambiguous) {
                        format!("Multiple installed registrations match this package ({}). Installer exit code: {code}. Review these installations before retrying; no automatic retry will run.", matches.iter().map(|a| a.version.as_str()).collect::<Vec<_>>().join(", "))
                    } else {
                        format!("Requested state not confirmed. Installer exit code: {code} (0x{:08X}). Review the local output log before retrying.", code as u32)
                    }
                    .into();
                }
                Err(e) => {
                    if before.winget_id.is_none() {
                        independent_apps_can_continue = scan(self).unwrap_or(false);
                    }
                    receipt["status"] = "unverified".into();
                    receipt["detail"] = e.into();
                }
            }
            receipt["finishedAt"] = now().into();
            let verified = receipt["status"] == "verified";
            let reason = receipt["detail"].as_str().unwrap_or_default().to_string();
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
            crate::knowledge::enrich(&mut self.catalog);
            self.save()?;
            status("Operation recorded", &self.catalog);
            if !independent_apps_can_continue {
                return Err(format!("Stopped at {}: {reason}", before.name));
            }
            if !verified {
                issues.push(before.name.clone());
                status(
                    &format!("{} needs review; continuing remaining apps…", before.name),
                    &self.catalog,
                );
            }
        }
        if issues.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "{} app(s) need review: {}. Other eligible apps were attempted; failed apps were not retried.",
                issues.len(),
                issues.join(", ")
            ))
        }
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
        if (kind == "uninstall" && !actionable(a)) || (kind == "update" && !eligible(a)) {
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
        interactive: false,
        apps,
        created: Instant::now(),
    })
}
pub fn automatic_due(c: &Catalog) -> bool {
    c.settings["automatic"] == true
        && c.settings["automaticConsent"] == 2
        && c.apps.values().any(|a| {
            a.present && a.auto_update && a.extra.get("automaticConsent") == Some(&json!(2))
        })
        && c.settings["lastAutomaticAttempt"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .is_some_and(|t| chrono::Utc::now().signed_duration_since(t).num_seconds() >= 3600)
}
pub fn automatic_candidates(c: &Catalog) -> Vec<String> {
    c.apps
        .values()
        .filter(|a| {
            eligible(a)
                && a.auto_update
                && (actionable(a) || crate::github_update::automatic_allowed(a))
                && a.extra.get("automaticConsent") == Some(&json!(2))
                && c.receipts
                    .iter()
                    .rev()
                    .find(|r| {
                        (a.winget_id
                            .as_deref()
                            .is_some_and(|id| r["packageId"] == id)
                            || r["appId"] == a.uid)
                            && r["status"] != "skipped"
                    })
                    .is_none_or(|r| r["status"] == "verified")
        })
        .take(100)
        .map(|a| a.uid.clone())
        .collect()
}
fn validate_app(kind: &str, before: &App, catalog: &Catalog) -> Result<()> {
    let matches: Vec<_> = catalog
        .apps
        .values()
        .filter(|a| a.present && crate::github_update::same_installation(a, before))
        .collect();
    if matches.len() != 1 {
        return Err(format!(
            "{}: expected one installation, found {}. No installer started; refresh and review this app.",
            before.name,
            matches.len()
        ));
    }
    let a = matches[0];
    if (kind == "uninstall" && !actionable(a))
        || (before.winget_id.is_none()
            && crate::github_update::offer(a) != crate::github_update::offer(before))
        || a.version != before.version
        || (kind == "update" && (!eligible(a) || a.available != before.available))
    {
        return Err(format!(
            "{} changed since preview (installed: {}; available: {}). It may already be updated, paused or protected. No installer started; refresh and review this app.",
            before.name,
            a.version,
            if a.available.is_empty() {
                "none"
            } else {
                &a.available
            }
        ));
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
    apps.push(App {
        uid: "recipe-demo".into(),
        key: "registry:Demo:ExifToolGUI".into(),
        name: "ExifToolGUI version 6.0.0_64".into(),
        scope: "user".into(),
        publisher: "FrankB".into(),
        version: "6.0.0_64".into(),
        kind: "registry".into(),
        ..Default::default()
    });
    c.observe(apps);
    crate::knowledge::enrich(&mut c);
    crate::releases::apply(
        c.apps.get_mut("recipe-demo").unwrap(),
        &json!({"repository":"FrankBijnen/ExifToolGui","latest":"V6.1.0","checkedAt":"Synthetic preview","publishedAt":"Synthetic preview"}),
    );
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn github_automatic_requires_bound_consent_and_ignores_other_null_package_receipts() {
        let mut s = Service::open(PathBuf::new(), true).unwrap();
        let mut a = crate::github_update::tests::app();
        a.name = "meetily".into();
        a.publisher = "meetily".into();
        a.version = "1.0".into();
        a.knowledge = crate::knowledge::detect(&a);
        let w = a.extra.get_mut("watch").unwrap();
        w["repository"] = "Zackriya-Solutions/meetily".into();
        w["assets"][0]["name"] = "meetily_2.0_x64-setup.exe".into();
        w["assets"][0]["browser_download_url"]="https://github.com/Zackriya-Solutions/meetily/releases/download/v2.0/meetily_2.0_x64-setup.exe".into();
        crate::github_update::synchronize(&mut a);
        let id = a.uid.clone();
        s.catalog.apps.insert(id.clone(), a);
        assert!(!automatic_candidates(&s.catalog).contains(&id));
        s.automatic_preference(Some(&id), true).unwrap();
        assert!(automatic_candidates(&s.catalog).contains(&id));
        s.catalog
            .receipts
            .push(json!({"packageId":null,"appId":"unrelated","status":"failed"}));
        assert!(automatic_candidates(&s.catalog).contains(&id));
        s.catalog
            .receipts
            .push(json!({"packageId":null,"appId":id,"status":"unverified"}));
        assert!(!automatic_candidates(&s.catalog).contains(&id));
        s.catalog
            .receipts
            .push(json!({"appId":id,"status":"verified"}));
        assert!(automatic_candidates(&s.catalog).contains(&id));
        s.catalog.apps.get_mut(&id).unwrap().publisher = "Changed".into();
        assert!(!automatic_candidates(&s.catalog).contains(&id));
    }
    #[test]
    fn mixed_queue_updates_github_without_matching_unrelated_unmanaged_apps() {
        let (mut s, mut plan) = batch_fixture();
        let mut a = crate::github_update::tests::app();
        crate::github_update::synchronize(&mut a);
        s.catalog.apps.insert(a.uid.clone(), a.clone());
        plan.apps.push(a.clone());
        assert!(prepare(&s.catalog, "uninstall", std::slice::from_ref(&a.uid)).is_err());
        s.execute_with(plan, |_, _| {}, |_| Ok(true), simulated_update)
            .unwrap();
        assert_eq!(s.catalog.receipts.len(), 4);
        let r = s.catalog.receipts.last().unwrap();
        assert_eq!(r["status"], "verified");
        assert_eq!(r["provider"], "GitHub");
        assert_eq!(r["appId"], a.uid);
        assert_eq!(r["githubAsset"]["id"], 1);
    }
    #[test]
    fn github_changed_hash_is_skipped_before_installer_and_failure_does_not_block_winget() {
        let (mut s, mut plan) = batch_fixture();
        let mut a = crate::github_update::tests::app();
        crate::github_update::synchronize(&mut a);
        s.catalog.apps.insert(a.uid.clone(), a.clone());
        plan.apps.insert(0, a.clone());
        s.catalog
            .apps
            .get_mut(&a.uid)
            .unwrap()
            .extra
            .get_mut("watch")
            .unwrap()["assets"][0]["digest"] = format!("sha256:{}", "b".repeat(64)).into();
        assert!(validate_app("update", &a, &s.catalog).is_err());
        s.catalog.apps.insert(a.uid.clone(), a);
        assert!(
            s.execute_with(
                plan,
                |_, _| {},
                |_| Ok(true),
                |s, k, a| if a.winget_id.is_none() {
                    Err("Hash mismatch: nothing installed".into())
                } else {
                    simulated_update(s, k, a)
                }
            )
            .is_err()
        );
        assert_eq!(s.catalog.receipts.len(), 4);
        assert_eq!(s.catalog.receipts[0]["status"], "unverified");
        assert!(
            s.catalog.receipts[1..]
                .iter()
                .all(|r| r["status"] == "verified")
        );
    }
    #[test]
    fn store_changed_preview_never_starts_an_update() {
        let mut service = Service::open(PathBuf::new(), true).unwrap();
        let plan = crate::store::demo_check();
        let error = service
            .execute_store_with(
                plan,
                |_| Ok(crate::store::Check::default()),
                || panic!("inventory unnecessary"),
                || panic!("must not update"),
            )
            .unwrap_err();
        assert!(error.contains("changed since preview"));
        assert!(service.catalog.receipts.is_empty());
    }
    #[test]
    fn store_failed_preflight_never_starts_an_update() {
        let mut service = Service::open(PathBuf::new(), true).unwrap();
        let plan = crate::store::demo_check();
        assert!(
            service
                .execute_store_with(
                    plan,
                    |_| Err("Store unavailable".into()),
                    || panic!("inventory unnecessary"),
                    || panic!("must not update")
                )
                .is_err()
        );
        assert!(service.catalog.receipts.is_empty());
    }
    #[test]
    fn store_success_exit_without_version_change_is_unverified() {
        let mut service = Service::open(PathBuf::new(), true).unwrap();
        let plan = crate::store::demo_check();
        let current = plan.clone();
        let mut calls = 0;
        let result = service.execute_store_with(
            plan,
            |_| Ok(current.clone()),
            || Ok(vec![]),
            || {
                calls += 1;
                Ok(winget::Output {
                    code: 0,
                    stdout: "Synthetic CLI returned without updating".into(),
                    stderr: String::new(),
                })
            },
        );
        assert!(result.is_err());
        assert_eq!(calls, 1);
        assert_eq!(
            service.catalog.receipts.last().unwrap()["status"],
            "unverified"
        );
        assert!(!service.catalog.extra.contains_key("storeCheck"));
    }
    #[test]
    fn store_async_batch_waits_rechecks_and_blocks_duplicate_submission() {
        let mut service = Service::open(PathBuf::new(), true).unwrap();
        let mut plan = crate::store::demo_check();
        plan.updates.truncate(1);
        let row = &plan.updates[0];
        let old = App {
            key: "appx:Orchid_signed".into(),
            kind: "appx".into(),
            name: row.name.clone(),
            publisher: row.publisher.clone(),
            version: row.version.clone(),
            extra: [
                ("signatureKind".into(), "Store".into()),
                ("rawPublisher".into(), "CN=Orchid".into()),
            ]
            .into(),
            ..Default::default()
        };
        service
            .execute_store_with(
                plan.clone(),
                |_| Ok(plan.clone()),
                || Ok(vec![old.clone()]),
                || {
                    Ok(winget::Output {
                        code: 0,
                        stdout: "Updates started".into(),
                        stderr: String::new(),
                    })
                },
            )
            .unwrap();
        assert_eq!(
            service.catalog.receipts.last().unwrap()["status"],
            "pending"
        );
        assert!(
            service
                .execute_store(plan)
                .unwrap_err()
                .contains("still processing")
        );
        service.recheck_store_pending(std::slice::from_ref(&old));
        assert_eq!(
            service.catalog.receipts.last().unwrap()["status"],
            "pending"
        );
        service.recheck_store_pending(&[App {
            version: "2026.11.0.0".into(),
            ..old
        }]);
        assert_eq!(
            service.catalog.receipts.last().unwrap()["status"],
            "verified"
        );
    }
    #[test]
    fn store_update_verifies_versions_and_keeps_existing_catalogue() {
        let mut service = Service::open(PathBuf::new(), true).unwrap();
        let original_count = service.catalog.apps.len();
        let mut plan = crate::store::demo_check();
        plan.updates.truncate(1);
        let row = &plan.updates[0];
        let old = App {
            key: "appx:Orchid_signed".into(),
            kind: "appx".into(),
            name: row.name.clone(),
            publisher: row.publisher.clone(),
            version: row.version.clone(),
            extra: [
                ("signatureKind".into(), "Store".into()),
                ("rawPublisher".into(), "CN=Orchid".into()),
            ]
            .into(),
            ..Default::default()
        };
        let current = plan.clone();
        let mut scans = 0;
        let mut installs = 0;
        service
            .execute_store_with(
                plan,
                |_| Ok(current.clone()),
                || {
                    scans += 1;
                    Ok(vec![App {
                        version: if scans == 1 {
                            old.version.clone()
                        } else {
                            "2026.11.0.0".into()
                        },
                        ..old.clone()
                    }])
                },
                || {
                    installs += 1;
                    Ok(winget::Output {
                        code: 0,
                        stdout: "Synthetic update".into(),
                        stderr: String::new(),
                    })
                },
            )
            .unwrap();
        assert_eq!(installs, 1);
        assert_eq!(scans, 2);
        assert_eq!(service.catalog.apps.len(), original_count + 1);
        assert_eq!(
            service.catalog.receipts.last().unwrap()["status"],
            "verified"
        );
    }
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
        assert!(validate_app(&p.kind, &p.apps[0], &s.catalog).is_err());
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
        assert_ne!(
            s.catalog.receipts[0]["batchId"],
            s.catalog.receipts[1]["batchId"]
        );
    }
    fn batch_fixture() -> (Service, Plan) {
        let s = Service::open(PathBuf::new(), true).unwrap();
        let ids = s
            .catalog
            .apps
            .values()
            .filter(|a| eligible(a))
            .map(|a| a.uid.clone())
            .collect::<Vec<_>>();
        let plan = prepare(&s.catalog, "update", &ids).unwrap();
        assert_eq!(plan.apps.len(), 3);
        (s, plan)
    }
    fn simulated_update(s: &mut Service, _: &str, before: &App) -> Result<winget::Output> {
        let a = s.catalog.apps.get_mut(&before.uid).unwrap();
        a.version = before.available.clone();
        a.available.clear();
        Ok(winget::Output {
            code: 0,
            stdout: "Synthetic installer completed".into(),
            stderr: String::new(),
        })
    }
    #[test]
    fn batch_skips_stale_target_and_updates_unaffected_apps() {
        let (mut s, p) = batch_fixture();
        let changed = p.apps[1].uid.clone();
        s.catalog.apps.get_mut(&changed).unwrap().available = "99.0".into();
        let mut invoked = vec![];
        s.execute_with(
            p,
            |_, _| {},
            |_| Ok(true),
            |s, kind, a| {
                invoked.push(a.uid.clone());
                simulated_update(s, kind, a)
            },
        )
        .unwrap();
        assert_eq!(invoked.len(), 2);
        assert!(!invoked.contains(&changed));
        assert_eq!(s.catalog.apps[&changed].available, "99.0");
        assert_eq!(s.catalog.receipts[1]["status"], "skipped");
        assert_eq!(s.catalog.extra["lastBatch"]["status"], "partial");
        assert!(
            s.catalog.extra["lastBatch"]["summary"]
                .as_str()
                .unwrap()
                .contains("2 verified, 1 skipped")
        );
    }
    #[test]
    fn earlier_installer_changes_later_app_so_it_is_rechecked() {
        let (mut s, p) = batch_fixture();
        let changed = p.apps[1].uid.clone();
        let mut count = 0;
        s.execute_with(
            p,
            |_, _| {},
            |_| Ok(true),
            |s, kind, a| {
                count += 1;
                let result = simulated_update(s, kind, a);
                s.catalog.apps.get_mut(&changed).unwrap().version = "99.0".into();
                result
            },
        )
        .unwrap();
        assert_eq!(count, 2);
        assert_eq!(s.catalog.receipts[1]["status"], "skipped");
    }
    #[test]
    fn duplicate_registrations_are_unverified_but_independent_apps_continue() {
        let (mut s, p) = batch_fixture();
        let mut count = 0;
        let error = s
            .execute_with(
                p,
                |_, _| {},
                |_| Ok(true),
                |s, kind, a| {
                    count += 1;
                    let result = simulated_update(s, kind, a);
                    let mut duplicate = a.clone();
                    duplicate.uid = "second-registration".into();
                    duplicate.ambiguous = true;
                    s.catalog.apps.get_mut(&a.uid).unwrap().ambiguous = true;
                    s.catalog.apps.insert(duplicate.uid.clone(), duplicate);
                    result
                },
            )
            .unwrap_err();
        assert_eq!(count, 3);
        assert!(error.contains("0 not run"));
        assert!(
            s.catalog.receipts[0]["detail"]
                .as_str()
                .unwrap()
                .contains("Multiple installed registrations")
        );
        assert_eq!(s.catalog.receipts[0]["status"], "unverified");
        assert_eq!(
            s.catalog.receipts[0]["observedVersions"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(s.catalog.extra["lastBatch"]["status"], "attention");
    }
    #[test]
    fn failed_preflight_is_persisted_without_starting_an_installer() {
        let (mut s, p) = batch_fixture();
        let directory = std::env::temp_dir().join(format!(
            "apptide-batch-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        s.directory = directory.clone();
        s.demo = false;
        let error = s
            .execute_with(
                p,
                |_, _| {},
                |_| Err("Synthetic source unavailable".into()),
                |_, _, _| panic!("Installer must not start"),
            )
            .unwrap_err();
        assert!(error.contains("3 not run"));
        assert!(s.catalog.receipts.is_empty());
        let persisted = Catalog::load(&directory.join("catalog.json")).unwrap();
        assert_eq!(
            persisted.extra["lastBatch"]["detail"],
            "Synthetic source unavailable"
        );
        let file = directory.join("batches").join(format!(
            "{}.json",
            persisted.extra["lastBatch"]["id"].as_str().unwrap()
        ));
        let disk: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        assert_eq!(disk["status"], "attention");
        fs::remove_file(file).unwrap();
        fs::remove_dir(directory.join("batches")).unwrap();
        fs::remove_file(directory.join("catalog.json")).unwrap();
        fs::remove_file(directory.join("catalog.json.bak")).unwrap();
        fs::remove_dir(directory).unwrap();
    }
    #[test]
    fn cancelled_batch_explains_what_did_not_run() {
        let (mut s, p) = batch_fixture();
        let cancel = s.cancel.clone();
        s.execute_with(
            p,
            |_, _| {
                cancel.store(true, Ordering::Relaxed);
            },
            |_| Ok(true),
            |_, _, _| panic!("Cancelled"),
        )
        .unwrap();
        assert!(s.catalog.receipts.is_empty());
        assert_eq!(s.catalog.extra["lastBatch"]["status"], "cancelled");
        assert!(
            s.catalog.extra["lastBatch"]["summary"]
                .as_str()
                .unwrap()
                .contains("3 not run")
        );
    }
    #[test]
    fn download_404_at_fourth_item_does_not_abandon_thirty_remaining_updates() {
        let (mut s, p) = batch_fixture();
        let template = p.apps[0].clone();
        let mut ids = Vec::new();
        s.catalog.apps.clear();
        for index in 0..34 {
            let uid = format!("fixture-{index:02}");
            let app = App {
                uid: uid.clone(),
                key: format!("test:{index}"),
                name: format!("Example app {index}"),
                winget_id: Some(format!("Example.App{index}")),
                ..template.clone()
            };
            s.catalog.apps.insert(uid.clone(), app);
            ids.push(uid);
        }
        let plan = prepare(&s.catalog, "update", &ids).unwrap();
        let mut attempted = Vec::new();
        let error = s
            .execute_with(
                plan,
                |_, _| {},
                |_| Ok(true),
                |s, kind, a| {
                    attempted.push(a.uid.clone());
                    if attempted.len() == 4 {
                        Ok(winget::Output {
                            code: 0x80190194u32 as i32,
                            stdout: "Download request status is not success. HTTP 404".into(),
                            stderr: String::new(),
                        })
                    } else {
                        simulated_update(s, kind, a)
                    }
                },
            )
            .unwrap_err();
        assert_eq!(attempted, ids);
        assert!(error.contains("33 verified, 0 skipped, 1 need review, 0 not run"));
        assert_eq!(s.catalog.receipts[3]["status"], "failed");
        assert!(
            s.catalog.receipts[3]["detail"]
                .as_str()
                .unwrap()
                .contains("HTTP 404")
        );
        assert_eq!(s.catalog.receipts[33]["status"], "verified");
        assert_eq!(s.catalog.extra["lastBatch"]["status"], "attention");
    }
    #[test]
    fn verification_scan_failure_still_stops_remaining_installers() {
        let (mut s, p) = batch_fixture();
        let mut scans = 0;
        let mut installs = 0;
        let error = s
            .execute_with(
                p,
                |_, _| {},
                |_| {
                    scans += 1;
                    Ok(scans == 1)
                },
                |s, k, a| {
                    installs += 1;
                    simulated_update(s, k, a)
                },
            )
            .unwrap_err();
        assert_eq!(installs, 1);
        assert!(error.contains("2 not run"));
        assert_eq!(s.catalog.receipts[0]["status"], "unverified");
    }
    #[test]
    fn reboot_and_user_cancel_stop_even_if_new_version_is_visible() {
        for code in [3010, 0x8A150109u32 as i32, 0x8A15010Cu32 as i32] {
            let (mut s, p) = batch_fixture();
            let mut installs = 0;
            let error = s
                .execute_with(
                    p,
                    |_, _| {},
                    |_| Ok(true),
                    |s, k, a| {
                        installs += 1;
                        let mut result = simulated_update(s, k, a)?;
                        result.code = code;
                        Ok(result)
                    },
                )
                .unwrap_err();
            assert_eq!(installs, 1);
            assert!(error.contains("2 not run"));
        }
    }
    #[test]
    fn failed_installers_preserve_exit_codes_and_attempt_remaining_apps() {
        let (mut s, p) = batch_fixture();
        let error = s
            .execute_with(
                p,
                |_, _| {},
                |_| Ok(true),
                |_, _, _| {
                    Ok(winget::Output {
                        code: 1603,
                        stdout: String::new(),
                        stderr: "Synthetic failure".into(),
                    })
                },
            )
            .unwrap_err();
        assert!(error.contains("3 need review, 0 not run"));
        assert_eq!(s.catalog.receipts.len(), 3);
        assert_eq!(s.catalog.receipts[0]["status"], "failed");
        assert_eq!(s.catalog.receipts[0]["exitCode"], 1603);
    }
    #[test]
    fn corrupt_catalogue_is_not_reset() {
        let p = std::env::temp_dir().join(format!("apptide-corrupt-{}.json", std::process::id()));
        fs::write(&p, b"invalid").unwrap();
        assert!(Catalog::load(&p).is_err());
        assert_eq!(fs::read(&p).unwrap(), b"invalid");
        fs::remove_file(p).unwrap();
    }
    #[test]
    fn automatic_requires_fresh_dual_consent_and_blocks_unverified_retry() {
        let mut s = Service::open(PathBuf::new(), true).unwrap();
        let id = s
            .catalog
            .apps
            .values()
            .find(|a| eligible(a))
            .unwrap()
            .uid
            .clone();
        assert!(!automatic_due(&s.catalog));
        assert!(automatic_candidates(&s.catalog).is_empty());
        s.catalog.settings["automatic"] = true.into();
        s.catalog.apps.get_mut(&id).unwrap().auto_update = true;
        assert!(!automatic_due(&s.catalog));
        assert!(automatic_candidates(&s.catalog).is_empty());
        s.automatic_preference(Some(&id), true).unwrap();
        s.automatic_preference(None, true).unwrap();
        assert!(!automatic_due(&s.catalog));
        s.catalog.settings["lastAutomaticAttempt"] = (chrono::Utc::now()
            - chrono::Duration::hours(2))
        .to_rfc3339()
        .into();
        assert!(automatic_due(&s.catalog));
        assert_eq!(automatic_candidates(&s.catalog), vec![id.clone()]);
        let package = s.catalog.apps[&id].winget_id.clone();
        s.catalog
            .receipts
            .push(json!({"packageId":package,"status":"unverified"}));
        assert!(automatic_candidates(&s.catalog).is_empty());
        s.catalog
            .receipts
            .push(json!({"packageId":package,"status":"skipped"}));
        assert!(automatic_candidates(&s.catalog).is_empty());
        s.catalog
            .receipts
            .push(json!({"packageId":package,"status":"verified"}));
        s.catalog
            .receipts
            .push(json!({"packageId":package,"status":"skipped"}));
        assert_eq!(automatic_candidates(&s.catalog), vec![id.clone()]);
        s.catalog.apps.get_mut(&id).unwrap().excluded = true;
        assert!(automatic_candidates(&s.catalog).is_empty());
    }
    #[test]
    fn manual_catalogue_entries_keep_identity_and_versions_across_scans() {
        let mut s = Service::open(PathBuf::new(), true).unwrap();
        let source = crate::custom::Source {
            url: "https://example.org/downloads".into(),
            manual_version: "1.0".into(),
            ..Default::default()
        };
        s.custom_source(None, "Example utility", source.clone())
            .unwrap();
        let a = s
            .catalog
            .apps
            .values()
            .find(|a| a.kind == "manual")
            .unwrap()
            .clone();
        let native = winget::reconcile(vec![a.clone()], vec![]);
        assert_eq!(native[0].uid, a.uid);
        s.catalog.observe(native);
        assert!(s.catalog.apps[&a.uid].present);
        assert_eq!(s.catalog.apps[&a.uid].versions.len(), 1);
        s.custom_source(
            Some(&a.uid),
            "Renamed utility",
            crate::custom::Source {
                manual_version: "2.0".into(),
                ..source
            },
        )
        .unwrap();
        assert_eq!(s.catalog.apps[&a.uid].name, "Renamed utility");
        assert_eq!(s.catalog.apps[&a.uid].versions.len(), 2);
        assert!(!actionable(&s.catalog.apps[&a.uid]));
        let count = s.catalog.apps.len();
        assert!(
            s.custom_source(
                None,
                "Invalid",
                crate::custom::Source {
                    url: "file:///bad".into(),
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert_eq!(s.catalog.apps.len(), count);
    }
}
