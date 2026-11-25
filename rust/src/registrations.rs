use crate::catalog::{App, Result, known_version};
use crate::winget::{self, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registration {
    pub selector: String,
    pub version: String,
    pub publisher: String,
}

impl Registration {
    pub fn registry_key(&self) -> Option<String> {
        let parts: Vec<_> = self.selector.split('\\').collect();
        let ["ARP", scope, arch, key] = parts.as_slice() else {
            return None;
        };
        let hive = match *scope {
            "Machine" => "LocalMachine",
            "User" => "CurrentUser",
            _ => return None,
        };
        let view = match *arch {
            "X86" => "Registry32",
            "X64" => "Registry64",
            _ => return None,
        };
        if key.is_empty() || key.len() > 255 || key.chars().any(char::is_control) {
            return None;
        }
        Some(format!("registry:{hive}:{view}:{key}"))
    }
}

pub type Groups = HashMap<String, Vec<Registration>>;

// Only the indexed package headers and explicitly requested English fields carry identity.
// Incomplete or unfamiliar output leaves that group available for manual review only.
pub fn parse_details(output: &str) -> Groups {
    static HEADER: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"^\(\d+/\d+\) .+ \[([A-Za-z0-9._+-]+)\]$").unwrap()
    });
    let mut groups = Groups::new();
    let mut current = None;
    for line in output.lines().map(str::trim_end) {
        if line.starts_with('(') {
            current = HEADER.captures(line).map(|m| {
                let group = groups.entry(m[1].to_ascii_lowercase()).or_default();
                group.push(Registration::default());
                (m[1].to_ascii_lowercase(), group.len() - 1)
            });
        } else if let Some((id, i)) = &current {
            let record = &mut groups.get_mut(id).unwrap()[*i];
            if let Some(s) = line.strip_prefix("Version: ") {
                record.version = s.into();
            }
            if let Some(s) = line.strip_prefix("Publisher: ") {
                record.publisher = s.into();
            }
            if let Some(s) = line.strip_prefix("Local Identifier: ") {
                record.selector = s.into();
            }
        }
    }
    groups
}

pub fn read_groups(rows: &[Row]) -> Groups {
    let mut ids = std::collections::HashSet::new();
    if !rows.iter().any(|r| !ids.insert(r.id.to_ascii_lowercase())) {
        return Groups::new();
    }
    winget::run(
        "winget.exe",
        &[
            "list",
            "--source",
            "winget",
            "--details",
            "--output-locale",
            "en-US",
            "--disable-interactivity",
        ]
        .map(String::from),
        Some(Duration::from_secs(90)),
    )
    .ok()
    .filter(|o| o.code == 0)
    .map(|o| parse_details(&o.stdout))
    .unwrap_or_default()
}

pub fn bound<'a>(a: &App, group: &'a [Registration]) -> Option<&'a Registration> {
    group.iter().find(|r| {
        r.registry_key()
            .is_some_and(|k| k.eq_ignore_ascii_case(&a.key))
            && r.publisher.eq_ignore_ascii_case(&a.publisher)
            && r.version == a.version
    })
}

pub fn can_update(a: &App) -> bool {
    a.ambiguous
        && a.present
        && !a.protected
        && !a.excluded
        && !crate::steam::is_game(a)
        && a.extra.get("wingetStale") != Some(&serde_json::Value::Bool(true))
        && a.winget_id.as_deref().is_some_and(winget::valid_id)
        && a.registration_group.len() > 1
        && bound(a, &a.registration_group).is_some()
        && crate::releases::compare(&a.version, &a.available) == "Newer release"
        && a.registration_group.iter().all(|r| {
            matches!(
                crate::releases::compare(&r.version, &a.available),
                "Newer release" | "Installed version ahead"
            )
        })
}

// The row table supplies the offer. Detailed output supplies the local selector and publisher.
// Require a bijection with the native inventory before letting any row use that selector.
pub fn verified_group<'a>(
    rows: &[&Row],
    details: &'a [Registration],
    native: &[App],
) -> Option<&'a [Registration]> {
    if rows.len() < 2 || rows.len() != details.len() {
        return None;
    }
    let mut keys = std::collections::HashSet::new();
    for row in rows {
        let mut found = details.iter().filter(|r| r.version == row.version);
        let r = found.next()?;
        let key = r.registry_key()?;
        if found.next().is_some()
            || !known_version(&r.version)
            || r.publisher.is_empty()
            || !keys.insert(key.to_ascii_lowercase())
        {
            return None;
        }
        let mut registered = native
            .iter()
            .filter(|a| a.kind == "registry" && key.eq_ignore_ascii_case(&a.key));
        let a = registered.next()?;
        if registered.next().is_some()
            || crate::steam::is_game(a)
            || a.protected
            || !a.publisher.eq_ignore_ascii_case(&r.publisher)
            || winget::normalize(&a.name) != winget::normalize(&row.name)
        {
            return None;
        }
    }
    Some(details)
}

