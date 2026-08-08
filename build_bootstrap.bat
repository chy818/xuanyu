@echo off
REM ============================================
REM  玄语自举构建脚本 (Hybrid Bootstrap)
REM  阶段A: L1编译重模块 + L2编译胶水代码
REM ============================================

setlocal enabledelayedexpansion
set "L1=%CD%\target\release\xy.exe"
set "LLC=llc"
set "CLANG=clang"
set "RUNTIME=runtime\runtime.c"
set "BUILD_DIR=target\l2_compiler"

if not exist "%BUILD_DIR%" mkdir "%BUILD_DIR%"

echo.
echo [1/5] L1 编译完整 L2 源码 → IR
echo.
%L1% src\compiler_v2\xyc.xy --ir-file %BUILD_DIR%\xyc_full.ll
if errorlevel 1 (
    echo 错误: L1 编译失败！
    exit /b 1
)
echo 成功

echo.
echo [2/5] IR → obj
echo.
%LLC% %BUILD_DIR%\xyc_full.ll -filetype=obj -o %BUILD_DIR%\xyc_full.obj
if errorlevel 1 (
    echo 错误: llc 失败！
    exit /b 1
)
%CLANG% -c -O2 %RUNTIME% -o %BUILD_DIR%\runtime.obj
echo 成功

echo.
echo [3/5] 链接 → xyc.exe (L2编译器)
echo.
%CLANG% %BUILD_DIR%\xyc_full.obj %BUILD_DIR%\runtime.obj -o %BUILD_DIR%\xyc.exe "-Wl,/SUBSYSTEM:console"
if errorlevel 1 (
    echo 错误: 链接失败！
    exit /b 1
)
echo 成功

echo.
echo [4/5] L2 编译自举胶水 → IR
echo.
%BUILD_DIR%\xyc.exe bootstrap2.xy --ir-file %BUILD_DIR%\bootstrap.ll
if errorlevel 1 (
    echo 错误: L2 编译失败！
    exit /b 1
)
echo 成功

echo.
echo [5/5] 自举 IR → obj → xyc_boot.exe
echo.
%LLC% %BUILD_DIR%\bootstrap.ll -filetype=obj -o %BUILD_DIR%\bootstrap.obj
if errorlevel 1 (
    echo 错误: llc bootstrap 失败！
    exit /b 1
)
%CLANG% %BUILD_DIR%\bootstrap.obj %BUILD_DIR%\runtime.obj -o %BUILD_DIR%\xyc_boot.exe "-Wl,/SUBSYSTEM:console"
echo 成功

echo.
echo ========================================
echo  自举构建完成！
echo  L2编译器: %BUILD_DIR%\xyc.exe
echo  自举产物: %BUILD_DIR%\xyc_boot.exe
echo ========================================

REM 验证
echo.
echo --- 验证 L2编译器 ---
%BUILD_DIR%\xyc.exe --version 2>&1 | findstr "玄语"
echo --- 验证 自举产物 ---
%BUILD_DIR%\xyc_boot.exe 2>&1
echo.
echo 全流程通过！
