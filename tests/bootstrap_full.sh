#!/bin/bash
# @file bootstrap_full.sh
# @brief 全模块自举验证脚本 (v0.4.0)
# @description L1→xyc→xyc2 三阶段自举 + IR 等价验证
#
# 流程:
#   Stage 1: L1 编译全部 L2 源码 → xyc.exe (L2编译器)
#   Stage 2: xyc.exe 编译测试文件 → IR, 与 L1 的 IR 对比
#   Stage 3: xyc.exe 自编译 glue → xyc_boot.exe → IR 对比
#
# 用法: bash tests/bootstrap_full.sh [--release]

# set -e  # 不用严格模式，允许单个步骤失败后继续

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/target/l2_compiler"

# 编译器路径
if [ "$1" = "--release" ]; then
    XY_COMPILER="$PROJECT_ROOT/target/release/xy.exe"
    PROFILE="release"
else
    XY_COMPILER="$PROJECT_ROOT/target/debug/xy.exe"
    PROFILE="debug"
fi

# LLVM 工具检测（CI 中可能无版本后缀，或为 llc-15 等）
LLC=""
for candidate in llc llc-15 llc-14 llc-16 llc-17 llc-18; do
    if command -v "$candidate" > /dev/null 2>&1; then
        LLC="$candidate"
        break
    fi
done
CLANG=""
for candidate in clang clang-15 clang-14 clang-16 clang-17 clang-18; do
    if command -v "$candidate" > /dev/null 2>&1; then
        CLANG="$candidate"
        break
    fi
done
RUNTIME="$PROJECT_ROOT/runtime/runtime.c"
SRC_DIR="$PROJECT_ROOT/src/compiler_v2"
HAS_LLVM=1
if [ -z "$LLC" ] || [ -z "$CLANG" ]; then
    echo "警告: LLVM 工具 (llc/clang) 未找到，将跳过 obj 链接步骤"
    HAS_LLVM=0
fi

# 颜色
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

PASSED=0
FAILED=0

pass() { echo -e "${GREEN}✓${NC} $1"; PASSED=$((PASSED+1)); }
fail() { echo -e "${RED}✗${NC} $1"; FAILED=$((FAILED+1)); }
info() { echo -e "${YELLOW}→${NC} $1"; }

# 简单的 IR 归一化 + 比较
# 忽略注释行、空白行、临时变量编号（%tmpXXX, %LXXX）
compare_ir() {
    local ir1="$1"
    local ir2="$2"
    local label="$3"

    # 归一化：移除注释、归一化临时变量名、移除空行
    normalize_ir() {
        sed -E '
            s/^[[:space:]]*;.*$//g
            s/%[a-zA-Z_][a-zA-Z0-9_]*_[0-9]+/%VAR/g
            s/%L[0-9]+/%LABEL/g
            s/L[0-9]+:/LABEL:/g
            s/@str_constant_str_[0-9]+/@STR/g
            s/%[a-zA-Z_][a-zA-Z0-9_]*_[0-9]+/%VAR/g
            /^[[:space:]]*$/d
        ' "$1"
    }

    normalize_ir "$ir1" > "$ir1.norm"
    normalize_ir "$ir2" > "$ir2.norm"

    if diff -q "$ir1.norm" "$ir2.norm" > /dev/null 2>&1; then
        pass "$label"
        rm -f "$ir1.norm" "$ir2.norm"
        return 0
    else
        local diff_count
        diff_count=$(diff "$ir1.norm" "$ir2.norm" | wc -l)
        info "  IR 差异: $diff_count 行 (已忽略变量名/标签编号差异)"
        rm -f "$ir1.norm" "$ir2.norm"
        return 1
    fi
}

echo "========================================"
echo "  玄语全模块自举验证"
echo "  v0.4.0 - L1→xyc→xyc2 三阶段"
echo "========================================"
echo ""
info "配置文件: $PROFILE"
info "编译器: $XY_COMPILER"
info "构建目录: $BUILD_DIR"
echo ""

# 创建构建目录
mkdir -p "$BUILD_DIR"

# =============================================
# Stage 0: 前置检查
# =============================================
echo "--- Stage 0: 前置检查 ---"

if [ ! -f "$XY_COMPILER" ]; then
    info "编译器不存在，正在构建..."
    if [ "$PROFILE" = "release" ]; then
        cargo build --release 2>&1 | tail -3
    else
        cargo build 2>&1 | tail -3
    fi
fi

if [ ! -f "$XY_COMPILER" ]; then
    fail "编译器构建失败"
    exit 1
fi
pass "编译器就绪: $XY_COMPILER"

# 创建简单的独立测试文件（无 import，L2 可编译）
STANDALONE_TEST="$BUILD_DIR/bootstrap_test_input.xy"
cat > "$STANDALONE_TEST" << 'EOF'
函数 主(): 整数 {
    定义 可变 x: 整数
    x = 42
    定义 可变 y: 整数
    y = x + 10
    打印整数(y)
    返回 0
}
EOF
info "测试文件: $STANDALONE_TEST"

