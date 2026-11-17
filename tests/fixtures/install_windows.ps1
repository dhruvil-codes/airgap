param([string]$Installer, [string]$Binary)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$testRoot=$env:AIRGAP_INSTALL_TEST_ROOT
$null=New-Item -ItemType Directory -Force -Path "$testRoot/home","$testRoot/bin","$testRoot/fixture","$testRoot/.cursor"
[System.IO.File]::WriteAllText("$testRoot/.cursor/mcp.json",'{"mcpServers":{"keep":{"command":"existing"}}}')
[System.IO.File]::WriteAllText("$testRoot/CLAUDE.md",'# Existing team instructions')
Copy-Item -LiteralPath $Binary -Destination "$testRoot/fixture/airgap.exe"
foreach($name in @('LICENSE-MIT','LICENSE-APACHE')) { [System.IO.File]::WriteAllText("$testRoot/fixture/$name",'synthetic license fixture') }
Compress-Archive -LiteralPath "$testRoot/fixture/airgap.exe","$testRoot/fixture/LICENSE-MIT","$testRoot/fixture/LICENSE-APACHE" -DestinationPath "$testRoot/asset.zip"
$version=& $Binary --version
if ($LASTEXITCODE -ne 0) { throw "Fixture executable failed with exit $LASTEXITCODE" }
$tag='v'+($version -replace '^airgap ','')
$target=if([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString() -eq 'Arm64'){'aarch64-pc-windows-msvc'}else{'x86_64-pc-windows-msvc'}
$archiveName="airgap-$tag-$target.zip"
$env:AIRGAP_TEST_TAG=$tag
function global:Invoke-RestMethod { param($Uri,[int]$TimeoutSec) if($env:AIRGAP_TEST_MODE -eq 'missing'){throw 'Synthetic missing release'}; return @{tag_name=$env:AIRGAP_TEST_TAG} }
function global:Invoke-WebRequest {
    param([Parameter(Position=0)][string]$Uri,[string]$OutFile,[switch]$UseBasicParsing,[int]$TimeoutSec)
    $source=if($Uri.EndsWith('.sha256')){'checksum'}else{'asset.zip'}
    Copy-Item -LiteralPath (Join-Path $env:AIRGAP_INSTALL_TEST_ROOT $source) -Destination $OutFile
}
function Expect-Failure([string]$Message) {
    $failed=$false
    try { & $Installer } catch { $failed=$true; if($_.Exception.Message -notlike "*$Message*"){throw "Expected '$Message'; received: $($_.Exception.Message)"} }
    if(-not $failed){throw 'Installer accepted an invalid release'}
}
[System.IO.File]::WriteAllText("$testRoot/bin/airgap.exe",'existing-install')
$env:AIRGAP_TEST_MODE='missing'
Expect-Failure 'No published airgap release'
$env:AIRGAP_TEST_MODE='good'
[System.IO.File]::WriteAllText("$testRoot/checksum",(('0'*64)+"  $archiveName`n"))
Expect-Failure 'Checksum mismatch'
if([System.IO.File]::ReadAllText("$testRoot/bin/airgap.exe") -ne 'existing-install'){throw 'Existing installation was modified before verification'}
$hash=(Get-FileHash "$testRoot/asset.zip" -Algorithm SHA256).Hash.ToLowerInvariant()
[System.IO.File]::WriteAllText("$testRoot/checksum",($hash+"  $archiveName`n"))
& $Installer
if($LASTEXITCODE -ne 0){throw 'Valid installer failed'}
$config=[System.IO.File]::ReadAllText("$testRoot/.cursor/mcp.json")|ConvertFrom-Json
if($config.mcpServers.keep.command -ne 'existing'){throw 'Setup overwrote another server'}
$before=[System.IO.File]::ReadAllBytes("$testRoot/.mcp.json")
& $Installer
$after=[System.IO.File]::ReadAllBytes("$testRoot/.mcp.json")
if([Convert]::ToBase64String($before) -ne [Convert]::ToBase64String($after)){throw 'Repeated installation was not idempotent'}
if(-not [System.IO.File]::ReadAllText("$testRoot/CLAUDE.md").StartsWith('# Existing team instructions')){throw 'Team instructions were overwritten'}
$oldHash=(Get-FileHash "$testRoot/bin/airgap.exe").Hash
$zip=[System.IO.Compression.ZipFile]::Open("$testRoot/asset.zip",[System.IO.Compression.ZipArchiveMode]::Update)
try {$null=$zip.CreateEntry('../unexpected')}finally{$zip.Dispose()}
$hash=(Get-FileHash "$testRoot/asset.zip" -Algorithm SHA256).Hash.ToLowerInvariant()
[System.IO.File]::WriteAllText("$testRoot/checksum",($hash+"  $archiveName`n"))
Expect-Failure 'Unexpected archive members'
if((Get-FileHash "$testRoot/bin/airgap.exe").Hash -ne $oldHash){throw 'Invalid archive replaced the installed binary'}
'Installer checks passed.'
