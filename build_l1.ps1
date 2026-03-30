# build_l1.ps1
# L1: Verify all XY modules can be compiled to IR by Rust compiler
# Usage: .\build_l1.ps1

$ErrorActionPreference = "Stop"

Write-Host "========================================"
Write-Host "  L1: XY Compiler to IR Verification"
Write-Host "========================================"
Write-Host ""

$PROJECT_ROOT = $PSScriptRoot
$SRC_DIR = Join-Path $PROJECT_ROOT "src\compiler_v2"
$TARGET_DIR = Join-Path $PROJECT_ROOT "target"
$L1_DIR = Join-Path $TARGET_DIR "l1_ir"

$XY_COMPILER = Join-Path $TARGET_DIR "release\xy.exe"

Write-Host "[1/3] Checking Rust compiler..."
if (-not (Test-Path $XY_COMPILER)) {
    Write-Host "ERROR: Rust compiler not found: $XY_COMPILER"
    Write-Host "Run: cargo build"
    exit 1
}
Write-Host "OK: Rust compiler found: $XY_COMPILER"
Write-Host ""

Write-Host "[2/3] Creating output directory..."
if (-not (Test-Path $L1_DIR)) {
    New-Item -ItemType Directory -Path $L1_DIR -Force | Out-Null
}
Write-Host "OK: Output directory: $L1_DIR"
Write-Host ""

Write-Host "[3/3] Compiling XY modules to IR..."
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
$utf8NoBom = New-Object System.Text.UTF8Encoding $false

foreach ($xyFile in $xyFiles) {
    $fullPath = Join-Path $PROJECT_ROOT $xyFile
    $fileName = Split-Path $xyFile -Leaf
    $baseName = [System.IO.Path]::GetFileNameWithoutExtension($fileName)
    $irFile = Join-Path $L1_DIR "$baseName.ll"

    Write-Host "  Compiling: $fileName"

    if (-not (Test-Path $fullPath)) {
        Write-Host "    ERROR: File not found"
        $failCount++
        continue
    }

    # Clear cache
    $cacheFile = "$fullPath.cache"
    if (Test-Path $cacheFile) {
        Remove-Item $cacheFile -Force
    }

    # Compile with --ir-pure and save with UTF-8 no BOM
    $irContent = & $XY_COMPILER $fullPath "--ir-pure" 2>&1
    $exitCode = $LASTEXITCODE

    if ($exitCode -eq 0) {
        [System.IO.File]::WriteAllLines($irFile, $irContent, $utf8NoBom)
        Write-Host "    OK: IR saved to $irFile"
        $successCount++
    } else {
        Write-Host "    FAILED (exit code: $exitCode)"
        Write-Host "    Output: $irContent"
        $failCount++
    }
}

Write-Host ""
Write-Host "L1 Build Summary"
Write-Host "----------------------------------------"
Write-Host "  Success: $successCount"
Write-Host "  Failed:  $failCount"
Write-Host ""

if ($failCount -eq 0) {
    Write-Host "========================================"
    Write-Host "  L1 Success - All modules compiled!"
    Write-Host "========================================"
    Write-Host ""
    Write-Host "IR files saved to: $L1_DIR"
    Write-Host ""
    exit 0
} else {
    Write-Host "========================================"
    Write-Host "  L1 Build Failed"
    Write-Host "========================================"
    exit 1
}
