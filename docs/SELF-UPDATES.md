# AppTide self-updates

Starting in 0.2.1-beta.1, the native app checks its own public GitHub releases when it opens. The request runs away from the UI thread; a failed check does not stop inventory or open an error dialog. In 0.2.7, every page shows the check result and a direct check/update action. The normal **Check for updates** action also checks AppTide. Settings retains the startup opt-out. Demo mode never performs a self-update check.

The beta channel includes newer beta releases and stable native releases. Numeric semantic versions determine ordering; release dates alone do not. The [GitHub releases API](https://docs.github.com/en/rest/releases/releases#list-releases) is used because `/releases/latest` excludes prereleases. Only assets in `GitKama/AppTide` with the exact Windows x64 package name, an uploaded state, a bounded size and a SHA-256 digest can produce an update offer.

## Update and restart

1. Click **Update and restart** when offered. The app refuses to begin during a scan or installer operation.
2. AppTide rechecks release metadata and downloads into its private `updates` staging directory. HTTPS redirects are limited to GitHub's release hosts. It checks the declared size and GitHub asset digest before extracting the four exact package filenames. It also checks that the executable is Windows x64.
3. A temporary helper waits for the current AppTide process to exit. It rechecks the old executable and all staged file hashes, retains the old package files in an `AppTide-backup-*` folder beside the executable, then replaces the package files and relaunches AppTide with the same data directory.
4. Replacement failures trigger rollback. A launch failure attempts to restore the backed-up files. A local `self-update-result.json` records the result. Backup and staging folders remain available for recovery; they are not startup tasks or services.

The private catalogue is outside the package replacement set and is never overwritten by the updater. Its existing atomic-save backup remains separate. The helper does not request elevation. If the portable app folder is not writable, extract the new release to a writable location instead. Keep the packaged executable name `AppTide-Beta.exe`; renamed executables and development builds use the manual release download.

## What was tested

- Release ordering and rejection of wrong repositories, wrong asset names and invalid integrity metadata.
- A real download of a published AppTide release through the production download/verification/extraction function.
- A locked companion file causing rollback of the executable.
- A full handoff on disposable package copies: close a beta.2 demo, wait for its process, replace with 0.2.1, relaunch the demo, confirm the new executable hash, and confirm a private catalogue sentinel was unchanged.

The download test and disposable handoff test are separate. Local operational evidence also confirmed a 0.2.6 replacement/relaunch; that observation is not a repeatable VM acceptance test. The full cross-version acceptance matrix remains tracked in [#3](https://github.com/GitKama/AppTide/issues/3). Successful process launch does not prove every future version will remain healthy. Retained backups permit manual recovery. Release binaries remain unsigned: GitHub TLS and the asset digest provide transport/integrity checks, not an Authenticode publisher signature. Signed releases and explicit health acknowledgement remain on the roadmap.

## Recovery

Close AppTide before restoring files. The backup folder contains the previous executable and companion package files. Copy those back into the parent package directory, preserving the backup. The catalogue stays in `%LOCALAPPDATA%\AppTide-Beta` unless a custom data directory was chosen. Do not copy catalogue files from someone else's computer.
