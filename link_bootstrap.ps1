$objs = @(
    "target/bootstrap_final/l2/main.xy.o",
    "target/bootstrap_final/l2/lexer.xy.o",
    "target/bootstrap_final/l2/parser.xy.o",
    "target/bootstrap_final/l2/sema.xy.o",
    "target/bootstrap_final/l2/codegen.xy.o",
    "target/bootstrap_final/l2/runtime.xy.o",
    "target/bootstrap_final/l2/runtime.c.o"
)

$out = "target/bootstrap_final/l2/xyc.exe"

Write-Host "开始链接自展编译器..."
Write-Host "对象文件数量: $($objs.Count)"

# 使用 clang 链接
Write-Host "使用 clang 链接..."

# 先尝试直接链接（让 clang 自动找到链接器）
$env:CC = "clang"
$env:CFLAGS = "-target x86_64-pc-windows-msvc"
$result = clang $objs -o $out 2>&1

if ($LASTEXITCODE -eq 0) {
    Write-Host "[成功] xyc.exe 已生成: $out"
    if (Test-Path $out) {
        $size = (Get-Item $out).Length
        Write-Host "文件大小: $($size / 1KB) KB"
    }
} else {
    Write-Host "[失败] 链接失败，exit code: $LASTEXITCODE"
}