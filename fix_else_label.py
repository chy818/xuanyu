#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""修复 IR 文件中 if 语句的 else/end 标签前缺少 br 指令的问题 """

def fix_br_before_label(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    new_lines = []
    i = 0
    while i < len(lines):
        stripped = lines[i].strip()

        # 检测 else_、ifend_、loop_ 或其他循环标签
        is_special_label = False
        label_target = None
        if stripped.startswith('else_') or stripped.startswith('ifend_') or stripped.startswith('loop_') or stripped.startswith('while_') or stripped.startswith('for_'):
            is_special_label = True
            label_target = stripped[:-1]  # 去掉末尾的 :

        if is_special_label and new_lines:
            prev_stripped = new_lines[-1].strip()

            # 忽略函数结束的 }
            if prev_stripped == '}' or prev_stripped.startswith('}'):
                pass  # 不要在函数结束的 } 后添加 br
            else:
                # 检查前一行是否是 terminator
                is_terminator = (
                    prev_stripped.startswith('ret ') or
                    prev_stripped.startswith('br ') or
                    prev_stripped.startswith('switch ') or
                    prev_stripped.startswith('unreachable')
                )

                # 检查前一行是否以 : 结尾（是另一个标签）
                is_prev_label = ':' in prev_stripped and prev_stripped.split(':')[0].strip()

                # 如果前一行不是 terminator 且不是标签，需要添加 br
                if not is_terminator and not is_prev_label:
                    new_lines.append(f"  br label %{label_target}\n")
                    print(f"  Added br before {label_target} (prev was: {prev_stripped[:50]})")

                # 如果前一行也是标签，需要添加 br
                if is_prev_label and not is_terminator:
                    new_lines.append(f"  br label %{label_target}\n")
                    print(f"  Added br between labels: {prev_stripped[:30]} -> {label_target}")

        new_lines.append(lines[i])
        i += 1

    with open(filepath, 'w', encoding='utf-8') as f:
        f.writelines(new_lines)

    print(f"Fixed: {filepath}")

if __name__ == '__main__':
    import os
    files = [
        'target/l2_compiler/lexer.xy.ll',
        'target/l2_compiler/parser.xy.ll',
        'target/l2_compiler/sema.xy.ll',
        'target/l2_compiler/codegen.xy.ll',
        'target/l2_compiler/utils.xy.ll',
        'target/l2_compiler/main.xy.ll',
    ]
    for f in files:
        if os.path.exists(f):
            print(f"Processing {f}...")
            fix_br_before_label(f)
