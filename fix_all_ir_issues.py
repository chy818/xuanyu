#!/usr/bin/env python3
"""
综合修复所有IR文件中的标签问题
"""

import os
import re

def fix_ir_file(file_path):
    """综合修复单个IR文件"""
    print(f"修复 {os.path.basename(file_path)}...")
    
    # 读取原始文件内容
    with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
        content = f.read()
    
    # 1. 移除BOM字符
    if content.startswith('\xef\xbb\xbf'):
        content = content[3:]
    
    # 2. 修复标签缩进（左对齐）
    lines = content.split('\n')
    fixed_lines = []
    
    for line in lines:
        stripped = line.strip()
        if stripped.endswith(':') and not stripped.startswith('%'):
            # 标签左对齐
            fixed_lines.append(stripped)
        else:
            fixed_lines.append(line)
    
    content = '\n'.join(fixed_lines)
    
    # 3. 分割文件为函数
    functions = []
    current_function = []
    in_function = False
    
    for line in content.split('\n'):
        if 'define ' in line:
            if current_function:
                functions.append('\n'.join(current_function) + '\n')
            current_function = [line]
            in_function = True
        elif in_function and line.strip() == '}':
            current_function.append(line)
            functions.append('\n'.join(current_function) + '\n')
            current_function = []
            in_function = False
        elif in_function:
            current_function.append(line)
    
    # 4. 处理每个函数
    fixed_functions = []
    
    for function in functions:
        func_lines = function.split('\n')
        
        # 收集标签和引用
        labels = set()
        references = set()
        
        for line in func_lines:
            stripped = line.strip()
            # 收集标签
            if stripped.endswith(':'):
                label = stripped[:-1]
                labels.add(label)
            # 收集引用
            elif 'br i1 ' in stripped:
                # 提取标签引用
                parts = stripped.split('label %')
                for part in parts[1:]:
                    if ',' in part:
                        label = part.split(',')[0].strip()
                    else:
                        label = part.strip()
                    references.add(label)
        
        # 找出缺失的标签
        missing_labels = references - labels
        
        # 修复函数
        fixed_func_lines = []
        i = 0
        
        while i < len(func_lines):
            line = func_lines[i]
            stripped = line.strip()
            
            # 检查是否是分支指令
            if 'br i1 ' in stripped:
                fixed_func_lines.append(line)
                
                # 提取引用的标签
                parts = stripped.split('label %')
                for part in parts[1:]:
                    if ',' in part:
                        label = part.split(',')[0].strip()
                    else:
                        label = part.strip()
                    
                    # 检查标签是否缺失
                    if label in missing_labels:
                        # 在分支指令后添加缺失的标签
                        fixed_func_lines.append(label + ':')
                        labels.add(label)
                        missing_labels.remove(label)
            
            # 检查是否是ret指令
            elif 'ret ' in stripped:
                fixed_func_lines.append(line)
                # 跳过后面的不可达代码
                i += 1
                while i < len(func_lines):
                    next_stripped = func_lines[i].strip()
                    if next_stripped.endswith(':') or next_stripped == '}':
                        break
                    i += 1
                continue
            
            # 检查是否是重复标签
            elif stripped.endswith(':'):
                label = stripped[:-1]
                if label not in labels:
                    fixed_func_lines.append(line)
                    labels.add(label)
            
            else:
                fixed_func_lines.append(line)
            
            i += 1
        
        # 添加剩余的缺失标签（如果有）
        for label in missing_labels:
            fixed_func_lines.insert(-1, label + ':')
        
        fixed_functions.append('\n'.join(fixed_func_lines))
    
    # 5. 重新组合文件内容
    fixed_content = '\n'.join(fixed_functions)
    
    # 6. 写回文件
    with open(file_path, 'w', encoding='utf-8') as f:
        f.write(fixed_content)
    
    print(f"修复 {os.path.basename(file_path)} 成功")

def main():
    """主函数"""
    # 修复所有IR文件
    ir_files = [
        'target/l2_compiler/lexer.xy.ll',
        'target/l2_compiler/parser.xy.ll',
        'target/l2_compiler/sema.xy.ll',
        'target/l2_compiler/codegen.xy.ll',
        'target/l2_compiler/utils.xy.ll',
        'target/l2_compiler/main.xy.ll'
    ]
    
    for file_path in ir_files:
        if os.path.exists(file_path):
            fix_ir_file(file_path)
        else:
            print(f"文件不存在: {file_path}")

if __name__ == '__main__':
    main()