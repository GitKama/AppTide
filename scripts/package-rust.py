"""Package an already-built Rust beta, with dependency licence texts and hashes."""
import argparse
import hashlib
import json
import pathlib
import shutil
import subprocess
import zipfile

parser = argparse.ArgumentParser()
parser.add_argument("--cargo", default="cargo")
parser.add_argument("--out", required=True)
args = parser.parse_args()
repo = pathlib.Path(__file__).resolve().parents[1]
rust = repo / "rust"
metadata = json.loads(subprocess.check_output(
    [args.cargo, "+stable", "metadata", "--format-version=1", "--locked", "--filter-platform", "x86_64-pc-windows-msvc"], cwd=rust
))
root = metadata["resolve"]["root"]
packages = {p["id"]: p for p in metadata["packages"]}
app = packages[root]
name = f"AppTide-{app['version']}-windows-x64"
out = pathlib.Path(args.out).resolve()
folder = out / name
folder.mkdir(parents=True, exist_ok=False)
shutil.copy2(rust / "target/release/apptide-beta.exe", folder / "AppTide-Beta.exe")
shutil.copy2(repo / "LICENSE", folder / "LICENSE.txt")
shutil.copy2(rust / "README.md", folder / "README.md")
notices = ["AppTide Rust beta - third-party dependency notices\n", "Segoe UI is read from Windows at runtime and is not redistributed.\n"]
notices.append("Where a crate offers MIT OR Apache-2.0 and does not ship licence files, this distribution selects Apache-2.0. Standard texts follow.\n")
for file in sorted((repo / "docs/licenses").glob("*.txt")):
    notices.append(f"\n--- {file.name} ---\n" + file.read_text(encoding="utf-8"))
for node in sorted(metadata["resolve"]["nodes"], key=lambda n: n["id"]):
    if node["id"] == root:
        continue
    package = packages[node["id"]]
    directory = pathlib.Path(package["manifest_path"]).parent
    notices.append(f"\n{'='*72}\n{package['name']} {package['version']}\nAuthors: {', '.join(package.get('authors', []))}\nLicence: {package.get('license') or 'See licence text'}\nSource: {package.get('repository') or 'https://crates.io/crates/'+package['name']}\n")
    files = {p for pattern in ["LICENSE*", "LICENCE*", "COPYING*", "NOTICE*"] for p in directory.glob(pattern) if p.is_file()}
    if package.get("license_file"):
        files.add(directory / package["license_file"])
    if package["name"] == "epaint_default_fonts":
        files.update(directory.rglob("*.txt"))
    for file in sorted(files):
        notices.append(f"\n--- {file.name} ---\n" + file.read_text(encoding="utf-8", errors="replace"))
(folder / "THIRD-PARTY-NOTICES.txt").write_text("\n".join(notices), encoding="utf-8")
archive = out / (name + ".zip")
if archive.exists():
    raise RuntimeError("Archive already exists; refusing to replace it.")
with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
    for file in folder.iterdir():
        z.write(file, arcname=name + "/" + file.name)
digest = hashlib.sha256(archive.read_bytes()).hexdigest()
archive.with_suffix(".zip.sha256").write_text(f"{digest}  {archive.name}\n", encoding="ascii")
print(json.dumps({"archive": str(archive), "bytes": archive.stat().st_size, "sha256": digest, "exeBytes": (folder / "AppTide-Beta.exe").stat().st_size}))
