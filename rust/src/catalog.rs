use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
pub type Result<T> = std::result::Result<T, String>;
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
pub fn digest(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct App {
    pub uid: String,
    pub key: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub scope: String,
    pub kind: String,
    pub location: String,
    pub protected: bool,
    pub winget_id: Option<String>,
    pub available: String,
    pub evidence: String,
    pub ambiguous: bool,
    pub present: bool,
    pub first_seen: String,
    pub last_seen: String,
    pub observations: u64,
    pub versions: Vec<Version>,
    pub excluded: bool,
    pub auto_update: bool,
    pub log_mentions: usize,
    pub knowledge: Option<crate::knowledge::Knowledge>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Version {
    pub version: String,
    pub seen_at: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub schema: u32,
    pub scans: u64,
    pub apps: BTreeMap<String, App>,
    pub receipts: Vec<Value>,
    #[serde(default)]
    pub log_signals: Vec<Value>,
    #[serde(default)]
    pub last_scan: Option<String>,
    #[serde(default)]
    pub settings: Value,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
impl Default for Catalog {
    fn default() -> Self {
        Self {
            schema: 1,
            scans: 0,
            apps: BTreeMap::new(),
            receipts: vec![],
            log_signals: vec![],
            last_scan: None,
            settings: serde_json::json!({"automatic":false}),
            extra: BTreeMap::new(),
        }
    }
}
impl Catalog {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = fs::read(path).map_err(|e| e.to_string())?;
        let mut cat: Self = serde_json::from_slice(&data)
            .map_err(|e| format!("Catalogue unreadable; original preserved: {e}"))?;
        if cat.schema != 1 {
            return Err("Unsupported catalogue schema; original preserved.".into());
        }
        for receipt in &mut cat.receipts {
            if receipt["status"] == "running" {
                receipt["status"] = "unverified".into();
                receipt["detail"] =
                    "AppTide stopped before verification. Review before retrying.".into();
            }
        }
        Ok(cat)
    }
    pub fn save(&self, directory: &Path) -> Result<()> {
        fs::create_dir_all(directory).map_err(|e| e.to_string())?;
        let path = directory.join("catalog.json");
        let temp = directory.join("catalog.json.tmp");
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        use std::io::Write;
        let mut file = fs::File::create(&temp).map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        if path.exists() {
            fs::copy(&path, directory.join("catalog.json.bak")).map_err(|e| e.to_string())?;
        }
        replace_file(&temp, &path)
    }
    pub fn observe(&mut self, apps: Vec<App>) {
        let time = now();
        for app in self.apps.values_mut() {
            app.present = false;
        }
        for mut app in apps {
            // A unique native registration can gain a WinGet identity after mirror deduplication.
            let old = self.apps.get(&app.uid).or_else(|| {
                let mut matches = self
                    .apps
                    .values()
                    .filter(|a| a.key == app.key && a.kind == "registry" && !a.ambiguous);
                let first = matches.next();
                if matches.next().is_none() {
                    first
                } else {
                    None
                }
            });
            if let Some(old) = old {
                app.first_seen = old.first_seen.clone();
                app.observations = old.observations;
                app.versions = old.versions.clone();
                app.excluded = old.excluded;
                app.auto_update = old.auto_update;
                let fresh = std::mem::take(&mut app.extra);
                app.extra = old.extra.clone();
                app.extra.extend(fresh);
                if !old.publisher.is_empty()
                    && old.publisher != app.publisher
                    && !app.publisher.is_empty()
                {
                    app.auto_update = false;
                    app.extra.remove("automaticConsent");
                    app.extra.remove("watch");
                    app.extra.remove("customSource");
                    app.extra.insert(
                        "identityNotice".into(),
                        "Publisher changed; release association and automatic consent cleared."
                            .into(),
                    );
                }
            } else {
                app.first_seen = time.clone();
            }
            if !app.version.is_empty() && !app.versions.iter().any(|v| v.version == app.version) {
                app.versions.push(Version {
                    version: app.version.clone(),
                    seen_at: time.clone(),
                });
            }
            if app.versions.len() > 50 {
                app.versions.drain(..app.versions.len() - 50);
            }
            app.present = true;
            app.last_seen = time.clone();
            app.observations += 1;
            self.apps.insert(app.uid.clone(), app);
        }
        self.scans += 1;
        self.last_scan = Some(time);
    }
}
#[cfg(windows)]
fn replace_file(from: &Path, to: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    let a: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let b: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    // Paths are owned, NUL-terminated buffers and stay alive for the call.
    if unsafe {
        MoveFileExW(
            a.as_ptr(),
            b.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        Err(std::io::Error::last_os_error().to_string())
    } else {
        Ok(())
    }
}
#[cfg(not(windows))]
fn replace_file(from: &Path, to: &Path) -> Result<()> {
    fs::rename(from, to).map_err(|e| e.to_string())
}
pub fn data_directory() -> PathBuf {
    PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default()).join("AppTide-Beta")
}
pub fn known_version(s: &str) -> bool {
    !s.is_empty()
        && !s.to_lowercase().starts_with("unknown")
        && !s.starts_with(['<', '>'])
        && !s.contains('…')
}
pub fn actionable(a: &App) -> bool {
    a.present
        && a.winget_id.as_deref().is_some_and(crate::winget::valid_id)
        && !a.protected
        && !a.ambiguous
}
pub fn eligible(a: &App) -> bool {
    actionable(a) && !a.excluded && known_version(&a.version) && known_version(&a.available)
}
pub fn classify(
    kind: &str,
    before: &App,
    after: Option<&App>,
    code: i32,
    complete: bool,
) -> &'static str {
    if !complete {
        return "unverified";
    }
    if kind == "uninstall" && after.is_none() {
        return "verified";
    }
    if kind == "update"
        && after.is_some_and(|a| {
            a.version == before.available && a.version != before.version && !a.ambiguous
        })
    {
        return "verified";
    }
    if code == 0 { "unverified" } else { "failed" }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atomic_save_retains_backup_and_recovers_interrupted_receipts() {
        let dir = std::env::temp_dir().join(format!(
            "apptide-save-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let mut c = Catalog::default();
        c.save(&dir).unwrap();
        c.scans = 7;
        c.receipts.push(serde_json::json!({"status":"running"}));
        c.save(&dir).unwrap();
        let loaded = Catalog::load(&dir.join("catalog.json")).unwrap();
        assert_eq!(loaded.scans, 7);
        assert_eq!(loaded.receipts[0]["status"], "unverified");
        assert_eq!(
            Catalog::load(&dir.join("catalog.json.bak")).unwrap().scans,
            0
        );
        std::fs::remove_file(dir.join("catalog.json")).unwrap();
        std::fs::remove_file(dir.join("catalog.json.bak")).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn preserves_history_preferences_and_extensions() {
        let mut c = Catalog::default();
        let a = App {
            uid: "a".into(),
            version: "1".into(),
            ..Default::default()
        };
        c.observe(vec![a.clone()]);
        c.apps.get_mut("a").unwrap().excluded = true;
        c.apps.get_mut("a").unwrap().extra.insert(
            "watch".into(),
            serde_json::json!({"repository":"example/app"}),
        );
        c.observe(vec![App {
            version: "2".into(),
            ..a
        }]);
        assert!(c.apps["a"].excluded);
        assert_eq!(c.apps["a"].versions.len(), 2);
        assert!(c.apps["a"].extra.contains_key("watch"));
        c.observe(vec![]);
        assert!(!c.apps["a"].present);
    }
    #[test]
    fn success_exit_is_not_verification() {
        let a = App {
            version: "1".into(),
            available: "2".into(),
            ..Default::default()
        };
        assert_eq!(classify("update", &a, Some(&a), 0, true), "unverified");
        assert_eq!(classify("uninstall", &a, None, 0, false), "unverified");
        assert_eq!(
            classify(
                "update",
                &a,
                Some(&App {
                    version: "2".into(),
                    ..a.clone()
                }),
                0,
                true
            ),
            "verified"
        );
    }
    #[test]
    fn native_identity_migration_keeps_history_but_publisher_change_revokes_consent() {
        let mut c = Catalog::default();
        let a = App {
            uid: "native".into(),
            key: "registry:test".into(),
            kind: "registry".into(),
            publisher: "Original".into(),
            version: "1".into(),
            ..Default::default()
        };
        c.observe(vec![a.clone()]);
        let old = c.apps.get_mut("native").unwrap();
        old.auto_update = true;
        old.extra
            .insert("watch".into(), serde_json::json!({"repository":"test/app"}));
        old.extra.insert("signatureKind".into(), "Developer".into());
        let mut new = App {
            uid: "winget".into(),
            version: "2".into(),
            ..a.clone()
        };
        new.extra.insert("signatureKind".into(), "Store".into());
        c.observe(vec![new]);
        assert_eq!(c.apps["winget"].versions.len(), 2);
        assert_eq!(c.apps["winget"].extra["signatureKind"], "Store");
        assert!(c.apps["winget"].auto_update);
        c.observe(vec![App {
            uid: "winget".into(),
            publisher: "Different".into(),
            ..a
        }]);
        assert!(!c.apps["winget"].auto_update);
        assert!(!c.apps["winget"].extra.contains_key("watch"));
    }
}