fn matches_offer(a: &App, output: &winget::Output) -> bool {
    let Ok((rows, 0)) = winget::parse(&output.stdout) else {
        return false;
    };
    output.code == 0
        && matches!(rows.as_slice(), [row] if a.winget_id.as_deref().is_some_and(|id| id.eq_ignore_ascii_case(&row.id))
            && row.version == a.version && row.available == a.available)
}

pub fn check_offer(a: &App) -> Result<()> {
    let selector = &bound(a, &a.registration_group)
        .ok_or("Installation identity changed")?
        .selector;
    let output = winget::run(
        "winget.exe",
        &[
            "list",
            "--id",
            selector,
            "--exact",
            "--source",
            "winget",
            "--disable-interactivity",
            "--output-locale",
            "en-US",
        ]
        .map(String::from),
        Some(Duration::from_secs(30)),
    )?;
    if matches_offer(a, &output) {
        return Ok(());
    }
    Err("WinGet could no longer confirm this exact installation and update. Refresh and review it again.".into())
}

pub fn verified_outcome(before: &App, current: &[&App]) -> Option<&'static str> {
    let old = bound(before, &before.registration_group)?;
    let group = &current.first()?.registration_group;
    if group.is_empty()
        || current.len() != group.len()
        || current.iter().any(|a| {
            a.registration_group != *group
                || bound(a, group).is_none()
                || a.extra.get("wingetStale") == Some(&serde_json::Value::Bool(true))
        })
    {
        return None;
    }
    if before
        .registration_group
        .iter()
        .any(|r| r != old && !group.contains(r))
    {
        return None;
    }
    let mut targets = current.iter().filter(|a| {
        a.version == before.available
            && a.publisher == before.publisher
            && a.scope == before.scope
            && bound(a, group).is_some_and(|r| {
                r.selector.rsplit_once('\\').map(|p| p.0)
                    == old.selector.rsplit_once('\\').map(|p| p.0)
            })
    });
    targets.next()?;
    if targets.next().is_some() {
        return None;
    }
    let alongside = group.contains(old);
    if group.len() != before.registration_group.len() + usize::from(alongside) {
        return None;
    }
    Some(if alongside {
        "The requested version was installed alongside the previous versions."
    } else {
        "The selected version was updated. The other registered versions were confirmed unchanged."
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_local_selector_must_report_one_complete_unchanged_offer() {
        let a = App {
            winget_id: Some("Nvidia.CUDA".into()),
            version: "13.4".into(),
            available: "13.5".into(),
            ..Default::default()
        };
        let header = "Name  Id  Version  Available\n-------------------------------------\n";
        let line = "CUDA  Nvidia.CUDA  13.4  13.5";
        for (body, ok) in [
            (line.to_string(), true),
            (format!("{line}\n{line}"), false),
            (line.replace("13.4", "13.0"), false),
            (line.replace("13.5", "13.6"), false),
            (line.replace("Nvidia.CUDA", "Other.App"), false),
            (format!("{line}\nIncomplet…"), false),
        ] {
            assert_eq!(
                matches_offer(
                    &a,
                    &winget::Output {
                        code: 0,
                        stdout: format!("{header}{body}"),
                        stderr: String::new()
                    }
                ),
                ok
            );
        }
    }
    #[test]
    fn a_partial_or_untrusted_details_group_cannot_enable_updates() {
        let rows = ["13.0", "13.4"].map(|v| Row {
            name: format!("CUDA Toolkit {v}"),
            id: "Nvidia.CUDA".into(),
            version: v.into(),
            available: "13.5".into(),
        });
        let regs = rows.each_ref().map(|r| Registration {
            selector: format!(r"ARP\Machine\X64\CUDA_{}", r.version),
            version: r.version.clone(),
            publisher: "NVIDIA".into(),
        });
        let native = std::array::from_fn::<_, 2, _>(|i| App {
            key: regs[i].registry_key().unwrap(),
            kind: "registry".into(),
            name: rows[i].name.clone(),
            publisher: "NVIDIA".into(),
            ..Default::default()
        });
        let refs = rows.each_ref();
        assert!(verified_group(&refs, &regs, &native).is_some());
        assert!(verified_group(&refs, &regs[..1], &native).is_none());
        for i in 0..4 {
            let mut changed = regs.clone();
            match i {
                0 => changed[0].publisher = "Different".into(),
                1 => changed[0].selector = changed[1].selector.clone(),
                2 => changed[0].selector = r"ARP\Machine\unsupported\key".into(),
                _ => changed[0].version = changed[1].version.clone(),
            }
            assert!(verified_group(&refs, &changed, &native).is_none());
        }
    }
}
