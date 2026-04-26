Write-Host "构建 L2 编译器..."

# 创建输出目录
if (-not (Test-Path "target\l2_compiler")) {
    New-Item -ItemType Directory -Path "target\l2_compiler" -Force | Out-Null
}

Write-Host "编译模块到 LLVM IR..."
Write-Host "编译 runtime.xy..."
$env:RUST_BACKTRACE = "0"
$process = Start-Process -FilePath "cargo" -ArgumentList "run", "--release", "--", "src\compiler_v2\runtime.xy", "--ir-pure" -NoNewWindow -Wait -PassThru -RedirectStandardOutput "target\l2_compiler\runtime.xy.ll"
Write-Host "编译 lexer.xy..."
$process = Start-Process -FilePath "cargo" -ArgumentList "run", "--release", "--", "src\compiler_v2\lexer.xy", "--ir-pure" -NoNewWindow -Wait -PassThru -RedirectStandardOutput "target\l2_compiler\lexer.xy.ll"
Write-Host "编译 parser.xy..."
$process = Start-Process -FilePath "cargo" -ArgumentList "run", "--release", "--", "src\compiler_v2\parser.xy", "--ir-pure" -NoNewWindow -Wait -PassThru -RedirectStandardOutput "target\l2_compiler\parser.xy.ll"
Write-Host "编译 sema.xy..."
$process = Start-Process -FilePath "cargo" -ArgumentList "run", "--release", "--", "src\compiler_v2\sema.xy", "--ir-pure" -NoNewWindow -Wait -PassThru -RedirectStandardOutput "target\l2_compiler\sema.xy.ll"
Write-Host "编译 codegen.xy..."
$process = Start-Process -FilePath "cargo" -ArgumentList "run", "--release", "--", "src\compiler_v2\codegen.xy", "--ir-pure" -NoNewWindow -Wait -PassThru -RedirectStandardOutput "target\l2_compiler\codegen.xy.ll"
Write-Host "编译 utils.xy..."
$process = Start-Process -FilePath "cargo" -ArgumentList "run", "--release", "--", "src\compiler_v2\utils.xy", "--ir-pure" -NoNewWindow -Wait -PassThru -RedirectStandardOutput "target\l2_compiler\utils.xy.ll"
Write-Host "编译 main.xy..."
$process = Start-Process -FilePath "cargo" -ArgumentList "run", "--release", "--", "src\compiler_v2\main.xy", "--ir-pure" -NoNewWindow -Wait -PassThru -RedirectStandardOutput "target\l2_compiler\main.xy.ll"

# 编译 IR 为目标文件
Write-Host "编译 IR 为目标文件..."
llc target\l2_compiler\runtime.xy.ll -filetype=obj -o target\l2_compiler\runtime.xy.obj
llc target\l2_compiler\lexer.xy.ll -filetype=obj -o target\l2_compiler\lexer.xy.obj
llc target\l2_compiler\parser.xy.ll -filetype=obj -o target\l2_compiler\parser.xy.obj
llc target\l2_compiler\sema.xy.ll -filetype=obj -o target\l2_compiler\sema.xy.obj
llc target\l2_compiler\codegen.xy.ll -filetype=obj -o target\l2_compiler\codegen.xy.obj
llc target\l2_compiler\utils.xy.ll -filetype=obj -o target\l2_compiler\utils.xy.obj
llc target\l2_compiler\main.xy.ll -filetype=obj -o target\l2_compiler\main.xy.obj

# 编译运行时库
Write-Host "编译 C 运行时库..."
cl /c /O2 runtime\runtime.c /Fo"target\l2_compiler\runtime.obj"

# 链接所有目标文件
Write-Host "链接生成 L2 编译器..."
link /OUT:"target\l2_compiler\xyc.exe" target\l2_compiler\runtime.xy.obj target\l2_compiler\lexer.xy.obj target\l2_compiler\parser.xy.obj target\l2_compiler\sema.xy.obj target\l2_compiler\codegen.xy.obj target\l2_compiler\utils.xy.obj target\l2_compiler\main.xy.obj target\l2_compiler\runtime.obj /NOLOGO

Write-Host "L2 编译器构建完成！"
Write-Host "输出文件: target\l2_compiler\xyc.exe"