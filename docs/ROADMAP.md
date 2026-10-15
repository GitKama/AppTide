# Project status and remaining work

Reviewed against **0.2.7-beta.1 on 15 October 2026**. This is a status map, not a version/date promise. The linked GitHub issues contain completion criteria; completed capabilities belong in the changelog rather than an open umbrella issue.

## Shipped

| Capability | First native release / evidence |
| --- | --- |
| Rust immediate-mode UI, inventory, WinGet previews, updates and removal | [0.2.0-beta.1](https://github.com/GitKama/AppTide/releases/tag/v0.2.0-beta.1) |
| Reviewed catalogue routes, portable discovery, opt-in updates while open | [0.2.0-beta.2](https://github.com/GitKama/AppTide/releases/tag/v0.2.0-beta.2) |
| Private catalogue editor and verified AppTide self-updates | [0.2.1-beta.1](https://github.com/GitKama/AppTide/releases/tag/v0.2.1-beta.1) |
| Persistent queue outcomes, independent failure handling and built-in log viewer | [0.2.2](https://github.com/GitKama/AppTide/releases/tag/v0.2.2-beta.1), [0.2.3](https://github.com/GitKama/AppTide/releases/tag/v0.2.3-beta.1) |
| Richer installed metadata and Microsoft Store checks/updates | [0.2.4-beta.1](https://github.com/GitKama/AppTide/releases/tag/v0.2.4-beta.1) |
| GitHub installer updates, project detection and native AppTide icon | [0.2.5-beta.1](https://github.com/GitKama/AppTide/releases/tag/v0.2.5-beta.1) |
| Public app directory, Steam library, metadata fallbacks and pending Store reconciliation | [0.2.6-beta.1](https://github.com/GitKama/AppTide/releases/tag/v0.2.6-beta.1) |
| Correct empty Store checks and visible AppTide update status/actions on every page | [0.2.7-beta.1](https://github.com/GitKama/AppTide/releases/tag/v0.2.7-beta.1) |

## Beta reliability

- [#1 — Structured WinGet acquisition](https://github.com/GitKama/AppTide/issues/1): replace dependency on display-table layout while retaining a conservative fallback and exact identity.
- [#5 — Reviewed ambiguous identity associations](https://github.com/GitKama/AppTide/issues/5): review and persist unresolved mappings without merging editions, scopes or publishers.

The old catalogue umbrella [#2](https://github.com/GitKama/AppTide/issues/2) is complete for its delivered catalogue/source/update scope; its unresolved identity portion is tracked separately in #5.

## Stable-release gate

[#3 — Disposable Windows acceptance matrix](https://github.com/GitKama/AppTide/issues/3) tracks actual installer/update/removal, UAC, reboot, failure and self-update acceptance evidence. Passing synthetic tests and successful use on one PC do not establish this matrix.

Stable readiness also needs a supported Windows/locale matrix, keyboard/screen-reader validation, independent trust-boundary review, a private security-reporting channel and a release signing/health-check policy. These are release criteria, not claims that they are implemented.

## Deferred

[#4 — Updates while AppTide is closed](https://github.com/GitKama/AppTide/issues/4) is optional future work, labelled deferred and excluded from the active reliability milestone. Hourly checks while the app is open already exist. No background task will be installed without explicit opt-in.

Install-new-software browsing, portable archive replacement, catalogue export/restore/retention controls and additional vendor execution recipes are possible future work, without assigned versions or delivery dates. Public metadata coverage does not imply executable support for every record.

## Keeping this current

Review affected issues with every behavior change. Close completed work with source/release evidence; split only concrete unfinished requirements. Update this page, [validation](VALIDATION.md) and the [release checklist](RELEASING.md) together. Preserve historical issues, tags and releases.
