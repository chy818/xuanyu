# bootstrap_improved.ps1
# ========================================
# Improved Xuanyu Compiler Bootstrap Script
# ========================================

$ErrorActionPreference = "Continue"

Write-Host "========================================"
Write-Host "  Xuanyu Compiler Bootstrap Test"
Write-Host "========================================"
Write-Host ""

# ==================== Configuration ====================

$PROJECT_ROOT = Split-Path -Parent $PSScriptRoot
$XY_COMPILER = Join-Path $PROJECT_ROOT "target\debug\xy.exe"
$SRC_DIR = Join-Path $PROJECT_ROOT "src\compiler_v2"
$OUTPUT_DIR = Join-Path $PROJECT_ROOT "target\bootstrap_improved"
$L1_IR_DIR = Join-Path $OUTPUT_DIR "l1_ir"
$L2_DIR = Join-Path $OUTPUT_DIR "l2"
$L3_IR_DIR = Join-Path $OUTPUT_DIR "l3_ir"

$XY_MODULES = @(
    "runtime.xy",
    "lexer.xy",
    "parser.xy",
    "sema.xy",
    "codegen.xy",
    "main.xy"
)

# Create output directories
New-Item -ItemType Directory -Force -Path $OUTPUT_DIR | Out-Null
New-Item -ItemType Directory -Force -Path $L1_IR_DIR | Out-Null
New-Item -ItemType Directory -Force -Path $L2_DIR | Out-Null
New-Item -ItemType Directory -Force -Path $L3_IR_DIR | Out-Null

$L1_SUCCESS = $true
$L2_SUCCESS = $true
$L3_SUCCESS = $true
$VERIFY_SUCCESS = $true

# ==================== Helper Functions ====================

function Write-PhaseHeader {
    param([string]$Title)
    Write-Host ""
    Write-Host "========================================"
    Write-Host "  $Title"
    Write-Host "========================================"
    Write-Host ""
}

# 不需要 Extract-Pure-IR 了，因为我们用 --ir-pure 直接获取纯 IR

function Compile-Single-Module {
    param(
        [string]$ModuleName,
        [string]$ModulePath,
        [string]$OutputDir,
        [string]$CompilerPath
    )
    
    Write-Host "Compiling: $ModuleName"
    
    if (-not (Test-Path $ModulePath)) {
        Write-Host "  [SKIP] File not found: $ModulePath"
        return $false
    }

    # Clear cache
    $cacheFile = "$ModulePath.cache"
    if (Test-Path $cacheFile) {
        Remove-Item $cacheFile -Force
    }

    # 使用 --ir-pure 直接获取纯 IR，并用正确的编码保存
    $irContent = & $CompilerPath $ModulePath "--ir-pure" 2>&1
    $exitCode = $LASTEXITCODE
    
    if ($exitCode -eq 0) {
        $irFile = Join-Path $OutputDir "$ModuleName.ll"
        # 使用 UTF-8 无 BOM 编码保存
        $utf8NoBom = New-Object System.Text.UTF8Encoding $false
        [System.IO.File]::WriteAllLines($irFile, $irContent, $utf8NoBom)
        
        Write-Host "  [OK] Compiled successfully"
        Write-Host "  IR saved to: $irFile"
        return $true
    } else {
        Write-Host "  [FAIL] Compilation failed (exit code: $exitCode)"
        Write-Host "  Output:"
        Write-Host $irContent
        return $false
    }
}

function Build-Object-Files {
    param(
        [string]$IrDir,
        [string]$ObjDir
    )
    
    $objFiles = @()
    
    foreach ($module in $XY_MODULES) {
        $irPath = Join-Path $IrDir "$module.ll"
        $objPath = Join-Path $ObjDir "$module.o"
        
        if (-not (Test-Path $irPath)) {
            Write-Host "  [SKIP] $module (no IR)"
            continue
        }
        
        Write-Host "  Generating object file: $module"
        llc $irPath -filetype=obj -o $objPath
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host "    [OK]"
            $objFiles += $objPath
        } else {
            Write-Host "    [FAIL]"
        }
    }
    
    return $objFiles
}

function Compare-IR {
    param(
        [string]$ModuleName,
        [string]$L1IrPath,
        [string]$L3IrPath
    )
    
    Write-Host "Comparing: $ModuleName"
    
    if (-not (Test-Path $L1IrPath)) {
        Write-Host "  [FAIL] L1 IR not found"
        return $false
    }
    if (-not (Test-Path $L3IrPath)) {
        Write-Host "  [FAIL] L3 IR not found"
        return $false
    }
    
    $content1 = Get-Content $L1IrPath -Raw
    $content2 = Get-Content $L3IrPath -Raw
    
    if ($content1 -eq $content2) {
        Write-Host "  [OK] IR matches"
        return $true
    } else {
        Write-Host "  [WARN] IR differs"
        return $false
    }
}

# ==================== Phase L1 ====================

Write-PhaseHeader "L1: Compile all XY modules with Rust compiler"

