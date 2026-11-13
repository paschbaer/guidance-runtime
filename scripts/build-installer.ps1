<#
.SYNOPSIS
Builds the Guidance Runtime MSI installer with WiX v7 (installer/GuidanceRuntime.wxs).

.PARAMETER Arch
Target architecture for the MSI: x64 (default) or arm64.

.PARAMETER BinDir
Directory containing the built executables. Defaults to target\release.
Relative paths are resolved against the repository root (CI cross-compile layout:
target/<triple>/release).

.PARAMETER Version
Overrides the MSI ProductVersion (must be numeric, e.g. from the release workflow).
Defaults to the workspace version from Cargo.toml.

.PARAMETER SkipBuild
Reuse an existing target/release build instead of running cargo build --release.

.EXAMPLE
./scripts/build-installer.ps1
./scripts/build-installer.ps1 -Arch arm64 -SkipBuild
#>
param(
  [ValidateSet('x64','arm64')]
  [string]$Arch = 'x64',
  [string]$BinDir,
  [string]$Version,
  [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot

if (-not (Get-Command wix -ErrorAction SilentlyContinue)) {
  throw "WiX CLI not found. Install it with: dotnet tool install --global wix --version 7.*"
}

$requiredExes = @('guidance-service.exe','guidance.exe','guidance-tray.exe','guidance-updater.exe')
$binDir = if ($BinDir) {
  if ([System.IO.Path]::IsPathRooted($BinDir)) { $BinDir } else { Join-Path $repoRoot $BinDir }
} else {
  Join-Path $repoRoot 'target\release'
}

if (-not $SkipBuild) {
  cargo build --release
  if ($LASTEXITCODE -ne 0) { throw "cargo build --release failed with exit code $LASTEXITCODE" }
}

foreach ($exe in $requiredExes) {
  if (-not (Test-Path (Join-Path $binDir $exe))) {
    throw "Missing $exe in $binDir. Build first (drop -SkipBuild) or verify the target directory."
  }
}

if ($Version) {
  $resolvedVersion = $Version
} else {
  $versionLine = Select-String -Path (Join-Path $repoRoot 'Cargo.toml') -Pattern '^version="([^"]+)"' |
    Select-Object -First 1
  if (-not $versionLine) { throw "Could not read workspace version from Cargo.toml" }
  $resolvedVersion = $versionLine.Matches[0].Groups[1].Value
}

$distDir = Join-Path $repoRoot 'dist'
New-Item -ItemType Directory -Force $distDir | Out-Null
$msiPath = Join-Path $distDir "guidance-runtime-$resolvedVersion-$Arch.msi"

wix build `
  -acceptEula wix7 `
  -arch $Arch `
  -d "GuidanceVersion=$resolvedVersion" `
  -d "ReleaseBin=$binDir" `
  -d "ConfigDir=$(Join-Path $repoRoot 'config')" `
  -o $msiPath `
  (Join-Path $repoRoot 'installer\GuidanceRuntime.wxs')

if ($LASTEXITCODE -ne 0) { throw "wix build failed with exit code $LASTEXITCODE" }

Write-Host "MSI written to $msiPath"
Write-Output $msiPath
