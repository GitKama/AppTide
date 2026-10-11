# Releases and development history

AppTide retains public commits, annotated release notes, tags and downloadable assets for each milestone. Published tags and assets are not overwritten. Corrections get a new version. The Electron alpha remains in source/history while the native Rust implementation evolves.

## Versioning

- `0.2.0-beta.1`: first native Rust beta.
- `0.2.0-beta.2`: reviewed software catalogue and broader discovery.
- `0.2.1-beta.1`: private catalogue editor and AppTide self-updates.
- `0.2.2-beta.1`: queue recovery, explicit stop reasons and local diagnostics.
- Subsequent small changes increment the patch version, with a beta suffix until stability criteria are met. Additional candidates of the same milestone increment the beta number. A final stable version removes the prerelease suffix.

## Release checklist

1. Update `rust/Cargo.toml`, `Cargo.lock`, changelog, README and relevant validation notes. Keep previous changelog entries.
2. Run formatting, locked Rust tests, Clippy with warnings denied, and an optimized Windows x64 build. Inspect synthetic desktop/compact captures when the UI changes. Run bounded, read-only live checks in a separate private catalogue when providers change.
3. For self-update changes, run the opt-in public-download check and disposable-package handoff/rollback checks. Never test by mutating unrelated installed software.
4. Package using `scripts/package-rust.py`; it refuses to overwrite an output directory. Inspect the ZIP entries, dependency licences and SHA-256. Private catalogues, local paths, receipts, logs and personal screenshots are excluded.
5. Commit the reviewed source and push. Publish a new GitHub prerelease at that exact commit with the ZIP and checksum. Verify the uploaded asset digest. Record local validation separately from GitHub Actions results; do not call pending CI successful.
6. Preserve all previous releases. Link the current release in the README and document material limitations. Future development follows in another commit/version rather than rewriting history.

GitHub Actions runs Rust checks and the legacy Electron checks on pushes and pull requests. Rust dependency/build caching accelerates subsequent iterations. Contributions should use public source documentation and synthetic fixtures; see [catalogue contributions](CATALOGUE.md#contributing-a-bundled-recipe).
