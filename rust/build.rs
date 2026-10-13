use std::{env, fs, path::PathBuf, process::Command};
fn main() {
    println!("cargo:rerun-if-changed=../assets/apptide.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let icon =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../assets/apptide.ico");
    let rc = out.join("apptide.rc");
    fs::write(
        &rc,
        format!(
            "1 ICON \"{}\"\n",
            icon.display().to_string().replace('\\', "/")
        ),
    )
    .unwrap();
    let res = out.join("apptide.res");
    let compiler = env::var_os("RC").map(PathBuf::from).unwrap_or_else(|| {
        let kits = PathBuf::from(
            env::var_os("ProgramFiles(x86)").unwrap_or_else(|| "C:/Program Files (x86)".into()),
        )
        .join("Windows Kits/10/bin");
        let mut versions: Vec<_> = fs::read_dir(kits)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path().join("x64/rc.exe"))
            .filter(|p| p.is_file())
            .collect();
        versions.sort();
        versions.pop().unwrap_or_else(|| "rc.exe".into())
    });
    assert!(
        Command::new(compiler)
            .arg("/nologo")
            .arg("/fo")
            .arg(&res)
            .arg(&rc)
            .status()
            .expect("Windows SDK resource compiler required (or set RC)")
            .success(),
        "Icon resource compilation failed"
    );
    println!("cargo:rustc-link-arg={}", res.display());
}
