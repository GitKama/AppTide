"""Check public runtime dependencies of the shipped Windows target against OSV."""
import json
import subprocess
import sys
import urllib.request
from pathlib import Path

root = Path(__file__).resolve().parents[1] / "rust"
tree = subprocess.check_output(
    ["cargo", "tree", "--locked", "--target", "x86_64-pc-windows-msvc", "-e", "normal",
     "--prefix", "none", "--format", "{p}"], cwd=root, text=True
)
packages = sorted({tuple(line.split()[:2]) for line in tree.splitlines() if line and not line.startswith("apptide-beta ")})
matches = []
for offset in range(0, len(packages), 100):
    group = packages[offset:offset + 100]
    queries = [{"package": {"ecosystem": "crates.io", "name": name}, "version": version.removeprefix("v")} for name, version in group]
    request = urllib.request.Request("https://api.osv.dev/v1/querybatch", data=json.dumps({"queries": queries}).encode(), headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(request, timeout=30) as response:
        results = json.load(response)["results"]
    for (name, version), result in zip(group, results, strict=True):
        for advisory in result.get("vulns", []):
            matches.append(f"{name} {version} {advisory['id']}")
print(f"Windows runtime: {len(packages)} versions checked, {len(matches)} advisory matches")
for match in matches:
    print(match)
sys.exit(bool(matches))
