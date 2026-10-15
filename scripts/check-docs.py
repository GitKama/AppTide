"""Check current release references and local Markdown targets; no network needed."""
import re
import sys
import tomllib
from pathlib import Path
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]


def main():
    version = tomllib.loads((ROOT / "rust/Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    errors = []
    current = {
        "README.md": [f"Current release: **{version}**", f"releases/download/v{version}/AppTide-{version}-windows-x64.zip"],
        "rust/README.md": [f"Version {version}", f"releases/tag/v{version}"],
        "CHANGELOG.md": [f"## {version}"],
    }
    for filename, required in current.items():
        text = (ROOT / filename).read_text(encoding="utf-8")
        for item in required:
            if item not in text:
                errors.append(f"{filename}: missing current release reference: {item}")
    lock = tomllib.loads((ROOT / "rust/Cargo.lock").read_text(encoding="utf-8"))
    app = next(p for p in lock["package"] if p["name"] == "apptide-beta")
    if app["version"] != version:
        errors.append("rust/Cargo.lock: app version differs from Cargo.toml")

    files = sorted(set(ROOT.glob("*.md")) | set((ROOT / "docs").rglob("*.md"))
                   | set((ROOT / ".github").rglob("*.md")) | {ROOT / "rust/README.md"})
    checked = 0
    for path in files:
        # Skip fenced code examples; validate file targets, not heading fragments or remote URLs.
        text = re.sub(r"```.*?```", "", path.read_text(encoding="utf-8"), flags=re.S)
        for match in re.finditer(r"\[[^\]\n]*\]\(([^)\n]+)\)", text):
            target = match[1].strip()
            target = target[1:target.index(">")] if target.startswith("<") else target.split()[0]
            url = urlsplit(target)
            if url.scheme or url.netloc or not url.path:
                continue
            checked += 1
            resolved = (path.parent / unquote(url.path)).resolve()
            if not resolved.is_relative_to(ROOT) or not resolved.exists():
                errors.append(f"{path.relative_to(ROOT)}: broken local target: {target}")
    for error in errors:
        print(error, file=sys.stderr)
    print(f"Documentation: {len(files)} files, {checked} local links, release {version}; {len(errors)} errors")
    return int(bool(errors))


if __name__ == "__main__":
    sys.exit(main())
