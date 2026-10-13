use crate::catalog::{App, Result};
use std::time::Duration;

#[cfg(windows)]
pub fn read() -> Result<Vec<App>> {
    use winreg::{RegKey, enums::*};
    let mut apps = vec![];
    let mut hints = vec![];
    for (hive, label, scope) in [
        (HKEY_LOCAL_MACHINE, "LocalMachine", "machine"),
        (HKEY_CURRENT_USER, "CurrentUser", "user"),
    ] {
        for (view, label_view) in [
            (KEY_WOW64_64KEY, "Registry64"),
            (KEY_WOW64_32KEY, "Registry32"),
        ] {
            let root = RegKey::predef(hive);
            let uninstall = match root.open_subkey_with_flags(
                r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
                KEY_READ | view,
            ) {
                Ok(k) => k,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(format!("Incomplete registry inventory: {e}")),
            };
            for key in uninstall.enum_keys() {
                let key = key.map_err(|e| e.to_string())?;
                let app = uninstall
                    .open_subkey_with_flags(&key, KEY_READ)
                    .map_err(|e| format!("Registry changed during scan: {e}"))?;
                let get = |name: &str| app.get_value::<String, _>(name).unwrap_or_default();
                let name = get("DisplayName");
                if name.is_empty() {
                    continue;
                }
                let key = format!("registry:{label}:{label_view}:{key}");
                if get("DisplayVersion").trim().is_empty() || get("Publisher").trim().is_empty() {
                    hints.push(
                        serde_json::json!({"key":key,"name":name,"displayIcon":get("DisplayIcon"),"location":get("InstallLocation")}),
                    );
                }
                apps.push(App {
                    key,
                    name,
                    version: get("DisplayVersion"),
                    publisher: get("Publisher"),
                    location: get("InstallLocation"),
                    scope: scope.into(),
                    kind: "registry".into(),
                    protected: app.get_value::<u32, _>("SystemComponent").unwrap_or(0) == 1
                        || app.get_value::<u32, _>("NoRemove").unwrap_or(0) == 1,
                    extra: [("metadataSources".into(), serde_json::json!({"name":"Registry DisplayName","publisher":if get("Publisher").trim().is_empty(){"Not reported"}else{"Registry Publisher"},"version":if get("DisplayVersion").trim().is_empty(){"Not reported"}else{"Registry DisplayVersion"}})),
                        ("urlInfoAbout".into(), get("URLInfoAbout").into()), ("urlUpdateInfo".into(), get("URLUpdateInfo").into()), ("installDate".into(), get("InstallDate").into())].into(),
                    ..Default::default()
                });
            }
        }
    }
    // Data is passed as base64 JSON to a fixed read-only script, never interpolated as code.
    use base64::Engine;
    let encoded =
        base64::engine::general_purpose::STANDARD.encode(serde_json::to_vec(&hints).unwrap());
    let script = format!(
        "& {{ {} }} -RegistryJson ([Console]::In.ReadToEnd())",
        include_str!("../scripts/inventory-metadata.ps1")
    );
    let out = powershell_input(&script, 120, Some(encoded.into_bytes()))?;
    if out.code != 0 {
        return Err("AppX inventory failed; previous catalogue preserved.".into());
    }
    let report: serde_json::Value =
        serde_json::from_str(out.stdout.trim().trim_start_matches('\u{feff}'))
            .map_err(|e| format!("AppX inventory unreadable: {e}"))?;
    let appx: Vec<App> =
        serde_json::from_value(report["apps"].clone()).map_err(|e| e.to_string())?;
    for metadata in report["metadata"].as_array().into_iter().flatten() {
        if let Some(app) = apps
            .iter_mut()
            .find(|a| Some(a.key.as_str()) == metadata["key"].as_str())
        {
            apply_metadata(app, metadata);
        }
    }
    apps.extend(appx);
    Ok(deduplicate_mirrors(apps))
}
pub fn apply_metadata(app: &mut App, metadata: &serde_json::Value) {
    for field in ["publisher", "version"] {
        let current = if field == "publisher" {
            &mut app.publisher
        } else {
            &mut app.version
        };
        let value = metadata[field].as_str().unwrap_or("").trim();
        if current.trim().is_empty() && !value.is_empty() {
            *current = value.into();
            app.extra
                .entry("metadataSources".into())
                .or_insert_with(|| serde_json::json!({}))[field] = if field == "publisher" {
                "Executable CompanyName"
            } else {
                metadata["versionSource"]
                    .as_str()
                    .unwrap_or("Executable ProductVersion")
            }
            .into();
            app.extra
                .insert("metadataFile".into(), metadata["metadataFile"].clone());
        }
    }
}
#[cfg(not(windows))]
pub fn read() -> Result<Vec<App>> {
    Err("Live inventory requires Windows".into())
}

