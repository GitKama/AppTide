param([string]$RegistryJson = 'W10=', [switch]$MetadataOnly)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
$registry = ConvertFrom-Json ([Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($RegistryJson)))
$results = [Collections.Generic.List[object]]::new()
function Normalize-Product([string]$Value) {
    (($Value.ToLowerInvariant() -replace '\s*\((x64|x86|64-bit|32-bit)\)', '') -replace '\s+\d+(\.\d+)+(\s.*)?$', '').Trim()
}
function Test-LocalPath([string]$Path) {
    if ($Path -notmatch '^[A-Za-z]:\\') { return $false }
    try {
        $node=Get-Item -LiteralPath $Path
        while ($node) {
            if (($node.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { return $false }
            if ($node -is [IO.FileInfo]) { $node=$node.Directory } else { $node=$node.Parent }
        }
        return $true
    } catch { return $false }
}
foreach ($a in $registry) {
    # The registration's icon is a hint, not sufficient identity by itself.
    # Only accept an EXE with a product name matching this registration.
    $candidates = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    $icon = [Environment]::ExpandEnvironmentVariables([string]$a.displayIcon).Trim()
    if ($icon -match '^"([^"]+\.exe)"(?:\s*,\s*-?\d+)?$') { $icon = $Matches[1] }
    elseif ($icon -match '^([^"\r\n]+\.exe)(?:\s*,\s*-?\d+)?$') { $icon = $Matches[1] }
    else { $icon = '' }
    if ($icon -match '^[A-Za-z]:\\' -and [IO.File]::Exists($icon)) { [void]$candidates.Add($icon) }
    $location = [Environment]::ExpandEnvironmentVariables([string]$a.location).Trim().Trim('"')
    if ($location -match '^[A-Za-z]:\\.+' -and [IO.Directory]::Exists($location) -and (Test-LocalPath $location)) {
        # Only the registered directory, no recursion or guessed installation folders.
        @(Get-ChildItem -LiteralPath $location -Filter '*.exe' -File -ErrorAction SilentlyContinue | Select-Object -First 40) | ForEach-Object { [void]$candidates.Add($_.FullName) }
    }
    $matching = [Collections.Generic.List[object]]::new()
    foreach ($candidate in $candidates) { try {
        $file = Get-Item -LiteralPath $candidate
        if ($file.BaseName -match '^(unins|uninstall|setup|update|updater|crash|repair)' -or $file.BaseName -match '(installer|updater)$') { continue }
        $node = $file; $linked = $false
        while ($node) {
            if (($node.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { $linked = $true; break }
            if ($node -is [IO.FileInfo]) { $node = $node.Directory } else { $node = $node.Parent }
        }
        if ($linked) { continue }
        $v = [Diagnostics.FileVersionInfo]::GetVersionInfo($file.FullName)
        if (-not $v.ProductName -or (Normalize-Product $v.ProductName) -ne (Normalize-Product $a.name)) { continue }
        $version=[string]$v.ProductVersion; $source='Executable ProductVersion'
        if (-not $version) { $version=[string]$v.FileVersion; $source='Executable FileVersion' }
        $matching.Add(@{key=$a.key; publisher=[string]$v.CompanyName; version=$version; versionSource=$source; metadataFile=$file.FullName})
    } catch { continue } }
    # Multiple launchers are acceptable only when their product metadata agrees.
    if ($matching.Count -gt 0 -and @($matching | ForEach-Object { $_.publisher + '|' + $_.version } | Select-Object -Unique).Count -eq 1) {
        $results.Add($matching[0])
    }
}
if ($MetadataOnly) { @{metadata=@($results.ToArray())} | ConvertTo-Json -Depth 6 -Compress; return }
$friendly = @{}
try {
    $shell = New-Object -ComObject Shell.Application
    foreach ($item in $shell.NameSpace('shell:AppsFolder').Items()) {
        $family = [string]$item.ExtendedProperty('System.AppUserModel.PackageFamilyName')
        if ($family -and $item.Name) {
            if (-not $friendly.ContainsKey($family)) { $friendly[$family] = [Collections.Generic.List[string]]::new() }
            if (-not $friendly[$family].Contains([string]$item.Name)) { $friendly[$family].Add([string]$item.Name) }
        }
    }
} catch { } # Optional presentation metadata must not hide installed packages.
$apps = @(Get-AppxPackage -ErrorAction Stop | Where-Object {-not $_.IsResourcePackage} | ForEach-Object {
    $p = $_; $name = [string]$p.Name; $publisher = [string]$p.Publisher
    $nameSource = 'Package identity'; $publisherSource = 'Package publisher identity'
    try {
        $manifest = Get-AppxPackageManifest -Package $p.PackageFullName -ErrorAction Stop
        $display = [string]$manifest.Package.Properties.DisplayName
        $vendor = [string]$manifest.Package.Properties.PublisherDisplayName
        if ($display -and $display -notmatch '^ms-resource:') { $name=$display; $nameSource='Package manifest' }
        if ($vendor -and $vendor -notmatch '^ms-resource:') { $publisher=$vendor; $publisherSource='Package manifest' }
    } catch { }
    if ($friendly.ContainsKey($p.PackageFamilyName) -and $friendly[$p.PackageFamilyName].Count -eq 1) {
        $name=$friendly[$p.PackageFamilyName][0]; $nameSource='Windows AppsFolder'
    }
    if ($publisherSource -eq 'Package publisher identity' -and $publisher -match '^CN=([^,]+)') {
        $cn=$Matches[1].Trim('"')
        if ($cn -notmatch '^[0-9a-fA-F-]{36}$') { $publisher=$cn; $publisherSource='Package publisher CN' }
    }
    @{key=('appx:'+$p.PackageFamilyName);name=$name;version=$p.Version.ToString();publisher=$publisher;rawPublisher=[string]$p.Publisher;packageName=[string]$p.Name;packageFullName=[string]$p.PackageFullName;architecture=[string]$p.Architecture;scope='user';kind='appx';signatureKind=[string]$p.SignatureKind;location=[string]$p.InstallLocation;protected=[bool]($p.NonRemovable -or $p.IsFramework);metadataSources=@{name=$nameSource;publisher=$publisherSource;version='Installed AppX package'}}
})
@{apps=$apps; metadata=@($results.ToArray())} | ConvertTo-Json -Depth 6 -Compress
