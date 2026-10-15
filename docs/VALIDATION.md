# AppTide validation

## Rust 0.2.7-beta.1

- 78 default Rust tests passed; three opt-in download/installer tests remain ignored. Formatting, Clippy with warnings denied and the optimized Windows build passed.
- The Store regression reproduces the installed consumer CLI’s `No updates found.` response. ANSI/BOM variants are accepted; nonzero exit and failure-like text remain rejected. The release executable completed a real read-only Store check with zero updates and no error.
- UI interaction tests locate the self-update action on all eight pages at 1440x900 and 1040x650, block it during a running operation, and dispatch the update command when idle. The test intercepts the command without downloading or installing software.
- Native synthetic self-update notices were visually checked at 1440x900 and 1040x700; the UI detector reported no findings.
- AppTide release checks run at startup by default and with the normal manual scan. The result and retry control are visible outside Settings. Existing opt-out preferences remain respected at startup.
- No third-party application installation or removal was needed for this hotfix.

## Rust 0.2.6-beta.1

- 76 default Rust tests passed; three opt-in download/installer tests remain ignored. Formatting, Clippy with warnings denied and the optimized Windows x64 build passed.
- New regressions cover public catalogue identity and publisher checks, project conflicts and manual overrides, Steam manifest parsing and URI validation, numeric zero-padding verification, recovered-version duplicate suppression, interactive retry arguments, and asynchronous Store completion with duplicate-submission blocking. Existing queue and UI interaction regressions still pass.
- The release binary completed a read-only scan against a separate private catalogue copy. Steam manifests supplied build IDs; executable metadata recovered missing desktop versions; public metadata added GitHub associations; equivalent/current targets were removed from update offers. The original live catalogue was not edited.
- Native demo screenshots were checked at 1440x900 and 1040x700 for the Steam library and public directory. Search ranking was corrected and the final captures rechecked. The UI detector reported no findings. Public screenshots contain synthetic inventory only.
- The public catalogue contains 15,645 WinGet IDs plus 4,109 Scoop records, with 2,755 distinct GitHub repositories after case folding. Source revisions, download hashes and upstream licences are preserved. Python helper syntax checks passed.
- No real third-party installation, removal, forced Steam update or security bypass was performed. Store completion and installer retries were regression-tested with synthetic operations; upstream download and uninstall failures still require their documented recovery actions.

## Rust 0.2.5-beta.1

- 66 default Rust tests, 25 legacy tests, formatting, Clippy with warnings denied and the optimized Windows x64 build passed. Three opt-in tests are skipped by default; the new public-download and synthetic-launcher tests were explicitly run and passed.

- Mixed-provider queue, asset ambiguity, changed hashes, repository metadata conflicts/overrides, exact installer revisions, identity isolation and automatic-consent regressions.
- Actual egui clicks at desktop and compact sizes open the GitHub update preview, confirm it and verify the simulated operation through the worker.
- A public Meetily installer was downloaded by the production downloader and verified with SHA256 and size checks without running it. A harmless synthetic GUI executable exercised interactive, NSIS and Inno launcher modes with its expected exit code.
- A fresh scan against a private catalogue copy completed. It discovered additional GitHub associations from installed metadata, enabled the known pending GitHub installer, and correctly marked the already-current installer revision. Private inventory was not published.
- Native desktop/compact demo captures and the icon extracted from the built executable were visually inspected. The UI detector returned no findings.
- Real third-party installations and unattended installer compatibility remain unverified; no installed apps were updated or removed during validation.


## Rust 0.2.4-beta.1

- 55 Rust tests passed; the opt-in public-download test remains ignored by default. Formatting, Clippy with warnings denied and the optimized Windows x64 build passed using the repository's static MSVC runtime configuration.
- Store tests cover wrapped table rows, count mismatches, truncated/unreadable output, unique native identity matching, changed/failed preflight, a zero exit with no version change, and verified version advances without losing the existing catalogue.
- Actual egui pointer events open and confirm the Store preview through the worker at 1440x900 and 1040x650. Native synthetic Store-page captures at 1440x900 and 1040x700 were visually checked.
- Metadata tests cover multiple PowerShell records, a 100,000-byte stdin payload, preserving populated registry fields, and retaining source associations when only a package's publisher display name changes.
- The release executable completed real read-only inventory and Store checks, plus a full scan against a separate private catalogue copy. Existing receipts and private history were retained. No real Store or WinGet update, uninstall, registration change, alias change or policy change was executed.
- Store installation remains a user acceptance check. The installed Store client is a preview CLI with human-readable output. Unsupported/localized tables fail explicitly; missing aliases offer the Store UI. Store marketing names can differ from AppX names, so some otherwise successful changes remain unverified. See [Store support](MICROSOFT-STORE.md).

