# Update queues and local logs

Starting in 0.2.2-beta.1, the last queue result stays visible after the worker finishes and after reopening AppTide. **Activity** shows individual results; **Open local logs** opens the private data folder.

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
- **Failed / unverified:** an installer failed, verification failed, or the installed state remains ambiguous. The queue stops and shows the app, reason and number not run. AppTide does not silently retry it.
- **Not run:** the queue was stopped, its preflight failed, or an earlier installer needed review. It is not a claim that those installers failed.

Open Activity, review the reason, and run **Check for updates**. **Update all** builds a new preview of the currently eligible apps. It never silently changes a target from the preview or forces ambiguous packages. If Windows exposes two versions under one package ID, review them individually in Installed apps or Windows Settings; do not assume an old registration is safe to remove.

## Why a queue could appear to do nothing in 0.2.1

Code review and synthetic reproduction found two separate defects:

1. A single changed item rejected the entire preflight. That happened before operation receipts were created, leaving no persistent queue diagnostic.
2. After a failed or uncertain result, execution broke out of the loop and returned success to the UI worker. The worker then cleared the progress text. An individual receipt remained, but there was no clear batch stop reason or remaining count.

The patch validates each item against the refreshed inventory, records skipped items, and turns uncertain execution into an explicit attention result. It also preserves batch summaries and logs preflight failures. A zero installer exit code remains insufficient to prove that a single, unambiguous installation reached the target version.

Historical receipts without batch IDs cannot reconstruct exactly which individual operations were part of an earlier queue. The patch does not rewrite those receipts or retroactively mark them successful.

## Validation boundary

Regression tests simulate successful, stale, duplicate, failed and cancelled states through the production queue loop. A UI interaction test clicks Update all, confirms the preview, and verifies all three synthetic apps through the worker. Read-only diagnosis can inspect real Windows registrations and WinGet output; it does not authorize removing duplicate registrations or force-updating unrelated software. Real installer behavior varies and requires separate validation.
