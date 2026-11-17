param(
    [string]$BinaryPath = "$PSScriptRoot/../target/release/airgap.exe",
    [int]$Samples = 10,
    [string]$OutputPath = "$PSScriptRoot/../target/metrics.json"
)
$ErrorActionPreference = 'Stop'
if ($Samples -lt 1) { throw 'Samples must be positive' }
$binary = (Resolve-Path -LiteralPath $BinaryPath).Path
$startup = [System.Collections.Generic.List[double]]::new()
$latency = [System.Collections.Generic.List[double]]::new()
$workingSets = [System.Collections.Generic.List[long]]::new()

function Read-Response($Process) {
    $read = $Process.StandardOutput.ReadLineAsync()
    if (-not $read.Wait(5000)) { throw 'MCP response deadline exceeded' }
    if (-not $read.Result) { throw 'Server closed stdout' }
    return ($read.Result | ConvertFrom-Json)
}

for ($sample = 0; $sample -lt $Samples; $sample++) {
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo.FileName = $binary
    $process.StartInfo.UseShellExecute = $false
    $process.StartInfo.CreateNoWindow = $true
    $process.StartInfo.RedirectStandardInput = $true
    $process.StartInfo.RedirectStandardOutput = $true
    $process.StartInfo.RedirectStandardError = $true
    try {
        $watch = [System.Diagnostics.Stopwatch]::StartNew()
        if (-not $process.Start()) { throw 'Cannot start airgap' }
        $process.StandardInput.WriteLine('{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"measure","version":"1"}}}')
        $response = Read-Response $process
        $watch.Stop()
        if ($response.result.serverInfo.name -ne 'airgap') { throw 'Unexpected initialize response' }
        $startup.Add($watch.Elapsed.TotalMilliseconds)
        $process.StandardInput.WriteLine('{"jsonrpc":"2.0","method":"notifications/initialized"}')
        # Warm tool schema caches before timing repeated requests.
        $process.StandardInput.WriteLine('{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}')
        $null = Read-Response $process
        for ($request = 3; $request -lt 13; $request++) {
            $watch.Restart()
            $process.StandardInput.WriteLine('{"jsonrpc":"2.0","id":' + $request + ',"method":"tools/list","params":{}}')
            $response = Read-Response $process
            $watch.Stop()
            if ($response.result.tools.Count -ne 7) { throw 'Expected seven tools' }
            $latency.Add($watch.Elapsed.TotalMilliseconds)
        }
        $process.Refresh()
        $workingSets.Add($process.WorkingSet64)
        $process.StandardInput.Close()
        if (-not $process.WaitForExit(5000)) { throw 'Server failed to exit on EOF' }
        if ($process.ExitCode -ne 0) { throw $process.StandardError.ReadToEnd() }
    } finally {
        if ($process.Id -and -not $process.HasExited) { $process.Kill() }
        $process.Dispose()
    }
}

function Median($Values) {
    $sorted = @($Values | Sort-Object)
    $middle = [int][math]::Floor($sorted.Count / 2)
    if ($sorted.Count % 2) { return $sorted[$middle] }
    return ($sorted[$middle - 1] + $sorted[$middle]) / 2
}

$metrics = [ordered]@{
    measured_at = [DateTime]::UtcNow.ToString('o')
    platform = [Environment]::OSVersion.VersionString
    samples = $Samples
    binary_bytes = (Get-Item -LiteralPath $binary).Length
    startup_initialize_median_ms = Median $startup
    tools_list_roundtrip_median_ms = Median $latency
    working_set_median_bytes = Median $workingSets
    startup_initialize_samples_ms = @($startup)
    tools_list_roundtrip_samples_ms = @($latency)
    working_set_samples_bytes = @($workingSets)
    method = 'PowerShell/.NET process spawn through initialize response; warm tools/list roundtrips including host JSON parsing; working set after requests. No cloud calls.'
}
$json = $metrics | ConvertTo-Json -Depth 5
[System.IO.File]::WriteAllText([System.IO.Path]::GetFullPath($OutputPath), $json)
$json
