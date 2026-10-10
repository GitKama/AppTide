use crate::catalog::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
const FILES: [&str; 4] = [
    "AppTide-Beta.exe",
    "README.md",
    "LICENSE.txt",
    "THIRD-PARTY-NOTICES.txt",
];
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Release {
    pub version: String,
    pub url: String,
    pub hash: String,
    pub size: u64,
}
fn version(s: &str) -> Option<(u32, u32, u32, u32)> {
    let s = s.strip_prefix('v').unwrap_or(s);
    let (core, beta) = match s.split_once("-beta.") {
        Some((c, b)) => (c, b.parse().ok()?),
        None => (s, u32::MAX),
    };
    let n: Vec<u32> = core
        .split('.')
        .map(str::parse)
        .collect::<std::result::Result<_, _>>()
        .ok()?;
    (n.len() == 3).then(|| (n[0], n[1], n[2], beta))
}
fn valid_hash(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
pub fn select(data: &Value, current: &str) -> Result<Option<Release>> {
    let current = version(current).ok_or("Invalid current AppTide version")?;
    let list = data.as_array().ok_or("Invalid GitHub releases response")?;
    let mut candidates = vec![];
    for r in list {
        if r["draft"] != false {
            continue;
        }
        let Some(tag) = r["tag_name"].as_str() else {
            continue;
        };
        let Some(v) = version(tag) else {
            continue;
        };
        if v <= current
            || (!env!("CARGO_PKG_VERSION").contains("-beta.") && r["prerelease"] == true)
        {
            continue;
        }
        let number = tag.strip_prefix('v').unwrap_or(tag);
        let name = format!("AppTide-{number}-windows-x64.zip");
        let expected = format!("https://github.com/GitKama/AppTide/releases/download/{tag}/{name}");
        let Some(asset) = r["assets"].as_array().and_then(|a| {
            a.iter()
                .find(|a| a["name"] == name && a["state"] == "uploaded")
        }) else {
            continue;
        };
        let Some(hash) = asset["digest"]
            .as_str()
            .and_then(|s| s.strip_prefix("sha256:"))
            .filter(|s| valid_hash(s))
        else {
            continue;
        };
        let Some(size) = asset["size"]
            .as_u64()
            .filter(|s| *s > 0 && *s <= 60 * 1024 * 1024)
        else {
            continue;
        };
        if asset["browser_download_url"] != expected {
            continue;
        }
        candidates.push((
            v,
            Release {
                version: number.into(),
                url: expected,
                hash: hash.to_lowercase(),
                size,
            },
        ));
    }
    Ok(candidates.into_iter().max_by_key(|c| c.0).map(|c| c.1))
}
pub fn check() -> Result<Option<Release>> {
    select(
        &crate::releases::api("/repos/GitKama/AppTide/releases?per_page=20")?,
        env!("CARGO_PKG_VERSION"),
    )
}
pub fn hash(path: &Path) -> Result<String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(fs::read(path).map_err(|e| e.to_string())?)
    ))
}
#[derive(Serialize, Deserialize)]
pub struct Plan {
    pub target: PathBuf,
    pub directory: PathBuf,
    pub parent: u32,
    pub old_hash: String,
    pub hashes: Vec<String>,
    pub demo: bool,
}
pub fn stage(release: &Release, directory: &Path) -> Result<PathBuf> {
    let target = std::env::current_exe().map_err(|e| e.to_string())?;
    if target.file_name().and_then(|s| s.to_str()) != Some(FILES[0]) {
        return Err("Self-update requires the portable AppTide-Beta.exe package. Open the GitHub release to download it.".into());
    }
    // Refresh the repository metadata before trusting an earlier UI offer.
    let fresh = check()?.ok_or("The update is no longer available")?;
    if fresh.version != release.version || fresh.hash != release.hash {
        return Err("Release changed. Check for updates again.".into());
    }
    let root = directory.join("updates").join(format!(
        "stage-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let payload = download(&fresh, &root)?;
    // Verify that the destination is writable before asking the running app to close.
    let probe = target.with_file_name(format!(".apptide-write-probe-{}", std::process::id()));
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .map_err(
            |_| "App folder is not writable. Extract the new release into a writable folder.",
        )?;
    fs::remove_file(&probe).map_err(|e| e.to_string())?;
    let plan = Plan {
        target: target.clone(),
        directory: directory.into(),
        parent: std::process::id(),
        old_hash: hash(&target)?,
        hashes: FILES
            .iter()
            .map(|f| hash(&payload.join(f)))
            .collect::<Result<_>>()?,
        demo: false,
    };
    fs::write(root.join("plan.json"), serde_json::to_vec(&plan).unwrap())
        .map_err(|e| e.to_string())?;
    fs::copy(&target, root.join("AppTide-Updater.exe")).map_err(|e| e.to_string())?;
    Ok(root.join("plan.json"))
}
fn download(fresh: &Release, root: &Path) -> Result<PathBuf> {
    let payload = root.join("payload");
    fs::create_dir(&payload).map_err(|e| e.to_string())?;
    use base64::Engine;
    let config=base64::engine::general_purpose::STANDARD.encode(serde_json::to_vec(&json!({"url":fresh.url,"hash":fresh.hash,"size":fresh.size,"zip":root.join("release.zip"),"payload":payload,"prefix":format!("AppTide-{}-windows-x64/",fresh.version),"files":FILES})).unwrap());
    let script = format!(
        r#"$ErrorActionPreference='Stop'; Add-Type -AssemblyName System.Net.Http; Add-Type -AssemblyName System.IO.Compression.FileSystem; $cfg=ConvertFrom-Json ([Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('{config}'))); $h=[Net.Http.HttpClientHandler]::new(); $h.AllowAutoRedirect=$false; $c=[Net.Http.HttpClient]::new($h); $c.Timeout=[TimeSpan]::FromSeconds(45); $c.DefaultRequestHeaders.UserAgent.ParseAdd('AppTide-SelfUpdate'); try {{ $url=[Uri]$cfg.url; for($redirect=0;$redirect -lt 6;$redirect++) {{ if($url.Scheme -ne 'https' -or $url.Host -notin @('github.com','release-assets.githubusercontent.com','objects.githubusercontent.com')){{throw 'Untrusted download host'}}; $r=$c.GetAsync($url,[Net.Http.HttpCompletionOption]::ResponseHeadersRead).GetAwaiter().GetResult(); if([int]$r.StatusCode -ge 300 -and [int]$r.StatusCode -lt 400){{$url=[Uri]::new($url,$r.Headers.Location);$r.Dispose();continue}}; break }}; if([int]$r.StatusCode -ne 200){{throw 'Download failed'}}; $stream=$r.Content.ReadAsStreamAsync().GetAwaiter().GetResult(); $out=[IO.File]::Create($cfg.zip); try {{$buffer=New-Object byte[] 65536;$total=0;while(($n=$stream.Read($buffer,0,$buffer.Length)) -gt 0){{$total+=$n;if($total -gt $cfg.size){{throw 'Download exceeds declared size'}};$out.Write($buffer,0,$n)}}}}finally{{$out.Dispose()}}; $sha=[Security.Cryptography.SHA256]::Create();$hs=[IO.File]::OpenRead($cfg.zip);try{{$actual=[BitConverter]::ToString($sha.ComputeHash($hs)).Replace('-','')}}finally{{$hs.Dispose();$sha.Dispose()}}; if($total -ne $cfg.size -or $actual -ine $cfg.hash){{throw 'Download integrity mismatch'}}; $z=[IO.Compression.ZipFile]::OpenRead($cfg.zip);try{{foreach($name in $cfg.files){{$entries=@($z.Entries|Where-Object {{$_.FullName -ceq ($cfg.prefix+$name)}});if($entries.Count -ne 1 -or $entries[0].Length -gt 52428800){{throw 'Unexpected archive entry'}};[IO.Compression.ZipFileExtensions]::ExtractToFile($entries[0],(Join-Path $cfg.payload $name),$false)}}}}finally{{$z.Dispose()}} }}finally{{$c.Dispose()}}"#
    );
    let out = crate::inventory::powershell(&script, 180)?;
    if out.code != 0 {
        let _ = fs::write(root.join("download-error.txt"), &out.stderr);
        #[cfg(test)]
        eprintln!("{}", out.stderr);
        return Err(
            "Update download or integrity verification failed. Current app is unchanged.".into(),
        );
    }
    let bytes = fs::read(payload.join(FILES[0])).map_err(|e| e.to_string())?;
    let offset = bytes
        .get(60..64)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize)
        .unwrap_or(usize::MAX);
    if bytes.get(..2) != Some(b"MZ")
        || offset > bytes.len().saturating_sub(6)
        || bytes.get(offset..offset + 6) != Some(b"PE\0\0\x64\x86")
    {
        return Err("Release does not contain a Windows x64 executable.".into());
    }
    Ok(payload)
}
pub fn launch_helper(plan: &Path) -> Result<()> {
    let helper = plan
        .parent()
        .ok_or("Invalid update staging path")?
        .join("AppTide-Updater.exe");
    let mut command = std::process::Command::new(helper);
    command.arg("--apply-self-update").arg(plan);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
        .spawn()
        .map_err(|e| format!("Could not start update helper: {e}"))?;
    Ok(())
}
fn replace(root: &Path, plan: &Plan) -> Result<PathBuf> {
    if plan.target.file_name().and_then(|s| s.to_str()) != Some(FILES[0])
        || plan.hashes.len() != FILES.len()
        || hash(&plan.target)? != plan.old_hash
    {
        return Err("Current executable changed; replacement cancelled.".into());
    }
    for (file, expected) in FILES.iter().zip(&plan.hashes) {
        if !valid_hash(expected) || hash(&root.join("payload").join(file))? != *expected {
            return Err("Staged file integrity mismatch; replacement cancelled.".into());
        }
    }
    let parent = plan.target.parent().ok_or("Invalid target directory")?;
    for file in FILES {
        let p = parent.join(file);
        if p.exists() && !p.is_file() {
            return Err(
                "A package filename is occupied by a directory; replacement cancelled.".into(),
            );
        }
    }
    let backup = parent.join(format!(
        "AppTide-backup-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    fs::create_dir(&backup).map_err(|e| e.to_string())?;
    let mut moved = vec![];
    let mut installed = vec![];
    let result = (|| -> Result<()> {
        for file in FILES {
            let target = parent.join(file);
            if target.exists() {
                fs::rename(&target, backup.join(file)).map_err(|e| e.to_string())?;
                moved.push(file);
            }
            // Copy within the destination volume, then atomically rename into the final filename.
            let temp = backup.join(format!("new-{file}"));
            fs::copy(root.join("payload").join(file), &temp).map_err(|e| e.to_string())?;
            fs::rename(temp, &target).map_err(|e| e.to_string())?;
            installed.push(file);
        }
        Ok(())
    })();
    if let Err(e) = result {
        for file in installed {
            let _ = fs::remove_file(parent.join(file));
        }
        for file in moved {
            let _ = fs::rename(backup.join(file), parent.join(file));
        }
        return Err(format!(
            "Replacement failed; rollback attempted: {e}. Backup: {}",
            backup.display()
        ));
    }
    Ok(backup)
}
pub fn apply(path: &Path) -> Result<()> {
    let root = path.parent().ok_or("Invalid update plan")?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    if fs::canonicalize(root).ok() != exe.parent().and_then(|p| fs::canonicalize(p).ok()) {
        return Err("Update helper and plan must share a staging folder".into());
    }
    let plan: Plan = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::{
            Foundation::CloseHandle,
            System::Threading::{OpenProcess, WaitForSingleObject},
        };
        let handle = OpenProcess(0x00100000, 0, plan.parent);
        if !handle.is_null() {
            let status = WaitForSingleObject(handle, 60000);
            CloseHandle(handle);
            if status != 0 {
                return Err("AppTide did not close; current executable preserved.".into());
            }
        }
    }
    let outcome = (|| -> Result<PathBuf> {
        let backup = replace(root, &plan)?;
        let mut launch = std::process::Command::new(&plan.target);
        launch.arg("--data-dir").arg(&plan.directory);
        if plan.demo {
            launch.arg("--demo");
        }
        if let Err(e) = launch.spawn() {
            for file in FILES {
                let target = plan.target.parent().unwrap().join(file);
                if backup.join(file).exists() {
                    let _ = fs::remove_file(&target);
                    let _ = fs::rename(backup.join(file), target);
                }
            }
            return Err(format!(
                "Restart failed; backup restored where possible: {e}"
            ));
        }
        Ok(backup)
    })();
    let receipt = match &outcome {
        Ok(backup) => {
            json!({"status":"replaced-and-relaunched","backup":backup,"at":crate::catalog::now()})
        }
        Err(e) => json!({"status":"failed","detail":e,"at":crate::catalog::now()}),
    };
    fs::write(
        plan.directory.join("self-update-result.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    if outcome.is_err() && hash(&plan.target).ok().as_deref() == Some(plan.old_hash.as_str()) {
        let mut launch = std::process::Command::new(&plan.target);
        launch.arg("--data-dir").arg(&plan.directory);
        if plan.demo {
            launch.arg("--demo");
        }
        let _ = launch.spawn();
    }
    outcome.map(|_| ())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Explicit opt-in network check against public AppTide release assets"]
    fn public_release_download_passes_integrity_and_archive_checks() {
        let data = crate::releases::api("/repos/GitKama/AppTide/releases?per_page=20").unwrap();
        let release = select(&data, "0.0.0")
            .unwrap()
            .expect("published native release");
        let root =
            std::env::temp_dir().join(format!("apptide-download-test-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let payload = download(&release, &root).unwrap();
        assert_eq!(hash(&root.join("release.zip")).unwrap(), release.hash);
        for file in FILES {
            assert!(payload.join(file).is_file());
            fs::remove_file(payload.join(file)).unwrap();
        }
        fs::remove_file(root.join("release.zip")).unwrap();
        fs::remove_dir(payload).unwrap();
        fs::remove_dir(root).unwrap();
    }
    #[test]
    fn release_selection_requires_new_version_exact_owner_asset_and_digest() {
        let asset = json!({"name":"AppTide-0.2.1-beta.2-windows-x64.zip","state":"uploaded","browser_download_url":"https://github.com/GitKama/AppTide/releases/download/v0.2.1-beta.2/AppTide-0.2.1-beta.2-windows-x64.zip","digest":format!("sha256:{}","a".repeat(64)),"size":100});
        let mut data =
            json!([{"tag_name":"v0.2.1-beta.2","draft":false,"prerelease":true,"assets":[asset]}]);
        assert!(select(&data, "0.2.1-beta.1").unwrap().is_some());
        assert!(select(&data, "0.3.0-beta.1").unwrap().is_none());
        data[0]["assets"][0]["browser_download_url"] = "https://example.org/app.zip".into();
        assert!(select(&data, "0.2.0-beta.2").unwrap().is_none());
        assert!(version("0.2.1").unwrap() > version("0.2.1-beta.99").unwrap());
        assert!(version("0.2.1-beta.10").unwrap() > version("0.2.1-beta.2").unwrap());
        assert!(version("../../bad").is_none());
    }
    #[test]
    fn verified_payload_replaces_only_package_files_and_keeps_backup() {
        let root = std::env::temp_dir().join(format!("apptide-self-test-{}", std::process::id()));
        let target = root.join("installed");
        fs::create_dir_all(&target).unwrap();
        fs::create_dir_all(root.join("payload")).unwrap();
        for f in FILES {
            fs::write(target.join(f), b"old").unwrap();
            fs::write(root.join("payload").join(f), b"new").unwrap();
        }
        fs::write(target.join("catalog.json"), b"private").unwrap();
        let mut plan = Plan {
            target: target.join(FILES[0]),
            directory: root.clone(),
            parent: 0,
            old_hash: "0".repeat(64),
            hashes: FILES
                .iter()
                .map(|f| hash(&root.join("payload").join(f)).unwrap())
                .collect(),
            demo: true,
        };
        assert!(replace(&root, &plan).is_err());
        assert_eq!(fs::read(&plan.target).unwrap(), b"old");
        plan.old_hash = hash(&plan.target).unwrap();
        let backup = replace(&root, &plan).unwrap();
        assert_eq!(fs::read(&plan.target).unwrap(), b"new");
        assert_eq!(fs::read(backup.join(FILES[0])).unwrap(), b"old");
        assert_eq!(fs::read(target.join("catalog.json")).unwrap(), b"private");
        for f in FILES {
            fs::remove_file(target.join(f)).unwrap();
            fs::remove_file(backup.join(f)).unwrap();
            fs::remove_file(root.join("payload").join(f)).unwrap();
        }
        fs::remove_file(target.join("catalog.json")).unwrap();
        for p in [backup, root.join("payload"), target, root] {
            fs::remove_dir(p).unwrap();
        }
    }
    #[cfg(windows)]
    #[test]
    fn locked_companion_rolls_back_the_executable() {
        use std::os::windows::fs::OpenOptionsExt;
        let root = std::env::temp_dir().join(format!("apptide-rollback-{}", std::process::id()));
        let target = root.join("installed");
        fs::create_dir_all(&target).unwrap();
        fs::create_dir_all(root.join("payload")).unwrap();
        for f in FILES {
            fs::write(target.join(f), b"old").unwrap();
            fs::write(root.join("payload").join(f), b"new").unwrap();
        }
        let lock = fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(target.join("README.md"))
            .unwrap();
        let plan = Plan {
            target: target.join(FILES[0]),
            directory: root.clone(),
            parent: 0,
            old_hash: hash(&target.join(FILES[0])).unwrap(),
            hashes: FILES
                .iter()
                .map(|f| hash(&root.join("payload").join(f)).unwrap())
                .collect(),
            demo: true,
        };
        assert!(replace(&root, &plan).is_err());
        assert_eq!(fs::read(&plan.target).unwrap(), b"old");
        drop(lock);
        for f in FILES {
            assert_eq!(fs::read(target.join(f)).unwrap(), b"old");
            fs::remove_file(target.join(f)).unwrap();
            fs::remove_file(root.join("payload").join(f)).unwrap();
        }
        for entry in fs::read_dir(&target).unwrap() {
            fs::remove_dir(entry.unwrap().path()).unwrap();
        }
        fs::remove_dir(target).unwrap();
        fs::remove_dir(root.join("payload")).unwrap();
        fs::remove_dir(root).unwrap();
    }
}
