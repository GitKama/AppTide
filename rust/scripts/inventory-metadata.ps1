param([string]$RegistryJson = 'W10=', [switch]$MetadataOnly)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
$registry = ConvertFrom-Json ([Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($RegistryJson)))
$results = [Collections.Generic.List[object]]::new()
function Normalize-Product([string]$Value) {
    (($Value.ToLowerInvariant() -replace '\s*\((x64|x86|64-bit|32-bit)\)', '') -replace '\s+\d+(\.\d+)+(\s.*)?$', '') -replace '[^\p{L}\p{Nd}]',''
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
    $directories = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    if ($a.key -match ':Steam App \d+$') { continue } # Steam build IDs come from its own manifest.
    $icon = [Environment]::ExpandEnvironmentVariables([string]$a.displayIcon).Trim()
    if ($icon -match '^"([^"]+\.(?:exe|ico))"(?:\s*,\s*-?\d+)?$') { $icon = $Matches[1] }
    elseif ($icon -match '^([^"\r\n]+\.(?:exe|ico))(?:\s*,\s*-?\d+)?$') { $icon = $Matches[1] }
    else { $icon = '' }
    if ($icon -match '^[A-Za-z]:\\' -and [IO.File]::Exists($icon) -and (Test-LocalPath $icon)) {
        if ([IO.Path]::GetExtension($icon) -eq '.exe') { [void]$candidates.Add($icon) }
        [void]$directories.Add([IO.Path]::GetDirectoryName($icon))
    }
    # Parse only a literal EXE path to locate product metadata. Never execute this command.
    $uninstall=[Environment]::ExpandEnvironmentVariables([string]$a.uninstallHint).Trim()
    $hint=''
    if ($uninstall -match '^"([^"\r\n]+\.exe)"(?:\s|$)') { $hint=$Matches[1] }
    elseif ($uninstall -match '^([A-Za-z]:\\[^"\r\n]+?\.exe)(?:\s|$)') { $hint=$Matches[1] }
    if ($hint -and (Test-LocalPath $hint)) {
        $parent=[IO.Path]::GetDirectoryName($hint)
        if ([IO.Path]::GetFileName($parent) -match '^(setup|uninstall)$') { $parent=[IO.Path]::GetDirectoryName($parent) }
        [void]$directories.Add($parent)
    }
    $location = [Environment]::ExpandEnvironmentVariables([string]$a.location).Trim().Trim('"')
    if ($location -match '^[A-Za-z]:\\.+' -and [IO.Directory]::Exists($location) -and (Test-LocalPath $location)) {
        [void]$directories.Add($location)
    }
    foreach ($directory in @($directories)) {
        # Bounded, one-level search below registration hints; no disk-wide scan.
        if ($directory -eq [IO.Path]::GetPathRoot($directory) -or $directory -match '\\(Windows|System32|SysWOW64|Program Files(?: \(x86\))?)$' -or -not (Test-LocalPath $directory)) { continue }
        $scan=@($directory)+@(Get-ChildItem -LiteralPath $directory -Directory -ErrorAction SilentlyContinue | Where-Object {($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -eq 0} | Select-Object -First 12 -ExpandProperty FullName)
        foreach ($dir in $scan) {
            @(Get-ChildItem -LiteralPath $dir -Filter '*.exe' -File -ErrorAction SilentlyContinue | Select-Object -First 40) | ForEach-Object { [void]$candidates.Add($_.FullName) }
        }
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
        $productMatches=$v.ProductName -and (Normalize-Product $v.ProductName) -eq (Normalize-Product $a.name)
        # Some applications omit ProductName. A matching executable basename plus
        # the separately registered vendor can corroborate identity in that case.
        if (-not $v.ProductName -and $a.publisher -and $v.CompanyName) {
            $base=$file.BaseName -replace '[-_](i386|x86_64(?:-SSE4-AVX2)?|x64|x86|arm64)$',''
            $productMatches=(Normalize-Product $base) -eq (Normalize-Product $a.name)
        }
        if (-not $productMatches) { continue }
        if ($a.publisher -and $v.CompanyName) {
            $registered=($a.publisher.ToLowerInvariant() -replace '\b(incorporated|inc|corporation|corp|limited|ltd|llc|gmbh)\b','') -replace '[^\p{L}\p{Nd}]',''
            $company=($v.CompanyName.ToLowerInvariant() -replace '\b(incorporated|inc|corporation|corp|limited|ltd|llc|gmbh)\b','') -replace '[^\p{L}\p{Nd}]',''
            if ($registered -ne $company) { continue }
        }
        $version=[string]$v.ProductVersion; $source='Executable ProductVersion'
        if (-not $version) { $version=[string]$v.FileVersion; $source='Executable FileVersion' }
        $version=$version.Trim()
        if ($version -match '^\d+(\s*,\s*\d+){1,3}$') { $version=$version -replace '\s*,\s*','.' }
        $matching.Add(@{key=$a.key; publisher=[string]$v.CompanyName; version=$version; versionSource=$source; metadataFile=$file.FullName})
        if ($candidate -eq $icon) { break } # Exact registered executable outranks directory fallback.
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
