@echo off
setlocal enabledelayedexpansion

echo ========================================
echo     L2 Compiler Build Script
echo ========================================
echo.

REM 创建输出目录
if not exist "target\l2_compiler" mkdir "target\l2_compiler"

REM 定义编译器路径
set L1_COMPILER="cargo run --release --"
set LLC_EXE=llc
set CL_EXE=cl
set LINK_EXE=link

REM 定义模块列表
set MODULES=runtime.xy lexer.xy parser.xy sema.xy codegen.xy utils.xy main.xy

REM 编译每个模块到 IR
echo [1] 编译模块到 LLVM IR...
echo.

for %%m in (%MODULES%) do (
    echo 编译 %%m...
    %L1_COMPILER% "src\compiler_v2\%%m" --ir-pure > "target\l2_compiler\%%m.ll" 2>&1
    if errorlevel 1 (
        echo 编译 %%m 失败！
        exit /b 1
    )
    echo 编译 %%m 成功
    echo.
)

REM 编译 IR 为目标文件
echo [2] 编译 IR 为目标文件...
echo.

for %%m in (%MODULES%) do (
    echo 编译 %%m.ll...
    %LLC_EXE% "target\l2_compiler\%%m.ll" -filetype=obj -o "target\l2_compiler\%%m.obj"
    if errorlevel 1 (
        echo 编译 %%m.ll 失败！
        exit /b 1
    )
    echo 编译 %%m.ll 成功
    echo.
)

REM 编译运行时库
echo [3] 编译 C 运行时库...
echo.

%CL_EXE% /c /O2 "runtime\runtime.c" /Fo"target\l2_compiler\runtime.obj"
if errorlevel 1 (
    echo 编译 runtime.c 失败！
    exit /b 1
)
echo 编译 runtime.c 成功
echo.

REM 链接所有目标文件
echo [4] 链接生成 L2 编译器...
echo.

set OBJS=
target\l2_compiler\runtime.xy.obj 
target\l2_compiler\lexer.xy.obj 
target\l2_compiler\parser.xy.obj 
target\l2_compiler\sema.xy.obj 
target\l2_compiler\codegen.xy.obj 
target\l2_compiler\utils.xy.obj 
target\l2_compiler\main.xy.obj 
target\l2_compiler\runtime.obj

%LINK_EXE% /OUT:"target\l2_compiler\xyc.exe" %OBJS% /NOLOGO
if errorlevel 1 (
    echo 链接失败！
    exit /b 1
)
echo 链接成功！
echo.

echo ========================================
echo L2 编译器构建完成！
echo 输出文件: target\l2_compiler\xyc.exe
echo ========================================
echo.
echo 测试 L2 编译器...
echo.

REM 测试 L2 编译器
target\l2_compiler\xyc.exe --version
if errorlevel 1 (
    echo 测试失败！
    exit /b 1
)
echo 测试成功！
echo.
echo L2 编译器已准备就绪！
echo 可以使用: target\l2_compiler\xyc.exe
