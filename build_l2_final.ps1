Write-Host "====================================="
Write-Host "    L2 Compiler Build Script"
Write-Host "====================================="
Write-Host ""

# 创建输出目录
if (-not (Test-Path "target\l2_compiler")) {
    New-Item -ItemType Directory -Path "target\l2_compiler" -Force | Out-Null
}

# 定义编译器路径
$L1_COMPILER = "cargo run --release --"
$LLC_EXE = "llc"
$CLANG_EXE = "clang"

# 定义模块列表
$MODULES = @("runtime.xy", "lexer.xy", "parser.xy", "sema.xy", "codegen.xy", "utils.xy", "main.xy")

# 编译每个模块到 IR
Write-Host "[1] 编译模块到 LLVM IR..."
Write-Host ""

foreach ($module in $MODULES) {
    Write-Host "编译 $module..."
    & $L1_COMPILER "src\compiler_v2\$module" --ir-pure 2>&1 | Out-File "target\l2_compiler\$module.ll" -Encoding UTF8
    if ($LASTEXITCODE -ne 0) {
        Write-Host "编译 $module 失败！"
        exit 1
    }
    Write-Host "编译 $module 成功"
    Write-Host ""
}

# 修复IR文件中的BOM字符和非IR内容
Write-Host "[2] 修复 IR 文件..."
Write-Host ""

foreach ($module in $MODULES) {
    $irFile = "target\l2_compiler\$module.ll"
    Write-Host "修复 $irFile..."
    
    # 读取文件内容，去除BOM字符
    $content = Get-Content -Path $irFile -Encoding UTF8 -Raw
    $content = $content -replace '^\ufeff', ''
    
    # 过滤出有效的IR行
    $filteredLines = @()
    $lines = $content -split '\r?\n'
    foreach ($line in $lines) {
        $line = $line.Trim()
        if ($line -match '^(define|declare|@|%|;|source_filename|target|!|{})') {
            $filteredLines += $line
        }
    }
    
    # 写回文件
    $filteredContent = $filteredLines -join "`n"
    Set-Content -Path $irFile -Value $filteredContent -Encoding ASCII
    Write-Host "修复 $irFile 成功"
    Write-Host ""
}

# 编译 IR 为目标文件
Write-Host "[3] 编译 IR 为目标文件..."
Write-Host ""

foreach ($module in $MODULES) {
    Write-Host "编译 $module.ll..."
    & $LLC_EXE "target\l2_compiler\$module.ll" -filetype=obj -o "target\l2_compiler\$module.obj"
    if ($LASTEXITCODE -ne 0) {
        Write-Host "编译 $module.ll 失败！"
        exit 1
    }
    Write-Host "编译 $module.ll 成功"
    Write-Host ""
}

# 编译 C 运行时库
Write-Host "[4] 编译 C 运行时库..."
Write-Host ""

& $CLANG_EXE -c -O2 "runtime\runtime.c" -o "target\l2_compiler\runtime.obj"
if ($LASTEXITCODE -ne 0) {
    Write-Host "编译 runtime.c 失败！"
    exit 1
}
Write-Host "编译 runtime.c 成功"
Write-Host ""

# 链接所有目标文件
Write-Host "[5] 链接生成 L2 编译器..."
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

$OBJS_STRING = $OBJS -join " "
& $CLANG_EXE $OBJS_STRING -o "target\l2_compiler\xyc.exe"
if ($LASTEXITCODE -ne 0) {
    Write-Host "链接失败！"
    exit 1
}
Write-Host "链接成功！"
Write-Host ""

Write-Host "====================================="
Write-Host "L2 编译器构建完成！"
Write-Host "输出文件: target\l2_compiler\xyc.exe"
Write-Host "====================================="
Write-Host ""
Write-Host "测试 L2 编译器..."
Write-Host ""

# 测试 L2 编译器
& "target\l2_compiler\xyc.exe" --version
if ($LASTEXITCODE -ne 0) {
    Write-Host "测试失败！"
    exit 1
}
Write-Host "测试成功！"
Write-Host ""
Write-Host "L2 编译器已准备就绪！"
Write-Host "可以使用: target\l2_compiler\xyc.exe"