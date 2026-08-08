#!/usr/bin/env python3
"""
merge_l2_compact.py — 生成紧凑版的合并L2源文件
去掉注释、压缩空白，大幅减小文件体积
"""
import re

SRC = "src/compiler_v2"
ORDER = [
    "types.xy", "runtime.xy", "utils_s.xy",
    "lexer_s.xy", "parser_s.xy", "sema_s.xy",
    "codegen_s.xy", "compiler_new.xy",
]

IMPORT_RE = re.compile(r'^\s*引入\s+"[^"]*"\s*$')

def process(filename):
    with open(f"{SRC}/{filename}", "r", encoding="utf-8") as f:
        content = f.read()

    # Remove block comments
    content = re.sub(r'/\*.*?\*/', '', content, flags=re.DOTALL)
    # Remove line comments (but keep // in strings)
    lines = []
    for line in content.split('\n'):
        # Simple: remove // comments (ignore strings for now)
        if '//' in line and '"' not in line:
            line = line[:line.index('//')]
        lines.append(line)
    content = '\n'.join(lines)

    # Remove import lines
    result = []
    for line in content.split('\n'):
        if IMPORT_RE.match(line.strip()):
            continue
        # Skip top-level print
        if '打印("工具模块已加载")' in line:
            continue
        result.append(line)
    content = '\n'.join(result)

    # Remove trailing whitespace, collapse multiple blank lines
    content = re.sub(r'[ \t]+$', '', content, flags=re.MULTILINE)
    content = re.sub(r'\n{3,}', '\n\n', content)

    return content


def main():
    parts = []
    parts.append("// xyc_merged_compact.xy - auto-generated")
    parts.append("")
    for fn in ORDER:
        parts.append(f"// ==== {fn} ====")
        parts.append(process(fn))
        parts.append("")
    output = '\n'.join(parts)
    print(output)


if __name__ == "__main__":
    main()
