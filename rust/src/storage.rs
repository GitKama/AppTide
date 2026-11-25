//! Inspect AppTide-owned caches. Every deletion is tied to a reviewed file snapshot.
use crate::catalog::Result;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

#[derive(Clone, PartialEq)]
struct Candidate {
    path: PathBuf,
    bytes: u64,
    modified: SystemTime,
}
#[derive(Clone, Default)]
pub struct Preview {
    pub downloads: u64,
    pub updates: u64,
    pub history: u64,
    files: Vec<Candidate>,
}
impl Preview {
    pub fn bytes(&self) -> u64 {
        self.files.iter().map(|f| f.bytes).sum()
    }
    pub fn count(&self) -> usize {
        self.files.len()
    }
}
fn linked(m: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if m.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    m.file_type().is_symlink()
}
fn inspect(root: &Path, relative: &Path, preview: &mut Preview) -> Result<()> {
    let path = root.join(relative);
    let m = match fs::symlink_metadata(&path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.to_string()),
    };
    if linked(&m) || relative.components().count() > 5 {
        return Ok(());
    }
    if m.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            inspect(
                root,
                &relative.join(entry.map_err(|e| e.to_string())?.file_name()),
                preview,
            )?;
        }
        return Ok(());
    }
    if !m.is_file() {
        return Ok(());
    }
    let parts: Vec<_> = relative.iter().map(|p| p.to_string_lossy()).collect();
    let owned = match parts[0].as_ref() {
        "downloads" => {
            preview.downloads += m.len();
            parts.len() == 3
                && parts[1].starts_with("github-")
                && relative
                    .extension()
                    .and_then(|s| s.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("exe") || e.eq_ignore_ascii_case("msi"))
        }
        "updates" => {
            preview.updates += m.len();
            parts.len() == 3
                && parts[1].starts_with("stage-")
                && parts[2] == "release.zip"
                && root
                    .join(relative.parent().unwrap())
                    .join("window-ready")
                    .is_file()
        }
        _ => {
            preview.history += m.len();
            false
        }
    };
    let modified = m.modified().map_err(|e| e.to_string())?;
    if owned
        && modified
            .elapsed()
            .is_ok_and(|t| t >= Duration::from_secs(7 * 86400))
    {
        preview.files.push(Candidate {
            path: relative.into(),
            bytes: m.len(),
            modified,
        });
    }
    Ok(())
}
pub fn preview(directory: &Path) -> Result<Preview> {
    let root = directory.canonicalize().map_err(|e| e.to_string())?;
    let mut result = Preview::default();
    for category in ["downloads", "updates", "logs", "receipts", "batches"] {
        inspect(&root, Path::new(category), &mut result)?;
    }
    Ok(result)
}
pub fn clean(directory: &Path, reviewed: &Preview) -> Result<()> {
    let root = directory.canonicalize().map_err(|e| e.to_string())?;
    let fresh = preview(&root)?;
    for file in &reviewed.files {
        if fresh.files.contains(file) {
            let path = root.join(&file.path);
            if path.canonicalize().map_err(|e| e.to_string())? != path {
                return Err("Cache path changed; refresh the preview.".into());
            }
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleanup_revalidates_reviewed_files_and_preserves_history_and_staging() {
        let dir = std::env::temp_dir().join(format!(
            "apptide-storage-{}",
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        let installer = dir.join("downloads/github-42-1/setup.exe");
        let history = dir.join("receipts/operation.log");
        let pending = dir.join("updates/stage-1/release.zip");
        let old = SystemTime::now() - Duration::from_secs(9 * 86400);
        for f in [&installer, &history, &pending] {
            fs::create_dir_all(f.parent().unwrap()).unwrap();
            fs::write(f, b"fixture").unwrap();
            fs::File::options()
                .write(true)
                .open(f)
                .unwrap()
                .set_modified(old)
                .unwrap();
        }
        let p = preview(&dir).unwrap();
        assert_eq!(p.count(), 1);
        fs::write(&installer, b"changed after preview").unwrap();
        clean(&dir, &p).unwrap();
        assert!(installer.exists());
        fs::File::options()
            .write(true)
            .open(&installer)
            .unwrap()
            .set_modified(old)
            .unwrap();
        clean(&dir, &preview(&dir).unwrap()).unwrap();
        assert!(!installer.exists() && history.exists() && pending.exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
