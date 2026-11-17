& {
$ErrorActionPreference = 'Stop'
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) { throw 'Use scripts/install.sh on Linux and macOS' }
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
Add-Type -AssemblyName System.IO.Compression.FileSystem
$architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
$target = switch ($architecture) {
    'X64' { 'x86_64-pc-windows-msvc' }
    'Arm64' { 'aarch64-pc-windows-msvc' }
    default { throw 'airgap supports 64-bit x86 and ARM Windows.' }
}
$base = 'https://github.com/dhruvil-codes/airgap/releases'
if ($env:AIRGAP_VERSION) { $tag = 'v' + $env:AIRGAP_VERSION.TrimStart('v') }
else {
    try { $tag = (Invoke-RestMethod 'https://api.github.com/repos/dhruvil-codes/airgap/releases/latest' -TimeoutSec 10).tag_name }
    catch { throw 'No published airgap release could be resolved. See the repository for source installation.' }
}
if ($tag -notmatch '^v[0-9][0-9A-Za-z.+-]*$') { throw 'Invalid release version' }
$archiveName = "airgap-$tag-$target.zip"
$userHome = $env:USERPROFILE
if (-not $userHome) { $userHome = [Environment]::GetFolderPath('UserProfile') }
$installDir = if ($env:AIRGAP_INSTALL_DIR) { $env:AIRGAP_INSTALL_DIR } else { Join-Path $userHome '.cargo/bin' }
if (-not [System.IO.Path]::IsPathRooted($installDir)) { throw 'AIRGAP_INSTALL_DIR must be absolute' }
$installDir = [System.IO.Path]::GetFullPath($installDir)
$tempParent = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd([System.IO.Path]::DirectorySeparatorChar)
$tempRoot = Join-Path $tempParent ('airgap-' + [guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path $tempRoot
$staged = $null
try {
    $archivePath = Join-Path $tempRoot $archiveName
    $checksumPath = "$archivePath.sha256"
    Invoke-WebRequest -UseBasicParsing "$base/download/$tag/$archiveName" -TimeoutSec 120 -OutFile $archivePath
    Invoke-WebRequest -UseBasicParsing "$base/download/$tag/$archiveName.sha256" -TimeoutSec 120 -OutFile $checksumPath
    $checksum = [System.IO.File]::ReadAllText($checksumPath)
    if ($checksum -notmatch '\A([a-f0-9]{64})[ \t]+([^\r\n]+)\r?\n?\z' -or $matches[2] -cne $archiveName) { throw 'Invalid release checksum file' }
    $expected = $matches[1]
    if ((Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant() -cne $expected) { throw 'Checksum mismatch; the installed binary was not changed' }
    $zip = [System.IO.Compression.ZipFile]::OpenRead($archivePath)
    try {
        $names = @($zip.Entries | ForEach-Object FullName | Sort-Object)
        $expectedNames = @('airgap.exe', 'LICENSE-MIT', 'LICENSE-APACHE') | Sort-Object
        if (($names -join ',') -cne ($expectedNames -join ',')) { throw 'Unexpected archive members' }
        $entry = $zip.GetEntry('airgap.exe')
        if ($entry.Length -gt 128MB) { throw 'Release executable is unexpectedly large' }
        $binaryPath = Join-Path $tempRoot 'airgap.exe'
        $source = $entry.Open()
        $destination = [System.IO.File]::Open($binaryPath, [System.IO.FileMode]::CreateNew)
        try { $source.CopyTo($destination) } finally { $source.Dispose(); $destination.Dispose() }
    } finally { $zip.Dispose() }
    $version = & $binaryPath --version
    if ($LASTEXITCODE -ne 0 -or $version -cne "airgap $($tag.Substring(1))") { throw 'Executable version does not match the release' }
    $null = New-Item -ItemType Directory -Force -Path $installDir
    $installed = Join-Path $installDir 'airgap.exe'
    $staged = Join-Path $installDir ('.airgap-' + [guid]::NewGuid().ToString('N') + '.exe')
    Copy-Item -LiteralPath $binaryPath -Destination $staged
    if (Test-Path -LiteralPath $installed) { [System.IO.File]::Replace($staged, $installed, [NullString]::Value) }
    else { [System.IO.File]::Move($staged, $installed) }
    $staged = $null
    if ($env:AIRGAP_NO_PATH_UPDATE -ne '1') {
        try {
            $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
            if (@($userPath -split ';') -notcontains $installDir) { [Environment]::SetEnvironmentVariable('Path', ($installDir + ';' + $userPath), 'User') }
        } catch { Write-Warning 'Could not persist PATH. Editor configuration will still use the full executable path.' }
        if (@($env:PATH -split ';') -notcontains $installDir) { $env:PATH = $installDir + ';' + $env:PATH }
    }
    & $installed setup --yes
    if ($LASTEXITCODE -ne 0) { throw 'Installed airgap, but setup failed; run airgap setup --yes after resolving the reported configuration error' }
    Write-Output "Installed $tag to $installed"
} finally {
    if ($staged -and (Test-Path -LiteralPath $staged)) { Remove-Item -LiteralPath $staged -Force }
    $resolvedTemp = [System.IO.Path]::GetFullPath($tempRoot)
    if (-not $resolvedTemp.StartsWith($tempParent + [System.IO.Path]::DirectorySeparatorChar, [StringComparison]::Ordinal)) { throw 'Refusing cleanup outside the temporary directory' }
    Remove-Item -LiteralPath $resolvedTemp -Recurse -Force
}
}
