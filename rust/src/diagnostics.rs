use crate::catalog::{Result, now};
use std::{fs, io::Write, path::Path};

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
        fs::write(&path, vec![b' '; 1024 * 1024]).unwrap();
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
