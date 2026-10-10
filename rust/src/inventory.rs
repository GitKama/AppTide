use crate::catalog::{App, Result};
use std::time::Duration;

#[cfg(windows)]
pub fn read() -> Result<Vec<App>> {
    use winreg::{RegKey, enums::*};
    let mut apps = vec![];
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
                apps.push(App {
                    key: format!("registry:{label}:{label_view}:{key}"),
                    name,
                    version: get("DisplayVersion"),
                    publisher: get("Publisher"),
                    location: get("InstallLocation"),
                    scope: scope.into(),
                    kind: "registry".into(),
                    protected: app.get_value::<u32, _>("SystemComponent").unwrap_or(0) == 1
                        || app.get_value::<u32, _>("NoRemove").unwrap_or(0) == 1,
                    ..Default::default()
                });
            }
        }
    }
    // The beta uses the installed Windows AppX cmdlet for current-user packages.
    // This fixed, read-only script never contains user input or changes script policy.
    let script = r#"$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); $apps=@(Get-AppxPackage -ErrorAction Stop | Where-Object {-not $_.IsResourcePackage} | ForEach-Object { @{key=('appx:'+$_.PackageFamilyName);name=$_.Name;version=$_.Version.ToString();publisher=$_.Publisher;scope='user';kind='appx';signatureKind=[string]$_.SignatureKind;location=[string]$_.InstallLocation;protected=[bool]($_.NonRemovable -or $_.IsFramework)} }); ConvertTo-Json -InputObject $apps -Depth 4 -Compress"#;
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
    let out = crate::winget::run(
        &program.to_string_lossy(),
        &[
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-EncodedCommand".into(),
            encoded,
        ],
        Some(Duration::from_secs(90)),
    )?;
    if out.code != 0 {
        return Err("AppX inventory failed; previous catalogue preserved.".into());
    }
    let appx: Vec<App> = serde_json::from_str(out.stdout.trim().trim_start_matches('\u{feff}'))
        .map_err(|e| format!("AppX inventory unreadable: {e}"))?;
    apps.extend(appx);
    Ok(deduplicate_mirrors(apps))
}
#[cfg(not(windows))]
pub fn read() -> Result<Vec<App>> {
    Err("Live inventory requires Windows".into())
}

pub fn powershell(script: &str, timeout: u64) -> Result<crate::winget::Output> {
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
    crate::winget::run(
        &program.to_string_lossy(),
        &[
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-EncodedCommand".into(),
            encoded,
        ],
        Some(Duration::from_secs(timeout)),
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