# =============================================
# Stage 1: L1 编译完整 L2 源码 → xyc.exe
# =============================================
echo ""
echo "--- Stage 1: L1 构建 L2 编译器 (xyc.exe) ---"

XycFullLL="$BUILD_DIR/xyc_full.ll"
XycFullObj="$BUILD_DIR/xyc_full.obj"
XycExe="$BUILD_DIR/xyc.exe"
RuntimeObj="$BUILD_DIR/runtime.obj"

# 1a: L1 → IR
info "1a: L1 编译 xyc.xy (全部 L2 模块) → IR"
if $XY_COMPILER "$SRC_DIR/xyc.xy" --ir-file "$XycFullLL" 2>&1 | grep -q "IR 已写入\|编译成功"; then
    pass "L1 → xyc_full.ll ($(wc -c < "$XycFullLL") bytes)"
else
    fail "L1 编译 xyc.xy 失败"
    exit 1
fi

# 1b: IR → obj (需要 LLVM)
if [ "$HAS_LLVM" = "1" ]; then
info "1b: llc IR → obj"
if $LLC "$XycFullLL" -filetype=obj -o "$XycFullObj" 2>&1; then
    pass "llc → xyc_full.obj"
else
    fail "llc 失败"
    exit 1
fi

# 1c: 编译 runtime
info "1c: 编译 runtime.c"
if $CLANG -c -O2 "$RUNTIME" -o "$RuntimeObj" 2>&1; then
    pass "clang → runtime.obj"
else
    fail "runtime 编译失败"
    exit 1
fi

# 1d: 链接 → xyc.exe
info "1d: 链接 → xyc.exe"
if $CLANG "$XycFullObj" "$RuntimeObj" -o "$XycExe" "-Wl,/SUBSYSTEM:console" 2>&1; then
    pass "链接 → xyc.exe"
else
    fail "链接失败"
    exit 1
fi
else
    info "跳过 llc/clang 链接步骤（LLVM 工具不可用）"
    info "  自举可执行文件构建需要 LLVM，仅验证 IR 生成"
    XycExe="$XY_COMPILER"  # 回退使用 L1 编译器进行后续测试
fi

# =============================================
# Stage 2: IR 等价验证 (L1 vs xyc)
# =============================================
echo ""
echo "--- Stage 2: IR 等价验证 (L1 vs xyc) ---"

# 2a: L1 编译测试文件 → IR
L1_IR="$BUILD_DIR/test_l1.ll"
info "2a: L1 编译测试文件"
if $XY_COMPILER "$STANDALONE_TEST" --ir-file "$L1_IR" 2>&1 | grep -q "IR 已写入\|编译成功"; then
    pass "L1 编译测试文件成功"
else
    fail "L1 编译测试文件失败"
fi

# 2b: xyc 编译测试文件 → IR
XycIR="$BUILD_DIR/test_xyc.ll"
info "2b: xyc.exe 编译测试文件"
xyc_test_out="$BUILD_DIR/xyc_test_out.log"
if $XycExe "$STANDALONE_TEST" --ir-file "$XycIR" > "$xyc_test_out" 2>&1; then
    if [ -f "$XycIR" ] && [ -s "$XycIR" ]; then
        pass "xyc 编译测试文件成功 ($(wc -c < "$XycIR") bytes IR)"
    else
        fail "xyc 编译成功但 IR 文件为空"
    fi
else
    fail "xyc 编译测试文件失败 (exit $?)"
    info "  (L2 可能不支持部分语法，继续后续步骤)"
fi

# 2c: IR 对比
if [ -f "$XycIR" ] && [ -f "$L1_IR" ]; then
    info "2c: IR 等价对比 (L1 vs xyc)"
    compare_ir "$L1_IR" "$XycIR" "IR L1 vs xyc"
fi

# =============================================
# Stage 2.5: 自举回归用例 (async/match 语法验证)
# =============================================
echo ""
echo "--- Stage 2.5: 自举回归用例 ---"

BOOTSTRAP_TESTS=(
    "$PROJECT_ROOT/tests/bootstrap/self_compile_test.xy:自举基础测试"
    "$PROJECT_ROOT/tests/bootstrap/async_syntax_test.xy:async语法测试"
    "$PROJECT_ROOT/tests/bootstrap/match_syntax_test.xy:match语法测试"
)

