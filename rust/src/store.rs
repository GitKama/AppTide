//! Microsoft's consumer Store CLI owns this bulk update route, separately from WinGet.
use crate::{
    catalog::{App, Result, now},
    winget,
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Update {
    pub name: String,
    pub publisher: String,
    pub version: String,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub checked_at: String,
    pub updates: Vec<Update>,
    pub error: Option<String>,
}
pub fn program() -> Result<PathBuf> {
    let path =
        PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("Local app directory unavailable")?)
            .join("Microsoft/WindowsApps/store.exe");
    if !path.exists() {
        return Err("Microsoft Store CLI is unavailable. Update Microsoft Store, enable its store.exe execution alias, or open Store Downloads.".into());
    }
    Ok(path)
}
pub fn run(apply: bool) -> Result<winget::Output> {
    let args = if apply {
        vec!["updates".into(), "--apply".into()]
    } else {
        vec!["updates".into()]
    };
    // stdin is null: a check cannot accept the CLI's default-yes interactive prompt.
    // Apply is only dispatched after the user confirms the Store-specific scope.
    winget::run(
        &program()?.to_string_lossy(),
        &args,
        Some(Duration::from_secs(if apply { 1800 } else { 120 })),
    )
}
pub fn parse(text: &str) -> Result<Vec<Update>> {
    let ansi = regex::Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]").unwrap();
    let cleaned = ansi.replace_all(text, "").replace('\r', "");
    let cleaned = cleaned.trim_start_matches('\u{feff}');
    let count = regex::Regex::new(r"Updates available \((\d+) found\)").unwrap();
    let expected = count
        .captures(cleaned)
        .and_then(|c| c[1].parse::<usize>().ok());
    let Some(expected) = expected else {
        if cleaned.lines().any(|l| {
            matches!(
                l.trim(),
                "No updates available."
                    | "No updates available"
                    | "No updates found."
                    | "No updates found"
                    | "All apps are up to date."
                    | "All apps are up to date!"
            )
        }) {
            return Ok(vec![]);
        }
        return Err("Store output could not be read. Open the local Store check log or use Store Downloads. No update was started.".into());
    };
    let mut rows: Vec<Update> = vec![];
    let mut in_table = false;
    for line in cleaned.lines() {
        if line.starts_with('┌') {
            in_table = true;
            continue;
        }
        if line.starts_with('└') {
            in_table = false;
            continue;
        }
        if !in_table || !line.starts_with('│') {
            continue;
        }
        let cells: Vec<_> = line.trim_matches('│').split('│').map(str::trim).collect();
        if cells.len() != 4 {
            return Err("Unsupported Store table layout; use Store Downloads.".into());
        }
        if cells[0] == "Name" && cells[1] == "Publisher" {
            continue;
        }
        if cells.iter().any(|s| s.contains('…') || s.contains("...")) {
            return Err(
                "Store truncated its update list. Open Store Downloads to review it.".into(),
            );
        }
        if !cells[2].is_empty() {
            if cells[0].is_empty() || !crate::catalog::known_version(cells[2]) {
                return Err("Store returned an incomplete app identity.".into());
            }
            rows.push(Update {
                name: cells[0].into(),
                publisher: cells[1].into(),
                version: cells[2].into(),
            });
        } else if let Some(row) = rows.last_mut() {
            for (target, continuation) in
                [(&mut row.name, cells[0]), (&mut row.publisher, cells[1])]
            {
                if !continuation.is_empty() {
                    target.push(' ');
                    target.push_str(continuation);
                }
            }
        } else {
            return Err("Store returned an unexpected wrapped row.".into());
        }
    }
    if rows.len() != expected || expected > 500 {
        return Err(
            "Store update count did not match its readable list. Use Store Downloads.".into(),
        );
    }
    let mut identities = std::collections::BTreeSet::new();
    if rows.iter().any(|r| {
        !identities.insert((
            r.name.to_lowercase(),
            r.publisher.to_lowercase(),
            r.version.clone(),
        ))
    }) {
        return Err("Store returned duplicate update rows; review in Store Downloads.".into());
    }
    Ok(rows)
}
pub fn check_output(output: &winget::Output) -> Result<Check> {
    if output.code != 0 || !output.stderr.trim().is_empty() {
        return Err(format!(
            "Microsoft Store check failed (exit {}). Review the local log or open Store Downloads.",
            output.code
        ));
    }
    Ok(Check {
        checked_at: now(),
        updates: parse(&output.stdout)?,
        error: None,
    })
}
pub fn same_updates(a: &[Update], b: &[Update]) -> bool {
    let key = |rows: &[Update]| {
        let mut k: Vec<_> = rows
            .iter()
            .map(|r| (&r.name, &r.publisher, &r.version))
            .map(|(n, p, v)| format!("{n}\0{p}\0{v}"))
            .collect();
        k.sort();
        k
    };
    key(a) == key(b)
}
pub fn match_installed<'a>(update: &Update, apps: &'a [App]) -> Option<&'a App> {
    let mut matches = apps.iter().filter(|a| {
        a.kind == "appx"
            && a.extra.get("signatureKind").and_then(|v| v.as_str()) == Some("Store")
            && a.name.eq_ignore_ascii_case(&update.name)
            && a.publisher.eq_ignore_ascii_case(&update.publisher)
            && a.version == update.version
    });
    let first = matches.next();
    if matches.next().is_some() {
        None
    } else {
        first
    }
}
pub fn verification(updates: &[Update], before: &[App], after: &[App]) -> (usize, usize) {
    let verified = updates
        .iter()
        .filter(|row| {
            let Some(old) = match_installed(row, before) else {
                return false;
            };
            let mut matches = after.iter().filter(|a| a.key == old.key);
            let current = matches.next();
            matches.next().is_none()
                && current.is_some_and(|a| {
                    crate::catalog::known_version(&a.version)
                        && crate::releases::compare(&old.version, &a.version) == "Newer release"
                        && a.extra.get("rawPublisher") == old.extra.get("rawPublisher")
                        && a.extra.get("signatureKind").and_then(|v| v.as_str()) == Some("Store")
                })
        })
        .count();
    (verified, updates.len() - verified)
}
pub fn demo_check() -> Check {
    Check {
        checked_at: "Synthetic demonstration".into(),
        updates: vec![
            Update {
                name: "Orchid Photos".into(),
                publisher: "Orchid Software".into(),
                version: "2026.10.1.0".into(),
            },
            Update {
                name: "Pocket Radio".into(),
                publisher: "Pocket Project".into(),
                version: "2.4.0.0".into(),
            },
        ],
        error: None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn live_store_no_updates_wording_is_success_not_a_parse_failure() {
        // Exact response shape from Store CLI 22608.1401.6.0, no private inventory.
        let output = winget::Output {
            code: 0,
            stdout: "Checking for updates…\r\n\r\nNo updates found.\r\n".into(),
            stderr: String::new(),
        };
        let check = check_output(&output).unwrap();
        assert!(check.error.is_none());
        assert!(check.updates.is_empty());
        assert!(
            parse("\u{feff}\u{1b}[32mNo updates found.\u{1b}[0m\r\n")
                .unwrap()
                .is_empty()
        );
        assert!(parse("No updates found because the network check failed.").is_err());
        assert!(check_output(&winget::Output { code: 1, ..output }).is_err());
    }
    #[test]
    fn parses_wrapped_rows_without_treating_installed_version_as_target() {
        let text = "Updates available (2 found)\n┌──────┐\n│ Name │ Publisher │ Version │ Date │\n│ Pocket Radio │ Pocket │ 2.4.0.0 │ 2026-01-01 │\n│ Player │ Project │ │ │\n│ Photos │ Orchid │ 1.0.0.0 │ 2026-01-01 │\n└──────┘\nWould you like to install?\nFailed to read input in non-interactive mode.";
        let rows = parse(text).unwrap();
        assert_eq!(rows[0].name, "Pocket Radio Player");
        assert_eq!(rows[0].publisher, "Pocket Project");
        assert_eq!(rows[0].version, "2.4.0.0");
        assert!(parse(&text.replace("(2 found)", "(3 found)")).is_err());
        assert!(parse(&text.replace("Photos", "Photo…")).is_err());
        assert!(parse("Network failed").is_err());
        assert!(parse("No updates available.").unwrap().is_empty());
    }
    #[test]
    fn verification_requires_unique_native_identity_and_newer_version() {
        let row = Update {
            name: "Photos".into(),
            publisher: "Orchid".into(),
            version: "1.0.0.0".into(),
        };
        let old = App {
            key: "appx:Orchid_signed".into(),
            name: row.name.clone(),
            publisher: row.publisher.clone(),
            version: row.version.clone(),
            kind: "appx".into(),
            extra: [
                ("signatureKind".into(), "Store".into()),
                ("rawPublisher".into(), "CN=Orchid".into()),
            ]
            .into(),
            ..Default::default()
        };
        let new = App {
            version: "2.0.0.0".into(),
            ..old.clone()
        };
        assert_eq!(
            verification(
                std::slice::from_ref(&row),
                std::slice::from_ref(&old),
                std::slice::from_ref(&new)
            ),
            (1, 0)
        );
        assert_eq!(
            verification(
                std::slice::from_ref(&row),
                std::slice::from_ref(&old),
                std::slice::from_ref(&old)
            ),
            (0, 1)
        );
        assert_eq!(verification(&[row], &[old.clone(), old], &[new]), (0, 1));
    }
}
