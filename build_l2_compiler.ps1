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
$CL_EXE = "$LLVM_PATH\clang.exe"
$LINK_EXE = "$LLVM_PATH\lld-link.exe"

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

Write-Host "[2] Compiling IR to object files..."
Write-Host ""

foreach ($module in $MODULES) {
    Write-Host "Assembling $module.ll..."
    & $LLC_EXE "target\l2_compiler\$module.ll" -filetype=obj -o "target\l2_compiler\$module.obj"
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Failed to assemble $module.ll!"
        exit 1
    }
    Write-Host "Assembled $module.ll successfully"
    Write-Host ""
}

Write-Host "[3] Compiling C runtime library..."
Write-Host ""

# 使用 clang 替代 cl 编译 runtime.c
# 添加 -w 选项抑制所有警告以减少干扰
& $CL_EXE -c -O2 -w "runtime\runtime.c" -o "target\l2_compiler\runtime.obj"
if ($LASTEXITCODE -ne 0) {
    Write-Host "Failed to compile runtime.c!"
    exit 1
}
Write-Host "Compiled runtime.c successfully"
Write-Host ""

Write-Host "[4] Linking to generate L2 compiler..."
Write-Host ""

$OBJS = @(
    "target\l2_compiler\runtime.xy.obj",
    "target\l2_compiler\lexer.xy.obj",
    "target\l2_compiler\parser.xy.obj",
    "target\l2_compiler\sema.xy.obj",
    "target\l2_compiler\codegen.xy.obj",
    "target\l2_compiler\utils.xy.obj",
    "target\l2_compiler\main.xy.obj",
    "target\l2_compiler\runtime.obj"
)

# 使用 lld-link 进行链接
# /FORCE 允许重复符号，强制生成可执行文件
# 添加 /MANIFEST:NO 避免嵌入清单导致的问题
& $LINK_EXE -OUT:"target\l2_compiler\xyc.exe" $OBJS "/FORCE" "/MANIFEST:NO"
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