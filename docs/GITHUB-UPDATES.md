# GitHub installer updates

Introduced in 0.2.5-beta.1. Previously, GitHub associations were release watches: checking succeeded but there was no installer execution path. The action opened a browser. The new provider feeds compatible release installers into the same reviewed, serialized queue as WinGet.

## Using it

1. Scan installed apps. AppTide recognizes bundled products by name and publisher, and discovers GitHub project URLs from Windows uninstall metadata (`URLInfoAbout` / `URLUpdateInfo`). Conflicting projects are left unresolved. You can connect or change a project in the details panel.
2. Select an app. A single compatible installer enables **Update this app** when its numeric version is newer. If several assets qualify, choose the Windows installer first. The selection is tied to that release asset; a later release may need a new choice.
3. Review the project, filename and target version, then confirm. GitHub installers also join **Update all**. Existing WinGet matches retain WinGet as their preferred provider.
4. For reviewed recipes, enable the app's automatic checkbox and the hourly Settings switch. The window must remain open. Automatic updates are off by default; failed/unverified receipts block automatic retries until a successful manual update.

## Supported installers

| Route | Behavior |
| --- | --- |
| Meetily Community, recognized publisher, current-user installation | Exact x64 NSIS setup asset; reviewed `/S` mode, manual confirmation or per-app automatic opt-in. |
| ExifToolGUI, FrankB, current-user installation | Highest numeric installer revision within the stable release; reviewed Inno mode, no forced restart or forced application closure. |
| Other registry apps with connected/detected repositories | Attached MSI or EXE named setup/install; interactive Windows setup only. Multiple assets require selection. |
| Portable archives, raw program EXEs, ARM/x86 assets, missing digests, nonnumeric versions, manually tracked entries | Release page remains available; no automatic extraction or replacement. |

AppTide is a Windows x64 beta. Generic filenames cannot prove installer edition or architecture: review the named installer before confirming. Machine-wide installations use interactive setup; only the two reviewed current-user routes expose unattended consent. Different products, private forks and publishers do not inherit a recipe merely by sharing a repository URL.

ExifToolGUI sometimes publishes an installer revision newer than its release tag. Comparing the installer version fixes the misleading “Installed version ahead” result for those revisions. Numeric comparisons normalize leading `v`/`V`, trailing zero components, and `_32`/`_64` installed suffixes.

## Execution and receipts

- Require a stable public release, uploaded asset, asset ID, exact repository download URL, SHA256 digest and bounded size (512 MiB maximum).
- Re-fetch the release before execution. A changed target, asset or digest invalidates the preview rather than silently installing something else.
- Download into a unique private staging directory. HTTPS redirects are limited to GitHub and its release-asset hosts. Check size and SHA256 in both the downloader and Rust before launch.
- Command-line switches come from code reviewed for the recipe. Repository descriptions, release notes, URLs and local notes never become shell commands. Unknown installers get no guessed silent flags.
- Record the provider and asset in Activity receipts. Download logs and staged files remain under the private catalogue's `downloads` directory. Receipt logs remain accessible in Activity.
- Wait for setup, rescan Windows, and require one matching registration with the expected installed version. Exit zero alone is not success. Missing/changed registration identities remain unverified; no invented success or automatic retry.
- Package failures can continue to independent apps. Cancellation, restart requirements and loss of reliable verification stop the queue. In-progress installers are not killed.

SHA256 verifies that the downloaded file matches GitHub's asset; it is not an Authenticode trust assertion or a malware audit. Old staging folders are retained for diagnosis in this beta. GitHub API rate limits and offline checks produce visible errors; they do not authorize cached assets without a fresh pre-install check.

## Research and validation

The reviewed installer types come from [Meetily's Tauri bundle configuration](https://github.com/Zackriya-Solutions/meetily/blob/HEAD/frontend/src-tauri/tauri.conf.json) and [ExifToolGUI's Inno Setup source](https://github.com/FrankBijnen/ExifToolGui/blob/HEAD/Redist/ExifToolGUI_install.iss). Fixed flags follow [NSIS installer usage](https://nsis.sourceforge.io/Docs/Chapter3.html) and [Inno Setup command-line documentation](https://jrsoftware.org/ishelp/topic_setupcmdline.htm). Asset integrity fields follow [GitHub's release-assets API](https://docs.github.com/en/rest/releases/assets).

Validation covers mixed queues, stale hashes, ambiguous assets, repository detection/overrides, numeric installer revisions, confirmation through the actual UI worker, a real public installer download with hash verification, and a harmless synthetic executable through the launcher. Real third-party installation compatibility remains a user acceptance-test boundary; these tests did not update or uninstall the developer's existing apps.
