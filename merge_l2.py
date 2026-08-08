#!/usr/bin/env python3
"""
merge_l2.py — 合并 L2 编译器模块为单文件

将 src/compiler_v2/ 下的所有 .xy 模块按依赖顺序合并为一个文件，
去除重复的 引入 语句，解决 codegen.xy 与 codegen_s.xy 的冲突，
生成可被 L2 编译器直接编译的单文件源码。

用法: python merge_l2.py > xyc_merged.xy
"""

import re
import sys

SRC_DIR = "src/compiler_v2"

# 合并顺序（依赖优先）
ORDER = [
    "types.xy",         # 共享类型定义（结构体/枚举）
    "runtime.xy",       # 外部函数声明
    # codegen.xy 跳过 — 与 codegen_s.xy 函数重复，Codegen.output 类型冲突
    "utils_s.xy",       # 工具函数
    "lexer_s.xy",       # 词法分析器
    "parser_s.xy",      # 语法解析器
    "sema_s.xy",        # 语义分析器
    "codegen_s.xy",     # 代码生成器（主版本）
    "compiler_new.xy",  # 编译器驱动（含 主() 入口）
]

# 引入语句模式
IMPORT_RE = re.compile(r'^\s*引入\s+"[^"]*"\s*$')

def process_file(filename, remove_imports=True):
    """读取并处理一个 .xy 文件"""
    filepath = f"{SRC_DIR}/{filename}"
    with open(filepath, "r", encoding="utf-8") as f:
        lines = f.readlines()

    result = []
    in_comment = False
    for line in lines:
        # 跳过引入语句
        if remove_imports and IMPORT_RE.match(line.strip()):
            continue

        # 跳过 utils_s.xy 的顶层打印
        if filename == "utils_s.xy" and '打印("工具模块已加载")' in line:
            continue

        result.append(line.rstrip())

    return result


def main():
    output_lines = []
    output_lines.append("/**")
    output_lines.append(" * @file xyc_merged.xy")
    output_lines.append(" * @brief 玄语L2编译器 - 合并源码（自举用）")
    output_lines.append(" * @description 由 merge_l2.py 自动生成，合并所有编译器模块")
    output_lines.append(f" * 模块数: {len(ORDER)}")
    output_lines.append(" */")
    output_lines.append("")

    for filename in ORDER:
        output_lines.append(f"/* ======== 模块: {filename} ======== */")
        output_lines.append("")

        try:
            lines = process_file(filename)
            for line in lines:
                output_lines.append(line)
            output_lines.append("")
        except FileNotFoundError:
            print(f"错误: 找不到文件 {SRC_DIR}/{filename}", file=sys.stderr)
            sys.exit(1)

    # 输出到stdout
    for line in output_lines:
        print(line)


if __name__ == "__main__":
    main()
