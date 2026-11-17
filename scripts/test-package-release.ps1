$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$repo = (Resolve-Path -LiteralPath "$PSScriptRoot/..").Path
$testParent = [System.IO.Path]::GetFullPath("$repo/target/package-tests")
$testRoot = Join-Path $testParent ([guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Force -Path $testRoot
function Assert($Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
try {
    $fixture = Join-Path $testRoot 'fixture.bin'
    [System.IO.File]::WriteAllText($fixture, 'synthetic binary fixture')
    [System.IO.File]::WriteAllText((Join-Path $testRoot '.env'), 'PRIVATE_VALUE=must-not-be-packaged')
    foreach ($target in @('x86_64-pc-windows-gnu', 'x86_64-unknown-linux-musl')) {
        $result = & "$PSScriptRoot/package-release.ps1" -Target $target -BinaryPath $fixture -OutputDirectory $testRoot | ConvertFrom-Json
        $checksum = [System.IO.File]::ReadAllText($result.checksum)
        $hash = (Get-FileHash -LiteralPath $result.archive -Algorithm SHA256).Hash.ToLowerInvariant()
        Assert ($checksum -ceq ($hash + '  ' + [System.IO.Path]::GetFileName($result.archive) + "`n")) 'Checksum does not describe the archive'
        $extract = Join-Path $testRoot $target
        $null = New-Item -ItemType Directory -Path $extract
        if ($target -match '-windows-') {
            [System.IO.Compression.ZipFile]::ExtractToDirectory($result.archive, $extract)
            $name = 'airgap.exe'
        } else {
            & tar -xzf $result.archive -C $extract
            Assert ($LASTEXITCODE -eq 0) 'Cannot extract tar archive'
            $name = 'airgap'
        }
        $names = @(Get-ChildItem -LiteralPath $extract -Recurse -File | ForEach-Object Name | Sort-Object)
        Assert (($names -join ',') -ceq ((@($name, 'LICENSE-MIT', 'LICENSE-APACHE') | Sort-Object) -join ',')) 'Unexpected or missing archive members'
        Assert ((Get-FileHash (Join-Path $extract $name)).Hash -eq (Get-FileHash $fixture).Hash) 'Packaged binary differs from input'
        $refused = $false
        try { $null = & "$PSScriptRoot/package-release.ps1" -Target $target -BinaryPath $fixture -OutputDirectory $testRoot } catch { $refused = $true }
        Assert $refused 'An existing release archive was overwritten'
    }
    $refused = $false
    try { $null = & "$PSScriptRoot/package-release.ps1" -Target x86_64-pc-windows-gnu -BinaryPath $fixture -OutputDirectory $testRoot -ExpectedTag v999.0.0 } catch { $refused = $true }
    Assert $refused 'Mismatched release tag was accepted'
    'Release packaging checks passed: ZIP, tar.gz, exact members, binary integrity, checksums, overwrite protection and version matching.'
} finally {
    $fullTestRoot = [System.IO.Path]::GetFullPath($testRoot)
    if (-not $fullTestRoot.StartsWith($testParent + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::Ordinal)) { throw 'Refusing cleanup outside package test directory' }
    Remove-Item -LiteralPath $fullTestRoot -Recurse -Force
}
