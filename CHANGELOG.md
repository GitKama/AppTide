# Changes

## 0.2.6-beta.1 — 15 October 2026

- Embed a searchable metadata-only directory of 19,754 WinGet/Scoop package records and 2,755 distinct GitHub projects, with source revisions, licences and refresh scripts.
- Associate installed apps with public records using exact WinGet identity or name/publisher evidence; preserve manual choices and reject conflicting installed project URLs.
- Recover missing versions through bounded metadata reads using registered icon, directory and uninstaller-path hints; keep source labels and reject conflicting results.
- Add a separate Steam library: titles, build IDs, local download state, disk use and Steam navigation. Exclude games from desktop update/removal queues.
- Compare numeric version padding correctly and reconcile missing-version WinGet rows only with corroborated publisher evidence.
- Track accepted Store updates as pending, recheck installed versions later and prevent duplicate batch submission.
- Explain common package failures, add source refresh and offer reviewed interactive installer retries. Existing hash checks, exact targets and automatic-consent rules remain enforced.

## 0.2.5-beta.1 — 13 October 2026

- Turn compatible GitHub release watches into verified installer updates, individually or in Update all; add a chooser for multiple Windows installers.
- Detect GitHub projects from installed metadata and reviewed product/publisher recipes, preserving user overrides and disabled watches.
- Add separate automatic opt-in for reviewed current-user Meetily and ExifToolGUI installers. Generic EXE/MSI installers remain interactive.
- Compare ExifToolGUI installer revisions; bind previews to release assets and SHA256, verify installed versions, and retain private Activity receipts.
- Refresh old watch-only caches; use hourly caching with immediate manual refresh; continue other repositories after a failed check.
- Add an original teal A/wave icon to the executable, native window/taskbar and legacy packaging.


## 0.2.4-beta.1 — 11 October 2026

- Add a Microsoft Store page with read-only checks, a separate bulk-update preview/confirmation, a fresh preflight, private output logs and installed-version verification through the installed consumer Store CLI.
- Keep Store-owned batches separate from exact-version WinGet updates; explain dependencies, pause limitations, unavailable clients and uncertain outcomes. Never infer success solely from exit code zero.
- Resolve friendly AppX names and readable publisher metadata while preserving raw package identity. Show architecture and metadata provenance in app details.
- Fill missing registry metadata from matching local executable resources, with bounded discovery, helper/link exclusions and conflict rejection. Existing registry values remain authoritative.
- Pass inventory payloads through stdin to avoid Windows command-length limits; cover multiple-record PowerShell enumeration and preserve associations when only the publisher display name changes.
- Add regression coverage for metadata, Store parsing, stale/failed preflight, unchanged versions and actual confirmation clicks at desktop and compact sizes. Real Store installations remain a user acceptance check.

## 0.2.3-beta.1 - 11 October 2026

- Correct Update all semantics: one package download/installer failure no longer abandons unrelated updates. Keep individual failed/unverified receipts and return an aggregate attention result after attempting the remaining apps.
- Stop for failed verification scans, persistence problems, cancellation and shared installer/reboot signals; keep exact targets, hash checks and automatic retry blocks.
- Explain HTTP 404/403 download failures without guessing replacement installers.
- Replace the unreliable Explorer log-folder launch with a built-in, bounded, read-only log viewer, with file selection, refresh and copy-folder-path controls.
- Remove duplicate queue error banners. Add a 34-app regression with HTTP 404 at item four, plus scan/reboot/cancellation boundaries and a real-file UI log-viewer test.

## 0.2.2-beta.1 - 11 October 2026

- Fix silent queue termination: show a persistent summary with verified, skipped, uncertain and unattempted counts, plus the app and reason that stopped execution.
- Recheck each reviewed item against the refreshed inventory; skip stale targets without blocking unrelated updates or silently selecting another version. Keep stopping on failed or uncertain installer outcomes.
- Explain duplicate installed registrations with observed versions and WinGet exit codes. Preserve the distinction between installer success and verified installed state.
- Save private queue plans and outcomes, including preflight failures, and add bounded application event logs. Activity opens the local diagnostic folder.
- Add regression coverage for full Update all confirmation through the UI worker, partial batches, failed sources/installers, cancellation, duplicate registrations and log rotation.

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
