use crate::catalog::{App, Result, now};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Source {
    pub url: String,
    pub instructions: String,
    pub repository: String,
    pub updated_at: String,
    pub manual_version: String,
}
pub fn valid_url(url: &str) -> bool {
    url.is_empty()
        || (url.len() <= 2048
            && regex::Regex::new(
                r"^https://[A-Za-z0-9](?:[A-Za-z0-9.-]*[A-Za-z0-9])?(?::443)?(?:/[^\s<>\x22\\]*)?$",
            )
            .unwrap()
            .is_match(url)
            && url
                .strip_prefix("https://")
                .unwrap_or("")
                .split('/')
                .next()
                .is_some_and(|h| h.contains('.') && !h.contains('@')))
}
pub fn get(a: &App) -> Option<Source> {
    a.extra
        .get("customSource")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
}
pub fn save(a: &mut App, mut source: Source) -> Result<()> {
    source.url = source.url.trim().into();
    source.repository = crate::releases::normalize_repository(&source.repository);
    if !valid_url(&source.url) {
        return Err("Use a full HTTPS website address without credentials, for example https://example.org/downloads.".into());
    }
    if !source.repository.is_empty() && !crate::releases::valid_repository(&source.repository) {
        return Err("Use GitHub owner/repository, without a URL.".into());
    }
    if source.instructions.len() > 8000 {
        return Err("Keep update notes under 8,000 characters.".into());
    }
    if a.kind == "manual" {
        if source.manual_version.len() > 100 {
            return Err("Keep version text under 100 characters.".into());
        }
        a.version = source.manual_version.trim().into();
        if !a.version.is_empty() && !a.versions.iter().any(|v| v.version == a.version) {
            a.versions.push(crate::catalog::Version {
                version: a.version.clone(),
                seen_at: now(),
            });
        }
        if a.versions.len() > 50 {
            a.versions.drain(..a.versions.len() - 50);
        }
    }
    if source.url.is_empty()
        && source.repository.is_empty()
        && source.instructions.trim().is_empty()
    {
        a.extra.remove("customSource");
        crate::releases::set_watch(a, "")?;
        return Ok(());
    }
    crate::releases::set_watch(a, &source.repository)?;
    source.updated_at = now();
    a.extra
        .insert("customSource".into(), serde_json::to_value(source).unwrap());
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_sources_are_notes_never_installer_authority() {
        for url in [
            "file:///C:/bad.exe",
            "https://user:pass@example.org/",
            "javascript:alert(1)",
            "https://example.org/\nwrong",
            "https://example.org/\"bad",
        ] {
            assert!(!valid_url(url));
        }
        let mut a = App::default();
        save(
            &mut a,
            Source {
                url: "https://example.org/downloads".into(),
                instructions: "Use Help > Updates".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(get(&a).is_some());
        assert!(!crate::catalog::actionable(&a));
    }
}
