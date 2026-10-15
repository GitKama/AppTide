# Security policy

## Supported versions

| Line | Status |
| --- | --- |
| Latest 0.2.x Rust beta | Active development; fixes ship in a newer beta |
| Earlier 0.2.x betas | Upgrade to the latest beta before reproducing a problem |
| 0.1.x Electron alpha | Historical source/releases; no active feature or security support |

AppTide is beta software that can invoke installers and uninstallers. It is not yet independently security-reviewed. Release binaries are unsigned; download hashes do not establish an Authenticode publisher identity.

## Reporting

**Do not disclose vulnerabilities, credentials, raw catalogues or installer logs in a public issue.** Include only the affected version, relevant trust boundary and a sanitized reproduction when a private channel has been agreed.

GitHub private vulnerability reporting is **not enabled** for this repository as of 15 October 2026. Until a private channel is configured, a public issue may request a private contact method without including vulnerability details. Do not assume the public bug-report form is confidential. There is no published response-time guarantee.

## Trust boundaries

- Names, catalogue metadata and log sightings do not authorize executable commands.
- WinGet identities and GitHub release assets are checked before execution; integrity failures are not bypassed.
- A successful installer exit is not proof of the requested installed state.
- Third-party installers are not transactional. AppTide package rollback does not roll back other apps.
- The catalogue uses the current Windows user's filesystem permissions; it is not an encrypted vault.

See [architecture](docs/ARCHITECTURE.md), [GitHub update rules](docs/GITHUB-UPDATES.md) and [self-update limits](docs/SELF-UPDATES.md).
