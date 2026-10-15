# Update diagnostics

Different failures need different responses. Activity preserves the provider output, exit code, reviewed target and observed installed versions. A failed application does not stop unrelated queue items unless cancellation, reboot, shared installer/source failure or an incomplete verification scan requires it.

| Evidence | Meaning and response |
| --- | --- |
| HTTP 404 | The package manifest points to a missing vendor download. Refresh WinGet sources, then retry after the package maintainer fixes the URL. |
| Hash mismatch | The downloaded file differs from the manifest's expected digest. Keep it blocked; do not bypass verification or substitute a random download. |
| Previous-version uninstall failure | The installer could not remove the existing version. Close the app and use **Review retry with installer prompts**; the original installation may need repair. AppTide does not force removal. |
| No applicable upgrade | Installer technology or channel can differ from the installed copy. Review the actual output and publisher migration instructions. |
| Cancelled operation | An installer or Windows permission prompt may have been cancelled. The queue stops; resume after reviewing it. |
| Installer succeeds but versions appear different only by zero padding | Strict numeric versions such as `1.2.3` and `1.2.3.0` now compare equivalently. Verification still requires a real change from the previously installed version and a unique identity. |
| Store check says output could not be read, but the log says “No updates found.” | Fixed in 0.2.7. Update AppTide, then check Store updates again. Other unsupported/localized output still requires review. |
| Microsoft Store reports “Updates started” | Store accepted asynchronous work. AppTide records **pending**, blocks duplicate batch submission and rechecks installed versions on the next scan or Store check. It does not immediately call the update failed or verified. |

The Store page offers progress handoff to Store Downloads. Pending operations become verified only after every previewed app has a newer installed package version with the same identity and publisher. After 24 hours without complete verification, the receipt asks for review; there is no automatic retry.

**Refresh WinGet sources** in Activity updates only the WinGet source metadata, then rescans. **Show installer prompts** in an update preview switches the installer mode while retaining the exact package ID, source and version target. Automatic updates keep their unattended mode and existing consent requirements.

Some upstream downloads and third-party uninstallers remain outside AppTide's control. These changes improve diagnosis, queue behavior and verification; they do not claim to repair a vendor's server, ignore a checksum or guarantee every installer will succeed.

References: [WinGet return codes](https://github.com/microsoft/winget-cli/blob/master/doc/windows/package-manager/winget/returnCodes.md), [WinGet error definitions](https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerSharedLib/Public/AppInstallerErrors.h), [WinGet upgrade options](https://learn.microsoft.com/en-us/windows/package-manager/winget/upgrade).