## Rust 0.2.3-beta.1

- 44 Rust tests passed; the opt-in external download test remains ignored by default. Formatting, Clippy with warnings denied and the optimized Windows build passed.
- The critical regression models a 34-app queue: item four returns HTTP 404, its failed receipt is retained, and all 30 remaining items are attempted exactly once. Duplicate registrations remain unverified without blocking unrelated apps.
- Verification scan failure, cancellation/reboot signals (including a visible new version) still stop remaining installers. Automatic retry blocks remain intact.
- A real-file UI interaction test clicks View local logs and reads a temporary log without starting Explorer. Reader checks cover traversal rejection and a visible bounded-read notice; existing rotation checks still pass.
- Native log-viewer captures at 1440x900 and 1040x700 were visually inspected. All public fixtures and captures are synthetic. Real local diagnostic evidence stayed private.
- The public installer URL identified during diagnosis returned HTTP 404 on a read-only HEAD request. No third-party update, removal, alternative installer or hash bypass was executed as part of this fix. The regression proves queue behavior, not that an upstream vendor download was repaired.

## Rust 0.2.2-beta.1

Validated locally on Windows x64 on 11 October 2026.

- 40 Rust tests passed; the separate opt-in network download test remains ignored by default. Formatting, Clippy with warnings denied, and optimized release compilation passed.
- An actual egui pointer sequence at compact size clicked Update all, confirmed the three-app preview, ran the simulated operations through the worker and retained the completed summary.
- Queue regression checks cover a changed target, a later app changed by an earlier operation, duplicate installed registrations after successful execution, source/preflight failure persisted to disk, installer failure and cancellation. Skipped receipts do not clear a previous automatic retry block.
- Application diagnostic JSON-line output and bounded rotation were checked. Public fixtures contain synthetic metadata only.
- Native Activity captures at 1440x900 and 1040x700 were inspected: summary, reason, log access and receipt details remain readable, with lower history scrolling independently.
- The packaged-code release executable completed a read-only live scan against a copy of the private catalogue. Real diagnostic files were reviewed locally; none were published. No third-party installation, removal, registry cleanup or forced retry was performed during this fix.
- Historical receipts lacked batch IDs, so they cannot prove the precise membership of a past queue. Duplicate registrations still produce an uncertain result rather than a fabricated success. See [troubleshooting and RCA](TROUBLESHOOTING.md).

## Rust 0.2.1-beta.1

- 32 default Rust tests passed; one additional network test passed when explicitly enabled. Default CI skips external download testing. Formatting, Clippy and release compilation passed locally.
- Catalogue editor interaction was exercised at the minimum window size through actual egui pointer events and the worker; manual identity/version persistence and rejected sources were tested.
- The production self-update download function fetched a published AppTide asset, verified its GitHub SHA-256 and extracted the exact permitted package files.
- A disposable beta.2 package was closed, replaced with 0.2.1 and relaunched through the helper. The resulting executable hash matched, one new process was running, a backup existed, and a private catalogue sentinel was unchanged.
- Locked-companion rollback and rejection of changed target hashes were tested. A future release initiated through the production update banner remains unexercised; these are separate download and handoff checks. See SELF-UPDATES.md for boundaries.

## Rust 0.2.0-beta.2

- 25 Rust tests, formatting, Clippy with warnings denied, and optimized Windows build passed. New tests cover strict recipe matching, Store signature kinds, mirrored registry views, consent and retry blocking, cached release comparisons, history migration, and native custom-route layout.
- A real read-only scan against a copy of the private catalogue identified additional update routes. The active catalogue was not used as the test output; raw inventory and paths remain private. Both initial GitHub watch providers returned valid stable release data.
- Portable discovery was exercised with copied test executables in a folder containing an apostrophe and three nested levels; only the allowed levels were read and no executable was launched.
- Native demo captures at desktop and compact sizes were inspected. Update/remove controls remain visible; public screenshots use synthetic data only.
- Automatic scheduling policy and simulated WinGet operations were tested separately. Real unattended installers, UAC, reboot behavior, vendor update execution and arbitrary asset installation were not exercised. Custom routes are guidance and watch-only sources.

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
