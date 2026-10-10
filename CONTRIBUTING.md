# Contributing

Use Node.js 22+, `npm ci`, and `npm test`. Desktop interaction tests run on Windows with `npm run test:desktop`; they use synthetic data. Do not run live update or uninstall commands as part of a test on someone's primary PC.

Keep provider changes narrowly scoped. Add fixtures for locale, truncation, ambiguous identity, stale plans, errors, and post-operation verification. Never add shell execution of registry strings, trust promotion from logs, automatic hash bypass, or collection of private inventories.

All examples, screenshots, and fixtures in pull requests must be synthetic or explicitly sanitized. Never attach your raw catalogue, registry inventory, credentials, or installer logs to a public issue.

For new providers, first implement discovery and a watch-only preview. Execution support requires a documented trust model, exact identities, integrity checks, and tests using disposable Windows environments.
