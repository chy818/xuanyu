# link_simple.ps1
# 简单的链接脚本，用于链接单个玄语程序

param(
    [Parameter(Mandatory=$true)]
    [string]$IRFile,
    
    [Parameter(Mandatory=$true)]
    [string]$OutputExe
)

$ErrorActionPreference = "Continue"

$PROJECT_ROOT = $PSScriptRoot
$RUNTIME_PATH = Join-Path $PROJECT_ROOT "runtime\runtime.c"
$BUILD_DIR = Join-Path $PROJECT_ROOT "target\simple_build"

# 创建构建目录
New-Item -ItemType Directory -Force -Path $BUILD_DIR | Out-Null

Write-Host "========================================"
Write-Host "  链接玄语程序"
Write-Host "========================================"
Write-Host "IR文件: $IRFile"
Write-Host "输出文件: $OutputExe"
Write-Host ""

# 编译 IR 为对象文件
Write-Host "[1/3] 编译 IR 为对象文件..."
$objFile = Join-Path $BUILD_DIR "program.o"
llc $IRFile -filetype=obj -o $objFile

if ($LASTEXITCODE -eq 0) {
    Write-Host "  OK"
} else {
    Write-Host "  FAIL"
    exit 1
}

Write-Host ""
Write-Host "[2/3] 编译 runtime.c..."
$runtimeObj = Join-Path $BUILD_DIR "runtime_c.o"
clang -c $RUNTIME_PATH -o $runtimeObj -D_CRT_SECURE_NO_WARNINGS

if ($LASTEXITCODE -eq 0) {
    Write-Host "  OK"
} else {
    Write-Host "  FAIL"
    exit 1
}

Write-Host ""
Write-Host "[3/3] 链接..."
clang $objFile $runtimeObj -o $OutputExe "-Wl,/subsystem:console"

if ($LASTEXITCODE -eq 0) {
    Write-Host "========================================"
    Write-Host "  链接成功!"
    Write-Host "========================================"
    Write-Host "可执行文件: $OutputExe"
} else {
    Write-Host "========================================"
    Write-Host "  链接失败"
    Write-Host "========================================"
    exit 1
}
