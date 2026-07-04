@echo off

set "L1_COMPILER=cargo run --release --"
set "LLC_EXE=llc"
set "CLANG_EXE=clang"

rem 创建输出目录
if not exist "target\l2_compiler" mkdir "target\l2_compiler"

echo [1] 编译模块到 LLVM IR...
echo.

for %%m in (runtime.xy lexer.xy parser.xy sema.xy codegen.xy utils.xy main.xy) do (
    echo 编译 %%m...
    %L1_COMPILER% src\compiler_v2\%%m --ir-pure 2>&1 > target\l2_compiler\%%m.ll
    if errorlevel 1 (
        echo 编译 %%m 失败！
        exit /b 1
    )
    echo 编译 %%m 成功
    echo.
)

echo [2] 编译 IR 为目标文件...
echo.

rem 为所有 IR 文件添加列表操作函数定义

rem 为所有 IR 文件添加 rt_list_set 声明
for %%m in (lexer.xy parser.xy sema.xy codegen.xy) do (
    echo 为 %%m.ll 添加 rt_list_set 声明...
    rem 创建一个临时文件，包含新的声明和原始内容
    echo declare void @rt_list_set(i8*, i64, i8*) > target\l2_compiler\%%m.ll.tmp
    type target\l2_compiler\%%m.ll >> target\l2_compiler\%%m.ll.tmp
    rem 替换原始文件
    move /y target\l2_compiler\%%m.ll.tmp target\l2_compiler\%%m.ll
    echo 添加声明成功
)
echo.

rem 为所有 IR 文件添加列表操作函数定义
for %%m in (lexer.xy parser.xy sema.xy codegen.xy) do (
    echo 为 %%m.ll 添加函数定义...
    rem 创建一个临时文件，包含原始内容和新的函数定义
    type target\l2_compiler\%%m.ll > target\l2_compiler\%%m.ll.tmp
    echo >> target\l2_compiler\%%m.ll.tmp
    echo ; 列表操作函数定义 >> target\l2_compiler\%%m.ll.tmp
    echo define i64 @_u5217_u8868_u8ffd_u52a0(i8* %list%, i64 %value%) { >> target\l2_compiler\%%m.ll.tmp
    echo   entry: >> target\l2_compiler\%%m.ll.tmp
    echo   %value%_i8 = inttoptr i64 %value% to i8* >> target\l2_compiler\%%m.ll.tmp
    echo   call void @rt_list_append(i8* %list%, i8* %value%_i8) >> target\l2_compiler\%%m.ll.tmp
    echo   ret i64 0 >> target\l2_compiler\%%m.ll.tmp
    echo } >> target\l2_compiler\%%m.ll.tmp
    echo >> target\l2_compiler\%%m.ll.tmp
    echo define i64 @_u5217_u8868_u8bbe_u7f6e(i8* %list%, i64 %index%, i64 %value%) { >> target\l2_compiler\%%m.ll.tmp
    echo   entry: >> target\l2_compiler\%%m.ll.tmp
    echo   %value%_i8 = inttoptr i64 %value% to i8* >> target\l2_compiler\%%m.ll.tmp
    echo   call void @rt_list_set(i8* %list%, i64 %index%, i8* %value%_i8) >> target\l2_compiler\%%m.ll.tmp
    echo   ret i64 0 >> target\l2_compiler\%%m.ll.tmp
    echo } >> target\l2_compiler\%%m.ll.tmp
    rem 替换原始文件
    move /y target\l2_compiler\%%m.ll.tmp target\l2_compiler\%%m.ll
    echo 添加函数定义成功
    echo.
)

for %%m in (runtime.xy lexer.xy parser.xy sema.xy codegen.xy utils.xy main.xy) do (
    echo 编译 %%m.ll...
    %LLC_EXE% target\l2_compiler\%%m.ll -filetype=obj -o target\l2_compiler\%%m.obj
    if errorlevel 1 (
        echo 编译 %%m.ll 失败！
        exit /b 1
    )
    echo 编译 %%m.ll 成功
    echo.
)

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
    target\l2_compiler\runtime.xy.obj ^
    target\l2_compiler\lexer.xy.obj ^
    target\l2_compiler\parser.xy.obj ^
    target\l2_compiler\sema.xy.obj ^
    target\l2_compiler\codegen.xy.obj ^
    target\l2_compiler\utils.xy.obj ^
    target\l2_compiler\main.xy.obj ^
    target\l2_compiler\runtime.obj ^
    -o target\l2_compiler\xyc.exe
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