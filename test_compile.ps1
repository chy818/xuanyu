Write-Host "编译 lexer.xy..."
$L1_COMPILER = "cargo run --release --"
& $L1_COMPILER "src\compiler_v2\lexer.xy" --ir-pure 2>&1 | Out-File "lexer.xy.ll" -Encoding UTF8
if ($LASTEXITCODE -ne 0) {
    Write-Host "编译失败！"
    exit 1
}
Write-Host "编译成功"

Write-Host "编译 IR 文件..."
$LLC_EXE = "llc"
& $LLC_EXE "lexer.xy.ll" -filetype=obj -o "lexer.xy.obj"
if ($LASTEXITCODE -ne 0) {
    Write-Host "编译 IR 失败！"
    exit 1
}
Write-Host "编译 IR 成功"
