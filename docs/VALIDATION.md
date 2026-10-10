# AppTide validation

## Rust 0.2.0-beta.1

Validated on Windows x64 on 10 October 2026 with Rust 1.99.0 and MSVC Build Tools 2022.

- 15 automated checks passed: parser rejection, exact arguments, duplicate identities, stale preflight, simulated update/removal, bounded log matching, catalogue history and extension preservation, atomic backup/recovery, corruption handling, exclusive writer lock, stdout/stderr separation, actual UI click-to-preview, and long-list action visibility at 1440×900 and 1040×650.
- `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, and optimized Windows compilation passed locally.
- Native render captures were visually inspected at 1440×900 and 1040×700. The compact capture scrolls far into 186 synthetic records; both action buttons remain visible. Public screenshots contain synthetic data only.
- A real read-only registry/AppX/WinGet scan completed. Unsupported WinGet rows were reported rather than inferred. Individual app inventories and logs remain private.
- Alpha catalogue import completed in a separate test directory. The alpha file's SHA-256 was unchanged across import and scan.
- PE dependency inspection showed only Windows system DLL imports after static CRT linking. This does not prove compatibility with every GPU/Windows configuration.
- No real third-party update or removal was performed. Installer/elevation/reboot behavior, signing, locale coverage and screen-reader behavior remain unproven. CLI omissions prevent claiming verified uninstall absence.

The Rust CI job repeats formatting, automated checks, Clippy and release compilation. Local results and remote CI status are distinct.

## Initial alpha validation

Validated locally on Windows x64 on 10 October 2026. No real installed application was updated or removed during this work.

## Automated checks

- 25 Node test-runner checks: table parsing, malformed/truncated input, command injection, exact argument construction, unknown versions, conservative matching, duplicate identities, durable identity across registry changes, catalogue persistence/backup/corruption, interrupted receipts, bounded log evidence, GitHub source validation, stale preflight, partial inventory, update/removal verification, exclusions, automatic allowlists, prototype safety, write serialization, and retry blocking.
- Electron desktop smoke checks: isolated renderer, update list, details, search empty state, narrow desktop overflow, preview, three simulated updates, and verified history receipts.
- The same synthetic desktop flow was exercised in the packaged Windows executable.
- Read-only inventory and WinGet scans ran from source and from the packaged Windows executable. Aggregate coverage and private catalogues were retained locally, not published.
- Desktop and compact screenshots were inspected. The public README screenshot is explicitly synthetic.
- Dependency installation reported zero known vulnerabilities at build time. This is not an external security audit.

## Unproven boundaries

Version 0.1.1 adds desktop regression coverage for 100 extra synthetic records: actions remain visible far down the list, selecting another app resets detail scrolling, and compact selection brings the action controls into view. Desktop tests use an isolated profile and require a passing receipt so an existing AppTide instance cannot silently consume the test launch.

Real installer/uninstaller behavior, elevation dialogs, restart handling, installer licensing, Windows locale coverage beyond parser fixtures, AppTide signing, and unattended scheduled-task operation have not been validated. GitHub release-source code is read-only and has no installer execution path. No external security review has been completed.

CI repeats unit tests, the synthetic desktop flow, and Windows packaging. CI completion is separate from local results and should be checked on the repository's Actions page.
