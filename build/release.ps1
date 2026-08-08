# ==============================================================
# Xuanyu Compiler Release Packaging Script (PowerShell)
# --------------------------------------------------------------
# Mirrors build_l2.bat and produces a distributable release:
#   - target/release/xy(.exe)   the compiler
#   - runtime/runtime.c         runtime library
#   - src/compiler_v2/xyc.xy    L2 self-hosting compiler source
#   - examples/                 sample programs
#   - docs/                     docs (README/CHANGELOG/API_REFERENCE)
#   - VERSION                   version file
# Usage: powershell -File build/release.ps1 [version]
# Example: powershell -File build/release.ps1 v0.2.0-beta
# ==============================================================

param(
    [string]$Version = "0.2.0-beta"
)

$ErrorActionPreference = "Stop"

function Write-Step([string]$msg) {
    Write-Host ""
    Write-Host "[$msg]" -ForegroundColor Cyan
}

function Assert-Last([string]$step) {
    if ($LASTEXITCODE -ne 0) {
        Write-Error "[$step] failed, exit code $LASTEXITCODE"
        exit 1
    }
}

$root = (Resolve-Path "$PSScriptRoot\..").Path
$releaseDir = Join-Path $root "dist"
$versionDir  = Join-Path $releaseDir "xuanyu-$Version"

# ---------- 1. Build the release compiler ----------
Write-Step "1/6 Build release compiler"
Push-Location $root
cargo build --release
Assert-Last "cargo build --release"
Pop-Location

# ---------- 2. Run tests ----------
Write-Step "2/6 Run tests"
Push-Location $root
cargo test --all-targets
Assert-Last "cargo test"
Pop-Location

# ---------- 3. Prepare packaging dir ----------
Write-Step "3/6 Prepare packaging dir"
if (Test-Path $releaseDir) {
    Remove-Item -LiteralPath $releaseDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $releaseDir | Out-Null
New-Item -ItemType Directory -Force -Path $versionDir | Out-Null

# ---------- 4. Collect artifacts ----------
Write-Step "4/6 Collect artifacts"

$xyExe = Join-Path $root "target\release\xy.exe"
if (Test-Path $xyExe) {
    Copy-Item $xyExe -Destination (Join-Path $versionDir "xy.exe")
} else {
    # Non-Windows: xy
    $xy = Join-Path $root "target/release/xy"
    if (Test-Path $xy) {
        Copy-Item $xy -Destination (Join-Path $versionDir "xy")
    } else {
        Write-Error "xy executable not found"
        exit 1
    }
}

# Runtime library
Copy-Item (Join-Path $root "runtime\runtime.c") -Destination (Join-Path $versionDir "runtime.c")

# L2 self-hosting compiler source
$xycSrc = Join-Path $root "src\compiler_v2\xyc.xy"
if (Test-Path $xycSrc) {
    Copy-Item $xycSrc -Destination (Join-Path $versionDir "xyc.xy")
}

# Examples
$examplesDir = Join-Path $versionDir "examples"
New-Item -ItemType Directory -Force -Path $examplesDir | Out-Null
if (Test-Path (Join-Path $root "examples\hello.xy")) {
    Copy-Item (Join-Path $root "examples\*.xy") -Destination $examplesDir
}

# Docs
$docsDir = Join-Path $versionDir "docs"
New-Item -ItemType Directory -Force -Path $docsDir | Out-Null
Copy-Item (Join-Path $root "README.md") -Destination (Join-Path $versionDir "README.md")
Copy-Item (Join-Path $root "docs\CHANGELOG.md") -Destination (Join-Path $docsDir "CHANGELOG.md")
Copy-Item (Join-Path $root "docs\API_REFERENCE.md") -Destination (Join-Path $docsDir "API_REFERENCE.md")

# Version info
Set-Content -LiteralPath (Join-Path $versionDir "VERSION") -Value $Version -Encoding UTF8

Write-Host "Collected $((Get-ChildItem -LiteralPath $versionDir -Recurse -File).Count) files"

# ---------- 5. Archive ----------
Write-Step "5/6 Archive zip"
$zipPath = Join-Path $releaseDir "xuanyu-$Version.zip"
if (Test-Path $zipPath) { Remove-Item -LiteralPath $zipPath -Force }
Compress-Archive -LiteralPath $versionDir -DestinationPath $zipPath -CompressionLevel Optimal

# ---------- 6. Self-check ----------
Write-Step "6/6 Self-check: packaged xy compiles hello.xy"
New-Item -ItemType Directory -Force -Path (Join-Path $root "dist\_smoke") | Out-Null
$smokeExe = Join-Path $root "dist\_smoke\xy.exe"
Copy-Item (Join-Path $versionDir "xy.exe") -Destination $smokeExe -Force
$helloPath = Join-Path $versionDir "examples\hello.xy"
$logFile = Join-Path $releaseDir "_smoke.log"
# Run via cmd /c so native stderr is not wrapped as PowerShell error records
cmd /c "`"$smokeExe`" `"$helloPath`" --run > `"$logFile`" 2>&1"
if ($LASTEXITCODE -eq 0) {
    Write-Host "Self-check passed: hello.xy compiled and ran" -ForegroundColor Green
} else {
    Write-Warning "Self-check failed, see dist/_smoke.log"
    exit 1
}
Remove-Item -LiteralPath (Join-Path $root "dist\_smoke") -Recurse -Force
Remove-Item -LiteralPath (Join-Path $releaseDir "_smoke.log") -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath (Join-Path $versionDir "xy.exe") -Force -ErrorAction SilentlyContinue

Write-Step "Release complete"
Write-Host "Artifacts: $releaseDir"
Write-Host "  - xuanyu-$Version/    extracted dist dir"
Write-Host "  - xuanyu-$Version.zip dist archive"
Write-Host ""
