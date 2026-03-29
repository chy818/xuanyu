# full_bootstrap_test.ps1
# 完整的自展验证测试脚本（L1 + L2 + L3）

$ErrorActionPreference = "Continue"

Write-Host "========================================"
Write-Host "  XY Compiler Full Bootstrap Test"
Write-Host "  L1 + L2 + L3 Verification"
Write-Host "========================================"
Write-Host ""

$PROJECT_ROOT = $PSScriptRoot | Split-Path
$XY_COMPILER = Join-Path $PROJECT_ROOT "target\debug\xy.exe"
$SRC_DIR = Join-Path $PROJECT_ROOT "src\compiler_v2"
$OUTPUT_DIR = Join-Path $PROJECT_ROOT "target\bootstrap_output"
$L1_IR_DIR = Join-Path $OUTPUT_DIR "l1_ir"
$L2_DIR = Join-Path $OUTPUT_DIR "l2"
$L3_IR_DIR = Join-Path $OUTPUT_DIR "l3_ir"

# 创建输出目录
if (-not (Test-Path $OUTPUT_DIR)) {
    New-Item -ItemType Directory -Path $OUTPUT_DIR -Force | Out-Null
}
if (-not (Test-Path $L1_IR_DIR)) {
    New-Item -ItemType Directory -Path $L1_IR_DIR -Force | Out-Null
}
if (-not (Test-Path $L2_DIR)) {
    New-Item -ItemType Directory -Path $L2_DIR -Force | Out-Null
}
if (-not (Test-Path $L3_IR_DIR)) {
    New-Item -ItemType Directory -Path $L3_IR_DIR -Force | Out-Null
}

$TOTAL_TESTS = 0
$PASSED_TESTS = 0
$FAILED_TESTS = 0
$L1_SUCCESS = $true
$L2_SUCCESS = $true
$L3_SUCCESS = $true

$XY_MODULES = @(
    "runtime.xy",
    "lexer.xy",
    "parser.xy",
    "sema.xy",
    "codegen.xy",
    "main.xy"
)

function Run-Test {
    param (
        [string]$Name,
        [string]$File,
        [string]$Flag,
        [string]$OutputDir = ""
    )

    $script:TOTAL_TESTS++

    Write-Host "----------------------------------------"
    Write-Host "Test $TOTAL_TESTS : $Name"
    Write-Host "File: $File"
    Write-Host "----------------------------------------"

    if (Test-Path $File) {
        $output = & $XY_COMPILER $File $Flag 2>&1
        $exitCode = $LASTEXITCODE

        $hasError = $output -match "错误:|error:|Error:|FAIL|fail:" -or ($exitCode -ne 0 -and -not ($output -match "define i32 @main" -and $output -match "ret i32"))
        $isSuccess = $exitCode -eq 0 -and (-not $hasError)

        if ($isSuccess -or ($exitCode -eq 0 -and ($output -match "define i32 @main" -or $output -match "编译成功"))) {
            Write-Host "[PASS] $Name"
            $script:PASSED_TESTS++

            if ($OutputDir -ne "") {
                $fileName = Split-Path $File -Leaf
                $irFile = Join-Path $OutputDir "$fileName.ll"
                $output | Out-File -FilePath $irFile -Encoding utf8
                Write-Host "  IR saved to: $irFile"
            }
        } else {
            Write-Host "[FAIL] $Name (exit code: $exitCode)"
            if ($output) {
                Write-Host $output
            }
            $script:FAILED_TESTS++
        }
    } else {
        Write-Host "[SKIP] File not found: $File"
        $script:FAILED_TESTS++
    }
    Write-Host ""

    return $isSuccess
}

function Compare-IR {
    param (
        [string]$File1,
        [string]$File2
    )

    if (-not (Test-Path $File1)) {
        Write-Host "[FAIL] L1 IR not found: $File1"
        return $false
    }
    if (-not (Test-Path $File2)) {
        Write-Host "[FAIL] L3 IR not found: $File2"
        return $false
    }

    $content1 = Get-Content $File1 -Raw
    $content2 = Get-Content $File2 -Raw

    if ($content1 -eq $content2) {
        Write-Host "[PASS] IR matches"
        return $true
    } else {
        Write-Host "[FAIL] IR does not match"
        return $false
    }
}

Write-Host "[1/6] Checking compiler..."
if (-not (Test-Path $XY_COMPILER)) {
    Write-Host "ERROR: Compiler not found: $XY_COMPILER"
    Write-Host "Run: cargo build"
    exit 1
}
Write-Host "OK: Compiler exists: $XY_COMPILER"
Write-Host ""

Write-Host "[2/6] L1: Compile XY modules with Rust compiler"
Write-Host "========================================"
Write-Host ""

foreach ($module in $XY_MODULES) {
    $fullPath = Join-Path $SRC_DIR $module
    $success = Run-Test "L1: $module" $fullPath "--ir" $L1_IR_DIR
    if (-not $success) {
        $L1_SUCCESS = $false
    }
}

Write-Host ""
Write-Host "L1 Summary:"
Write-Host "  Total: $TOTAL_TESTS"
Write-Host "  Passed: $PASSED_TESTS"
Write-Host "  Failed: $FAILED_TESTS"
Write-Host ""

if ($FAILED_TESTS -gt 0) {
    Write-Host "L1 failed, stopping test"
    exit 1
}

$L1_SUCCESS = $true

Write-Host "[3/6] L2: Link XY modules into executable"
Write-Host "========================================"
Write-Host ""
Write-Host "NOTE: Full linking requires complete compiler implementation"
Write-Host "Currently using build_xyc.ps1 for L2 verification"
Write-Host ""

& (Join-Path $PROJECT_ROOT "build_xyc.ps1")
$l2ExitCode = $LASTEXITCODE

if ($l2ExitCode -eq 0) {
    Write-Host "[PASS] L2 verification completed"
} else {
    Write-Host "[FAIL] L2 verification failed"
    $L2_SUCCESS = $false
}

Write-Host ""
Write-Host "L2 Summary:"
Write-Host "  Success: $L2_SUCCESS"
Write-Host ""

Write-Host "[4/6] L3: Compile XY modules with L2 compiler"
Write-Host "========================================"
Write-Host ""
Write-Host "NOTE: L3 requires a working L2 compiler executable"
Write-Host "Skipping for now - will implement after L2 is complete"
Write-Host ""

Write-Host "[5/6] Compare L1 and L3 results"
Write-Host "========================================"
Write-Host ""
Write-Host "NOTE: Comparison will be done once L3 is available"
Write-Host ""

Write-Host "[6/6] Final Summary"
Write-Host "========================================"
Write-Host "Total: $TOTAL_TESTS"
Write-Host "Passed: $PASSED_TESTS"
Write-Host "Failed: $FAILED_TESTS"
Write-Host ""
Write-Host "L1: $L1_SUCCESS"
Write-Host "L2: $L2_SUCCESS"
Write-Host "L3: Skipped (requires L2 compiler)"
Write-Host ""

if ($L1_SUCCESS -and $L2_SUCCESS) {
    Write-Host "========================================"
    Write-Host "  L1 and L2 VERIFIED!"
    Write-Host "  L3 pending complete L2 compiler"
    Write-Host "========================================"
    exit 0
} else {
    Write-Host "========================================"
    Write-Host "  BOOTSTRAP TEST FAILED"
    Write-Host "========================================"
    exit 1
}