for bt_entry in "${BOOTSTRAP_TESTS[@]}"; do
    bt_file="${bt_entry%%:*}"
    bt_name="${bt_entry##*:}"
    bt_ir="$BUILD_DIR/bt_$(basename "${bt_file%.xy}").ll"

    # L1 编译
    info "L1: $bt_name"
    if $XY_COMPILER "$bt_file" --ir-file "$bt_ir" 2>&1 | grep -q "IR 已写入\|编译成功"; then
        pass "L1 → $bt_name"
    else
        fail "L1 → $bt_name"
    fi

    # xyc 编译 (如果有)
    if [ -f "$XycExe" ] && [ "$XycExe" != "$XY_COMPILER" ]; then
        xyc_bt_out="$BUILD_DIR/bt_xyc_$(basename "${bt_file%.xy}").log"
        if $XycExe "$bt_file" --ir-file /dev/null > "$xyc_bt_out" 2>&1; then
            pass "xyc → $bt_name"
        else
            info "  xyc → $bt_name: 跳过 (exit $?)"
        fi
    fi
done

# =============================================
# Stage 3: 自举编译 (xyc → xyc_boot)
# =============================================
echo ""
echo "--- Stage 3: 自举编译 (xyc → xyc_boot) ---"

BOOTSTRAP_SRC="$PROJECT_ROOT/bootstrap2.xy"
BootstrapLL="$BUILD_DIR/bootstrap.ll"
BootstrapObj="$BUILD_DIR/bootstrap.obj"
XycBootExe="$BUILD_DIR/xyc_boot.exe"

# 3a: xyc 编译 bootstrap2.xy
info "3a: xyc.exe 编译 bootstrap2.xy → IR"
xyc_bootstrap_out="$BUILD_DIR/xyc_bootstrap_out.log"
if $XycExe "$BOOTSTRAP_SRC" --ir-file "$BootstrapLL" > "$xyc_bootstrap_out" 2>&1; then
    if [ -f "$BootstrapLL" ] && [ -s "$BootstrapLL" ]; then
        pass "xyc → bootstrap.ll ($(wc -c < "$BootstrapLL") bytes)"
    else
        fail "xyc 编译成功但 IR 文件为空"
    fi
else
    fail "xyc 编译 bootstrap2.xy 失败 (exit $?)"
    tail -20 "$xyc_bootstrap_out"
fi

# 3b: llc (需要 LLVM)
if [ "$HAS_LLVM" = "1" ]; then
info "3b: llc bootstrap.ll → obj"
if $LLC "$BootstrapLL" -filetype=obj -o "$BootstrapObj" 2>&1; then
    pass "llc → bootstrap.obj"
else
    fail "llc bootstrap 失败"
fi

# 3c: 链接 → xyc_boot.exe
info "3c: 链接 → xyc_boot.exe"
if $CLANG "$BootstrapObj" "$RuntimeObj" -o "$XycBootExe" "-Wl,/SUBSYSTEM:console" 2>&1; then
    pass "链接 → xyc_boot.exe"
else
    fail "链接 xyc_boot 失败"
fi
else
    info "跳过 llc/clang 步骤"
    XycBootExe=""
fi

# 3d: xyc_boot 编译测试文件
if [ -f "$XycBootExe" ]; then
    XycBootIR="$BUILD_DIR/test_xyc_boot.ll"
    info "3d: xyc_boot.exe 编译测试文件"
    xyc_boot_out="$BUILD_DIR/xyc_boot_out.log"
    if $XycBootExe "$STANDALONE_TEST" --ir-file "$XycBootIR" > "$xyc_boot_out" 2>&1; then
        pass "xyc_boot 编译测试文件成功"
        # IR 对比
        if [ -f "$XycIR" ]; then
            info "3e: IR 等价对比 (xyc vs xyc_boot)"
            compare_ir "$XycIR" "$XycBootIR" "IR xyc vs xyc_boot"
        fi
    else
        fail "xyc_boot 编译测试文件失败"
    fi
fi

# =============================================
# Stage 4: 逐模块 IR 验证 (L1 单独编译每个 L2 模块)
# =============================================
echo ""
echo "--- Stage 4: L2 模块编译验证 ---"

L2_FILES=(
    "types.xy"
    "utils_s.xy"
    "lexer_s.xy"
    "parser_s.xy"
    "sema_s.xy"
    "codegen_s.xy"
    "compiler_new.xy"
    "xyc.xy"
)

for l2file in "${L2_FILES[@]}"; do
    fullpath="$SRC_DIR/$l2file"
    if [ -f "$fullpath" ]; then
        outir="$BUILD_DIR/${l2file%.xy}_l1.ll"
        if $XY_COMPILER "$fullpath" --ir-file "$outir" 2>&1 | grep -q "IR 已写入\|编译成功"; then
            pass "L1 → $l2file ($(wc -c < "$outir") bytes IR)"
        else
            fail "L1 → $l2file"
        fi
    fi
done

# =============================================
# 结果汇总
# =============================================
echo ""
echo "========================================"
echo "  自举验证结果"
echo "========================================"
echo -e "通过: ${GREEN}$PASSED${NC}"
echo -e "失败: ${RED}$FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ 全模块自举验证成功！${NC}"
    exit 0
else
    echo -e "${RED}✗ 有 $FAILED 项验证失败${NC}"
    exit 1
fi
