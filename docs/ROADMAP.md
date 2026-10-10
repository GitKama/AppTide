# Roadmap

## 0.1 — A working foundation

Implemented: local catalogue, Windows inventory, optional portable scanning, WinGet update/removal plans, operation verification, exclusions, opt-in updates while open, bounded log sightings, watch-only GitHub releases, desktop UI, tests, and Windows packaging.

## 0.2 — Reliable identity and broader discovery

Beta.2 delivered reviewed detection recipes, private learning summaries, update-owner guidance, cached stable-release watches, bounded portable discovery and opt-in updates while open. Next: a simpler private catalogue editor and structured WinGet data.

- Replace display-table parsing with a structured WinGet adapter; retain a tested CLI fallback.
- Persist explicit aliases and product-code/architecture/scope evidence; reconcile duplicate records without merging ambiguous installations.
- Surface suggested catalogue associations with their supporting evidence and user review.
- Refresh saved release watches during scheduled scans with caching, rate limits, and release-channel controls.
- Add discovery providers for selected portable manifests, Store identities, and known application-specific metadata. Keep unsupported detections visible.
- Add catalogue backup/restore, retention settings, and schema migration tests.

## 0.3 — Reviewed custom update recipes

- Versioned recipe schema: publisher identity, version discovery, architecture/scope, release channel, immutable download URL, SHA-256 and signature policy, silent install/uninstall arguments, and verification rule.
- Start with explicitly reviewed GitHub Releases and official vendor feeds. Do not execute arbitrary search results or scripts from logs.
- Separate local private catalogue metadata from sanitized, opt-in community recipe contributions.
- Recipe CI in disposable Windows environments, including wrong-architecture, signature mismatch, installation failure, restart-required, and uninstall tests.
- Add install-new-software browsing with clear publisher and source provenance.

## 0.4 — Unattended maintenance

- Explicit opt-in Windows scheduled task, visible status, uninstall/disable flow, and documented execution context.
- Maintenance windows, running-app detection, deferrals, metered-network handling, and reboot coordination.
- Avoid elevating the full UI; narrow any privileged helper to validated operations.
- Signed releases, update-channel policy, and a verifiable AppTide self-update path.

## Acceptance before calling it stable

- Disposable-VM end-to-end tests with real installers, updates, removals, UAC, failure, and reboot cases.
- Independent review of provider trust boundaries, privilege handling, and update integrity.
- Supported Windows versions and locales validated; accessibility and keyboard-only workflows tested.
- Clear distinction between detected, supported, source-verified, and operation-verified software.
