@echo off

set "L1_COMPILER=target\release\xy.exe"
set "LLC_EXE=llc"
set "CLANG_EXE=clang"

if not exist "target\l2_compiler" mkdir "target\l2_compiler"

echo [1] 使用 xyc.xy 统一编译入口生成 LLVM IR...
echo.

%L1_COMPILER% src\compiler_v2\xyc.xy --ir-file target\l2_compiler\xyc.ll
if errorlevel 1 (
    echo 编译 xyc.xy 失败！
    exit /b 1
)
echo 编译 xyc.xy 成功
echo.

echo [2] 编译 IR 为目标文件...
echo.

%LLC_EXE% target\l2_compiler\xyc.ll -filetype=obj -o target\l2_compiler\xyc.obj
if errorlevel 1 (
    echo 编译 xyc.ll 失败！
    exit /b 1
)
echo 编译 xyc.ll 成功
echo.

echo [3] 编译 C 运行时库...
echo.

%CLANG_EXE% -c -O2 runtime\runtime.c -o target\l2_compiler\runtime.obj
if errorlevel 1 (
    echo 编译 runtime.c 失败！
    exit /b 1
)
echo 编译 runtime.c 成功
echo.

echo [4] 链接生成 L2 编译器...
echo.

%CLANG_EXE% ^
    target\l2_compiler\xyc.obj ^
    target\l2_compiler\runtime.obj ^
    -o target\l2_compiler\xyc.exe ^
    "-Wl,/SUBSYSTEM:console"
if errorlevel 1 (
    echo 链接失败！
    exit /b 1
)
echo 链接成功！
echo.

echo =====================================
echo L2 编译器构建完成！
echo 输出文件: target\l2_compiler\xyc.exe
echo =====================================
echo.

echo 测试 L2 编译器...
echo.

target\l2_compiler\xyc.exe --version
if errorlevel 1 (
    echo 测试失败！
    exit /b 1
)
echo 测试成功！
echo.

echo L2 编译器已准备就绪！
echo 可以使用: target\l2_compiler\xyc.exe