pub fn powershell(script: &str, timeout: u64) -> Result<crate::winget::Output> {
    powershell_input(script, timeout, None)
}
fn powershell_input(
    script: &str,
    timeout: u64,
    input: Option<Vec<u8>>,
) -> Result<crate::winget::Output> {
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(
        script
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>(),
    );
    let program =
        std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap_or("C:\\Windows".into()))
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    crate::winget::run_with_input(
        &program.to_string_lossy(),
        &[
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-EncodedCommand".into(),
            encoded,
        ],
        Some(Duration::from_secs(timeout)),
        input,
    )
}
pub fn deduplicate_mirrors(mut apps: Vec<App>) -> Vec<App> {
    // HKCU uninstall registrations can be shared across WOW64 views. Never merge different keys,
    // installations, architectures, publishers, locations or protection flags by name alone.
    apps.sort_by_key(|a| a.key.contains(":Registry32:"));
    let mut result: Vec<App> = Vec::new();
    for a in apps {
        let mirror = a
            .key
            .strip_prefix("registry:CurrentUser:Registry32:")
            .map(|key| format!("registry:CurrentUser:Registry64:{key}"));
        if mirror.as_ref().is_some_and(|key| {
            result.iter().any(|old| {
                old.key == *key
                    && old.name == a.name
                    && old.version == a.version
                    && old.publisher == a.publisher
                    && old.location == a.location
                    && old.protected == a.protected
            })
        }) {
            continue;
        }
        result.push(a);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn metadata_script_processes_multiple_records_without_running_executables() {
        let script = format!(
            r#"$p=Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'; $v=[Diagnostics.FileVersionInfo]::GetVersionInfo($p); $rows=@(@{{key='first';name=$v.ProductName;displayIcon=$p}},@{{key='second';name=$v.ProductName;displayIcon=$p}},@{{key='wrong';name='Unrelated synthetic product';displayIcon=$p}}); $data=[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes((ConvertTo-Json -InputObject $rows))); & {{ {} }} -RegistryJson $data -MetadataOnly"#,
            include_str!("../scripts/inventory-metadata.ps1")
        );
        let out = powershell(&script, 20).unwrap();
        assert_eq!(out.code, 0, "{}", out.stderr);
        let value: serde_json::Value = serde_json::from_str(out.stdout.trim()).unwrap();
        let rows = value["metadata"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["key"], "first");
        assert_eq!(rows[1]["key"], "second");
        assert!(
            rows.iter()
                .all(|r| !r["version"].as_str().unwrap().is_empty())
        );
    }
    #[cfg(windows)]
    #[test]
    fn large_inventory_payload_uses_stdin_without_command_line_limits() {
        let out = powershell_input(
            "[Console]::Write([Console]::In.ReadToEnd().Length)",
            20,
            Some(vec![b'a'; 100_000]),
        )
        .unwrap();
        assert_eq!(out.code, 0, "{}", out.stderr);
        assert_eq!(out.stdout.trim(), "100000");
    }
    #[test]
    fn metadata_fills_gaps_without_overwriting_registration() {
        let mut app = App {
            publisher: "Registered vendor".into(),
            ..Default::default()
        };
        apply_metadata(
            &mut app,
            &serde_json::json!({"publisher":"File vendor","version":"2.4","metadataFile":"example.exe"}),
        );
        assert_eq!(app.publisher, "Registered vendor");
        assert_eq!(app.version, "2.4");
        assert_eq!(
            app.extra["metadataSources"]["version"],
            "Executable ProductVersion"
        );
        apply_metadata(&mut app, &serde_json::json!({"version":"9.0"}));
        assert_eq!(app.version, "2.4");
    }
    #[test]
    fn deduplicates_only_identical_current_user_mirrors() {
        let a = App {
            key: "registry:CurrentUser:Registry64:Example".into(),
            name: "Example".into(),
            version: "1".into(),
            ..Default::default()
        };
        let b = App {
            key: a.key.replace("64", "32"),
            ..a.clone()
        };
        assert_eq!(deduplicate_mirrors(vec![b.clone(), a.clone()]).len(), 1);
        assert_eq!(
            deduplicate_mirrors(vec![
                App {
                    version: "2".into(),
                    ..b.clone()
                },
                a.clone()
            ])
            .len(),
            2
        );
        assert_eq!(
            deduplicate_mirrors(vec![
                App {
                    key: b.key.replace("CurrentUser", "LocalMachine"),
                    ..b
                },
                a
            ])
            .len(),
            2
        );
    }
}
