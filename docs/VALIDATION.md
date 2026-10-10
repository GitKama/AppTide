# Initial alpha validation

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
