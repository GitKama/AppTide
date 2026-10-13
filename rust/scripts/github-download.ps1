$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Net.Http
$cfg = ConvertFrom-Json ([Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('__CONFIG__')))
$handler = [Net.Http.HttpClientHandler]::new()
$handler.AllowAutoRedirect = $false
$client = [Net.Http.HttpClient]::new($handler)
$client.Timeout = [TimeSpan]::FromSeconds(45)
$client.DefaultRequestHeaders.UserAgent.ParseAdd('AppTide-GitHubUpdate')
try {
    $url = [Uri]$cfg.url
    for ($redirect = 0; $redirect -lt 6; $redirect++) {
        if ($url.Scheme -ne 'https' -or $url.Host -notin @('github.com', 'release-assets.githubusercontent.com', 'objects.githubusercontent.com') -or $url.UserInfo -or $url.Port -ne 443) { throw 'Untrusted download destination' }
        $response = $client.GetAsync($url, [Net.Http.HttpCompletionOption]::ResponseHeadersRead).GetAwaiter().GetResult()
        if ([int]$response.StatusCode -ge 300 -and [int]$response.StatusCode -lt 400) {
            $url = [Uri]::new($url, $response.Headers.Location)
            $response.Dispose()
            continue
        }
        break
    }
    if ([int]$response.StatusCode -ne 200) { throw ('Download HTTP ' + [int]$response.StatusCode) }
    $stream = $response.Content.ReadAsStreamAsync().GetAwaiter().GetResult()
    $output = [IO.File]::Create($cfg.file)
    try {
        $buffer = New-Object byte[] 65536
        $total = 0
        while (($count = $stream.Read($buffer, 0, $buffer.Length)) -gt 0) {
            $total += $count
            if ($total -gt $cfg.size) { throw 'Download exceeds expected size' }
            $output.Write($buffer, 0, $count)
        }
    } finally { $output.Dispose() }
    $sha = [Security.Cryptography.SHA256]::Create()
    $input = [IO.File]::OpenRead($cfg.file)
    try { $actual = [BitConverter]::ToString($sha.ComputeHash($input)).Replace('-', '') }
    finally { $input.Dispose(); $sha.Dispose() }
    if ($total -ne $cfg.size -or $actual -ine $cfg.hash) { throw 'Download integrity mismatch' }
    [Console]::WriteLine('Release asset downloaded; SHA256 and size verified.')
} finally { $client.Dispose() }
