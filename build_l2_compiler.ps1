Write-Host "====================================="
Write-Host "    L2 Compiler Build Script"
Write-Host "====================================="
Write-Host ""

if (-not (Test-Path "target\l2_compiler")) {
    New-Item -ItemType Directory -Path "target\l2_compiler" -Force | Out-Null
}

$LLC_EXE = "llc"
$CL_EXE = "cl"
$LINK_EXE = "link"

$MODULES = @("runtime.xy", "lexer.xy", "parser.xy", "sema.xy", "codegen.xy", "utils.xy", "main.xy")

Write-Host "[1] Compiling modules to LLVM IR..."
Write-Host ""

foreach ($module in $MODULES) {
    Write-Host "Compiling $module..."
    cargo run --release -- "src\compiler_v2\$module" --ir-pure 2>&1 | Out-File "target\l2_compiler\$module.ll" -Encoding UTF8
    if ($LASTEXITCODE -ne 0) {
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

& $CL_EXE /c /O2 "runtime\runtime.c" /Fo"target\l2_compiler\runtime.obj"
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

$OBJS_STRING = $OBJS -join " "
& $LINK_EXE /OUT:"target\l2_compiler\xyc.exe" $OBJS_STRING /NOLOGO
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
