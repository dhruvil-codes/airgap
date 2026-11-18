& {
$ErrorActionPreference = 'Stop'
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) { throw 'Use scripts/install.sh on Linux and macOS' }
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
Add-Type -AssemblyName System.IO.Compression.FileSystem
$isArm64 = ($env:PROCESSOR_ARCHITEW6432 -eq 'ARM64') -or ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64')
$target = if ($isArm64) {
    'aarch64-pc-windows-msvc'
} elseif ([Environment]::Is64BitOperatingSystem) {
    'x86_64-pc-windows-msvc'
} else {
    throw 'airgap supports 64-bit x86 and ARM Windows.'
}
$base = 'https://github.com/dhruvil-codes/airgap/releases'
if ($env:AIRGAP_VERSION) {
    $tag = 'v' + ($env:AIRGAP_VERSION -replace '^v', '')
} else {
    try {
        $tag = (Invoke-RestMethod 'https://api.github.com/repos/dhruvil-codes/airgap/releases/latest' -TimeoutSec 10).tag_name
    } catch {
        throw 'No published airgap release could be resolved. See the repository for source installation.'
    }
}
if (-not $tag -or $tag -notmatch '^v[0-9][0-9A-Za-z.+-]*$') { throw 'Invalid release version' }
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
        if (-not $entry) { throw 'Archive does not contain airgap.exe' }
        if ($entry.Length -gt 256MB) { throw 'Release executable is unexpectedly large' }
        $binaryPath = Join-Path $tempRoot 'airgap.exe'
        $source = $entry.Open()
        $destination = [System.IO.File]::Open($binaryPath, [System.IO.FileMode]::CreateNew)
        try { $source.CopyTo($destination) } finally { $source.Dispose(); $destination.Dispose() }
    } finally { $zip.Dispose() }
    $expectedVer = 'airgap ' + ($tag -replace '^v', '')
    $versionOutput = $null
    $prevEAP = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $versionOutput = (& $binaryPath --version 2>&1 | Out-String).Trim()
    } finally {
        $ErrorActionPreference = $prevEAP
    }
    if ($LASTEXITCODE -ne 0 -or $versionOutput -ne $expectedVer) {
        throw "Executable version ($versionOutput) does not match expected release ($expectedVer)"
    }
    $null = New-Item -ItemType Directory -Force -Path $installDir
    $installed = Join-Path $installDir 'airgap.exe'
    $staged = Join-Path $installDir ('.airgap-' + [guid]::NewGuid().ToString('N') + '.exe')
    Move-Item -LiteralPath $binaryPath -Destination $staged
    if (Test-Path -LiteralPath $installed) {
        $oldInstalled = Join-Path $installDir ('.airgap-' + [guid]::NewGuid().ToString('N') + '.old')
        Move-Item -LiteralPath $installed -Destination $oldInstalled -Force
        try {
            Move-Item -LiteralPath $staged -Destination $installed -Force
            $staged = $null
        } catch {
            Move-Item -LiteralPath $oldInstalled -Destination $installed -Force
            throw
        }
        try { Remove-Item -LiteralPath $oldInstalled -Force -ErrorAction SilentlyContinue } catch {}
    } else {
        Move-Item -LiteralPath $staged -Destination $installed -Force
        $staged = $null
    }
    if ($env:AIRGAP_NO_PATH_UPDATE -ne '1') {
        try {
            $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
            if (@($userPath -split ';') -notcontains $installDir) { [Environment]::SetEnvironmentVariable('Path', ($installDir + ';' + $userPath), 'User') }
        } catch { Write-Warning 'Could not persist PATH. Editor configuration will still use the full executable path.' }
        if (@($env:PATH -split ';') -notcontains $installDir) { $env:PATH = $installDir + ';' + $env:PATH }
    }
    $prevEAP = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        & $installed setup --yes
    } finally {
        $ErrorActionPreference = $prevEAP
    }
    if ($LASTEXITCODE -ne 0) { throw 'Installed airgap, but setup failed; run airgap setup --yes after resolving the reported configuration error' }
    Write-Output "Installed $tag to $installed"
} finally {
    if ($staged -and (Test-Path -LiteralPath $staged)) {
        Remove-Item -LiteralPath $staged -Force -ErrorAction SilentlyContinue
    }
    if ($tempRoot -and (Test-Path -LiteralPath $tempRoot)) {
        $resolvedTemp = [System.IO.Path]::GetFullPath($tempRoot)
        if ($tempParent -and $resolvedTemp.StartsWith($tempParent + [System.IO.Path]::DirectorySeparatorChar, [StringComparison]::Ordinal)) {
            Remove-Item -LiteralPath $resolvedTemp -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}
}
