# Research and design decisions

Historical design research, reviewed 10 October 2026 against upstream documentation and repository descriptions. AppTide is an original implementation; no upstream application source was copied into this project.

| Project | Licence / scope | Useful lesson | AppTide decision |
| --- | --- | --- | --- |
| [WinGet](https://github.com/microsoft/winget-cli) | MIT; Windows package management | Detects software installed by other means; exposes CLI, PowerShell, and COM APIs | Use its existing catalogue, installer handling, and integrity checks first |
| [UniGetUI](https://github.com/Devolutions/UniGetUI) | MIT; GUI for multiple package managers | Update queues, exclusions, package-specific choices, and visible operation history matter | Start with one reliable provider and a separate inventory/catalogue layer |
| [Winget-AutoUpdate](https://github.com/Romanitho/Winget-AutoUpdate) | MIT; scheduled updates | Allowlists, blocklists, user/system context, notifications, and unknown-version handling are essential | Automatic updates require per-app opt-in and never retry uncertain operations unattended |
| [Bulk Crap Uninstaller](https://github.com/BCUninstaller/Bulk-Crap-Uninstaller) | Apache-2.0; broad Windows uninstallation | Inventory and uninstallation extend beyond a package repository | Read native inventory separately; use exact WinGet removal and a Settings fallback first |
| [RuckZuck](https://github.com/rzander/ruckzuck) | Open-source clients; server components no longer public | Existing installations can be correlated with a maintained catalogue | Keep source observations, catalogue records, and executable installation recipes separate |

## Primary references

- [Microsoft: WinGet list](https://learn.microsoft.com/en-us/windows/package-manager/winget/list): includes apps installed outside WinGet.
- [Microsoft: upgrade](https://learn.microsoft.com/en-us/windows/package-manager/winget/upgrade): exact IDs, target versions, installer behavior, pinning, agreements, and safety switches.
- [Microsoft: uninstall](https://learn.microsoft.com/en-us/windows/package-manager/winget/uninstall): supports installed apps beyond those originally installed through WinGet; ambiguous selections need filtering.
- [Microsoft: troubleshooting and logs](https://learn.microsoft.com/en-us/windows/package-manager/winget/troubleshooting): diagnostic logs and their location.
- [Electron security](https://www.electronjs.org/docs/latest/tutorial/security): isolated contexts, renderer sandboxing, restricted navigation, sender validation, and a narrow preload bridge.

## Conclusions

There is no Windows-wide universal update protocol. Discovery can become broad; safe automation still needs trustworthy app identity, versions, architecture, scope, download provenance, integrity, and installation behavior. A name match or successful log line is insufficient.

AppTide therefore treats an observed application, a possible update source, an approved executable recipe, and a verified operation as different things. This allows the catalogue to grow without silently widening what may execute.

For the first alpha, WinGet source-restricted CLI output is parsed conservatively because it is already installed on the target machine. No extra WinGet PowerShell module is installed. A structured COM/PowerShell provider is the next reliability improvement; CLI display parsing remains a beta limitation tracked in [#1](https://github.com/GitKama/AppTide/issues/1). See [current architecture](ARCHITECTURE.md) for the implemented providers.
