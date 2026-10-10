use crate::catalog::{App, Result, now};
use serde_json::{Value, json};

pub fn valid_repository(s: &str) -> bool {
    let parts: Vec<_> = s.split('/').collect();
    parts.len() == 2
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.len() <= 100
                && !p.starts_with('.')
                && p.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
}
pub fn normalize_repository(s: &str) -> String {
    let s = s.trim().trim_end_matches('/');
    let s = s.strip_prefix("https://github.com/").unwrap_or(s);
    s.strip_suffix("/releases")
        .unwrap_or(s)
        .trim_end_matches('/')
        .to_string()
}
pub fn repository(a: &App) -> Option<&str> {
    if a.extra.get("watchDisabled") == Some(&json!(true)) {
        return None;
    }
    a.extra
        .get("watch")
        .filter(|w| w["association"] != "bundled-recipe")
        .and_then(|w| w["repository"].as_str())
        .or_else(|| a.knowledge.as_ref().map(|k| k.repository.as_str()))
        .filter(|s| valid_repository(s))
}
pub fn set_watch(a: &mut App, repo: &str) -> Result<()> {
    let normalized = normalize_repository(repo);
    let repo = normalized.as_str();
    if repo.is_empty() {
        a.extra.remove("watch");
        a.extra.insert("watchDisabled".into(), true.into());
        return Ok(());
    }
    if !valid_repository(repo) {
        return Err("Use a public GitHub owner/repository, without a URL or spaces.".into());
    }
    a.extra.remove("watchDisabled");
    if let Some(w) = a.extra.get_mut("watch")
        && w["repository"] == repo
    {
        w["association"] = "user-selected".into();
        return Ok(());
    }
    a.extra.insert(
        "watch".into(),
        json!({"repository":repo,"association":"user-selected","status":"Not checked"}),
    );
    Ok(())
}
fn numeric(s: &str) -> Option<Vec<u64>> {
    let s = s.strip_prefix(['v', 'V']).unwrap_or(s);
    let s = s
        .strip_suffix("_64")
        .or_else(|| s.strip_suffix("_32"))
        .unwrap_or(s);
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
        return None;
    }
    let mut parts = s
        .split('.')
        .map(str::parse)
        .collect::<std::result::Result<Vec<u64>, _>>()
        .ok()?;
    if parts.len() > 8 {
        return None;
    }
    while parts.len() > 1 && parts.last() == Some(&0) {
        parts.pop();
    }
    Some(parts)
}
pub fn compare(installed: &str, latest: &str) -> &'static str {
    match (numeric(installed), numeric(latest)) {
        (Some(a), Some(b)) if b > a => "Newer release",
        (Some(a), Some(b)) if a == b => "Matches release",
        (Some(_), Some(_)) => "Installed version ahead",
        _ => "Review version format",
    }
}
pub fn fetch(repo: &str) -> Result<Value> {
    if !valid_repository(repo) {
        return Err("Invalid release repository".into());
    }
    let v = api(&format!("/repos/{repo}/releases/latest"))?;
    if v["draft"] != false
        || v["prerelease"] != false
        || v["tag_name"]
            .as_str()
            .is_none_or(|s| s.is_empty() || s.len() > 100)
    {
        return Err("No valid stable release returned".into());
    }
    Ok(
        json!({"repository":repo,"latest":v["tag_name"],"checkedAt":now(),"publishedAt":v["published_at"]}),
    )
}
pub fn api(path: &str) -> Result<Value> {
    if !path.starts_with("/repos/")
        || path.len() > 300
        || !path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._-?=&".contains(&b))
    {
        return Err("Invalid GitHub API path".into());
    }
    // Only a validated public repository is interpolated. No credentials or inventory leave the PC.
    // Headers-only completion + bounded stream prevents unexpectedly large JSON allocations.
    let script = format!(
        r#"$ErrorActionPreference='Stop'; Add-Type -AssemblyName System.Net.Http; $h=[Net.Http.HttpClientHandler]::new(); $h.AllowAutoRedirect=$false; $c=[Net.Http.HttpClient]::new($h); $c.Timeout=[TimeSpan]::FromSeconds(20); $c.DefaultRequestHeaders.UserAgent.ParseAdd('AppTide-Beta/0.2'); $c.DefaultRequestHeaders.Accept.ParseAdd('application/vnd.github+json'); try {{ $r=$c.GetAsync('https://api.github.com{path}',[Net.Http.HttpCompletionOption]::ResponseHeadersRead).GetAwaiter().GetResult(); if ([int]$r.StatusCode -ne 200) {{ throw ('GitHub HTTP '+[int]$r.StatusCode) }}; $s=$r.Content.ReadAsStreamAsync().GetAwaiter().GetResult(); $m=[IO.MemoryStream]::new(); $b=New-Object byte[] 8192; $total=0; while(($n=$s.Read($b,0,$b.Length)) -gt 0) {{ $total+=$n; if($total -gt 1048576){{throw 'Release response exceeds 1 MiB'}}; $m.Write($b,0,$n) }}; [Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); [Console]::Write([Text.Encoding]::UTF8.GetString($m.ToArray())) }} finally {{ $c.Dispose() }}"#
    );
    let out = crate::inventory::powershell(&script, 30)?;
    if out.code != 0 {
        return Err(
            "Release check failed (network, repository, or GitHub rate limit). Try later.".into(),
        );
    }
    let v: Value = serde_json::from_str(out.stdout.trim().trim_start_matches('\u{feff}'))
        .map_err(|_| "Invalid release response")?;
    Ok(v)
}
pub fn apply(a: &mut App, release: &Value) {
    let mut watch = release.clone();
    watch["status"] = compare(&a.version, release["latest"].as_str().unwrap_or("")).into();
    watch["association"] = if a
        .extra
        .get("watch")
        .is_some_and(|w| w["association"] == "user-selected")
    {
        "user-selected"
    } else {
        "bundled-recipe"
    }
    .into();
    a.extra.insert("watch".into(), watch);
}
pub fn newer(a: &App) -> bool {
    a.extra
        .get("watch")
        .is_some_and(|w| w["status"] == "Newer release")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn watch_editor_accepts_project_urls_and_can_disable_bundled_watches() {
        let mut a = App {
            knowledge: Some(crate::knowledge::Knowledge {
                repository: "example/app".into(),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(repository(&a), Some("example/app"));
        set_watch(&mut a, "").unwrap();
        assert!(repository(&a).is_none());
        set_watch(&mut a, "https://github.com/example/app/releases/").unwrap();
        assert_eq!(repository(&a), Some("example/app"));
    }
    #[test]
    fn repository_is_data_not_a_command_or_url() {
        for r in [
            "../x",
            "a/b/c",
            "https://github.com/a/b",
            "a/b';exit",
            "a/b?x",
            "a/",
            "a/..",
        ] {
            assert!(!valid_repository(r));
        }
        assert!(valid_repository("FrankBijnen/ExifToolGui"));
    }
    #[test]
    fn versions_are_conservative_and_numeric() {
        assert_eq!(compare("6.3.12.1_64", "V6.3.13"), "Newer release");
        assert_eq!(compare("1.10", "1.9"), "Installed version ahead");
        assert_eq!(compare("1.2.0", "v1.2"), "Matches release");
        assert_eq!(compare("nightly", "v2.0"), "Review version format");
        assert_eq!(compare("1.0", "v2.0-beta"), "Review version format");
    }
}
