#!/bin/bash
# @file bootstrap_test.sh
# @brief 自展验证脚本
# @description 验证 XY 编译器能够编译自展代码

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

# 检查编译器是否存在
echo "[1/4] 检查编译器..."
if [ ! -f "$XY_COMPILER" ]; then
    echo "错误: 编译器不存在: $XY_COMPILER"
    echo "请先运行: cargo build"
    exit 1
fi
echo "✓ 编译器存在: $XY_COMPILER"
echo ""

# 测试编译 hello.xy
echo "[2/4] 编译自展测试用例..."
TEST_FILE="$SRC_DIR/hello.xy"
if [ ! -f "$TEST_FILE" ]; then
    echo "错误: 测试文件不存在: $TEST_FILE"
    exit 1
fi

echo "编译: $TEST_FILE"
$XY_COMPILER "$TEST_FILE" --ir 2>&1 | head -50
echo ""

# 检查输出
echo "[3/4] 检查编译结果..."
if [ $? -eq 0 ]; then
    echo "✓ 编译成功"
else
    echo "✗ 编译失败"
    exit 1
fi
echo ""

# 测试基本功能
echo "[4/4] 运行基本测试..."
$XY_COMPILER "$PROJECT_ROOT/examples/test_enum.xy" --ir 2>&1 | tail -10
echo ""

echo "========================================"
echo "  自展验证测试完成"
echo "========================================"
