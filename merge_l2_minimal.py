#!/usr/bin/env python3
"""
merge_l2_minimal.py — 创建最小合并文件用于快速迭代测试
只包含核心模块：types + runtime + lexer_s + parser_s + sema_s + compiler_new
暂时排除 codegen_s（功能太复杂，先测试lexer/parser/sema流水线）
"""

import re
import sys

SRC_DIR = "src/compiler_v2"

# 精简合并顺序（仅核心模块）
ORDER = [
    "types.xy",
    "runtime.xy",
    "lexer_s.xy",
    "parser_s.xy",
    "sema_s.xy",
    "compiler_new.xy",
]

IMPORT_RE = re.compile(r'^\s*引入\s+"[^"]*"\s*$')

def process_file(filename):
    filepath = f"{SRC_DIR}/{filename}"
    with open(filepath, "r", encoding="utf-8") as f:
        lines = f.readlines()

    result = []
    for line in lines:
        if IMPORT_RE.match(line.strip()):
            continue
        # Skip utils_s top-level print
        if '打印("工具模块已加载")' in line:
            continue
        result.append(line.rstrip())
    return result


def main():
    print("/**")
    print(" * @file xyc_minimal.xy")
    print(" * @brief 玄语L2编译器 - 最小合并源码（测试用）")
    print(" */")
    print("")

    for filename in ORDER:
        print(f"/* ======== 模块: {filename} ======== */")
        print("")
        try:
            lines = process_file(filename)
            for line in lines:
                print(line)
            print("")
        except FileNotFoundError:
            print(f"错误: 找不到文件 {SRC_DIR}/{filename}", file=sys.stderr)
            sys.exit(1)


if __name__ == "__main__":
    main()
