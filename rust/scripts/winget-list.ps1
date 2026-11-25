$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
Import-Module Microsoft.WinGet.Client -ErrorAction Stop
$rows = @(Get-WinGetPackage -Source winget -ErrorAction Stop | ForEach-Object {
    if ($_.Source -ne 'winget') { throw 'Unexpected package source' }
    $target = ''
    if ($_.IsUpdateAvailable) { $target = [string]($_.AvailableVersions | Select-Object -First 1) }
    @{ name = [string]$_.Name; id = [string]$_.Id; version = [string]$_.InstalledVersion; available = $target }
})
[Console]::Write((ConvertTo-Json -InputObject $rows -Compress))
