use crate::catalog::{Result, now};
use std::{fs, io::Write, path::Path};

pub fn log_files(directory: &Path) -> Result<Vec<String>> {
    let root = directory
        .canonicalize()
        .map_err(|e| format!("Cannot read data folder: {e}"))?;
    let mut files = Vec::new();
    for name in ["logs", "receipts", "batches"] {
        let folder = root.join(name);
        if !folder.exists() {
            continue;
        }
        let resolved = folder.canonicalize().map_err(|e| e.to_string())?;
        if !resolved.starts_with(&root) {
            continue;
        }
        for entry in fs::read_dir(folder)
            .map_err(|e| e.to_string())?
            .take(10000)
            .flatten()
        {
            let path = entry.path();
            if !matches!(
                path.extension().and_then(|s| s.to_str()),
                Some("log" | "json" | "jsonl")
            ) {
                continue;
            }
            if let Ok(meta) = entry.metadata()
                && meta.is_file()
            {
                files.push((
                    meta.modified().ok(),
                    format!("{name}/{}", entry.file_name().to_string_lossy()),
                ));
            }
        }
    }
    files.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    Ok(files.into_iter().take(100).map(|(_, name)| name).collect())
}

pub fn read_log(directory: &Path, relative: &str) -> Result<String> {
    use std::io::Read;
    let parts: Vec<_> = Path::new(relative).components().collect();
    if parts.len() != 2
        || !parts
            .iter()
            .all(|p| matches!(p, std::path::Component::Normal(_)))
        || !matches!(
            parts[0].as_os_str().to_str(),
            Some("logs" | "receipts" | "batches")
        )
    {
        return Err("Only AppTide diagnostic files can be opened here.".into());
    }
    let root = directory.canonicalize().map_err(|e| e.to_string())?;
    let path = root
        .join(relative)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !path.starts_with(&root)
        || !matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("log" | "json" | "jsonl")
        )
    {
        return Err("Log must remain inside the AppTide data folder.".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(256 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let truncated = bytes.len() > 256 * 1024;
    bytes.truncate(256 * 1024);
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    if truncated {
        text.push_str("\n[Showing the first 256 KiB; full file remains on disk.]");
    }
    Ok(text)
}

// Private, bounded application events; inventories and installer output belong in their own files.
pub fn record(directory: &Path, event: &str, detail: &str) -> Result<()> {
    let folder = directory.join("logs");
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let path = folder.join("app.jsonl");
    if fs::metadata(&path).is_ok_and(|m| m.len() >= 1024 * 1024) {
        fs::copy(&path, folder.join("app.previous.jsonl")).map_err(|e| e.to_string())?;
        fs::File::create(&path).map_err(|e| e.to_string())?;
    }
    let detail: String = detail.chars().take(4000).collect();
    let line = serde_json::json!({"at":now(),"version":env!("CARGO_PKG_VERSION"),"event":event,"detail":detail});
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    writeln!(file, "{line}").map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostic_events_are_json_lines_and_rotate() {
        let dir = std::env::temp_dir().join(format!(
            "apptide-log-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        record(&dir, "test", "line one\nline two").unwrap();
        let path = dir.join("logs/app.jsonl");
        let text = fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(), 1);
        let line: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(line["detail"], "line one\nline two");
        assert_eq!(log_files(&dir).unwrap(), vec!["logs/app.jsonl"]);
        assert!(
            read_log(&dir, "logs/app.jsonl")
                .unwrap()
                .contains("line one")
        );
        assert!(read_log(&dir, "../catalog.json").is_err());
        assert!(read_log(&dir, "catalog.json").is_err());
        fs::write(&path, vec![b' '; 1024 * 1024]).unwrap();
        assert!(
            read_log(&dir, "logs/app.jsonl")
                .unwrap()
                .ends_with("full file remains on disk.]")
        );
        record(&dir, "rotated", "new event").unwrap();
        assert_eq!(
            fs::metadata(dir.join("logs/app.previous.jsonl"))
                .unwrap()
                .len(),
            1024 * 1024
        );
        assert!(fs::metadata(&path).unwrap().len() < 1000);
        fs::remove_file(path).unwrap();
        fs::remove_file(dir.join("logs/app.previous.jsonl")).unwrap();
        fs::remove_dir(dir.join("logs")).unwrap();
        fs::remove_dir(dir).unwrap();
    }
}
