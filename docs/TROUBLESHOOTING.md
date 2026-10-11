# Update queues and local logs

The last queue result stays visible after the worker finishes and after reopening AppTide. In 0.2.3-beta.1, **Activity → View local logs** opens a built-in, read-only viewer. Select a file, refresh the list, or copy the folder path. Log access no longer depends on Explorer successfully navigating to a directory.

## Where to look

Default location: `%LOCALAPPDATA%\AppTide-Beta`. A custom `--data-dir` uses that folder instead.

| File | Purpose |
| --- | --- |
| `logs/app.jsonl` | Timestamped startup, worker actions and errors. Rotates at 1 MiB, retaining `app.previous.jsonl`. |
| `receipts/*.log` | A completed WinGet process's exact arguments, decimal/hex exit code, standard output and standard error. Earlier versions recorded only output. |
| `batches/*.json` | Reviewed queue items, batch ID, start/end, outcomes, skipped reasons and stop reason. A running record is saved before preflight so failures before the first installer are no longer invisible. |
| `catalog.json` | Private inventory, operation receipts, version history and the latest batch summary. `catalog.json.bak` is the catalogue backup. |

Logs may contain product names, download URLs and local paths. Keep them private; share only a reviewed, redacted excerpt when reporting an issue. Nothing is uploaded automatically. Per-operation and batch files are retained; only the application event log rotates. Process output is saved when WinGet exits, so a still-running installer may not yet have an output file. Interrupted operations become unverified on reopening; the Activity summary marks an interrupted queue.

## What the result means

- **Verified:** a fresh inventory confirms the requested state.
- **Skipped:** the app changed since the preview, is now paused/protected/ambiguous, or no longer has the reviewed target. No installer was started for that item. Other unchanged items can proceed.
- **Failed / unverified:** a package download/installer failed, or its installed state remains ambiguous. AppTide records the result, does not retry that app, and continues independent updates while a fresh verification scan remains available. The final summary reports the aggregate problems. Global verification/persistence failures, cancellation, source-integrity problems, reboot requests and shared installer-lock signals still stop the queue.
- **Not run:** the queue was stopped, its preflight failed, or a global verification/reboot condition prevented continuing. It is not a claim that those installers failed.

Open Activity, review the reason, and run **Check for updates**. **Update all** builds a new preview of the currently eligible apps. It never silently changes a target from the preview or forces ambiguous packages. If Windows exposes two versions under one package ID, review them individually in Installed apps or Windows Settings; do not assume an old registration is safe to remove.

## Why a queue could appear to do nothing in 0.2.1

Code review and synthetic reproduction found two separate defects:

1. A single changed item rejected the entire preflight. That happened before operation receipts were created, leaving no persistent queue diagnostic.
2. After a failed or uncertain result, execution broke out of the loop and returned success to the UI worker. The worker then cleared the progress text. An individual receipt remained, but there was no clear batch stop reason or remaining count.

Version 0.2.2 improved visibility but retained an overly broad stop-on-any-failure policy. Version 0.2.3 separates individual package failures from conditions that prevent safe continuation. A zero installer exit code still does not prove a single, unambiguous installation reached the target version.

Historical receipts without batch IDs cannot reconstruct exactly which individual operations were part of an earlier queue. The patch does not rewrite those receipts or retroactively mark them successful.

## Validation boundary

Regression tests simulate successful, stale, duplicate, failed and cancelled states through the production queue loop. A UI interaction test clicks Update all, confirms the preview, and verifies all three synthetic apps through the worker. Another regression attempts 34 synthetic updates, fails item four with HTTP 404, and verifies that all 30 subsequent items are attempted exactly once. A UI click test reads a real temporary log file through the built-in viewer. Read-only diagnosis can inspect real Windows registrations and WinGet output; it does not authorize removing duplicate registrations or force-updating unrelated software. Real installer behavior varies and requires separate validation.

## Download failures and the 0.2.3 correction

A missing vendor download is a per-package failure. For example, the public [RealVNC WinGet manifest](https://github.com/microsoft/winget-pkgs/blob/master/manifests/r/RealVNC/VNCViewer/7.15.1.18/RealVNC.VNCViewer.installer.yaml) pointed to an installer URL that returned HTTP 404 during diagnosis on 11 October 2026. AppTide cannot repair a vendor URL by treating it as successful or substituting an unreviewed installer. It can record the failure and finish unrelated updates. This matches the direction of [WinGet bulk failure handling](https://github.com/microsoft/winget-cli/issues/1080).

The earlier log button only spawned Explorer and ignored the result. Folder existence did not guarantee successful Shell navigation; Microsoft documents that folder opening is a separate [Shell operation](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shellexecutew). The built-in viewer removes that dependency. Files are restricted to AppTide diagnostic subfolders, canonical paths must stay inside the data folder, only the 100 most recent files are listed, and reads are bounded to 256 KiB with a visible truncation notice. The precise Windows Explorer failure was not established; successful process creation was never proof that the user could read the logs.
