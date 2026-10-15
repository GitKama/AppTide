# Releases and development history

AppTide retains public commits, annotated release notes, tags and downloadable assets for each milestone. Published tags and assets are not overwritten. Corrections get a new version. The Electron alpha remains in source/history while the native Rust implementation evolves.

## Versioning

- `0.2.0-beta.1`: first native Rust beta.
- `0.2.0-beta.2`: reviewed software catalogue and broader discovery.
- `0.2.1-beta.1`: private catalogue editor and AppTide self-updates.
- `0.2.2-beta.1`: queue recovery, explicit stop reasons and local diagnostics.
- `0.2.3-beta.1`: independent batch failure handling and an in-app log viewer.
- `0.2.4-beta.1`: richer installed metadata and Microsoft Store bulk-update support.
- `0.2.5-beta.1`: GitHub installer updates and native application icon.
- `0.2.6-beta.1`: public metadata directory, Steam library and improved diagnostics.
- `0.2.7-beta.1`: empty Store check fix and visible AppTide self-update controls.
- Subsequent small changes increment the patch version, with a beta suffix until stability criteria are met. Additional candidates of the same milestone increment the beta number. A final stable version removes the prerelease suffix.

## Release checklist

1. Update `rust/Cargo.toml`, `Cargo.lock`, changelog, README and relevant validation notes. Keep previous changelog entries.
2. Run formatting, locked Rust tests, Clippy with warnings denied, and an optimized Windows x64 build. Inspect synthetic desktop/compact captures when the UI changes. Run bounded, read-only live checks in a separate private catalogue when providers change.
3. For self-update changes, run the opt-in public-download check and disposable-package handoff/rollback checks. Never test by mutating unrelated installed software.
4. Package using `scripts/package-rust.py`; it refuses to overwrite an output directory. Inspect the ZIP entries, dependency licences and SHA-256. Private catalogues, local paths, receipts, logs and personal screenshots are excluded.
5. Commit the reviewed source and push. Publish a new GitHub prerelease at that exact commit with the ZIP and checksum. Verify the uploaded asset digest. Record local validation separately from GitHub Actions results; do not call pending CI successful.
6. Review every affected issue: mark delivered criteria with source/release evidence, close completed work, and split only concrete unfinished requirements. Update roadmap status and milestones without inventing delivery dates.
7. Run `python scripts/check-docs.py` from the repository root; check README/download links, support policy, current UI labels and source metadata. Documentation-only maintenance does not need a new application version or replacement binaries.
8. Preserve all previous releases. Link the current release in the README and document material limitations. Future development follows in another commit/version rather than rewriting history.

GitHub Actions runs Rust checks and the legacy Electron checks on pushes and pull requests. Rust dependency/build caching accelerates subsequent iterations. Contributions should use public source documentation and synthetic fixtures; see [catalogue contributions](CATALOGUE.md#contributing-a-bundled-recipe).

## Repository upkeep

Use the issue forms for reproducible failures, catalogue evidence and bounded features. The **Beta reliability** milestone holds current technical gaps; **Stable readiness** holds acceptance evidence. The **deferred** label means no active implementation commitment. Before each release, review all open issues and remove shipped requirements from open umbrella work. Do not auto-close reports just because they are old.

Keep the repository description/topics, documentation index, release notes and supported-version policy aligned with the current product. Historical research and per-release validation remain dated records. The documentation CI checks local Markdown targets and release/version consistency; human review still determines whether feature claims and issue status are true.
