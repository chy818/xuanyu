# simple_link_test.ps1
# 简单的链接测试脚本

$ErrorActionPreference = "Continue"

$PROJECT_ROOT = $PSScriptRoot
$L1_IR_DIR = Join-Path $PROJECT_ROOT "target\l1_ir"
$BUILD_DIR = Join-Path $PROJECT_ROOT "target\link_test"
$RUNTIME_PATH = Join-Path $PROJECT_ROOT "runtime\runtime.c"

# 创建构建目录
New-Item -ItemType Directory -Force -Path $BUILD_DIR | Out-Null

Write-Host "========================================"
Write-Host "  Simple Link Test"
Write-Host "========================================"
Write-Host ""

$modules = @("runtime", "lexer", "parser", "sema", "codegen", "main")
$objFiles = @()

# 编译每个 IR 为对象文件
Write-Host "[1/3] Compiling IR to object files..."
foreach ($module in $modules) {
    $irFile = Join-Path $L1_IR_DIR "$module.ll"
    $objFile = Join-Path $BUILD_DIR "$module.o"
    
    if (Test-Path $irFile) {
        Write-Host "  Compiling: $module.ll"
        llc $irFile -filetype=obj -o $objFile
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host "    OK"
            $objFiles += $objFile
        } else {
            Write-Host "    FAIL"
        }
    }
}

Write-Host ""
Write-Host "[2/3] Compiling runtime.c..."
$runtimeObj = Join-Path $BUILD_DIR "runtime_c.o"
clang -c $RUNTIME_PATH -o $runtimeObj -D_CRT_SECURE_NO_WARNINGS

if ($LASTEXITCODE -eq 0) {
    Write-Host "  OK"
    $objFiles += $runtimeObj
} else {
    Write-Host "  FAIL"
}

Write-Host ""
Write-Host "[3/3] Linking..."
$exePath = Join-Path $BUILD_DIR "xyc_test.exe"
clang $objFiles -o $exePath "-Wl,/subsystem:console"

if ($LASTEXITCODE -eq 0) {
    Write-Host "========================================"
    Write-Host "  SUCCESS!"
    Write-Host "========================================"
    Write-Host "Executable: $exePath"
} else {
    Write-Host "========================================"
    Write-Host "  FAILURE"
    Write-Host "========================================"
}
