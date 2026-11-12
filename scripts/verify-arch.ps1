#!/usr/bin/env pwsh
# Verifies a built guidance executable matches the target architecture it was
# compiled for, by reading the PE/COFF header machine field.
#
# Sysmon/ActivityLog-level smoke tests are impossible for a cross-compiled
# arm64 binary on an x64 runner (Windows refuses to start it, os error 216),
# so the strongest available check is the one the loader itself would make:
# does the image declare the expected machine type?
#
# Usage: scripts/verify-arch.ps1 -Path <exe> -Machine ARM64|x64
#        scripts/verify-arch.ps1 -Path <exe> -ExpectedTarget aarch64-pc-windows-msvc
param(
    [Parameter(Mandatory)][string]$Path,
    [string]$Machine,
    [string]$ExpectedTarget
)

$ErrorActionPreference = 'Stop'

# Map Rust target triples onto the PE machine values the Windows loader checks.
$targetMachine = @{
    'x86_64-pc-windows-msvc'  = 'x64'
    'aarch64-pc-windows-msvc' = 'ARM64'
}
$machineValue = @{
    'x64'   = 0x8664
    'ARM64' = 0xAA64
}
$name = @{
    'x64'   = 'IMAGE_FILE_MACHINE_AMD64'
    'ARM64' = 'IMAGE_FILE_MACHINE_ARM64'
}

if ($ExpectedTarget) {
    if (-not $targetMachine.ContainsKey($ExpectedTarget)) {
        throw "unsupported target '$ExpectedTarget' (known: $($targetMachine.Keys -join ', '))"
    }
    if ($Machine -and $Machine -ne $targetMachine[$ExpectedTarget]) {
        throw "-Machine '$Machine' contradicts -ExpectedTarget '$ExpectedTarget'"
    }
    $Machine = $targetMachine[$ExpectedTarget]
}
if (-not $Machine) {
    throw 'either -Machine or -ExpectedTarget is required'
}
if (-not $machineValue.ContainsKey($Machine)) {
    throw "unsupported machine '$Machine' (known: $($machineValue.Keys -join ', '))"
}

if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
    throw "not a file: $Path"
}

# PE layout: 'MZ' at 0x00, e_lfanew (u32 LE) at 0x3C points at the PE signature,
# the COFF header follows 4 bytes later and starts with the machine field.
$stream = [System.IO.File]::OpenRead((Resolve-Path -LiteralPath $Path))
try {
    $reader = [System.IO.BinaryReader]::new($stream)
    if ($stream.Length -lt 0x40) {
        throw "$Path is too small to be a PE image"
    }
    $stream.Position = 0x3C
    $lfanew = $reader.ReadUInt32()
    if (($lfanew + 24) -gt $stream.Length) {
        throw "$Path has a truncated PE header (e_lfanew=$lfanew)"
    }
    $stream.Position = $lfanew
    $signature = $reader.ReadUInt32()
    if ($signature -ne 0x00004550) {
        throw ("{0} has no PE signature (found 0x{1:X8})" -f $Path, $signature)
    }
    $actual = $reader.ReadUInt16()
} finally {
    $stream.Dispose()
}

$expected = $machineValue[$Machine]
if ($actual -ne $expected) {
    throw ("{0}: PE machine is 0x{1:X4}, expected 0x{2:X4} ({3})" -f $Path, $actual, $expected, $name[$Machine])
}

Write-Host ("OK  {0} -> {1} (0x{2:X4})" -f $Path, $name[$Machine], $actual)
exit 0
