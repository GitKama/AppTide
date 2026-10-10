# Changes

## 0.2.1-beta.1 - 10 October 2026

- Add a private catalogue editor for official sources, GitHub projects and update notes. Users can add, rename, version and stop tracking manual entries.
- Expand bundled recipes with 7-Zip, VLC, KeePass and Notepad++. Accept GitHub project URLs in watch fields and allow disabling bundled watches.
- Check AppTide releases asynchronously at startup, with beta-aware version ordering, a manual check and an opt-out.
- Add verified self-update downloads, bounded archive extraction, a close/replace/relaunch helper, package backups and rollback on replacement failure. Private catalogue files are excluded from replacement.
- Validate the production download function against a real published asset and the restart handoff against disposable package copies.
- Preserve release tags, changelogs and prior assets; document versioning and add Rust CI caching.

## 0.2.0-beta.2 - 10 October 2026

- Embed ten reviewed product/family recipes and two channel rules for update ownership beyond WinGet. Keep official provenance and private observations separate.
- Add Beyond WinGet, Needs research and Release watches filters, with fixed primary actions for official instructions/releases.
- Track public GitHub stable releases with bounded requests, caching and conservative version comparison; allow per-app watch editing.
- Deduplicate identical current-user registry mirrors and preserve history when a native identity gains WinGet correlation. Revoke automatic consent on publisher changes.
- Add hourly opt-in WinGet updates while open, failed/unverified retry blocking, and bounded portable-folder discovery.
- Add tests for consent, cache invalidation, metadata discovery, identity rules and custom-route layout. No real app was updated or removed during validation.

## 0.2.0-beta.1 — 10 October 2026

- Add a native Rust application with a custom egui/eframe immediate-mode interface; preserve the Electron alpha alongside it.
- Virtualize long app lists and keep update/uninstall controls outside both scrolling areas.
- Port registry inventory, WinGet operations, catalogue history, exclusions, log sightings, previews, preflight checks and receipts to Rust. AppX uses a fixed read-only Windows command.
- Separate the beta catalogue from the alpha, provide import, retain backups and prevent simultaneous writers.
- Separate process stdout from stderr so PowerShell diagnostic messages cannot corrupt inventory JSON.
- Statically link the MSVC runtime for a portable executable using only Windows system DLL imports.
- Add 15 Rust checks and Windows CI. Validate native screenshots, a live read-only scan, and an alpha import that preserves the original file hash.
- This first Rust beta does not yet enable automatic updates, portable discovery or release-watch editing. Real installer and uninstaller behavior remains unvalidated.

## 0.1.1 alpha — 10 October 2026

- Keep the application details pane visible while scrolling a long inventory. Additional details scroll independently.
- Reset the pane to its actions when another app is selected. On compact windows, bring the stacked details into view.
- Display the running application version in the sidebar.
- Exercise scrolling and selection with 100 extra synthetic app records in desktop tests.
- Isolate desktop-test profiles from running AppTide windows and require a passing test receipt, preventing a second-instance handoff from silently skipping checks.

## 0.1.0 alpha — 10 October 2026

Initial Windows inventory, WinGet operations, local catalogue, source watches, and desktop application.
