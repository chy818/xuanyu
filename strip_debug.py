#!/usr/bin/env python3
"""
strip_debug.py — 正确注释掉 L2 源文件中的调试打印语句
"""
import re

def strip_debug_prints(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    result = []
    in_debug_block = False

    for i, line in enumerate(lines):
        stripped = line.strip()

        # Detect start of debug block
        is_debug_start = (stripped.startswith('打印("DEBUG') or
                          stripped.startswith('打印("DBG ') or
                          stripped.startswith('打印("[调试]'))

        if is_debug_start:
            in_debug_block = True
            # Properly comment out: wrap in /* */
            result.append('/*[DBG] ' + line.rstrip() + ' */')
            continue

        if in_debug_block:
            is_print = stripped.startswith('打印(') or stripped.startswith('打印整数(')
            is_newline = stripped == '打印("\\n")' or stripped.startswith('打印("\\n")')

            if is_print or is_newline:
                result.append('/*[DBG] ' + line.rstrip() + ' */')
                continue
            else:
                in_debug_block = False
                result.append(line.rstrip())
                continue

        result.append(line.rstrip())

    return result


def main():
    files = [
        "src/compiler_v2/lexer_s.xy",
        "src/compiler_v2/parser_s.xy",
        "src/compiler_v2/sema_s.xy",
        "src/compiler_v2/codegen_s.xy",
        "src/compiler_v2/compiler_new.xy",
    ]

    for filepath in files:
        try:
            new_lines = strip_debug_prints(filepath)
            with open(filepath, 'w', encoding='utf-8') as f:
                for line in new_lines:
                    f.write(line + '\n')
            print(f"OK: {filepath}")
        except FileNotFoundError:
            print(f"SKIP: {filepath} not found")


if __name__ == "__main__":
    main()
