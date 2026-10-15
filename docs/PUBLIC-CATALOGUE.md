# Public app directory and Steam library

AppTide 0.2.6 embeds 19,754 public package records: 15,645 WinGet IDs and 4,109 records from Scoop Main and Extras. The snapshot contains 2,755 distinct GitHub projects, ignoring repository-name casing. It covers common desktop software, developer tools, editions and portable builds. It is not a popularity ranking or a claim that every entry is a unique installed application.

## Recognition and project association

The app directory is searchable offline. Its records contain names, publishers, public websites, project repositories and descriptions. Installer commands, Scoop hooks, scripts and installer URLs are not imported into AppTide's execution engine.

An existing WinGet ID can match its public record directly. An unmanaged installation needs an exact normalized product name and corroborating publisher. Automatic GitHub association prioritizes installed project URLs, then reviewed recipes, then the public catalogue. Conflicting installed URLs block association; manual project choices and disabled watches take precedence. Name-only matches do not authorize installers.

AppTide still reads installed versions from the PC. A public catalogue version is never substituted for a missing installed version. WinGet remains the preferred update provider where already matched. GitHub updates still require compatible release assets, SHA256 verification, a preview, and installed-version verification. A larger recognition catalogue does not mean every application supports automatic updating.

## Missing version recovery

Registry icon, install-directory and uninstaller-path hints lead to a bounded read-only metadata search. Uninstall commands are never executed during discovery. The search skips links, junctions, system-wide roots and setup/uninstaller binaries. Product name and vendor corroborate a match; when ProductName is absent, an exact executable basename plus the registered vendor can provide evidence. Conflicting fallback versions remain unresolved. Numeric Windows version-resource commas are normalized to dots.

Metadata sources are visible on the app profile. Apps that report no usable version say **Not reported**. AppTide does not invent a version from the newest public release or a launcher build hash.

## Steam

**Steam library** reads `libraryfolders.vdf` and `appmanifest_*.acf` from local Steam libraries. It shows titles, Steam build IDs, size on disk, local library and the download state recorded by Steam. Windows paths are deduplicated without regard to case or slash style. Missing manifests clear stale build metadata.

Steam entries are separated from desktop update/removal lists. **Manage in Steam** opens the game's library page; **Open Steam Downloads** opens downloads. These actions do not force an update, start a game or uninstall anything. Build IDs are explicitly labelled; local manifests do not prove that the installed build is the newest online release. Steam remains responsible for downloads, scheduling and removal.

## Maintain the snapshot

Run from the repository root with Python 3, without extra packages:

```powershell
python scripts/download-community-sources.py --out C:/scratch/apptide-public-sources
python scripts/build-community-catalog.py --sources C:/scratch/apptide-public-sources
```

The download helper reads public metadata only. The builder enriches a bounded selection of WinGet locale manifests and records source revisions and SHA256 hashes in `rust/data/community.json`. Review the diff and attribution before releasing. Downloads and intermediate indexes belong outside the repository; only the generated public metadata and licences are shipped. AppTide updates bring new snapshots; personal inventory is never uploaded to build them.

## Sources and attribution

- [Microsoft WinGet community manifests](https://github.com/microsoft/winget-pkgs), MIT licence; public source index at `https://cdn.winget.microsoft.com/cache/source.msix`.
- [Scoop Main](https://github.com/ScoopInstaller/Main) and [Scoop Extras](https://github.com/ScoopInstaller/Extras), Unlicense. [Manifest format](https://github.com/ScoopInstaller/Scoop/wiki/App-Manifests) informs recognition metadata extraction.
- [UniGetUI](https://github.com/Devolutions/UniGetUI) illustrates separating provider operations from the package-management interface. No application code was copied.
- [Steam browser protocol](https://developer.valvesoftware.com/wiki/Steam_browser_protocol) and Steam's local manifests inform the handoff to its client.

Licence texts are in `docs/licenses/` and the packaged third-party notices. Each application retains its own licence; catalogue inclusion does not make proprietary software open source or free of charge.
