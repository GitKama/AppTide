//! Release assets are data, never commands. Only reviewed recipes get silent switches.
use crate::catalog::{App, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    pub repository: String,
    pub tag: String,
    pub version: String,
    pub id: u64,
    pub name: String,
    pub url: String,
    pub hash: String,
    pub size: u64,
    pub mode: String,
}
pub fn candidates(a: &App) -> Vec<Offer> {
    let Some(w) = a.extra.get("watch") else {
        return vec![];
    };
    let Some(repo) = crate::releases::repository(a) else {
        return vec![];
    };
    let tag = w["latest"].as_str().unwrap_or("");
    if !crate::releases::valid_repository(repo) || tag.is_empty() {
        return vec![];
    }
    let mut found = vec![];
    for asset in w["assets"].as_array().into_iter().flatten() {
        let name = asset["name"].as_str().unwrap_or("");
        let lower = name.to_ascii_lowercase();
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            || ["arm", "aarch", "x86", "win32", "i686", "portable"]
                .iter()
                .any(|s| lower.contains(s))
        {
            continue;
        }
        let mut version = tag.trim_start_matches(['v', 'V']).to_string();
        let mut mode = if lower.ends_with(".msi") {
            "msi"
        } else if lower.ends_with(".exe") && (lower.contains("setup") || lower.contains("install"))
        {
            "interactive"
        } else {
            continue;
        };
        // Name + publisher recipe recognition is required, not just a user-entered repository.
        let reviewed = a.scope == "user"
            && a.knowledge
                .as_ref()
                .is_some_and(|k| k.repository.eq_ignore_ascii_case(repo));
        if reviewed && repo.eq_ignore_ascii_case("Zackriya-Solutions/meetily") {
            if name != format!("meetily_{version}_x64-setup.exe") {
                continue;
            }
            mode = "nsis";
        } else if reviewed && repo.eq_ignore_ascii_case("FrankBijnen/ExifToolGui") {
            let Some(v) = name
                .strip_prefix("ExifToolGUI_install_")
                .and_then(|s| s.strip_suffix(".exe"))
            else {
                continue;
            };
            version = v.into();
            mode = "inno";
        }
        if crate::releases::compare(&version, &version) != "Matches release" {
            continue;
        }
        let url = asset["browser_download_url"].as_str().unwrap_or("");
        let prefix = format!("https://github.com/{repo}/releases/download/");
        let Some(rest) = url.strip_prefix(&prefix) else {
            continue;
        };
        // Exactly a tag and filename; disallow query strings, credentials, traversal and fragments.
        let Some((url_tag, file)) = rest.split_once('/') else {
            continue;
        };
        if url_tag != tag
            || file != name
            || tag.contains(['/', '?', '#', '\\'])
            || tag == "."
            || tag == ".."
        {
            continue;
        }
        let hash = asset["digest"]
            .as_str()
            .unwrap_or("")
            .strip_prefix("sha256:")
            .unwrap_or("");
        let size = asset["size"].as_u64().unwrap_or(0);
        let id = asset["id"].as_u64().unwrap_or(0);
        if hash.len() != 64
            || !hash.bytes().all(|b| b.is_ascii_hexdigit())
            || size == 0
            || size > 512 * 1024 * 1024
            || id == 0
            || asset["state"] != "uploaded"
        {
            continue;
        }
        found.push(Offer {
            repository: repo.into(),
            tag: tag.into(),
            version,
            id,
            name: name.into(),
            url: url.into(),
            hash: hash.to_ascii_lowercase(),
            size,
            mode: mode.into(),
        });
    }
    // ExifToolGUI publishes multiple installer revisions under a single release tag.
    if found.iter().any(|o| o.mode == "inno") {
        found.sort_by(
            |a, b| match crate::releases::compare(&a.version, &b.version) {
                "Newer release" => std::cmp::Ordering::Less,
                "Installed version ahead" => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            },
        );
        return found.pop().into_iter().collect();
    }
    found
}
pub fn offer(a: &App) -> Option<Offer> {
    if !a.present || a.kind != "registry" || a.protected || a.ambiguous || a.winget_id.is_some() {
        return None;
    }
    let mut offers = candidates(a);
    if let Some(id) = a.extra.get("githubAssetId").and_then(Value::as_u64) {
        return offers.into_iter().find(|o| o.id == id);
    }
    if offers.len() == 1 {
        offers.pop()
    } else {
        None
    }
}
pub fn ready(a: &App) -> bool {
    !a.excluded
        && offer(a).is_some_and(|o| {
            crate::releases::compare(&a.version, &o.version) == "Newer release"
                && a.available == o.version
        })
}
pub fn consent_key(a: &App) -> Option<String> {
    let o = offer(a)?;
    if !matches!(o.mode.as_str(), "nsis" | "inno") {
        return None;
    }
    Some(format!(
        "{}|{}|{}|{}",
        o.repository, o.mode, a.key, a.publisher
    ))
}
pub fn automatic_allowed(a: &App) -> bool {
    consent_key(a).is_some_and(|key| {
        a.extra
            .get("githubAutomaticConsent")
            .and_then(Value::as_str)
            == Some(key.as_str())
    })
}
pub fn synchronize(a: &mut App) {
    if a.winget_id.is_none() {
        a.available = offer(a)
            .filter(|o| crate::releases::compare(&a.version, &o.version) == "Newer release")
            .map(|o| o.version)
            .unwrap_or_default();
    }
}
pub fn same_installation(a: &App, b: &App) -> bool {
    if let Some(id) = &b.winget_id {
        a.winget_id.as_ref() == Some(id) || a.key == b.key
    } else {
        a.key == b.key && a.publisher == b.publisher && a.kind == b.kind
    }
}
pub fn select(a: &mut App, id: u64) -> Result<()> {
    if !candidates(a).iter().any(|o| o.id == id) {
        return Err("Installer is no longer available. Refresh the release.".into());
    }
    a.extra.insert("githubAssetId".into(), id.into());
    a.extra.remove("githubAutomaticConsent");
    a.auto_update = false;
    synchronize(a);
    Ok(())
}
pub fn download(o: &Offer, root: &Path) -> Result<PathBuf> {
    let stage = root.join(format!(
        "github-{}-{}",
        o.id,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    std::fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
    let file = stage.join(&o.name);
    use base64::Engine;
    let config = base64::engine::general_purpose::STANDARD.encode(
        serde_json::to_vec(
            &serde_json::json!({"url":o.url,"hash":o.hash,"size":o.size,"file":file}),
        )
        .unwrap(),
    );
    let script = include_str!("../scripts/github-download.ps1").replace("__CONFIG__", &config);
    let out = crate::inventory::powershell(&script, 600)?;
    std::fs::write(
        stage.join("download.log"),
        format!("{}\n{}", out.stdout, out.stderr),
    )
    .map_err(|e| e.to_string())?;
    if out.code != 0 {
        return Err(format!(
            "GitHub download failed; no installer started. Details: {}",
            stage.join("download.log").display()
        ));
    }
    // Validate again in Rust immediately before handing the file to the installer launcher.
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(&file).map_err(|e| e.to_string())?;
    if bytes.len() as u64 != o.size || format!("{:x}", Sha256::digest(&bytes)) != o.hash {
        return Err("Installer integrity check failed; no installer started.".into());
    }
    Ok(file)
}
pub fn install(a: &App, root: &Path) -> Result<crate::winget::Output> {
    let expected = offer(a).ok_or("No compatible GitHub installer selected")?;
    let fresh = crate::releases::fetch(&expected.repository)?;
    let mut checked = a.clone();
    crate::releases::apply(&mut checked, &fresh);
    if offer(&checked).as_ref() != Some(&expected) {
        return Err("GitHub release changed since preview. Refresh and review the new installer; nothing installed.".into());
    }
    let file = download(&expected, root)?;
    launch_installer(&file, &expected.mode)
}
fn launch_installer(file: &Path, mode: &str) -> Result<crate::winget::Output> {
    use base64::Engine;
    let cfg = base64::engine::general_purpose::STANDARD
        .encode(serde_json::to_vec(&serde_json::json!({"file":file,"mode":mode})).unwrap());
    let script = format!(
        r#"$ErrorActionPreference='Stop';$c=ConvertFrom-Json ([Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('{cfg}')));$exe=$c.file;$a=@();switch($c.mode){{'nsis'{{$a=@('/S')}}'inno'{{$a=@('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/RESTARTEXITCODE=3010','/NOCLOSEAPPLICATIONS','/SP-')}}'msi'{{$exe=Join-Path $env:SystemRoot 'System32\msiexec.exe';$a=@('/i',('"'+$c.file+'"'),'/norestart')}}}};try{{if($a.Count){{$p=Start-Process -FilePath $exe -ArgumentList $a -Wait -PassThru}}else{{$p=Start-Process -FilePath $exe -Wait -PassThru}};[Console]::WriteLine('Installer exit code: '+$p.ExitCode);exit $p.ExitCode}}catch{{$e=$_.Exception;while($e){{if($e -is [ComponentModel.Win32Exception] -and $e.NativeErrorCode -eq 1223){{exit 1602}};$e=$e.InnerException}};[Console]::Error.WriteLine($_.Exception.Message);exit 1}}"#
    );
    // Do not time out and orphan a running installer. The queue waits for its completion.
    use base64::engine::general_purpose::STANDARD;
    let encoded = STANDARD.encode(
        script
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>(),
    );
    let program = PathBuf::from(std::env::var_os("SystemRoot").unwrap_or("C:\\Windows".into()))
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    crate::winget::run(
        &program.to_string_lossy(),
        &[
            "-NoLogo".into(),
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-EncodedCommand".into(),
            encoded,
        ],
        None,
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn app() -> App {
        let mut a = App {
            present: true,
            scope: "user".into(),
            kind: "registry".into(),
            key: "one".into(),
            uid: "github-fixture".into(),
            version: "1.0".into(),
            ..Default::default()
        };
        a.extra.insert("watch".into(), serde_json::json!({"repository":"example/app","association":"user-selected","latest":"v2.0","assets":[{"id":1,"name":"app-setup-x64.exe","browser_download_url":"https://github.com/example/app/releases/download/v2.0/app-setup-x64.exe","digest":format!("sha256:{}","a".repeat(64)),"size":123,"state":"uploaded"}]}));
        a
    }
    #[test]
    #[ignore = "Runs only an explicitly supplied synthetic GUI fixture"]
    fn synthetic_installer_launcher_preserves_exit_code() {
        let path = std::env::var_os("APPTIDE_INSTALLER_TEST_EXE")
            .map(PathBuf::from)
            .expect("Set synthetic fixture path");
        for mode in ["interactive", "nsis", "inno"] {
            let out = launch_installer(&path, mode).unwrap();
            assert_eq!(out.code, 17, "{mode}: {}", out.stderr);
        }
    }
    #[test]
    fn asset_selection_and_consent_follow_identity() {
        let mut a = app();
        let mut other = a.extra["watch"]["assets"][0].clone();
        other["id"] = 2.into();
        other["name"] = "app-setup-x64.msi".into();
        other["browser_download_url"] =
            "https://github.com/example/app/releases/download/v2.0/app-setup-x64.msi".into();
        a.extra.get_mut("watch").unwrap()["assets"]
            .as_array_mut()
            .unwrap()
            .push(other);
        synchronize(&mut a);
        assert!(!ready(&a));
        select(&mut a, 2).unwrap();
        assert!(ready(&a));
        assert_eq!(offer(&a).unwrap().mode, "msi");
        assert!(select(&mut a, 99).is_err());
        a.extra.get_mut("watch").unwrap()["assets"][1]["id"] = 3.into();
        assert!(!ready(&a));
    }
    #[test]
    fn reviewed_exif_revision_uses_installer_version_and_normalizes_installed_suffix() {
        let mut a = app();
        a.version = "6.3.12.1_64".into();
        a.name = "ExifToolGUI".into();
        a.publisher = "FrankB".into();
        a.knowledge = crate::knowledge::detect(&a);
        let assets:Vec<_>=["6.3.12.0","6.3.12.1"].iter().enumerate().map(|(i,v)|serde_json::json!({"id":i+1,"name":format!("ExifToolGUI_install_{v}.exe"),"browser_download_url":format!("https://github.com/FrankBijnen/ExifToolGui/releases/download/V6.3.12/ExifToolGUI_install_{v}.exe"),"digest":format!("sha256:{}","a".repeat(64)),"size":123,"state":"uploaded"})).collect();
        crate::releases::apply(
            &mut a,
            &serde_json::json!({"repository":"FrankBijnen/ExifToolGui","latest":"V6.3.12","assets":assets}),
        );
        assert_eq!(offer(&a).unwrap().version, "6.3.12.1");
        assert_eq!(a.extra["watch"]["status"], "Matches release");
        assert!(!ready(&a));
        a.version = "6.3.12.0_64".into();
        synchronize(&mut a);
        assert!(ready(&a));
        let key = consent_key(&a).unwrap();
        a.extra.insert("githubAutomaticConsent".into(), key.into());
        assert!(automatic_allowed(&a));
        a.publisher = "Other".into();
        assert!(!automatic_allowed(&a));
    }
    #[test]
    #[ignore = "Downloads a public release installer without running it"]
    fn live_meetily_download_only() {
        let mut a = App {
            present: true,
            scope: "user".into(),
            kind: "registry".into(),
            name: "meetily".into(),
            publisher: "meetily".into(),
            version: "0.4.0".into(),
            ..Default::default()
        };
        a.knowledge = crate::knowledge::detect(&a);
        let release = crate::releases::fetch("Zackriya-Solutions/meetily").unwrap();
        crate::releases::apply(&mut a, &release);
        let o = offer(&a).unwrap();
        assert_eq!(o.mode, "nsis");
        assert!(ready(&a));
        let root = std::env::var_os("APPTIDE_DOWNLOAD_TEST_DIR")
            .map(PathBuf::from)
            .expect("Set private output folder");
        let file = download(&o, &root).unwrap();
        assert!(file.is_file());
        println!("Verified download only: {}", file.display());
    }
    #[test]
    fn untrusted_download_host_cannot_return_an_installer() {
        // Invalid remote address must fail before any execution path is reachable.
        let mut o = offer(&app()).unwrap();
        o.url = "https://invalid.example/setup.exe".into();
        let root =
            std::env::temp_dir().join(format!("apptide-download-denied-{}", std::process::id()));
        assert!(download(&o, &root).is_err());
    }
    #[test]
    fn generic_installer_is_manual_and_requires_matching_version() {
        let mut a = app();
        synchronize(&mut a);
        assert!(ready(&a));
        assert!(consent_key(&a).is_none());
        a.version = "2.0".into();
        synchronize(&mut a);
        assert!(!ready(&a));
    }
    #[test]
    fn reject_untrusted_or_ambiguous_assets() {
        for (key, v) in [
            (
                "browser_download_url",
                Value::from("https://evil.test/setup.exe"),
            ),
            ("digest", Value::from("")),
            ("size", Value::from(0)),
            ("name", Value::from("app-arm64-setup.exe")),
        ] {
            let mut a = app();
            a.extra.get_mut("watch").unwrap()["assets"][0][key] = v;
            assert!(offer(&a).is_none());
        }
        let mut a = app();
        let v = a.extra["watch"]["assets"][0].clone();
        a.extra.get_mut("watch").unwrap()["assets"]
            .as_array_mut()
            .unwrap()
            .push(v);
        assert!(offer(&a).is_none());
    }
    #[test]
    fn no_null_identity_match_and_winget_remains_preferred() {
        let mut a = app();
        let mut b = a.clone();
        b.key = "other".into();
        assert!(!same_installation(&a, &b));
        a.winget_id = Some("Example.App".into());
        assert!(offer(&a).is_none());
    }
}
