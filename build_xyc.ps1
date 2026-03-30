# build_xyc.ps1
# L2: Complete build - Compile XY modules to object files and link to xyc.exe
# Usage: .\build_xyc.ps1

$ErrorActionPreference = "Stop"

Write-Host "========================================"
Write-Host "  XY Compiler Builder (Complete L2)"
Write-Host "========================================"
Write-Host ""

$PROJECT_ROOT = $PSScriptRoot
$SRC_DIR = Join-Path $PROJECT_ROOT "src\compiler_v2"
$TARGET_DIR = Join-Path $PROJECT_ROOT "target"
$XYC_DIR = Join-Path $TARGET_DIR "xyc"

$XY_COMPILER = Join-Path $TARGET_DIR "release\xy.exe"

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

Write-Host "[3/6] Compiling XY modules to IR..."
$xyFiles = @(
    "src\compiler_v2\compiler.xy"
)

$successCount = 0
$failCount = 0
$objFiles = @()
$utf8NoBom = New-Object System.Text.UTF8Encoding $false

foreach ($xyFile in $xyFiles) {
    $fullPath = Join-Path $PROJECT_ROOT $xyFile
    $fileName = Split-Path $xyFile -Leaf
    $baseName = [System.IO.Path]::GetFileNameWithoutExtension($fileName)
    $irFile = Join-Path $XYC_DIR "$baseName.ll"
    $objFile = Join-Path $XYC_DIR "$baseName.o"

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
        
        # Compile IR to object file
        Write-Host "    Assembling to object file..."
        llc $irFile -filetype=obj -o $objFile
        if ($LASTEXITCODE -eq 0) {
            Write-Host "    OK: Object file saved to $objFile"
            $objFiles += $objFile
            $successCount++
        } else {
            Write-Host "    ERROR: Failed to assemble IR"
            $failCount++
        }
    } else {
        Write-Host "    FAILED (exit code: $exitCode)"
        Write-Host "    Output: $irContent"
        $failCount++
    }
}

Write-Host ""
Write-Host "[3/4] Compilation Summary"
Write-Host "----------------------------------------"
Write-Host "  Success: $successCount"
Write-Host "  Failed:  $failCount"
Write-Host ""

if ($failCount -gt 0) {
    Write-Host "========================================"
    Write-Host "  Build Failed - Some modules failed"
    Write-Host "========================================"
    exit 1
}

Write-Host "[4/4] Linking xyc.exe..."
$runtimePath = Join-Path $PROJECT_ROOT "runtime\runtime.c"
if (-not (Test-Path $runtimePath)) {
    Write-Host "ERROR: runtime.c not found: $runtimePath"
    exit 1
}

$xycExe = Join-Path $XYC_DIR "xyc.exe"
Write-Host "  Linking with runtime: $runtimePath"

clang $runtimePath $objFiles -o $xycExe "-Wl,/subsystem:console"
if ($LASTEXITCODE -eq 0) {
    Write-Host "  OK: xyc.exe created at $xycExe"
} else {
    Write-Host "  ERROR: Linking failed"
    exit 1
}

Write-Host ""
Write-Host "[4/4] Final Summary"
Write-Host "----------------------------------------"
Write-Host "  L2 Build Complete!"
Write-Host "  xyc.exe: $xycExe"
Write-Host ""
Write-Host "========================================"
Write-Host "  L2 Build Complete - xyc.exe ready!"
Write-Host "========================================"
Write-Host ""
Write-Host "Next: Test L3 bootstrap"
Write-Host ""
exit 0
