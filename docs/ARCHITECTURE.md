# Native Rust architecture

This describes the active 0.2.x Rust beta. The original Electron implementation remains in the [0.1.1 alpha tag](https://github.com/GitKama/AppTide/tree/v0.1.1-alpha); its renderer/IPC model does not describe the native app.

## Data and operations

```text
Registry + AppX + selected portable folders + Steam manifests
                            |
                       observations
                            v
WinGet CLI ----------> private catalogue <---------- bounded log sightings
                            ^
                 reviewed recipes + public metadata
                            |
                preview and fresh provider preflight
                            v
             serialized WinGet / GitHub operation queue
                            v
                   fresh installed-state check
                            v
                verified / failed / unverified receipt

Microsoft Store: separate reviewed batch -> Store client -> pending / verification
AppTide itself: GitHub release -> verified package -> replace / restart / rollback
Steam: overview and handoff to the Steam client; no AppTide game updater
```

| Module | Responsibility |
| --- | --- |
| [`ui.rs`](../rust/src/ui.rs) | egui layout, user previews, worker messages and visible progress; no web renderer |
| [`service.rs`](../rust/src/service.rs) | Serialized operations, fresh scans, consent, cancellation, receipts and pending Store reconciliation |
| [`catalog.rs`](../rust/src/catalog.rs) | Private records, identity/history, version comparison and atomic persistence |
| [`inventory.rs`](../rust/src/inventory.rs), [`portable.rs`](../rust/src/portable.rs) | Registry/AppX and bounded executable metadata discovery |
| [`winget.rs`](../rust/src/winget.rs) | Conservative display parsing, exact argument arrays and failure/queue-stop classification |
| [`knowledge.rs`](../rust/src/knowledge.rs), [`community.rs`](../rust/src/community.rs), [`releases.rs`](../rust/src/releases.rs) | Reviewed recognition, metadata directory matching and cached GitHub associations |
| [`github_update.rs`](../rust/src/github_update.rs) | Compatible installer selection, digest/size checks and execution policy |
| [`store.rs`](../rust/src/store.rs), [`steam.rs`](../rust/src/steam.rs) | Store CLI checks and identity verification; local Steam manifest inventory |
| [`self_update.rs`](../rust/src/self_update.rs) | AppTide release selection, package staging, helper handoff and rollback |
| [`diagnostics.rs`](../rust/src/diagnostics.rs) | Bounded local events and log reading |

## Trust boundaries

1. The native UI sends typed worker commands. Previews retain identities and target versions and expire after ten minutes; fresh preflight rejects changed or ambiguous targets.
2. Child processes receive argument arrays. Registry uninstall strings, public metadata and log text never become shell commands.
3. WinGet uses exact IDs and reviewed versions. GitHub updates additionally bind the preview to compatible assets and integrity metadata; Store batches have their own fresh preview and verification rules.
4. Independent package failures do not abandon unrelated updates. Cancellation, reboot/shared installer signals, failed verification scans and persistence failures can stop the queue. See [diagnostics](UPDATE-DIAGNOSTICS.md).
5. Exit zero alone is not verification. Updates need fresh installed-state evidence; removal needs a successful scan confirming absence. Store acceptance may remain pending.
6. Automatic execution requires global and per-app consent while AppTide is open. Failed/uncertain operations block unattended retries. There is no installed scheduler or service.
7. A public directory match provides source evidence, not blanket installation permission. User notes and logs cannot widen execution authority.

## Persistence and identity

The process locks its catalogue directory. Saves retain the previous valid document and atomically replace the current catalogue. Invalid or unsupported data is reported instead of silently reset. Version history, preferences, observations and bounded receipt metadata stay local; raw operation/batch files are separate and have no automatic retention policy. Application event logs rotate. The catalogue is not encrypted or uploaded.

Matching uses exact provider identity or corroborated name/version/publisher evidence, depending on the source. Known installed-version metadata can resolve some unknown WinGet rows; public latest versions never fill an unknown installed version. Ambiguous and side-by-side records remain distinct. See [public catalogue rules](PUBLIC-CATALOGUE.md) and the [remaining identity work](https://github.com/GitKama/AppTide/issues/5).

Structured WinGet acquisition remains open in [#1](https://github.com/GitKama/AppTide/issues/1). Display-table parsing is still a beta limitation.
