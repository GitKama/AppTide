param([string]$PortableFolder = '')
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
$items = [System.Collections.Generic.List[object]]::new()
$warnings = [System.Collections.Generic.List[string]]::new()
if ($PortableFolder) {
    $root = Get-Item -LiteralPath $PortableFolder
    if (-not $root.PSIsContainer) { throw 'Select a folder.' }
    $queue = [System.Collections.Generic.Queue[object]]::new()
    $queue.Enqueue(@{ Path = $root.FullName; Depth = 0 })
    while ($queue.Count -gt 0 -and $items.Count -lt 500) {
        $entry = $queue.Dequeue()
        foreach ($file in Get-ChildItem -LiteralPath $entry.Path -Force -ErrorAction SilentlyContinue) {
            if (($file.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { continue }
            if ($file.PSIsContainer) {
                if ($entry.Depth -lt 2) { $queue.Enqueue(@{ Path = $file.FullName; Depth = $entry.Depth + 1 }) }
            } elseif ($file.Extension -ieq '.exe' -and $items.Count -lt 500) {
                $v = $file.VersionInfo
                $items.Add([pscustomobject]@{ key = 'portable:' + $file.FullName; name = $(if ($v.ProductName) { $v.ProductName } else { $file.BaseName }); version = $v.ProductVersion; publisher = $v.CompanyName; scope = 'portable'; kind = 'portable'; location = $file.FullName; protected = $false })
            }
        }
    }
    $warnings.Add('Portable discovery is limited to EXE files, three directory levels, and 500 files; links are skipped.')
} else {
    foreach ($hive in @([Microsoft.Win32.RegistryHive]::LocalMachine, [Microsoft.Win32.RegistryHive]::CurrentUser)) {
        foreach ($view in @([Microsoft.Win32.RegistryView]::Registry64, [Microsoft.Win32.RegistryView]::Registry32)) {
            $base = $null; $uninstall = $null
            try {
                $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey($hive, $view)
                $uninstall = $base.OpenSubKey('SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall')
                if (-not $uninstall) { continue }
                foreach ($key in $uninstall.GetSubKeyNames()) {
                    $app = $uninstall.OpenSubKey($key)
                    if (-not $app) { continue }
                    try {
                        $name = [string]$app.GetValue('DisplayName')
                        if (-not $name) { continue }
                        $items.Add([pscustomobject]@{ key = "registry:$hive`:$view`:$key"; name = $name; version = [string]$app.GetValue('DisplayVersion'); publisher = [string]$app.GetValue('Publisher'); scope = $(if ($hive -eq 'LocalMachine') { 'machine' } else { 'user' }); kind = 'registry'; location = [string]$app.GetValue('InstallLocation'); protected = ($app.GetValue('SystemComponent', 0) -eq 1 -or $app.GetValue('NoRemove', 0) -eq 1) })
                    } finally { $app.Dispose() }
                }
            } catch { $warnings.Add("Registry inventory incomplete for $hive/$view.") }
            finally { if ($uninstall) { $uninstall.Dispose() }; if ($base) { $base.Dispose() } }
        }
    }
    try {
        foreach ($app in Get-AppxPackage -ErrorAction Stop) {
            if ($app.IsResourcePackage) { continue }
            $items.Add([pscustomobject]@{ key = 'appx:' + $app.PackageFamilyName; name = $app.Name; version = $app.Version.ToString(); publisher = $app.Publisher; scope = 'user'; kind = 'appx'; location = $app.InstallLocation; protected = [bool]($app.NonRemovable -or $app.IsFramework) })
        }
    } catch { $warnings.Add('Microsoft Store / AppX inventory could not be read for the current user.') }
}
[pscustomobject]@{ apps = @($items.ToArray()); warnings = @($warnings.ToArray()) } | ConvertTo-Json -Depth 6 -Compress
