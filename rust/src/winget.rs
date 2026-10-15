use crate::catalog::{App, Result, digest, known_version};
use regex::Regex;
use std::{
    collections::HashMap,
    io::Read,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub fn valid_id(s: &str) -> bool {
    (2..=200).contains(&s.len())
        && s.as_bytes()[0].is_ascii_alphanumeric()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
        && !s.contains("...")
}
#[derive(Clone, Debug)]
pub struct Row {
    pub name: String,
    pub id: String,
    pub version: String,
    pub available: String,
}
pub fn parse(text: &str) -> Result<(Vec<Row>, usize)> {
    let ansi = Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]").unwrap();
    let cleaned = ansi.replace_all(text, "").replace('\r', "");
    let lines: Vec<_> = cleaned.lines().collect();
    let separator = lines
        .iter()
        .position(|s| s.trim().len() >= 10 && s.trim().chars().all(|c| c == '-'))
        .ok_or("WinGet returned no readable table. Open Terminal and check winget list.")?;
    if separator == 0 {
        return Err("WinGet table has no header.".into());
    }
    let split = Regex::new(r"\s{2,}").unwrap();
    let columns = split.split(lines[separator - 1].trim()).count();
    if !(3..=5).contains(&columns) {
        return Err("Unsupported WinGet table layout.".into());
    }
    let mut rows = vec![];
    let mut rejected = 0;
    for line in &lines[separator + 1..] {
        if line.trim().is_empty() {
            continue;
        }
        let cells: Vec<_> = split.split(line.trim()).collect();
        if cells.len() < 3
            || cells.len() > columns
            || !valid_id(cells[1])
            || cells[2].contains('…')
            || cells[2].contains("...")
        {
            rejected += 1;
            continue;
        }
        let available = cells.get(3).copied().unwrap_or("");
        let available = if ["winget", "msstore"].contains(&available)
            || available.contains('…')
            || available.contains("...")
        {
            ""
        } else {
            available
        };
        rows.push(Row {
            name: cells[0].into(),
            id: cells[1].into(),
            version: cells[2].into(),
            available: available.into(),
        });
    }
    Ok((rows, rejected))
}
pub fn normalize(s: &str) -> String {
    static ARCH: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\s*\((?:x64|x86|64-bit|32-bit)\)").unwrap());
    static VERSION: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\s+\d+(?:\.\d+)+(?:\s.*)?$").unwrap());
    let a = &*ARCH;
    let b = &*VERSION;
    b.replace_all(&a.replace_all(&s.to_lowercase(), ""), "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn reconcile(mut inventory: Vec<App>, rows: Vec<Row>) -> Vec<App> {
    for a in &mut inventory {
        a.uid = digest(&a.key)[..24].into();
        a.evidence = match a.kind.as_str() {
            "manual" => "Manually tracked; installation not detected",
            "portable" => "Executable metadata in a selected folder",
            _ => "Windows inventory",
        }
        .into();
    }
    for row in rows {
        let matches: Vec<_> = inventory
            .iter()
            .enumerate()
            .filter(|(_, a)| {
                a.kind == "registry"
                    && !crate::steam::is_game(a)
                    && normalize(&a.name) == normalize(&row.name)
                    && (a.version == row.version
                        || crate::releases::compare(&a.version, &row.version) == "Matches release"
                        || ((!known_version(&a.version) || !known_version(&row.version))
                            && crate::community::confirms(a, &row.id)))
            })
            .map(|(i, _)| i)
            .collect();
        let uid = digest(&format!("winget:{}", row.id))[..24].to_string();
        if matches.len() == 1 && inventory[matches[0]].winget_id.is_none() {
            let a = &mut inventory[matches[0]];
            a.uid = uid;
            a.winget_id = Some(row.id);
            a.available = row.available;
            a.evidence = if a.version == row.version {
                "WinGet: unique name and installed version"
            } else {
                "WinGet: unique name, public publisher identity or equivalent numeric version"
            }
            .into();
            if !known_version(&a.version) && known_version(&row.version) {
                a.version = row.version;
                a.extra
                    .entry("metadataSources".into())
                    .or_insert_with(|| serde_json::json!({}))["version"] =
                    "Installed WinGet observation".into();
            }
        } else {
            inventory.push(App {
                uid,
                key: format!("winget:{}", row.id),
                name: row.name,
                version: row.version,
                winget_id: Some(row.id),
                available: row.available,
                kind: "winget".into(),
                scope: "WinGet".into(),
                evidence: "WinGet catalogue entry".into(),
                ..Default::default()
            })
        }
    }
    let mut counts = HashMap::new();
    for a in &mut inventory {
        a.extra
            .insert("versionComparison".into(), serde_json::Value::Null);
        if !a.available.is_empty()
            && ["Matches release", "Installed version ahead"]
                .contains(&crate::releases::compare(&a.version, &a.available))
        {
            a.extra.insert("versionComparison".into(),serde_json::json!(format!("Local version {} is equal to or newer than WinGet target {}; no upgrade queued.",a.version,a.available)));
            a.available.clear();
        }
    }
    for a in &inventory {
        if let Some(id) = &a.winget_id {
            *counts.entry(id.clone()).or_insert(0) += 1;
        }
    }
    for a in &mut inventory {
        if let Some(id) = &a.winget_id {
            a.ambiguous = counts[id] > 1;
            let id = id.to_lowercase();
            a.protected |= id == "microsoft.appinstaller"
                || id.starts_with("microsoft.windowsappruntime")
                || id.starts_with("microsoft.windowsterminal")
                || id.starts_with("apptide.");
        }
        a.protected |= a.name.to_lowercase().starts_with("apptide");
        if a.ambiguous {
            a.uid = digest(&format!("{}:{}:{}", a.key, a.version, a.scope))[..24].into();
        }
    }
    inventory
}
#[cfg(test)]
pub fn args(kind: &str, a: &App) -> Result<Vec<String>> {
    args_mode(kind, a, false)
}
pub fn args_mode(kind: &str, a: &App, interactive: bool) -> Result<Vec<String>> {
    let id = a
        .winget_id
        .as_deref()
        .filter(|s| valid_id(s))
        .ok_or("Invalid WinGet ID")?;
    if !["update", "uninstall"].contains(&kind) {
        return Err("Invalid operation".into());
    }
    let mut args: Vec<String> = vec![
        if kind == "update" {
            "upgrade"
        } else {
            "uninstall"
        },
        "--id",
        id,
        "--exact",
        "--source",
        "winget",
        "--disable-interactivity",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    if kind == "update" {
        if !known_version(&a.version) || !known_version(&a.available) {
            return Err("No verified update target".into());
        }
        args.extend([
            "--version".into(),
            a.available.clone(),
            if interactive {
                "--interactive"
            } else {
                "--silent"
            }
            .into(),
        ]);
    } else if known_version(&a.version) {
        args.extend(["--version".into(), a.version.clone()]);
    }
    Ok(args)
}
pub struct Output {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}
pub fn failure_hint(code: i32) -> Option<&'static str> {
    match code as u32 {
        0x80190194 => Some(
            "The installer download returned HTTP 404 (file not found). The package source points to an unavailable file. This update failed; AppTide will continue other apps and will not substitute another installer or bypass its hash.",
        ),
        0x80190193 => Some(
            "The installer download returned HTTP 403 (access denied). Review the package's official download requirements. Other apps can continue.",
        ),
        0x8A150011 => Some(
            "The downloaded file does not match the publisher's expected checksum. AppTide blocked it. Refresh sources and try later; never bypass the checksum. The package maintainer may need to replace the manifest.",
        ),
        0x8A150030 => Some(
            "The previous version could not be uninstalled by its installer. Save your work, close this app, then retry with installer prompts. If it still fails, repair the existing installation using its official installer before updating. AppTide will not force removal.",
        ),
        0x8A15002B => Some(
            "WinGet found no applicable upgrade for this installation. Its installer technology or channel may differ from the available update. Review the output log and publisher migration instructions; AppTide will not force replacement.",
        ),
        0x800704C7 | 0x80004004 | 1602 => Some(
            "The operation was cancelled, including a possible declined Windows administrator prompt. Retry when ready and respond to the installer or Windows permission prompt.",
        ),
        0x8A150049 | 1603 => Some(
            "The installer reported an error. Close the app and retry with installer prompts. The local output log may identify a missing installer component or a required repair.",
        ),
        _ => None,
    }
}
pub fn queue_stop_code(code: i32) -> bool {
    // User cancellation and Windows reboot/installer-lock signals apply to the whole queue.
    matches!(
        code as u32,
        0x80004004
            | 0x800704C7
            | 1602
            | 1618
            | 1641
            | 3010
            | 0x8A150005
            | 0x8A15006A
            | 0x8A150077
            | 0x8A150102
            | 0x8A150109..=0x8A15010C | 0x8A15000A | 0x8A15003F
    )
}
pub fn run(program: &str, args: &[String], timeout: Option<Duration>) -> Result<Output> {
    run_with_input(program, args, timeout, None)
}
pub fn run_with_input(
    program: &str,
    args: &[String],
    timeout: Option<Duration>,
    input: Option<Vec<u8>>,
) -> Result<Output> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("Could not start {program}: {e}"))?;
    if let Some(input) = input {
        let mut stdin = child.stdin.take().ok_or("Child input unavailable")?;
        std::thread::spawn(move || {
            use std::io::Write;
            let _ = stdin.write_all(&input);
        });
    }
    // Drain both pipes concurrently; retain at most 8 MiB each without blocking the child.
    fn drain(mut pipe: impl Read) -> String {
        let mut stored = Vec::new();
        let mut b = [0; 8192];
        while let Ok(n) = pipe.read(&mut b) {
            if n == 0 {
                break;
            }
            let keep = n.min((8 * 1024 * 1024usize).saturating_sub(stored.len()));
            stored.extend_from_slice(&b[..keep]);
        }
        String::from_utf8_lossy(&stored).into_owned()
    }
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let out = std::thread::spawn(move || drain(stdout));
    let err = std::thread::spawn(move || drain(stderr));
    let start = Instant::now();
    let code = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status.code().unwrap_or(-1);
        }
        if timeout.is_some_and(|t| start.elapsed() > t) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "{program} scan timed out; previous catalogue preserved."
            ));
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    Ok(Output {
        code,
        stdout: out.join().unwrap_or_default(),
        stderr: err.join().unwrap_or_default(),
    })
}
pub fn list() -> Result<(Vec<Row>, usize)> {
    let o = run(
        "winget.exe",
        &["list", "--source", "winget", "--disable-interactivity"].map(String::from),
        Some(Duration::from_secs(120)),
    )?;
    if o.code != 0 {
        return Err(format!(
            "WinGet scan failed ({}). Check App Installer and source agreements in Terminal.",
            o.code
        ));
    }
    parse(&o.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_winget_version_requires_corroborated_publisher() {
        let a = App {
            key: "registry:example".into(),
            name: "Battle.net".into(),
            publisher: "Blizzard Entertainment".into(),
            version: "2.53.5.17932".into(),
            kind: "registry".into(),
            ..Default::default()
        };
        let row = || Row {
            name: "Battle.net".into(),
            id: "Blizzard.BattleNet".into(),
            version: "Unknown".into(),
            available: "".into(),
        };
        let matched = reconcile(vec![a.clone()], vec![row()]);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].version, a.version);
        assert_eq!(matched[0].winget_id.as_deref(), Some("Blizzard.BattleNet"));
        let separate = reconcile(
            vec![App {
                publisher: "Different publisher".into(),
                ..a
            }],
            vec![row()],
        );
        assert_eq!(separate.len(), 2);
    }
    #[test]
    fn recovered_current_or_newer_versions_are_not_queued_again() {
        let a = App {
            key: "registry:example".into(),
            name: "Geekbench 6".into(),
            publisher: "Primate Labs Inc.".into(),
            version: "6.4.0.0".into(),
            kind: "registry".into(),
            ..Default::default()
        };
        for offered in ["6.4.0", "6.3.0"] {
            let result = reconcile(
                vec![a.clone()],
                vec![Row {
                    name: a.name.clone(),
                    id: "PrimateLabs.Geekbench.6".into(),
                    version: "Unknown".into(),
                    available: offered.into(),
                }],
            );
            assert_eq!(result.len(), 1);
            assert!(result[0].available.is_empty());
        }
    }
    #[test]
    fn interactive_retry_changes_only_installer_mode() {
        let a = App {
            winget_id: Some("Example.App".into()),
            version: "1.0".into(),
            available: "2.0".into(),
            ..Default::default()
        };
        let silent = args_mode("update", &a, false).unwrap();
        let interactive = args_mode("update", &a, true).unwrap();
        assert!(silent.contains(&"--silent".into()));
        assert!(interactive.contains(&"--interactive".into()));
        assert!(!interactive.contains(&"--silent".into()));
        assert_eq!(
            silent[..silent.len() - 1],
            interactive[..interactive.len() - 1]
        );
    }
    #[cfg(windows)]
    #[test]
    fn diagnostics_do_not_contaminate_machine_readable_stdout() {
        let program = std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let output = run(
            &program.to_string_lossy(),
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "[Console]::Out.Write('[]'); [Console]::Error.Write('diagnostic')",
            ]
            .map(String::from),
            Some(Duration::from_secs(20)),
        )
        .unwrap();
        assert_eq!(output.code, 0);
        assert_eq!(output.stdout, "[]");
        assert_eq!(output.stderr, "diagnostic");
        assert!(serde_json::from_str::<Vec<serde_json::Value>>(&output.stdout).is_ok());
    }
    #[test]
    fn rejects_truncation_and_shell_strings() {
        for id in ["--all", "X;whoami", "A…B", "A...B", "a", "A B"] {
            assert!(!valid_id(id));
        }
        assert!(valid_id("Vendor.App"));
    }
    #[test]
    fn table_fails_closed() {
        assert!(parse("nonsense").is_err());
        let (r,n)=parse("Name  Id  Version  Available\n------------------------------------\nExample  Vendor.Example  1.0  2.0\nBad  Vendor.…  1\n2 upgrades available.").unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(n, 2);
        assert_eq!(r[0].available, "2.0");
    }
    #[test]
    fn ambiguous_rows_stay_separate() {
        let rows = vec![
            Row {
                name: "A".into(),
                id: "Vendor.A".into(),
                version: "1".into(),
                available: "2".into(),
            },
            Row {
                name: "A".into(),
                id: "Vendor.A".into(),
                version: "0".into(),
                available: "2".into(),
            },
        ];
        let apps = reconcile(vec![], rows);
        assert!(apps.iter().all(|a| a.ambiguous));
        assert_ne!(apps[0].uid, apps[1].uid);
    }
    #[test]
    fn exact_update_arguments() {
        let a = App {
            winget_id: Some("Vendor.App".into()),
            version: "1".into(),
            available: "2".into(),
            ..Default::default()
        };
        let v = args("update", &a).unwrap();
        assert!(v.contains(&"--exact".into()));
        assert!(!v.contains(&"--force".into()));
        assert!(args("other", &a).is_err());
    }
}
