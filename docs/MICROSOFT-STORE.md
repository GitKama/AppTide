# Microsoft Store updates and richer inventory

## Microsoft Store

AppTide 0.2.4-beta.1 uses Microsoft's consumer `store.exe` client. This is separate from WinGet's `msstore` catalogue and from the developer publishing tool `msstore`.

1. Open **Microsoft Store** in AppTide's sidebar and choose **Check Store updates**. The main **Check for updates** action also checks Store availability.
2. Review the reported apps and their installed versions. Choose **Update Store apps…** and confirm the Store-specific preview.
3. AppTide rechecks the list before invoking `store updates --apply`. A changed or unreadable list prevents launch and requires a new review.
4. Results appear in **Activity**. **View local logs** includes the Store check and update output.

Microsoft owns the bulk operation. Its pending set can change after the recheck and can include dependencies; AppTide's per-app pause and automatic-update preferences do not govern this separate batch. Save work in Store apps first. AppTide cannot cancel the Store operation once dispatched. There is no automatic Store update schedule or automatic retry.

The CLI reports installed versions, not exact target versions. AppTide checks fresh current-user AppX inventory after the command exits. Verification requires a unique name/publisher/version match before the update, the same package identity afterwards, and a newer installed version. Store marketing names sometimes differ from local app names, so an app can remain **unverified** even if Store updated it. A zero exit code alone never proves success. Use Store Downloads and the local receipt when review is needed.

The client must already be available at the user's Windows app-execution-alias location. AppTide does not install Store, enable aliases, alter policies, accept licences, sign in, or bypass restrictions. If the CLI is unavailable or returns an unsupported/localized/truncated table, the UI reports that limitation and offers **Open Store Downloads**. The beta parser supports the current English, four-column Store-managed update table, including wrapped rows.

Read-only checks run with stdin disconnected. The current preview client prompts to install even when checking; it cannot receive an affirmative answer from AppTide's check. Only the confirmed action includes `--apply`.

## Installed metadata

- Registry DisplayName, Publisher and DisplayVersion remain authoritative when populated.
- For missing desktop metadata, AppTide reads the registered icon executable and up to 40 EXEs in the registered installation directory. It requires a matching product name, ignores installer/uninstaller/updater helpers, rejects links and remote paths, and leaves conflicting metadata unresolved. It never launches these executables.
- The details pane distinguishes registry versions, executable ProductVersion/FileVersion, and installed AppX package versions. Executable resource versions can differ from an app's marketing version or game build; these are reported values, not a promise of newest available software.
- AppX names use package manifests and Windows AppsFolder. Publishers use manifest display names or the readable certificate common name, retaining the original publisher identity separately. Cosmetic display-name changes do not discard saved source associations.
- Architecture and metadata sources are shown where available. Missing values remain explicitly **not reported**. No online guessing from an app name or package-ID prefix supplies installer authority.

## Research and validation boundary

Microsoft documents the consumer CLI and its update command in [Enhanced developer tools on the Microsoft Store](https://blogs.windows.com/windowsdeveloper/2026/02/11/enhanced-developer-tools-on-the-microsoft-store/). This integration also checks the installed client's `store --help`, `store updates --help`, and read-only output.

Local package fields are read through [Get-AppxPackageManifest](https://learn.microsoft.com/en-us/powershell/module/appx/get-appxpackagemanifest) and [FileVersionInfo.ProductVersion](https://learn.microsoft.com/en-us/dotnet/api/system.diagnostics.fileversioninfo.productversion).

Real inventory and read-only Store checks were exercised privately. Update dispatch, confirmation, version verification and failure handling are tested with synthetic operations. Actual Store installations remain a user acceptance test; they were not run against the developer's installed apps during validation.

## Asynchronous updates in 0.2.6

When the client returns “Updates started”, AppTide keeps a pending receipt with its baseline. Subsequent scans or Store checks verify newer installed versions. A pending batch cannot be submitted twice. Unresolved verification after 24 hours becomes a review item. Open Store Downloads for actual progress or errors. See [diagnostics](UPDATE-DIAGNOSTICS.md).

## Empty checks in 0.2.7

The consumer Store CLI 22608.1401.6.0 returned `Checking for updates…` followed by `No updates found.` with exit zero and empty stderr. Earlier AppTide versions did not recognize that exact wording and incorrectly reported a parsing failure. Version 0.2.7 accepts it as an empty successful check; nonzero exits, stderr and unsupported text still fail explicitly. The regression uses the observed response, including ANSI/BOM handling, without publishing installed-app data.
