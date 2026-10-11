# Contributing

The active application is the native Rust beta in `rust/`. On Windows x64, use rustup and the Microsoft C++ Build Tools with a Windows SDK. The repository pins its Rust toolchain. See [native build instructions](rust/README.md).

```powershell
cd rust
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
cargo run --locked -- --demo
```

Use synthetic demo data for UI work and screenshots. Do not run live update or uninstall commands as part of a test on someone's primary PC. Validate real installer behavior only in disposable Windows environments.

The legacy Electron alpha remains at the repository root for historical development. For alpha-specific changes, use Node.js 22+, `npm ci`, and `npm test`; `npm run test:desktop` runs synthetic desktop interaction tests on Windows.

Keep provider changes narrowly scoped. Add fixtures for locale, truncation, ambiguous identity, stale plans, errors, and post-operation verification. Never add shell execution of registry strings, trust promotion from logs, automatic hash bypass, or collection of private inventories.

All examples, screenshots, and fixtures in pull requests must be synthetic or explicitly sanitized. Never attach your raw catalogue, registry inventory, credentials, or installer logs to a public issue.

For new providers, first implement discovery and a watch-only preview. Execution support requires a documented trust model, exact identities, integrity checks, and tests using disposable Windows environments.
