# AppTide Rust beta

Version 0.2.7-beta.1 is the native Rust implementation alongside the Electron alpha.
The Rust beta has no JavaScript engine, browser renderer, Node runtime, or web server.

**Build status:** compiled and validated locally on Windows x64. Automated checks, Clippy with warnings denied, native desktop/compact screenshot checks, live read-only inventory, and alpha import checks passed. Real third-party updates and uninstalls have not been exercised; the operation tests are synthetic. See [validation](../docs/VALIDATION.md).

## Build and run

Windows x64, Rust 1.95 or newer, and the Microsoft C++ Build Tools with a Windows SDK:

```powershell
cd rust
cargo test
cargo run -- --demo
cargo build --release
.\target\release\apptide-beta.exe
```

WinGet must be available through Windows App Installer for WinGet operations. GitHub installers use their own verified release assets.
Download the beta ZIP from [Releases](https://github.com/GitKama/AppTide/releases/tag/v0.2.7-beta.1), extract it, and launch `AppTide-Beta.exe`. The generated EXE is portable. The development toolchain is not needed to run it. The MSVC runtime is statically linked; the executable imports only Windows system DLLs. The beta is unsigned.

## Current scope

- Offline public directory: 19,754 package records and 2,755 GitHub projects, with conservative installed identity matching. See [public catalogue](../docs/PUBLIC-CATALOGUE.md).
- Separate Steam library with local build IDs, disk use, manifest download state and controls to open Steam.
- Clear update diagnoses, source refresh and installer-prompts retry previews; asynchronous Store batches rechecked on later scans. See [diagnostics](../docs/UPDATE-DIAGNOSTICS.md).

- Native Rust registry inventory (machine/user, 32/64 bit), plus current-user AppX inventory through a fixed, read-only Windows PowerShell command.
- Microsoft Store update checks and confirmed bulk updates through the installed consumer Store CLI, with local logs and fresh installed-version verification. See [Store support and metadata](../docs/MICROSOFT-STORE.md).
- Friendly AppX names and publishers; bounded, read-only executable metadata fallbacks for incomplete desktop registrations.
- GitHub installer updates, metadata-based project detection, asset selection and opt-in unattended recipes. See [GitHub updates](../docs/GITHUB-UPDATES.md).
- Authored AppTide icon embedded in the executable, window and taskbar.
- WinGet discovery, individual/batch updates, and individual removals. A preview is followed by a fresh preflight, serialized operations, and inventory verification.
- Private schema-1 catalogue, version observations, pause preferences, bounded WinGet-log sightings, and receipts.
- A custom immediate-mode UI: virtualized app rows, a separate details scroll area, and update/uninstall controls outside both scrolling areas.
- Background worker for scans and installers; event-driven repainting while idle; AccessKit support and named keyboard-focusable controls.
- Isolated synthetic demo mode and demo-only screenshot capture.

Beta.2 adds a bundled structured recipe database, Store/Steam ownership detection, public GitHub release tracking, and coverage filters. See [catalogue design and sources](../docs/CATALOGUE.md).

Automatic WinGet and reviewed GitHub updates require both the global Settings switch and an individual app opt-in. Checks run hourly while the window is open, starting one hour after enabling. No service or startup task is installed. Unverified or failed receipts block automatic retries. These execution paths are tested with synthetic operations; real unattended installer compatibility remains unproven.

Add portable folders in Settings, then scan. Discovery reads executable metadata only, two subfolder levels, up to 10 roots / 1,000 directories / 10,000 files / 500 executables; it skips links and junctions. An inaccessible folder or exceeded bound preserves the previous inventory rather than claiming absent apps. Release watches can be edited/refreshed per app; stable release tags are compared conservatively and never become installer commands.

## Your catalogue and AppTide updates

In My catalogue, choose **Add custom app** to track a product manually. Select any app and choose **Edit my catalogue entry** to save its official HTTPS source, GitHub project URL, version (manual entries only), and update notes. These records remain private. Stop tracking retains history and never uninstalls software.

AppTide checks its own GitHub beta releases at startup; Settings also offers a manual check and opt-out. **Update AppTide and restart** verifies the release, retains the old package files and relaunches with the same private catalogue. See [self-update validation and limits](https://github.com/GitKama/AppTide/blob/main/docs/SELF-UPDATES.md).

## Data and migration

Beta data: `%LOCALAPPDATA%\AppTide-Beta`. Alpha data remains in `%LOCALAPPDATA%\AppTide`.

Before the beta's first scan, **Settings → Import alpha catalogue** copies supported schema-1 data. It never overwrites the alpha. Automatic execution consent is reset. Only one process can open a beta catalogue at a time. Saves flush a temporary file, retain a backup, then replace the catalogue atomically on Windows. Corrupt or unsupported catalogues produce an error instead of an empty reset.

Receipt logs remain private. Log sightings never authorize installation or become executable recipes. The beta does not start a service, register startup, accept installer agreements, bypass hashes, kill an installer, or silently retry a failed batch.

Uninstall verification is deliberately conservative: if WinGet omits or truncates table rows, a missing row cannot prove removal. In that case the receipt is **unverified**, even when WinGet exits successfully. Structured WinGet data remains a future improvement.

## Developer checks

```powershell
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
.\target\release\apptide-beta.exe --demo --screenshot desktop.png
.\target\release\apptide-beta.exe --demo --compact --scrolled --screenshot scrolled.png
```

For an explicitly requested, read-only live inventory check, use `--data-dir <private-folder> --scan-report <private-report.json>`. Never commit the resulting catalogue, inventory, or receipts. Screenshot capture refuses to run outside demo mode.

See [the implementation decision](../docs/RUST-BETA.md) for research and tradeoffs.

## Queue diagnostics

Activity shows a persistent queue summary and opens a built-in, read-only log viewer. `logs/app.jsonl` records application events (rotated at 1 MiB with one previous file); `receipts/*.log` records WinGet commands, exit codes, stdout and stderr; `batches/*.json` records reviewed plans and final outcomes, including preflight failures. These files stay private. Stale items are skipped individually; individual failed or unverified apps are recorded while other apps continue. Global scan/persistence failures, cancellation and reboot signals stop the queue. See [troubleshooting](../docs/TROUBLESHOOTING.md).