# Check compiler
if (-not (Test-Path $XY_COMPILER)) {
    Write-Host "[ERROR] Compiler not found: $XY_COMPILER"
    Write-Host "Please run: cargo build"
    exit 1
}
Write-Host "Compiler: $XY_COMPILER"
Write-Host ""

foreach ($module in $XY_MODULES) {
    $fullPath = Join-Path $SRC_DIR $module
    $success = Compile-Single-Module $module $fullPath $L1_IR_DIR $XY_COMPILER
    if (-not $success) {
        $L1_SUCCESS = $false
    }
}

Write-Host ""
Write-Host "L1 Phase Summary: $(if ($L1_SUCCESS) { "SUCCESS" } else { "FAIL" })"
if (-not $L1_SUCCESS) {
    Write-Host "Note: Some modules failed to compile, L2 will use successfully compiled modules"
}

# ==================== Phase L2 ====================

Write-PhaseHeader "L2: Link into bootstrap compiler (xyc.exe)"

$objFiles = Build-Object-Files $L1_IR_DIR $L2_DIR

$runtimePath = Join-Path $PROJECT_ROOT "runtime\runtime.c"
if (-not (Test-Path $runtimePath)) {
    Write-Host "[ERROR] runtime.c not found"
    $L2_SUCCESS = $false
} else {
    $xycExe = Join-Path $L2_DIR "xyc.exe"
    Write-Host ""
    Write-Host "Linking executable: $xycExe"
    
    if ($objFiles.Count -gt 0) {
        # Add Windows subsystem parameter
        clang $runtimePath $objFiles -o $xycExe "-Wl,/subsystem:console"
        if ($LASTEXITCODE -eq 0) {
            Write-Host "[OK] L2 Success: $xycExe"
            $L2_SUCCESS = $true
        } else {
            Write-Host "[FAIL] Linking failed"
            $L2_SUCCESS = $false
        }
    } else {
        Write-Host "[SKIP] No successfully compiled modules to link"
        $L2_SUCCESS = $false
    }
}

Write-Host ""
Write-Host "L2 Phase Summary: $(if ($L2_SUCCESS) { "SUCCESS" } else { "FAIL" })"

# ==================== Phase L3 ====================

Write-PhaseHeader "L3: Recompile with bootstrap compiler"

if ($L2_SUCCESS) {
    $xycExe = Join-Path $L2_DIR "xyc.exe"
    Write-Host "Using compiler: $xycExe"
    Write-Host ""
    
    foreach ($module in $XY_MODULES) {
        $fullPath = Join-Path $SRC_DIR $module
        $success = Compile-Single-Module $module $fullPath $L3_IR_DIR $xycExe
        if (-not $success) {
            $L3_SUCCESS = $false
        }
    }
} else {
    Write-Host "[SKIP] L2 not successful, cannot proceed to L3"
    $L3_SUCCESS = $false
}

Write-Host ""
Write-Host "L3 Phase Summary: $(if ($L3_SUCCESS) { "SUCCESS" } else { "FAIL" })"

# ==================== Verification ====================

Write-PhaseHeader "Verify L1 and L3 consistency"

if ($L1_SUCCESS -and $L3_SUCCESS) {
    foreach ($module in $XY_MODULES) {
        $l1Path = Join-Path $L1_IR_DIR "$module.ll"
        $l3Path = Join-Path $L3_IR_DIR "$module.ll"
        $success = Compare-IR $module $l1Path $l3Path
        if (-not $success) {
            $VERIFY_SUCCESS = $false
        }
    }
} else {
    Write-Host "[SKIP] L1 or L3 not successful"
    $VERIFY_SUCCESS = $false
}

Write-Host ""
Write-Host "Verification Summary: $(if ($VERIFY_SUCCESS) { "SUCCESS" } else { "FAIL" })"

# ==================== Final Summary ====================

Write-PhaseHeader "Bootstrap Final Results"

Write-Host "L1 (Rust Compile):      $(if ($L1_SUCCESS) { "SUCCESS" } else { "FAIL" })"
Write-Host "L2 (Link Executable):   $(if ($L2_SUCCESS) { "SUCCESS" } else { "FAIL" })"
Write-Host "L3 (Bootstrap Recompile):$(if ($L3_SUCCESS) { "SUCCESS" } else { "FAIL" })"
Write-Host "Verify (L1 vs L3):      $(if ($VERIFY_SUCCESS) { "SUCCESS" } else { "FAIL" })"
Write-Host ""

$allOk = $L1_SUCCESS -and $L2_SUCCESS -and $L3_SUCCESS -and $VERIFY_SUCCESS

if ($allOk) {
    Write-Host "========================================"
    Write-Host "  FULL BOOTSTRAP SUCCESS!"
    Write-Host "========================================"
    exit 0
} else {
    Write-Host "========================================"
    Write-Host "  BOOTSTRAP INCOMPLETE"
    Write-Host "========================================"
    Write-Host ""
    Write-Host "Output directory: $OUTPUT_DIR"
    Write-Host "  - L1 IR: $L1_IR_DIR"
    Write-Host "  - L2 Compiler: $L2_DIR"
    Write-Host "  - L3 IR: $L3_IR_DIR"
    exit 1
}
