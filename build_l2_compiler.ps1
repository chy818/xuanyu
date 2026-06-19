Write-Host "====================================="
Write-Host "    L2 Compiler Build Script"
Write-Host "====================================="
Write-Host ""

if (-not (Test-Path "target\l2_compiler")) {
    New-Item -ItemType Directory -Path "target\l2_compiler" -Force | Out-Null
}

# LLVM 工具链路径
$LLVM_PATH = "C:\Program Files\LLVM\bin"
$LLC_EXE = "$LLVM_PATH\llc.exe"
$CLANG_EXE = "$LLVM_PATH\clang.exe"

# MinGW 路径
$MINGW_BIN = "C:\msys64\mingw64\bin"
$GCC_EXE = "$MINGW_BIN\gcc.exe"

$MODULES = @("runtime.xy", "lexer.xy", "parser.xy", "sema.xy", "codegen.xy", "utils.xy", "main.xy")

Write-Host "[1] Compiling modules to LLVM IR..."
Write-Host ""

foreach ($module in $MODULES) {
    Write-Host "Compiling $module..."
    $process = Start-Process -FilePath "cargo" -ArgumentList "run", "--release", "--", "src\compiler_v2\$module", "--ir-pure" -NoNewWindow -Wait -PassThru -RedirectStandardOutput "target\l2_compiler\$module.ll"
    if ($process.ExitCode -ne 0) {
        Write-Host "Failed to compile $module!"
        exit 1
    }
    Write-Host "Compiled $module successfully"
    Write-Host ""
}

Write-Host "[2] Compiling IR to native object files..."
Write-Host ""

foreach ($module in $MODULES) {
    Write-Host "Compiling $module.ll to object file..."
    # 使用 clang 作为集成汇编器，它可以处理 SEH 指令
    & $CLANG_EXE -c -O2 "target\l2_compiler\$module.ll" -o "target\l2_compiler\$module.obj"
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Failed to compile $module.ll!"
        exit 1
    }
    Write-Host "Compiled $module.ll successfully"
    Write-Host ""
}

Write-Host "[3] Compiling C runtime library..."
Write-Host ""

# 设置 PATH 以便找到 MinGW 的 gcc
$env:PATH = "$MINGW_BIN;$env:PATH"
& $GCC_EXE -c -O2 -w "runtime\runtime.c" -o "target\l2_compiler\runtime.obj"
if ($LASTEXITCODE -ne 0) {
    Write-Host "Failed to compile runtime.c!"
    exit 1
}
Write-Host "Compiled runtime.c successfully"
Write-Host ""

Write-Host "[4] Linking to generate L2 compiler..."
Write-Host ""

# 使用 MinGW 的 gcc 进行链接
# -Wl,-e,main 指定入口点为 main 函数
$env:PATH = "$MINGW_BIN;$env:PATH"
& $GCC_EXE -O2 "target\l2_compiler\runtime.xy.obj" "target\l2_compiler\lexer.xy.obj" "target\l2_compiler\parser.xy.obj" "target\l2_compiler\sema.xy.obj" "target\l2_compiler\codegen.xy.obj" "target\l2_compiler\utils.xy.obj" "target\l2_compiler\main.xy.obj" "target\l2_compiler\runtime.obj" -o "target\l2_compiler\xyc.exe" "-Wl,--allow-multiple-definition" "-Wl,-e,main"
if ($LASTEXITCODE -ne 0) {
    Write-Host "Linking failed!"
    exit 1
}
Write-Host "Linking successful!"
Write-Host ""

Write-Host "====================================="
Write-Host "L2 Compiler build completed!"
Write-Host "Output: target\l2_compiler\xyc.exe"
Write-Host "====================================="