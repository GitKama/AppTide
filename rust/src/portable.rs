use crate::catalog::{App, Result};
use std::path::Path;
pub fn validate_folder(path: &str) -> Result<()> {
    let p = Path::new(path);
    if !p.is_absolute() || !p.is_dir() || p.parent().is_none() || path.starts_with("\\\\") {
        return Err("Choose a local app folder, not a drive root or network share.".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if std::fs::symlink_metadata(p)
            .map_err(|e| e.to_string())?
            .file_attributes()
            & 0x400
            != 0
        {
            return Err("Choose the actual folder, not a junction or link.".into());
        }
    }
    Ok(())
}
pub fn scan(folders: &[String]) -> Result<Vec<App>> {
    if folders.is_empty() {
        return Ok(vec![]);
    }
    if folders.len() > 10 {
        return Err("Limit portable discovery to 10 folders.".into());
    }
    for folder in folders {
        validate_folder(folder)?;
    }
    use base64::Engine;
    let encoded =
        base64::engine::general_purpose::STANDARD.encode(serde_json::to_vec(folders).unwrap());
    // Literal paths arrive as base64 JSON. No executable is launched. A bounded breadth-first walk
    // ignores reparse points and reads PE version resources only, never user documents.
    let script = format!(
        r#"$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); $roots=ConvertFrom-Json ([Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('{encoded}'))); $q=[Collections.Generic.Queue[object]]::new(); foreach($r in $roots){{$q.Enqueue(@{{path=$r;depth=0}})}}; $apps=[Collections.Generic.List[object]]::new(); $seen=[Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase); $files=0; $dirs=0; while($q.Count -gt 0){{$entry=$q.Dequeue(); $dirs++; if($dirs -gt 1000){{throw 'Portable scan folder limit exceeded'}}; foreach($f in Get-ChildItem -LiteralPath $entry.path -Force -ErrorAction Stop){{if(($f.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0){{continue}}; if($f.PSIsContainer){{if($entry.depth -lt 2){{$q.Enqueue(@{{path=$f.FullName;depth=($entry.depth+1)}})}}; continue}}; $files++; if($files -gt 10000){{throw 'Portable scan file limit exceeded'}}; if($f.Extension -ine '.exe' -or -not $seen.Add($f.FullName)){{continue}}; if($apps.Count -ge 500){{throw 'Portable scan executable limit exceeded'}}; $v=[Diagnostics.FileVersionInfo]::GetVersionInfo($f.FullName); $name=$v.ProductName; if([string]::IsNullOrWhiteSpace($name)){{$name=$f.BaseName}}; $apps.Add(@{{key=('portable:'+$f.FullName.ToLowerInvariant());name=$name;version=[string]$v.ProductVersion;publisher=[string]$v.CompanyName;location=$f.FullName;kind='portable';scope='user';metadataSources=@{{name='Executable ProductName';publisher='Executable CompanyName';version='Executable ProductVersion'}}}})}}}}; ConvertTo-Json -InputObject @($apps.ToArray()) -Depth 4 -Compress"#
    );
    let out = crate::inventory::powershell(&script, 60)?;
    if out.code != 0 {
        return Err("Portable discovery incomplete: folder inaccessible or scan limit reached. Choose smaller folders; previous inventory preserved.".into());
    }
    serde_json::from_str(out.stdout.trim().trim_start_matches('\u{feff}'))
        .map_err(|e| format!("Portable metadata unreadable: {e}"))
}
#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn explicit_folder_reads_metadata_without_executing_and_respects_depth() {
        let dir =
            std::env::temp_dir().join(format!("apptide-portable-{}-quote's", std::process::id()));
        std::fs::create_dir_all(dir.join("child/grand/deeper")).unwrap();
        let exe = std::env::current_exe().unwrap();
        let paths = [
            "tool.exe",
            "child/tool.exe",
            "child/grand/tool.exe",
            "child/grand/deeper/tool.exe",
        ];
        for path in paths {
            std::fs::copy(&exe, dir.join(path)).unwrap();
        }
        let apps = scan(&[dir.display().to_string()]).unwrap();
        assert_eq!(apps.len(), 3);
        assert!(
            apps.iter()
                .all(|a| a.kind == "portable" && a.winget_id.is_none())
        );
        for path in paths {
            std::fs::remove_file(dir.join(path)).unwrap();
        }
        for path in ["child/grand/deeper", "child/grand", "child", ""] {
            std::fs::remove_dir(dir.join(path)).unwrap();
        }
        assert!(validate_folder(r"C:\").is_err());
        assert!(validate_folder(r"relative\folder").is_err());
    }
}
