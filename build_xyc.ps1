# build_xyc.ps1
# L2: Verify XY modules can be compiled by Rust compiler
# Usage: .\build_xyc.ps1

$ErrorActionPreference = "Stop"

Write-Host "========================================"
Write-Host "  XY Compiler Builder (L2)"
Write-Host "========================================"
Write-Host ""

$PROJECT_ROOT = $PSScriptRoot
$SRC_DIR = Join-Path $PROJECT_ROOT "src\compiler_v2"
$TARGET_DIR = Join-Path $PROJECT_ROOT "target"
$XYC_DIR = Join-Path $TARGET_DIR "xyc"

$XY_COMPILER = Join-Path $TARGET_DIR "debug\xy.exe"

Write-Host "[1/4] Checking Rust compiler..."
if (-not (Test-Path $XY_COMPILER)) {
    Write-Host "ERROR: Rust compiler not found: $XY_COMPILER"
    Write-Host "Run: cargo build"
    exit 1
}
Write-Host "OK: Rust compiler found: $XY_COMPILER"
Write-Host ""

Write-Host "[2/4] Creating output directory..."
if (-not (Test-Path $XYC_DIR)) {
    New-Item -ItemType Directory -Path $XYC_DIR -Force | Out-Null
}
Write-Host "OK: Output directory: $XYC_DIR"
Write-Host ""

Write-Host "[3/4] Compiling XY modules..."
$xyFiles = @(
    "src\compiler_v2\runtime.xy",
    "src\compiler_v2\lexer.xy",
    "src\compiler_v2\parser.xy",
    "src\compiler_v2\sema.xy",
    "src\compiler_v2\codegen.xy",
    "src\compiler_v2\main.xy"
)

$successCount = 0
$failCount = 0

foreach ($xyFile in $xyFiles) {
    $fullPath = Join-Path $PROJECT_ROOT $xyFile
    $fileName = Split-Path $xyFile -Leaf

    Write-Host "  Compiling: $fileName"

    if (-not (Test-Path $fullPath)) {
        Write-Host "    ERROR: File not found"
        $failCount++
        continue
    }

    $output = & $XY_COMPILER $fullPath --ir 2>&1
    $exitCode = $LASTEXITCODE

    if ($exitCode -eq 0) {
        Write-Host "    OK"
        $successCount++
    } else {
        Write-Host "    FAILED (exit code: $exitCode)"
        $failCount++
    }
}

Write-Host ""
Write-Host "[4/4] Build Summary"
Write-Host "----------------------------------------"
Write-Host "  Success: $successCount"
Write-Host "  Failed:  $failCount"
Write-Host ""

if ($failCount -eq 0) {
    Write-Host "========================================"
    Write-Host "  L2 Build Complete - All modules compiled"
    Write-Host "========================================"
    Write-Host ""
    Write-Host "Next: Run L3 bootstrap verification"
    Write-Host '  .\tests\bootstrap_test.ps1'
    Write-Host ""
    exit 0
} else {
    Write-Host "========================================"
    Write-Host "  L2 Build Failed"
    Write-Host "========================================"
    exit 1
}
