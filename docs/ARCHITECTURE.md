# Architecture

```text
Windows registry + current-user AppX + selected portable folders
                            |
                       observations
                            v
WinGet list ----------> local catalogue <---------- bounded log sightings
                            ^
                    user-added release watches
                            |
                     operation preview
                            v
                 fresh inventory / WinGet preflight
                            v
                 exact ID + version operation queue
                            v
                   fresh post-operation scan
                            v
               verified / failed / unverified receipt
```

`core/service.cjs` owns orchestration and mutation serialization. `core/catalog.cjs` manages atomic JSON persistence with a previous-file backup. `core/winget.cjs` owns parsing and argument construction. `scripts/inventory.ps1` only reads local software records and executable metadata. `desktop/main.cjs` exposes a fixed IPC API to an isolated, sandboxed renderer.

## Trust boundaries

1. Renderer text is escaped and cannot run Node.js. CSP blocks network access in the renderer.
2. IPC accepts only the application main frame. Operations are constructed in the main process from current catalogue identities, never from renderer-supplied commands.
3. A preview token is single-use and expires after ten minutes. Native confirmation names actual packages before a user-requested operation.
4. Preflight rejects stale versions, changed targets, ambiguous entries, exclusions, and protected components.
5. Child processes use argument arrays without a shell. Registry uninstall strings, log lines, and URLs never become commands.
6. No force, reboot, purge, hash-bypass, or security-bypass switches are added. No EULA is automatically accepted.
7. Exit zero is not verification. Updates require observing the requested version; removals require a successful scan showing absence. Failed/incomplete scans produce uncertainty.
8. Automatic updates are confined to explicitly allowed apps while the app is open; uncertain receipts block unattended retries.

## Persistence and privacy

The main document stores schema version, scans, app records, version observations, preferences, bounded log signals, and the last 1,000 receipts. Raw operation output is stored separately on the local machine. Catalogue writes replace a temporary file and retain the previous valid document as `.bak`; a corrupt document is preserved and reported, never silently discarded.

The alpha relies on the current user's Windows profile permissions; it does not encrypt the catalogue. Raw receipt files currently have no automatic retention cleanup. File export and cloud synchronization are deliberately absent. Git ignores local catalogues, receipt directories, logs, build artifacts, and secrets.

## Identity and matching

Only a unique normalized-name plus exact-version match attaches registry metadata to a WinGet row. The actionable identity remains WinGet's exact package ID. Ambiguous native records remain separate; duplicated WinGet IDs are blocked from operations. Matched catalogue UIDs derive from the package ID so a changed uninstall registry key does not erase history or preferences.

This is conservative correlation, not proof that records with similar names are the same product. A package can remain as both a Windows inventory record and a WinGet record. Manual deduplication and explicit association provenance are future work.
