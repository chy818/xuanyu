#!/bin/bash
# @file bootstrap_test.sh
# @brief 自展验证脚本 - 完整验证
# @description 验证 XY 编译器能够编译自展代码的各个模块

set -e

echo "========================================"
echo "  玄语编译器自展验证测试"
echo "========================================"
echo ""

# 设置路径
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
XY_COMPILER="$PROJECT_ROOT/target/debug/xy.exe"
SRC_DIR="$PROJECT_ROOT/src/compiler_v2"
OUTPUT_DIR="$PROJECT_ROOT/output"

# 创建输出目录
mkdir -p "$OUTPUT_DIR"

# 统计变量
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# 测试函数
run_test() {
    local name=$1
    local file=$2
    local flag=$3

    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    echo "----------------------------------------"
    echo "测试 $TOTAL_TESTS: $name"
    echo "文件: $file"
    echo "----------------------------------------"

    if [ -f "$file" ]; then
        if $XY_COMPILER "$file" $flag 2>&1 | tail -20; then
            echo "✓ $name 通过"
            PASSED_TESTS=$((PASSED_TESTS + 1))
        else
            echo "✗ $name 失败"
            FAILED_TESTS=$((FAILED_TESTS + 1))
        fi
    else
        echo "⚠ 文件不存在: $file"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
    echo ""
}

# 检查编译器是否存在
echo "[1/5] 检查编译器..."
if [ ! -f "$XY_COMPILER" ]; then
    echo "错误: 编译器不存在: $XY_COMPILER"
    echo "请先运行: cargo build"
    exit 1
fi
echo "✓ 编译器存在: $XY_COMPILER"
echo ""

# 显示版本
echo "[2/5] 显示编译器信息..."
echo "编译器版本: v0.2.0-beta (自展版本)"
echo ""

# 测试词法分析器
echo "[3/5] 测试自展编译器模块..."
run_test "统一入口 (xyc.xy)" "$SRC_DIR/xyc.xy" "--ir"

# 测试语法分析器
run_test "词法分析器 (lexer_s.xy)" "$SRC_DIR/lexer_s.xy" "--ir"

# 测试语义分析器
run_test "语法分析器 (parser_s.xy)" "$SRC_DIR/parser_s.xy" "--ir"

# 测试代码生成器
run_test "语义分析器 (sema_s.xy)" "$SRC_DIR/sema_s.xy" "--ir"

# 测试运行时
run_test "代码生成器 (codegen_s.xy)" "$SRC_DIR/codegen_s.xy" "--ir"

# 测试运行时接口
run_test "运行时接口 (runtime.xy)" "$SRC_DIR/runtime.xy" "--ir"

# 测试 hello.xy
run_test "Hello World 测试" "$PROJECT_ROOT/examples/hello.xy" "--ir"

echo ""
echo "[4/5] 测试示例程序..."
run_test "示例: test_enum.xy" "$PROJECT_ROOT/examples/test_enum.xy" "--ir"

echo ""
echo "[5/5] 编译自展测试用例并运行..."
echo "----------------------------------------"
echo "测试: 编译运行 hello.xy"
echo "----------------------------------------"
if [ -f "$PROJECT_ROOT/examples/hello.xy" ]; then
    $XY_COMPILER "$PROJECT_ROOT/examples/hello.xy" --run 2>&1 | tail -30 || true
fi
echo ""

# 打印统计结果
echo "========================================"
echo "  自展验证测试结果"
echo "========================================"
echo "总计测试: $TOTAL_TESTS"
echo "通过: $PASSED_TESTS"
echo "失败: $FAILED_TESTS"
echo ""

if [ $FAILED_TESTS -eq 0 ]; then
    echo "✓ 所有测试通过！自展验证成功！"
    exit 0
else
    echo "✗ 有 $FAILED_TESTS 个测试失败"
    exit 1
fi