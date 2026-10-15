//! Read Steam's local library manifests. Steam remains the installation/update owner.
use crate::catalog::{App, Result};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

pub fn app_id(a: &App) -> Option<String> {
    a.extra
        .get("steam")
        .and_then(|s| s["appId"].as_str())
        .or_else(|| {
            a.key
                .splitn(4, ':')
                .nth(3)
                .and_then(|s| s.strip_prefix("Steam App "))
        })
        .filter(|id| !id.is_empty() && id.len() <= 12 && id.bytes().all(|b| b.is_ascii_digit()))
        .map(String::from)
}
pub fn is_game(a: &App) -> bool {
    app_id(a).is_some()
}
pub fn vdf(text: &str) -> Result<Value> {
    if text.len() > 4 * 1024 * 1024 {
        return Err("Steam manifest exceeds 4 MiB".into());
    }
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                let mut s = String::new();
                let mut closed = false;
                while let Some(c) = chars.next() {
                    if c == '"' {
                        closed = true;
                        break;
                    }
                    if c == '\\' {
                        let n = chars.next().ok_or("Incomplete Steam escape")?;
                        if n != '\\' && n != '"' {
                            s.push('\\')
                        }
                        s.push(n)
                    } else {
                        s.push(c)
                    }
                }
                if !closed {
                    return Err("Incomplete Steam string".into());
                }
                tokens.push(s);
            }
            '{' | '}' => tokens.push(c.to_string()),
            '/' if chars.peek() == Some(&'/') => while chars.next().is_some_and(|c| c != '\n') {},
            c if c.is_whitespace() || c == '\u{feff}' => {}
            _ => return Err("Unsupported Steam manifest syntax".into()),
        }
        if tokens.len() > 100_000 {
            return Err("Steam manifest has too many fields".into());
        }
    }
    fn object(t: &[String], i: &mut usize, depth: usize) -> Result<Value> {
        if depth > 16 {
            return Err("Steam nesting exceeds limit".into());
        }
        let mut m = serde_json::Map::new();
        while *i < t.len() && t[*i] != "}" {
            let key = t[*i].clone();
            *i += 1;
            if key == "{" || *i >= t.len() {
                return Err("Invalid Steam field".into());
            }
            let value = if t[*i] == "{" {
                *i += 1;
                let v = object(t, i, depth + 1)?;
                if t.get(*i).map(String::as_str) != Some("}") {
                    return Err("Unclosed Steam object".into());
                }
                *i += 1;
                v
            } else {
                let s = t[*i].clone();
                *i += 1;
                Value::String(s)
            };
            if m.insert(key, value).is_some() {
                return Err("Duplicate Steam field".into());
            }
        }
        Ok(Value::Object(m))
    }
    let mut i = 0;
    let v = object(&tokens, &mut i, 0)?;
    if i != tokens.len() {
        return Err("Unexpected Steam closing brace".into());
    }
    Ok(v)
}
fn read(path: &Path) -> Result<Value> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return Err("Linked Steam manifest skipped".into());
        }
    }
    if !meta.is_file() || meta.len() > 4 * 1024 * 1024 {
        return Err("Invalid Steam manifest size/type".into());
    }
    vdf(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)
}
fn local(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        let s = path.to_string_lossy();
        if s.len() < 3 || s.as_bytes()[1] != b':' || s.starts_with("\\\\") {
            return false;
        }
        for ancestor in path.ancestors() {
            if let Ok(m) = std::fs::symlink_metadata(ancestor)
                && m.file_attributes() & 0x400 != 0
            {
                return false;
            }
        }
    }
    path.is_dir()
}
pub fn enrich(apps: &mut Vec<App>) -> Vec<String> {
    // Clear stale build metadata if a previously observed manifest is now missing.
    for a in apps.iter_mut() {
        if let Some(id) = app_id(a) {
            a.extra.insert("steam".into(),json!({"appId":id,"buildId":"","status":"Manifest not found; check this installation in Steam"}));
        }
    }
    let mut roots = BTreeSet::new();
    #[cfg(windows)]
    {
        use winreg::{RegKey, enums::*};
        if let Ok(k) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Software\\Valve\\Steam")
            && let Ok(s) = k.get_value::<String, _>("SteamPath")
        {
            roots.insert(PathBuf::from(s));
        }
    }
    for a in apps.iter().filter(|a| is_game(a)) {
        if let Some(p) = Path::new(&a.location)
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
        {
            roots.insert(p.to_path_buf());
        }
    }
    let mut libraries = roots.clone();
    let mut warnings = Vec::new();
    for root in roots.iter().filter(|p| local(p)) {
        let file = root.join("steamapps/libraryfolders.vdf");
        if !file.exists() {
            continue;
        }
        match read(&file) {
            Ok(v) => {
                for entry in v["libraryfolders"]
                    .as_object()
                    .into_iter()
                    .flat_map(|m| m.values())
                    .take(64)
                {
                    if let Some(p) = entry["path"].as_str().or_else(|| entry.as_str()) {
                        libraries.insert(PathBuf::from(p));
                    }
                }
            }
            Err(e) => warnings.push(format!("Steam library list: {e}")),
        }
    }
    let mut seen = BTreeSet::new();
    let mut seen_libraries = BTreeSet::new();
    for library in libraries.iter().filter(|p| local(p)).take(64) {
        let identity = library
            .to_string_lossy()
            .replace('\\', "/")
            .trim_end_matches('/')
            .to_lowercase();
        if !seen_libraries.insert(identity) {
            continue;
        }
        let Ok(files) = std::fs::read_dir(library.join("steamapps")) else {
            continue;
        };
        for file in files.flatten().take(5000) {
            let name = file.file_name().to_string_lossy().to_string();
            let Some(id) = name
                .strip_prefix("appmanifest_")
                .and_then(|s| s.strip_suffix(".acf"))
            else {
                continue;
            };
            if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
                continue;
            }
            let result = read(&file.path()).and_then(|v| manifest(&v, id, library));
            match result {
                Ok((name, data)) => {
                    if !seen.insert(id.to_string()) {
                        warnings.push(format!(
                            "Steam app {id} appears in multiple libraries; review in Steam."
                        ));
                        continue;
                    }
                    if let Some(a) = apps.iter_mut().find(|a| app_id(a).as_deref() == Some(id)) {
                        a.extra.insert("steam".into(), data);
                    } else {
                        apps.push(App {
                            key: format!("steam:{id}"),
                            name,
                            kind: "steam".into(),
                            publisher: "Steam library".into(),
                            extra: [("steam".into(), data)].into(),
                            ..Default::default()
                        });
                    }
                }
                Err(e) => warnings.push(format!("Steam manifest {id}: {e}")),
            }
        }
    }
    warnings
}
fn manifest(v: &Value, id: &str, library: &Path) -> Result<(String, Value)> {
    let a = &v["AppState"];
    if a["appid"].as_str() != Some(id) {
        return Err("App ID does not match filename".into());
    }
    let number = |key: &str| {
        a[key]
            .as_str()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0)
    };
    let name = a["name"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() < 300)
        .ok_or("Missing Steam title")?;
    let build = number("buildid");
    let flags = number("StateFlags");
    let pending = number("BytesToDownload").saturating_sub(number("BytesDownloaded"));
    Ok((
        name.into(),
        json!({"appId":id,"buildId":if build>0{build.to_string()}else{String::new()},"stateFlags":flags,"sizeBytes":number("SizeOnDisk"),"pendingBytes":pending,"library":library,"lastUpdated":number("LastUpdated"),"status":if pending>0{"Download pending in Steam"}else if flags & 2 !=0{"Update requested by Steam"}else{"Installed · managed by Steam"}}),
    ))
}
pub fn version_label(a: &App) -> String {
    if is_game(a) {
        if let Some(build) = a
            .extra
            .get("steam")
            .and_then(|s| s["buildId"].as_str())
            .filter(|s| !s.is_empty())
        {
            return format!("Build {build}");
        }
        return "Managed by Steam".into();
    }
    if crate::catalog::known_version(&a.version) {
        a.version.clone()
    } else {
        "Not reported".into()
    }
}
pub fn uri(id: Option<&str>) -> Option<String> {
    match id {
        None => Some("steam://nav/downloads".into()),
        Some(id) if !id.is_empty() && id.len() <= 12 && id.bytes().all(|b| b.is_ascii_digit()) => {
            Some(format!("steam://nav/games/details/{id}"))
        }
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_nested_manifest_and_build_without_fabricating_version() {
        let v=vdf(r#""AppState" { "appid" "42" "name" "Example" "buildid" "1234" "StateFlags" "6" "SizeOnDisk" "1024" "UserConfig" {"language" "english"} }"#).unwrap();
        let (_, s) = manifest(&v, "42", Path::new("C:/Steam")).unwrap();
        assert_eq!(s["buildId"], "1234");
        assert_eq!(s["status"], "Update requested by Steam");
        assert!(manifest(&v, "43", Path::new("C:/Steam")).is_err());
    }
    #[test]
    fn rejects_bad_vdf_and_protocol_injection() {
        assert!(vdf("\"x\" {\"y\" \"z\"").is_err());
        assert!(vdf("\"x\" \"1\" \"x\" \"2\"").is_err());
        assert!(uri(Some("42/../run")).is_none());
        assert_eq!(uri(None).unwrap(), "steam://nav/downloads");
    }
}
