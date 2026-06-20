#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
自动检测 IR 文件中缺失的外部函数声明
"""

import os
import re

# 需要检查和修复的文件
IR_FILES = [
    'target/l2_compiler/codegen.xy.ll',
    'target/l2_compiler/sema.xy.ll',
    'target/l2_compiler/parser.xy.ll',
]

# 所有已知的运行时函数声明（完整列表）
KNOWN_FUNCTIONS = [
    'declare i8* @int_to_str(i64)',
    'declare i8* @str_concat(i8*, i8*)',
    'declare i64 @str_to_int(i8*)',
    'declare i64 @str_len(i64)',
    'declare i64 @strlen(i8*)',
    'declare i8* @str_slice(i8*, i64, i64)',
    'declare i1 @str_equals(i8*, i8*)',
    'declare i1 @str_contains(i8*, i8*)',
    'declare double @int_to_float(i64)',
    'declare i64 @float_to_int(double)',
    'declare i8* @float_to_str(double)',
    'declare double @str_to_float(i8*)',
    'declare i64 @get_int_input()',
    'declare i8* @get_str_input()',
    'declare void @print_int(i64)',
    'declare void @print_str(i8*)',
    'declare void @print(i8*)',
    'declare i64 @file_size(i8*)',
    'declare i8* @file_read(i8*)',
    'declare i64 @file_write(i8*, i8*)',
    'declare i1 @file_exists(i8*)',
    'declare i1 @file_delete(i8*)',
    'declare i8* @rt_list_new()',
    'declare void @rt_list_append(i8*, i8*)',
    'declare i64 @rt_list_len(i8*)',
    'declare i8* @rt_list_get(i8*, i64)',
    'declare void @rt_list_set(i8*, i64, i8*)',
    'declare i64 @list_len(i8*)',
    'declare i8* @list_get(i8*, i64)',
    'declare void @list_add(i8*, i64)',
    'declare void @list_set(i8*, i64, i8*)',
    'declare void @list_insert(i8*, i64, i8*)',
    'declare void @list_remove(i8*, i64)',
    'declare i64 @list_size(i8*)',
    'declare i64 @rt_string_len(i8*)',
    'declare i8* @rt_string_concat(i8*, i8*)',
    'declare i8* @rt_string_slice(i8*, i64, i64)',
    'declare i8* @rt_readline()',
    'declare void @print_bool(i64)',
]

def get_declared_functions(content):
    """获取文件中已声明的所有函数"""
    declared = set()
    for match in re.finditer(r'declare\s+.*?\s+@(\w+)\s*\(', content):
        declared.add(match.group(1))
    return declared

def get_called_functions(content):
    """获取文件中调用的所有外部函数"""
    called = set()
    for match in re.finditer(r'call\s+.*?\s+@(\w+)\s*\(', content):
        called.add(match.group(1))
    return called

def has_function(content, func_name):
    """检查函数是否已声明"""
    pattern = r'declare\s+.*?\s+@' + re.escape(func_name) + r'\s*\('
    return bool(re.search(pattern, content))

def fix_ir_file(file_path):
    """修复单个 IR 文件"""
    if not os.path.exists(file_path):
        print(f"  文件不存在: {file_path}")
        return False

    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    declared = get_declared_functions(content)
    called = get_called_functions(content)

    # 找出缺失的函数
    missing = called - declared

    if not missing:
        print(f"  {file_path} - 无缺失声明")
        return True

    print(f"  {file_path} - 缺失 {len(missing)} 个函数: {sorted(missing)}")

    # 找到插入位置（第一个 define 或第一个非注释、非空行）
    lines = content.split('\n')
    insert_idx = 0
    for i, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith('define ') or (stripped.startswith('@') and not stripped.startswith('declare')):
            insert_idx = i
            break

    # 添加缺失的声明
    new_decls = []
    for func_name in sorted(missing):
        for decl in KNOWN_FUNCTIONS:
            if f'@{func_name}(' in decl:
                new_decls.append(decl)
                break
        else:
            # 如果不知道函数签名，生成一个通用的声明
            print(f"    警告: 未知函数 @ {func_name}，生成通用声明")
            new_decls.append(f'declare i64 @{func_name}()')

    if new_decls:
        lines.insert(insert_idx, '\n; === 补充的外部函数声明 ===')
        lines.insert(insert_idx + 1, '\n'.join(new_decls))
        content = '\n'.join(lines)

        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(content)

        print(f"    已添加 {len(new_decls)} 个声明")
        return True

    return True

def main():
    print("检测并修复 IR 文件中缺失的外部函数声明...")
    print()

    all_missing = []
    for ir_file in IR_FILES:
        print(f"处理 {ir_file}...")
        if not os.path.exists(ir_file):
            print(f"  文件不存在，跳过")
            continue

        with open(ir_file, 'r', encoding='utf-8') as f:
            content = f.read()

        declared = get_declared_functions(content)
        called = get_called_functions(content)
        missing = called - declared

        if missing:
            all_missing.extend([(ir_file, m) for m in missing])

        fix_ir_file(ir_file)
        print()

    if all_missing:
        print(f"\n总共发现 {len(all_missing)} 个缺失声明")
    else:
        print("\n所有函数声明完整!")

if __name__ == '__main__':
    main()
