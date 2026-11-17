param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[a-z0-9_]+(?:-[a-z0-9_]+)+$')]
    [string]$Target,
    [string]$BinaryPath = '',
    [string]$OutputDirectory = "$PSScriptRoot/../dist",
    [string]$ExpectedTag = ''
)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path -LiteralPath "$PSScriptRoot/..").Path
$metadataText = & cargo metadata --manifest-path "$repo/Cargo.toml" --locked --no-deps --format-version 1
if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed' }
$metadata = ($metadataText -join "`n") | ConvertFrom-Json
$package = @($metadata.packages | Where-Object name -eq 'airgap')
if ($package.Count -ne 1) { throw 'Expected one airgap package' }
$version = $package[0].version
if ($version -notmatch '^[0-9][0-9A-Za-z.+-]*$') { throw 'Invalid package version' }
if ($ExpectedTag -and $ExpectedTag -cne "v$version") { throw 'Release tag must match the Cargo package version' }

$windowsTarget = $Target -match '-windows-'
$binaryName = if ($windowsTarget) { 'airgap.exe' } else { 'airgap' }
if (-not $BinaryPath) { $BinaryPath = "$repo/target/$Target/release/$binaryName" }
$binary = (Resolve-Path -LiteralPath $BinaryPath).Path
if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { throw 'Binary must be a file' }
$outputRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
$null = New-Item -ItemType Directory -Force -Path $outputRoot
$extension = if ($windowsTarget) { 'zip' } else { 'tar.gz' }
$archive = Join-Path $outputRoot "airgap-v$version-$Target.$extension"
if (Test-Path -LiteralPath $archive) { throw 'Archive already exists; choose an empty output directory' }
$stage = Join-Path $outputRoot ('.stage-' + [guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path $stage
$members = @($binaryName, 'LICENSE-MIT', 'LICENSE-APACHE')
try {
    Copy-Item -LiteralPath $binary -Destination (Join-Path $stage $binaryName)
    foreach ($license in @('LICENSE-MIT', 'LICENSE-APACHE')) {
        Copy-Item -LiteralPath (Join-Path $repo $license) -Destination (Join-Path $stage $license)
    }
    if ($windowsTarget) {
        $files = @($members | ForEach-Object { Join-Path $stage $_ })
        Compress-Archive -LiteralPath $files -DestinationPath $archive -CompressionLevel Optimal
    } else {
        if ($env:OS -ne 'Windows_NT') {
            & chmod +x (Join-Path $stage $binaryName)
            if ($LASTEXITCODE -ne 0) { throw 'Cannot set executable permission' }
        }
        & tar -czf $archive -C $stage -- @members
        if ($LASTEXITCODE -ne 0) { throw 'Archive creation failed' }
    }
    $hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    $checksum = "$archive.sha256"
    [System.IO.File]::WriteAllText($checksum, ($hash + '  ' + [System.IO.Path]::GetFileName($archive) + "`n"), [System.Text.Encoding]::ASCII)
    @{ version=$version; target=$Target; archive=$archive; checksum=$checksum; sha256=$hash } | ConvertTo-Json
} finally {
    $stagePath = [System.IO.Path]::GetFullPath($stage)
    $allowedPrefix = $outputRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
    if (-not $stagePath.StartsWith($allowedPrefix, [System.StringComparison]::Ordinal)) { throw 'Refusing cleanup outside the output directory' }
    Remove-Item -LiteralPath $stagePath -Recurse -Force
}
