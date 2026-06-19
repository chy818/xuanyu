# 玄语自举构建脚本
# 使用V1 Rust编译器编译V2 XY编译器源码

param(
    [switch]$Run = $false,
    [switch]$Clean = $false
)

$ErrorActionPreference = "Stop"
$ProjectRoot = $PSScriptRoot
$OutputDir = Join-Path $ProjectRoot "target\selfhost"

# 清理
if ($Clean) {
    if (Test-Path $OutputDir) {
        Remove-Item -Recurse -Force $OutputDir
    }
}

# 创建输出目录
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  玄语编译器自举构建" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 第1步: 编译 V2 源码为 LLVM IR
Write-Host "[1/4] 编译 V2 源码为 LLVM IR..." -ForegroundColor Yellow
$XYC_Source = Join-Path $ProjectRoot "src\compiler_v2\xyc.xy"
$IR_Output = Join-Path $OutputDir "xyc_v2.ll"

if (-not (Test-Path $XYC_Source)) {
    Write-Host "错误: 找不到 xyc.xy 源文件: $XYC_Source" -ForegroundColor Red
    exit 1
}

$CargoArgs = @("run", "--release", "--", $XYC_Source, "--ir-pure")
$IR_Content = & cargo $CargoArgs 2>&1

if ($LASTEXITCODE -ne 0) {
    Write-Host "编译失败! 错误信息:" -ForegroundColor Red
    Write-Host $IR_Content
    exit 1
}

$IR_Content | Out-File -FilePath $IR_Output -Encoding utf8
Write-Host "  -> 生成 IR: $IR_Output" -ForegroundColor Green
Write-Host ""

# 第2步: 编译 IR 为目标文件
Write-Host "[2/4] 编译 IR 为目标文件..." -ForegroundColor Yellow
$ObjOutput = Join-Path $OutputDir "xyc_v2.obj"

$LLC_Result = & llc $IR_Output -filetype=obj -o $ObjOutput 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "llc 编译失败! 错误信息:" -ForegroundColor Red
    Write-Host $LLC_Result
    exit 1
}
Write-Host "  -> 生成目标文件: $ObjOutput" -ForegroundColor Green
Write-Host ""

# 第3步: 编译 C 运行时库
Write-Host "[3/4] 编译 C 运行时库..." -ForegroundColor Yellow
$RuntimeSource = Join-Path $ProjectRoot "runtime\runtime.c"
$RuntimeObj = Join-Path $OutputDir "runtime.obj"

$Clang_Result = & clang -c -O2 $RuntimeSource -o $RuntimeObj 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "clang 编译运行时失败! 错误信息:" -ForegroundColor Red
    Write-Host $Clang_Result
    exit 1
}
Write-Host "  -> 生成运行时目标文件: $RuntimeObj" -ForegroundColor Green
Write-Host ""

# 第4步: 链接为可执行文件
Write-Host "[4/4] 链接为可执行文件..." -ForegroundColor Yellow
$ExeOutput = Join-Path $OutputDir "xyc.exe"
if ($IsWindows -or $env:OS -eq "Windows_NT") {
    $Link_Result = & clang $RuntimeObj $ObjOutput -o $ExeOutput -Wl,/SUBSYSTEM:console 2>&1
} else {
    $Link_Result = & clang $RuntimeObj $ObjOutput -o $ExeOutput -lm 2>&1
}

if ($LASTEXITCODE -ne 0) {
    Write-Host "链接失败! 错误信息:" -ForegroundColor Red
    Write-Host $Link_Result
    exit 1
}
Write-Host "  -> 生成可执行文件: $ExeOutput" -ForegroundColor Green
Write-Host ""

# 完成
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  构建完成!" -ForegroundColor Cyan
Write-Host "  输出: $ExeOutput" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# 运行测试
if ($Run) {
    Write-Host ""
    Write-Host "测试 L2 编译器..." -ForegroundColor Yellow
    & $ExeOutput --version 2>&1

    Write-Host ""
    Write-Host "编译测试程序..." -ForegroundColor Yellow
    $HelloXY = Join-Path $ProjectRoot "examples\hello.xy"
    if (Test-Path $HelloXY) {
        & $ExeOutput $HelloXY --ir 2>&1
    }
}
