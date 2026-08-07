/**
 * @file codegen.rs
 * @brief 代码生成模块
 * @description 负责将AST转换为LLVM IR
 */

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::ast::*;
use crate::error::CodegenError;

/// 全局标志：运行时函数声明是否已经生成（跨多个 CodeGenerator 实例共享）
static RUNTIME_DECLS_EMITTED: AtomicBool = AtomicBool::new(false);

/**
 * 代码生成器
 */
pub struct CodeGenerator {
    /// 生成的IR代码
    ir: String,
    /// 变量映射（变量名 -> SSA值）
    variables: HashMap<String, String>,
    /// 变量类型映射
    variable_types: HashMap<String, String>,
    /// 标签计数器
    label_counter: usize,
    /// 字符串常量计数器（全局唯一）
    string_const_counter: usize,
    /// 字符串常量定义（需要在函数外部定义）
    string_constants: Vec<String>,
    /// 外部函数签名（函数名 -> (参数类型列表, 返回类型))
    extern_functions: HashMap<String, (Vec<String>, String)>,
    /// 用户定义函数签名（函数名 -> (参数类型列表, 返回类型))
    user_functions: HashMap<String, (Vec<String>, String)>,
    /// 当前函数名（用于生成唯一的变量名）
    current_function_name: String,
    /// 当前函数的返回类型（用于正确生成返回语句）
    current_function_return_type: String,
    /// 当前函数是否使用聚合返回 ABI（如结构体通过 sret）
    current_function_return_is_aggregate: bool,
    /// 循环 break/continue 标签栈（break_label, continue_label）
    loop_label_stack: Vec<(usize, usize)>,
    /// 结构体字段偏移映射（结构体名 -> [(字段名, 偏移量, LLVM类型)]）
    struct_field_layouts: HashMap<String, Vec<(String, i32, String)>>,
    /// 结构体中声明为列表类型的字段名集合（用于索引访问区分列表/字符串）
    list_typed_fields: HashSet<String>,
    /// 枚举值映射（枚举成员名 -> 整数值）
    enum_values: HashMap<String, i64>,
    /// 枚举类型集合（记录哪些类型是枚举）
    enum_types: HashSet<String>,
    /// 已生成函数签名集合（用于防止重复生成）
    generated_functions: HashSet<String>,
    /// 函数名映射（原始函数名 + 参数类型 -> 带参数类型的函数名）
    func_name_mapping: HashMap<String, String>,
    /// 当前模块名（用于生成唯一的函数名，避免多模块链接时符号冲突）
    module_name: String,
    /// 延迟生成的 lambda 函数定义（在模块级别生成，不能嵌套在函数内）
    lambda_defs: Vec<String>,
}

impl CodeGenerator {
    /**
     * 创建新的代码生成器
     */
    pub fn new() -> Self {
        Self {
            ir: String::new(),
            variables: HashMap::new(),
            variable_types: HashMap::new(),
            label_counter: 0,
            string_const_counter: 0,
            string_constants: Vec::new(),
            extern_functions: HashMap::new(),
            user_functions: HashMap::new(),
            current_function_name: String::new(),
            current_function_return_type: String::new(),
            current_function_return_is_aggregate: false,
            loop_label_stack: Vec::new(),
            struct_field_layouts: HashMap::new(),
            list_typed_fields: HashSet::new(),
            enum_values: HashMap::new(),
            enum_types: HashSet::new(),
            generated_functions: HashSet::new(),
            func_name_mapping: HashMap::new(),
            module_name: String::new(),
            lambda_defs: Vec::new(),
        }
    }

    pub fn with_module_name(module_name: &str) -> Self {
        let mut cg = Self::new();
        cg.module_name = module_name.to_string();
        cg
    }

    /**
     * 将字符串转换为 LLVM IR 安全格式
     * 非 ASCII 字符使用十六进制转义序列 \XX
     * 这样可以避免编码问题，确保在不同编码环境下都能正确编译
     */
    fn escape_string_for_llvm(&self, s: &str) -> String {
        let mut result = String::new();
        for byte in s.bytes() {
            match byte {
                b'\\' => result.push_str("\\\\"),  // 反斜杠
                b'"' => result.push_str("\\22"),    // 双引号（LLVM 十六进制转义）
                b'\n' => result.push_str("\\0A"),   // 换行符
                b'\r' => result.push_str("\\0D"),  // 回车符
                b'\t' => result.push_str("\\09"),  // 制表符
                0x20..=0x7E => result.push(byte as char),  // 可打印 ASCII 字符
                _ => result.push_str(&format!("\\{:02X}", byte)),  // 非 ASCII 字符使用十六进制
            }
        }
        result
    }

    /**
     * 生成IR代码
     */
    pub fn generate(&mut self, module: &Module) -> Result<String, CodegenError> {
        // 重置状态
        self.ir.clear();
        self.variables.clear();
        self.variable_types.clear();
        self.label_counter = 0;
        self.string_constants.clear();
        self.extern_functions.clear();
        self.user_functions.clear();
        self.current_function_name.clear();
        self.current_function_return_type.clear();
        self.current_function_return_is_aggregate = false;
        self.loop_label_stack.clear();
        self.struct_field_layouts.clear();
        self.generated_functions.clear();
        self.enum_values.clear();

        // 注册结构体字段布局（在生成函数之前）
        for struct_def in &module.structs {
            self.register_struct_layout(struct_def);
        }

        // 生成 LLVM 结构体类型定义
        for struct_def in &module.structs {
            self.emit_struct_type_definition(struct_def);
        }

        // 注册枚举值（在生成函数之前）
        for enum_def in &module.enums {
            self.register_enum_values(enum_def);
        }

        // 注册常量值
        for const_def in &module.constants {
            self.register_constant(const_def);
        }

        // 生成用户定义的外部函数声明（先处理，以便运行时声明可以跳过已有的）
        // 收集已定义函数的原始名称（用于跳过外部声明）
        let defined_orig_names: Vec<&str> = module.functions.iter()
            .map(|f| f.name.as_str())
            .collect();
        for extern_func in &module.extern_functions {
            // 如果同名函数已被定义（在合并模块中），跳过外部声明
            if defined_orig_names.contains(&extern_func.name.as_str()) {
                continue;
            }
            self.generate_extern_function(extern_func)?;
        }

        // 生成运行时库函数声明（跳过已在外部函数声明中定义的）
        self.emit_runtime_declarations();

        // 预先收集用户函数签名（用于类型推断）
        // 同时预先生成函数名映射，确保函数调用时能找到正确的带参数类型的函数名
        for func in &module.functions {
            let base_func_name = self.translate_def_name(&func.name);
            let return_type = self.translate_type(&func.return_type);
            let param_types: Vec<String> = func.params
                .iter()
                .map(|param| self.translate_type(&param.param_type))
                .collect();
            
            // 预先生成带参数类型的函数名和映射
            let sanitized_types: Vec<String> = param_types
                .iter()
                .map(|t| {
                    let mut simplified = t.clone();
                    if simplified.starts_with("%struct.") {
                        simplified = simplified.trim_start_matches("%struct.").to_string();
                    }
                    simplified.replace("*", "ptr").replace("%", "struct_").replace(".", "_")
                })
                .collect();
            let param_suffix = sanitized_types.join("_");
            // 无参数时不添加下划线后缀，保持与 generate_function 一致
            let mut func_name_with_types = if param_suffix.is_empty() {
                base_func_name.clone()
            } else {
                format!("{}_{}", base_func_name, param_suffix)
            };
            
            // 添加模块名前缀（与 generate_function 保持一致）
            if !self.module_name.is_empty() && !func_name_with_types.starts_with("xy_main") 
                && !CodeGenerator::is_builtin_func(&func_name_with_types)
            {
                func_name_with_types = format!("{}_module_{}", self.module_name, func_name_with_types);
            }
            
            // 存储带参数类型的函数签名（用于类型查找）
            self.user_functions.insert(func_name_with_types.clone(), (param_types.clone(), return_type.clone()));
            
            // 预先生成函数名映射
            let mapping_key = format!("{}({})", base_func_name, param_types.join(","));
            self.func_name_mapping.insert(mapping_key, func_name_with_types);
        }

        // 生成函数定义
        let mut has_xy_main = false;
        for func in &module.functions {
            self.generate_function(func)?;
            if func.name == "主" || func.name == "主函数" || func.name == "main" {
                has_xy_main = true;
            }
        }

        // 插入延迟的 Lambda 函数定义（必须在模块级别，不能嵌套在其他函数内）
        if !self.lambda_defs.is_empty() {
            self.emit("\n; === Lambda 函数定义 ===\n");
            let lambda_defs = std::mem::take(&mut self.lambda_defs);
            for lambda_def in &lambda_defs {
                self.emit(lambda_def);
            }
            self.emit("; === Lambda 定义结束 ===\n");
        }

        // 如果存在 XY 主函数，生成 C 兼容的 main 包装器
        if has_xy_main {
            self.emit("\ndefine i32 @main(i32 %argc, i8** %argv) {");
            self.emit("    call void @init_args(i32 %argc, i8** %argv)");

            // 根据主函数的参数数量生成调用
            let main_func = module.functions.iter()
                .find(|f| f.name == "主" || f.name == "主函数" || f.name == "main")
                .unwrap();
            
            let call_args = if main_func.params.is_empty() {
                "".to_string()
            } else {
                "i64 %argc_ext, i8* %argv_i8".to_string()
            };
            
            if !main_func.params.is_empty() {
                self.emit("    %argc_ext = sext i32 %argc to i64");
                self.emit("    %argv_i8 = bitcast i8** %argv to i8*");
            }
            
            let main_func_name = if main_func.params.is_empty() {
                "xy_main".to_string()
            } else {
                let param_types: Vec<String> = main_func.params
                    .iter()
                    .map(|param| self.translate_type(&param.param_type))
                    .collect();
                let sanitized_types: Vec<String> = param_types
                    .iter()
                    .map(|t| t.replace("*", "ptr").replace("%", "struct_"))
                    .collect();
                format!("xy_main_{}", sanitized_types.join("_"))
            };
            self.emit(&format!("    %result = call i64 @{}({})", main_func_name, call_args));
            self.emit("    %result_i32 = trunc i64 %result to i32");
            self.emit("    ret i32 %result_i32");
            self.emit("}\n");
        }

        // 在所有函数定义之后添加字符串常量定义
        for constant in &self.string_constants {
            self.ir.push_str(constant);
            self.ir.push('\n');
        }

        // 后处理：修复所有空基本块（标签后没有指令的）
        self.fix_empty_blocks();

        // 后处理：转换为 opaque pointer 格式（LLVM 15+ 要求）
        self.convert_to_opaque_pointers();

        Ok(self.ir.clone())
    }

    /// 将 typed-pointer LLVM IR 转换为 opaque-pointer 格式
    /// LLVM 15+ 使用 opaque pointers，所有指针类型统一为 `ptr`
    fn convert_to_opaque_pointers(&mut self) {
        // 逐行处理
        let lines: Vec<String> = self.ir.lines().map(|s| s.to_string()).collect();
        let mut result: Vec<String> = Vec::new();

        for line in &lines {
            let trimmed = line.trim();
            let indent_len = line.len() - trimmed.len();
            let indent = " ".repeat(indent_len);

            // 只处理指令行，跳过标签、注释、常量定义
            if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('@')
                || (trimmed.starts_with('L') && trimmed.ends_with(':')) {
                result.push(line.clone());
                continue;
            }

            let mut new_line = trimmed.to_string();

            // Rule 1: store <ty> <val>, <ty>* <ptr> → store <ty> <val>, ptr <ptr>
            if trimmed.starts_with("store ") {
                // 先处理值类型：如果值类型是指针（如 %struct.X*），转为 ptr
                let parts: Vec<&str> = new_line.split_whitespace().collect();
                if parts.len() >= 4 && parts[1].ends_with('*') && parts[1] != "ptr" {
                    let mut new_parts: Vec<String> = parts.iter().map(|s| s.to_string()).collect();
                    new_parts[1] = "ptr".to_string();
                    new_line = new_parts.join(" ");
                }
                // 再处理指针类型（逗号后）
                if let Some(comma_pos) = new_line.find(',') {
                    let before_comma = &new_line[..comma_pos];
                    let after_comma = &new_line[comma_pos+1..];
                    let after_parts: Vec<&str> = after_comma.split_whitespace().collect();
                    if after_parts.len() >= 2 && after_parts[0].ends_with('*') {
                        // 替换 typed pointer 为 ptr
                        let remaining: Vec<&str> = after_parts[1..].to_vec();
                        if remaining.len() == 1 {
                            new_line = format!("{}, ptr {}", before_comma, remaining[0]);
                        } else {
                            new_line = format!("{}, ptr {}", before_comma, remaining.join(" "));
                        }
                    }
                }
            }
            // Rule 2: load <ty>, <ty>* <ptr> → load <ty>, ptr <ptr>
            else if trimmed.starts_with("load ") {
                if let Some(comma_pos) = new_line.find(',') {
                    let before_comma = &new_line[..comma_pos];
                    let after_comma = &new_line[comma_pos+1..];
                    let after_parts: Vec<&str> = after_comma.split_whitespace().collect();
                    if after_parts.len() >= 2 && after_parts[0].ends_with('*') {
                        let remaining: Vec<&str> = after_parts[1..].to_vec();
                        if remaining.len() == 1 {
                            new_line = format!("{}, ptr {}", before_comma, remaining[0]);
                        } else {
                            new_line = format!("{}, ptr {}", before_comma, remaining.join(" "));
                        }
                    }
                }
            }
            // Rule 3: getelementptr <ty>, <ty>* <ptr> → getelementptr <ty>, ptr <ptr>
            else if trimmed.starts_with("getelementptr ") {
                // 找第一个逗号
                if let Some(comma_pos) = new_line.find(',') {
                    let before_comma = &new_line[..comma_pos];
                    let after_comma = &new_line[comma_pos+1..];
                    let after_parts: Vec<&str> = after_comma.split_whitespace().collect();
                    if after_parts.len() >= 2 && after_parts[0].ends_with('*') {
                        let remaining: Vec<&str> = after_parts[1..].to_vec();
                        if remaining.len() == 1 {
                            new_line = format!("{}, ptr {}", before_comma, remaining[0]);
                        } else {
                            new_line = format!("{}, ptr {}", before_comma, remaining.join(" "));
                        }
                    }
                }
            }
            // Rule 4: bitcast <ty>* %val to <ty2>* → ptr 版本
            else if trimmed.starts_with("bitcast ") && trimmed.contains(" to ") {
                // bitcast ptr %val to ptr — just simplify / keep as is
                let parts: Vec<&str> = new_line.split_whitespace().collect();
                if parts.len() >= 5 {
                    let src_ty = parts[1];
                    let dst_ty = parts[parts.len()-1];
                    // 如果源和目标是 typed pointers，转换为 ptr
                    if src_ty.ends_with('*') && dst_ty.ends_with('*') {
                        let result_reg = parts[0]; // %result =
                        let src_val = parts[2];     // %source
                        new_line = format!("{} = bitcast ptr {} to ptr", result_reg, src_val);
                    }
                }
            }
            // Rule 5: ptrtoint / inttoptr 中的 typed pointers → ptr
            // ptrtoint i8* %val to i64 → ptrtoint ptr %val to i64
            // inttoptr i64 %val to i8* → inttoptr i64 %val to ptr
            else if trimmed.starts_with("ptrtoint ") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 5 {
                    // ptrtoint <ty>* <val> to <to_ty>
                    new_line = format!("{} = ptrtoint ptr {} to {}", parts[0], parts[2], parts[4]);
                }
            } else if trimmed.starts_with("inttoptr ") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 5 {
                    // inttoptr <from_ty> <val> to <ty>*
                    new_line = format!("{} = inttoptr {} {} to ptr", parts[0], parts[1], parts[2]);
                }
            }
            // Rule 6: 函数签名中的 typed pointers → ptr
            // define/call/declare 中的 i8* %x, %struct.X* %x, i8** %x
            else if trimmed.starts_with("define ") || trimmed.starts_with("declare ")
                || trimmed.starts_with("call ") {
                // i8* % → ptr %
                while new_line.contains("i8* %") {
                    new_line = new_line.replace("i8* %", "ptr %");
                }
                while new_line.contains("i8** %") {
                    new_line = new_line.replace("i8** %", "ptr %");
                }
                // double* %, i32* %, i64* %
                for ty in &["double", "i32", "i64", "i1", "void"] {
                    let pat = format!("{}* %", ty);
                    while new_line.contains(&pat) {
                        new_line = new_line.replace(&pat, "ptr %");
                    }
                }
                // %struct.X*, %fnX*, 及其他自定义类型指针 → ptr %
                // 使用简单的启发式：%anything* % 替换为 ptr %
                // 但要保留 sret(%struct.X) 中的类型
                if let Some(ref paren_open) = new_line.find('(') {
                    let before_paren = &new_line[..*paren_open];
                    let after_paren = &new_line[*paren_open..];
                    // 只在括号之前的部分中替换（函数名、返回类型）
                    let fixed_before = before_paren
                        .replace("i8**", "ptr")
                        .replace("i8*", "ptr")
                        .replace("i32*", "ptr");
                    new_line = format!("{}{}", fixed_before, after_paren);
                }
            }
            // Rule 6: %struct.X** 对齐处理（很少见，但对齐格式需要特殊对待）
            // "align 8" 后缀保留不变

            result.push(format!("{}{}", indent, new_line));
        }

        self.ir = result.join("\n") + "\n";
    }

    /// 修复生成IR中的空基本块和无终止符的基本块
    fn fix_empty_blocks(&mut self) {
        let lines: Vec<String> = self.ir.lines().map(|s| s.to_string()).collect();
        let mut result: Vec<String> = Vec::new();
        let mut needs_terminator = false;
        let mut current_func_return_type: Option<String> = None;
        let mut inside_function = false;
        let mut i = 0;

        let is_label = |s: &str| -> bool {
            let t = s.trim();
            t.starts_with('L') && t.ends_with(':')
                && t[1..t.len()-1].chars().all(|c| c.is_ascii_digit())
        };

        let is_terminator = |s: &str| -> bool {
            let t = s.trim();
            t.starts_with("ret ") || t == "ret" || t.starts_with("br ") || t == "unreachable"
        };

        while i < lines.len() {
            let line = &lines[i];
            let trimmed = line.trim();

            // 检测函数定义开始，提取返回类型
            if trimmed.starts_with("define ") {
                // 解析函数签名: define <return_type> @<func_name>(...)
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    current_func_return_type = Some(parts[1].to_string());
                }
                if inside_function && needs_terminator {
                    result.push("    unreachable".to_string());
                    _ = needs_terminator;
                }
                inside_function = true;
                result.push(line.clone());
                i += 1;
                continue;
            }

            // 检测函数定义结束，清除当前函数信息
            if trimmed == "}" && current_func_return_type.is_some() {
                if needs_terminator {
                    // 对于void函数，添加ret void；对于其他函数，添加unreachable
                    if current_func_return_type.as_ref().unwrap() == "void" {
                        result.push("    ret void".to_string());
                    } else {
                        result.push("    unreachable".to_string());
                    }
                    _ = needs_terminator;
                }
                result.push(line.clone());
                current_func_return_type = None;
                inside_function = false;
                i += 1;
                continue;
            }

            if is_label(trimmed) {
                // 只在函数内部处理终止符需求
                if inside_function && needs_terminator {
                    let prev_label = trimmed.trim_end_matches(':');
                    result.push(format!("    br label %{}", prev_label));
                    _ = needs_terminator;
                }
                result.push(line.clone());
                i += 1;
                while i < lines.len() && lines[i].trim().is_empty() {
                    result.push(lines[i].clone());
                    i += 1;
                }
                if inside_function {
                    if i >= lines.len() {
                        result.push("    unreachable".to_string());
                    } else {
                        let next = lines[i].trim();
                        if is_label(next) || next == "}" {
                            if next == "}" && current_func_return_type.as_ref().map(|s| s.as_str()).unwrap_or("") == "void" {
                                result.push("    ret void".to_string());
                            } else {
                                result.push("    unreachable".to_string());
                            }
                        }
                    }
                }
                needs_terminator = false;
            } else if trimmed == "}" || trimmed.starts_with("declare ") {
                if inside_function && needs_terminator {
                    result.push("    unreachable".to_string());
                    _ = needs_terminator;
                }
                result.push(line.clone());
                i += 1;
            } else {
                if is_terminator(trimmed) {
                    _ = needs_terminator;
                } else if inside_function && !trimmed.is_empty() && !trimmed.starts_with(';') && !trimmed.starts_with('@') && !trimmed.starts_with("declare") && !trimmed.starts_with("define") && !trimmed.starts_with("entry:") {
                    needs_terminator = true;
                }
                result.push(line.clone());
                i += 1;
            }
        }
        if inside_function && needs_terminator {
            result.push("    unreachable".to_string());
        }
        self.ir = result.join("\n") + "\n";
    }

    /**
     * 生成运行时库函数声明
     */
    fn emit_runtime_declarations(&mut self) {
        // 避免重复生成运行时声明（多模块编译时每个模块都会调用 generate_ir）
        if RUNTIME_DECLS_EMITTED.swap(true, Ordering::SeqCst) {
            return;
        }

        // 辅助函数：只在未声明时发出声明
        let emit_if_new = |ir: &mut String, decl: &str, extern_funcs: &HashMap<String, (Vec<String>, String)>| {
            // 从声明中提取函数名
            if let Some(name_start) = decl.find('@') {
                let after_at = &decl[name_start + 1..];
                let func_name = if let Some(paren) = after_at.find('(') {
                    &after_at[..paren]
                } else {
                    after_at
                };
                if !extern_funcs.contains_key(func_name) {
                    ir.push_str(decl);
                    ir.push('\n');
                }
            } else {
                ir.push_str(decl);
                ir.push('\n');
            }
        };

        let extern_funcs = &self.extern_functions;

        // 内存管理
        emit_if_new(&mut self.ir, "declare i8* @rt_malloc(i64)", extern_funcs);
        emit_if_new(&mut self.ir, "declare void @rt_free(i8*)", extern_funcs);

        // 字符串处理
        emit_if_new(&mut self.ir, "declare i8* @rt_str_new(i8*)", extern_funcs);
        emit_if_new(&mut self.ir, "declare i8* @rt_str_concat(i8*, i8*)", extern_funcs);
        emit_if_new(&mut self.ir, "declare i64 @rt_str_len(i8*)", extern_funcs);

        // 列表操作
        emit_if_new(&mut self.ir, "declare i8* @rt_list_new()", extern_funcs);
        emit_if_new(&mut self.ir, "declare void @rt_list_append(i8*, i8*)", extern_funcs);
        emit_if_new(&mut self.ir, "declare i64 @rt_list_len(i8*)", extern_funcs);
        emit_if_new(&mut self.ir, "declare i8* @rt_list_get(i8*, i64)", extern_funcs);

        // 打印函数
        emit_if_new(&mut self.ir, "declare void @rt_print(i8*)", extern_funcs);
        emit_if_new(&mut self.ir, "declare void @rt_println(i8*)", extern_funcs);
        emit_if_new(&mut self.ir, "declare void @print_int(i64)", extern_funcs);
        emit_if_new(&mut self.ir, "declare void @print_float(double)", extern_funcs);
        emit_if_new(&mut self.ir, "declare void @print(i8*)", extern_funcs);

        // 类型转换函数
        emit_if_new(&mut self.ir, "declare i8* @rt_int_to_str(i64)", extern_funcs);
        emit_if_new(&mut self.ir, "declare i64 @rt_str_to_int(i8*)", extern_funcs);
        emit_if_new(&mut self.ir, "declare i8* @rt_float_to_str(double)", extern_funcs);
        emit_if_new(&mut self.ir, "declare double @rt_str_to_double(i8*)", extern_funcs);

        // 错误处理
        emit_if_new(&mut self.ir, "declare void @rt_error(i8*)", extern_funcs);

        // 参数初始化
        emit_if_new(&mut self.ir, "declare void @init_args(i32, i8**)", extern_funcs);

        // 列表设置
        emit_if_new(&mut self.ir, "declare void @rt_list_set(i8*, i64, i8*)", extern_funcs);

        // 字符串子串和切片
        emit_if_new(&mut self.ir, "declare i8* @rt_string_substring(i8*, i64, i64)", extern_funcs);
        emit_if_new(&mut self.ir, "declare i8* @rt_string_slice(i8*, i64, i64)", extern_funcs);
        emit_if_new(&mut self.ir, "declare i64 @rt_string_len(i8*)", extern_funcs);
        emit_if_new(&mut self.ir, "declare i64 @rt_string_indexOf(i8*, i8*)", extern_funcs);

        // 读取行
        emit_if_new(&mut self.ir, "declare i8* @rt_readline()", extern_funcs);

        // 字符串比较函数
        emit_if_new(&mut self.ir, "declare i64 @rt_str_eq(i8*, i8*)", extern_funcs);
        emit_if_new(&mut self.ir, "declare i64 @rt_str_ne(i8*, i8*)", extern_funcs);
        emit_if_new(&mut self.ir, "declare i64 @rt_str_lt(i8*, i8*)", extern_funcs);
        emit_if_new(&mut self.ir, "declare i64 @rt_str_le(i8*, i8*)", extern_funcs);
        emit_if_new(&mut self.ir, "declare i64 @rt_str_gt(i8*, i8*)", extern_funcs);
        emit_if_new(&mut self.ir, "declare i64 @rt_str_ge(i8*, i8*)", extern_funcs);

        // 字符串字符访问和编码
        emit_if_new(&mut self.ir, "declare i8* @rt_string_char_at(i8*, i64)", extern_funcs);
        emit_if_new(&mut self.ir, "declare i64 @rt_char_to_code(i8*)", extern_funcs);
        emit_if_new(&mut self.ir, "declare i8* @rt_code_to_char(i64)", extern_funcs);
        // 整数转单字符字符串
        emit_if_new(&mut self.ir, "declare i8* @rt_string_fromChar(i64)", extern_funcs);

        // 字符串包含
        emit_if_new(&mut self.ir, "declare i8* @str_contains(i8*, i8*)", extern_funcs);

        // 整数转字符串（别名）
        emit_if_new(&mut self.ir, "declare i64 @str_to_int(i8*)", extern_funcs);

        // 文件操作
        emit_if_new(&mut self.ir, "declare i8* @file_read(i8*)", extern_funcs);
        emit_if_new(&mut self.ir, "declare i32 @file_write(i8*, i8*)", extern_funcs);
        emit_if_new(&mut self.ir, "declare i32 @file_exists(i8*)", extern_funcs);

        // 命令执行
        emit_if_new(&mut self.ir, "declare i32 @exec_cmd(i8*)", extern_funcs);

        // 命令行参数
        emit_if_new(&mut self.ir, "declare i8* @argv(i64)", extern_funcs);
        emit_if_new(&mut self.ir, "declare i64 @argc()", extern_funcs);

        // UTF-8 helper functions
        emit_if_new(&mut self.ir, "declare i64 @rt_utf8_byte_length(i64)", extern_funcs);
        emit_if_new(&mut self.ir, "declare i64 @rt_is_utf8_leader(i64)", extern_funcs);
        emit_if_new(&mut self.ir, "declare i64 @rt_is_utf8_continuation(i64)", extern_funcs);
    }

    /**
     * 注册结构体字段布局
     */
    fn emit_struct_type_definition(&mut self, struct_def: &StructDefinition) {
        let struct_name = self.translate_struct_name(&struct_def.name);
        let mut field_types = Vec::new();
        
        for field in &struct_def.fields {
            let llvm_type = match &field.field_type {
                Type::Int | Type::Long | Type::Bool | Type::Char => "i64".to_string(),
                Type::Float | Type::Double => "double".to_string(),
                Type::String | Type::Pointer | Type::Any => "i8*".to_string(),
                Type::List(_) | Type::Array(_) | Type::Optional(_) | Type::Future(_) => "i8*".to_string(),
                Type::Function(_, _) => "i8*".to_string(),
                Type::Struct(s_name) | Type::Custom(s_name) => {
                    let translated_s_name = self.translate_struct_name(s_name);
                    format!("%struct.{}", translated_s_name)
                },
                Type::Void | Type::Unknown | Type::TypeVar(_) => "i64".to_string(),
            };
            field_types.push(llvm_type);
        }
        
        let fields_str = field_types.join(", ");
        self.emit(&format!("%struct.{} = type {{ {} }}", struct_name, fields_str));
    }

    fn register_struct_layout(&mut self, struct_def: &StructDefinition) {
        let struct_name = self.translate_struct_name(&struct_def.name);
        let mut fields = Vec::new();
        let mut offset = 0i32;

        for field in &struct_def.fields {
            let (llvm_type, field_size) = match &field.field_type {
                Type::Int | Type::Long | Type::Bool | Type::Char => ("i64".to_string(), 8),
                Type::Float | Type::Double => ("double".to_string(), 8),
                Type::String | Type::Pointer | Type::Any => ("i8*".to_string(), 8),
                Type::List(_) | Type::Array(_) | Type::Optional(_) | Type::Future(_) => ("i8*".to_string(), 8),
                Type::Function(_, _) => ("i8*".to_string(), 8),
                Type::Struct(s_name) | Type::Custom(s_name) => {
                    let translated_s_name = self.translate_struct_name(s_name);
                    let size = self.compute_struct_size(&translated_s_name);
                    (format!("%struct.{}", translated_s_name), size)
                },
                Type::Void | Type::Unknown | Type::TypeVar(_) => ("i64".to_string(), 8),
            };
            if matches!(field.field_type, Type::List(_) | Type::Array(_)) {
                self.list_typed_fields.insert(field.name.clone());
            }
            fields.push((field.name.clone(), offset, llvm_type));
            offset += field_size;
        }
        self.struct_field_layouts.insert(struct_name.clone(), fields);
        
        if struct_def.name == "Lexer" {
            eprintln!("DEBUG: Lexer struct layout:");
            for (name, offset, llvm_type) in &self.struct_field_layouts[&struct_name] {
                eprintln!("DEBUG:   {}: offset={}, type={}", name, offset, llvm_type);
            }
        }
    }

    /// 计算结构体的总大小（字节数），用于正确计算嵌套结构体偏移
    fn compute_struct_size(&self, struct_name: &str) -> i32 {
        if let Some(fields) = self.struct_field_layouts.get(struct_name) {
            if let Some((_, last_offset, _)) = fields.last() {
                return last_offset + 8;
            }
        }
        // 未注册的结构体：根据名称猜测大小
        // 支持原始名称和翻译后的名称
        if struct_name.starts_with("AST") || struct_name.contains("节点") {
            return 56;
        } else if struct_name.starts_with("Lexer") {
            return 120;
        } else if struct_name.starts_with("Parser") {
            return 80;
        } else if struct_name.starts_with("Sema") {
            return 120;
        } else if struct_name.starts_with("Codegen") {
            return 160;
        } else if struct_name.starts_with("Token") {
            return 48;
        } else if struct_name.contains("符号") || struct_name.starts_with("Symbol") {
            return 48;
        } else if struct_name.contains("作用域") || struct_name.starts_with("Scope") {
            return 32;
        } else if struct_name.contains("循环状态") || struct_name.starts_with("LoopState") {
            return 16;
        } else if struct_name.contains("循环上下文") || struct_name.starts_with("LoopContext") {
            return 32;
        } else if struct_name.starts_with("ErrorRecovery") {
            return 24;
        } else {
            eprintln!("[codegen] 未知结构体大小: {}, 默认80字节", struct_name);
            80
        }
    }

    /**
     * 注册枚举成员值
     */
    fn register_enum_values(&mut self, enum_def: &EnumDefinition) {
        self.enum_types.insert(enum_def.name.clone());
        for (i, variant) in enum_def.variants.iter().enumerate() {
            self.enum_values.insert(variant.name.clone(), i as i64);
        }
    }

    /**
     * 注册常量值
     */
    fn register_constant(&mut self, const_def: &ConstantDef) {
        // 评估常量值表达式
        if let Ok(_val) = self.evaluate_const_expr(&const_def.value) {
            // 将常量作为变量注册到当前作用域
            let var_name = const_def.name.clone();
            // 对于常量，我们直接记录其值用于内联替换
            let const_type = self.translate_type(&const_def.const_type);
            self.variable_types.insert(var_name, const_type);
        }
    }

    /**
     * 评估常量表达式（简单实现）
     */
    fn evaluate_const_expr(&self, expr: &Expr) -> Result<i64, CodegenError> {
        match expr {
            Expr::Literal(lit) => {
                match &lit.kind {
                    LiteralKind::Integer(v) => Ok(*v),
                    LiteralKind::Boolean(v) => Ok(if *v { 1 } else { 0 }),
                    _ => Err(CodegenError::new("不支持的常量表达式类型")),
                }
            }
            _ => Err(CodegenError::new("不支持的常量表达式")),
        }
    }

    /**
     * 生成用户定义的外部函数声明
     * 外部 函数 函数名(参数列表) -> 返回类型
     */
    fn generate_extern_function(&mut self, extern_func: &ExternFunction) -> Result<(), CodegenError> {
        // 保存当前函数名，并设置为特殊作用域（外部函数使用固定作用域，避免哈希冲突）
        let saved_function_name = self.current_function_name.clone();
        self.current_function_name = "__extern__".to_string();

        // 翻译函数名（处理中文函数名）
        let func_name = self.translate_func_name(&extern_func.name);

        // 恢复当前函数名
        self.current_function_name = saved_function_name;
        
        // 如果有链接名，使用链接名
        let final_name = extern_func.link_name.as_ref().unwrap_or(&func_name);
        
        // 翻译返回类型
        let return_type = self.translate_type(&extern_func.return_type);
        
        // 翻译参数类型
        let param_types: Vec<String> = extern_func.params
            .iter()
            .map(|param| self.translate_type(&param.param_type))
            .collect();
        
        let params_str = param_types.join(", ");
        
        // 生成 declare 语句
        self.emit(&format!("declare {} @{}({})", return_type, final_name, params_str));
        
        // 记录外部函数的签名，用于后续调用时确定参数类型
        self.extern_functions.insert(final_name.clone(), (param_types, return_type.clone()));
        
        Ok(())
    }

    /**
     * 生成函数定义
     */
    fn generate_function(&mut self, func: &Function) -> Result<(), CodegenError> {
        // 重置标签计数器，从 1000 开始
        // LLVM 参数编号为 %0, %1, %2...，从 1000 开始可以确保不会与参数编号冲突
        // 这个方法虽然简单，但可以有效避免标签编号与参数编号的冲突
        self.label_counter = 1000;
        
        // 翻译函数名（处理中文函数名）- 使用定义名以避免与外部声明冲突
        let base_func_name = self.translate_def_name(&func.name);
        
        // 获取参数类型列表
        let param_types: Vec<String> = func.params
            .iter()
            .map(|param| self.translate_type(&param.param_type))
            .collect();
        
        // 生成唯一的函数名（包含参数类型信息以支持函数重载）
        // 将参数类型中的特殊字符替换为合法字符
        // 对于结构体类型，只保留结构体名称部分，去掉 %struct. 前缀
        let sanitized_types: Vec<String> = param_types
            .iter()
            .map(|t| {
                let mut simplified = t.clone();
                if simplified.starts_with("%struct.") {
                    simplified = simplified.trim_start_matches("%struct.").to_string();
                }
                simplified.replace("*", "ptr").replace("%", "struct_").replace(".", "_")
            })
            .collect();
        let param_suffix = sanitized_types.join("_");
        // 无参数时不添加下划线后缀，避免函数名以多余下划线结尾
        let mut func_name = if param_suffix.is_empty() {
            base_func_name.clone()
        } else {
            format!("{}_{}", base_func_name, param_suffix)
        };

        // 如果设置了模块名，添加模块名前缀（主函数和内置函数除外）
        if !self.module_name.is_empty() && !func_name.starts_with("xy_main") && !CodeGenerator::is_builtin_func(&func_name) {
            func_name = format!("{}_module_{}", self.module_name, func_name);
        }
        
        // 检查是否已经生成过相同的函数
        if self.generated_functions.contains(&func_name) {
            return Ok(());
        }
        
        self.generated_functions.insert(func_name.clone());
        
        // 记录函数名映射（原始函数名 + 参数类型 -> 带参数类型的函数名）
        let mapping_key = format!("{}({})", base_func_name, param_types.join(","));
        eprintln!("DEBUG DEF: base_func_name={}, func_name={}, mapping_key={}, module_name={}", 
            base_func_name, func_name, mapping_key, self.module_name);
        self.func_name_mapping.insert(mapping_key, func_name.clone());
        
        // 同时将带参数类型的函数名也存储到 user_functions 中，用于类型查找
        let return_type_str = self.translate_type(&func.return_type);
        self.user_functions.insert(func_name.clone(), (param_types.clone(), return_type_str));
        
        // 设置当前函数名和返回类型，用于生成唯一的变量名和正确的返回语句
        self.current_function_name = func_name.clone();
        let return_info = self.function_return_signature(&func.return_type);
        self.current_function_return_type = return_info.2.clone();
        self.current_function_return_is_aggregate = return_info.1;
        
        // 生成函数签名
        let return_type = return_info.0.clone();
        let mut signature_params = Vec::new();
        if return_info.1 {
            signature_params.push(format!("{}* sret({}) %agg.result", return_info.2, return_info.2));
        }
        // 为每个参数添加明确的名称，避免 LLVM 自动编号导致的冲突
        let param_is_struct: Vec<bool> = param_types.iter()
            .map(|t| t.starts_with("%struct.")).collect();

        for (i, (param, param_type)) in func.params.iter().zip(param_types.iter()).enumerate() {
            let param_name = Self::sanitize_identifier(&param.name);
            if param_type.starts_with("%struct.") {
                // 结构体参数：传指针而非传值，使函数对 struct 的修改对调用者可见
                signature_params.push(format!("{}* %arg_{}_{}", param_type, i, param_name));
            } else {
                signature_params.push(format!("{} %arg_{}_{}", param_type, i, param_name));
            }
        }
        let params_str = signature_params.join(", ");
        self.emit(&format!("define {} @{}({}) {{\n", return_type, func_name, params_str));

        // 处理函数参数
        for (i, param) in func.params.iter().enumerate() {
            let safe_name = Self::sanitize_identifier(&param.name);
            let param_type = self.translate_type(&param.param_type);
            let param_is_ptr = param_is_struct[i];

            if param_is_ptr {
                // 结构体参数以指针传入，直接使用该指针作为变量地址
                // 无需 alloca + store，指针本身就是变量的"地址"
                let orig_name = param.name.clone();
                let ptr_name = format!("arg_{}_{}", i, safe_name);
                self.variables.insert(orig_name.clone(), ptr_name);
                self.variable_types.insert(orig_name, param_type);
            } else {
                let alloca = self.new_label(&safe_name);
                let llvm_param_name = format!("arg_{}_{}", i, safe_name);

                self.emit(&format!("    %{} = alloca {}, align 8", alloca, param_type));
                self.emit(&format!("    store {} %{}, {}* %{}", param_type, llvm_param_name, param_type, alloca));

                let orig_name = param.name.clone();
                self.variables.insert(orig_name.clone(), alloca);
                self.variable_types.insert(orig_name, param_type);
            }
        }
        
        // 记录生成函数体前的 IR 长度
        let _ir_len_before_body = self.ir.len();
        
        // 生成函数体
        self.generate_block(&func.body)?;

        // 检查函数体是否已经有返回语句，避免重复生成
        let has_return = self.ir.contains("ret ");
        if !has_return {
            if self.current_function_return_is_aggregate {
                self.emit("    ret void");
            } else if return_type != "void" {
                if return_type == "i8*" || return_type == "double" {
                    self.emit(&format!("    ret {} null", return_type));
                } else {
                    self.emit(&format!("    ret {} 0", return_type));
                }
            } else {
                self.emit("    ret void");
            }
        }

        self.emit("}\n");
        Ok(())
    }
    
    #[allow(dead_code)]
    fn generate_function_with_name(&mut self, func: &Function, func_name: &str, param_types: &[String]) -> Result<(), CodegenError> {
        self.current_function_name = func_name.to_string();
        let return_info = self.function_return_signature(&func.return_type);
        self.current_function_return_type = return_info.2.clone();
        self.current_function_return_is_aggregate = return_info.1;
        
        // 重置标签计数器，从 1000 开始
        // 这可以避免与 LLVM 参数编号（%0, %1, %2...）冲突
        self.label_counter = 1000;
        
        let return_type = return_info.0.clone();
        let mut signature_params = Vec::new();
        if return_info.1 {
            signature_params.push(format!("{}* sret({}) %agg.result", return_info.2, return_info.2));
        }
        signature_params.extend(param_types.iter().cloned());
        let params_str = signature_params.join(", ");
        self.emit(&format!("define {} @{}({}) {{\n", return_type, func_name, params_str));
        
        // 如果函数返回聚合类型，sret 参数是第 0 个参数，用户参数从第 1 个开始
        let param_start_index = if return_info.1 { 1 } else { 0 };
        for (i, param) in func.params.iter().enumerate() {
            let param_name = param.name.clone();
            let param_type = self.translate_type(&param.param_type);
            let alloca = self.new_label(&param_name);
            let llvm_param_index = i + param_start_index;
            
            self.emit(&format!("    %{} = alloca {}, align 8", alloca, param_type));
            self.emit(&format!("    store {} %{}, {}* %{}", param_type, llvm_param_index, param_type, alloca));
            
            self.variables.insert(param_name.clone(), alloca);
            self.variable_types.insert(param_name, param_type);
        }
        
        self.generate_block(&func.body)?;
        
        let has_return = self.ir.contains("ret ");
        if !has_return && return_type != "void" {
            if return_type == "i8*" || return_type == "double" {
                self.emit(&format!("    ret {} null", return_type));
            } else {
                self.emit(&format!("    ret {} 0", return_type));
            }
        } else if !has_return && return_type == "void" {
            self.emit("    ret void");
        }
        
        self.emit("}\n");
        Ok(())
    }

    /**
     * 生成代码块
     */
    fn generate_block(&mut self, block: &BlockStmt) -> Result<(), CodegenError> {
        for stmt in &block.statements {
            self.generate_statement(stmt)?;
        }
        Ok(())
    }

    /**
     * 生成语句
     */
    fn generate_statement(&mut self, stmt: &Stmt) -> Result<(), CodegenError> {
        match stmt {
            Stmt::Let(let_stmt) => {
                self.generate_let_stmt(let_stmt)?;
            }
            Stmt::Return(ret_stmt) => {
                self.generate_return_stmt(ret_stmt)?;
            }
            Stmt::If(if_stmt) => {
                self.generate_if_stmt(if_stmt)?;
            }
            Stmt::Loop(loop_stmt) => {
                self.generate_loop_stmt(loop_stmt)?;
            }
            Stmt::Expr(expr_stmt) => {
                self.generate_expression(&expr_stmt.expr)?;
            }
            Stmt::Assignment(assign_stmt) => {
                // 赋值语句：生成赋值表达式
                let value_val = self.generate_expression(&assign_stmt.value)?;
                match &assign_stmt.target {
                    Expr::Identifier(ident) => {
                        let var_name = ident.name.clone();
                        if let Some(alloca) = self.variables.get(&var_name).cloned() {
                            let var_type = self.variable_types.get(&var_name)
                                .cloned()
                                .unwrap_or_else(|| "i64".to_string());
                            let right_actual_type = self.variable_types.get(&value_val)
                                .cloned()
                                .unwrap_or_else(|| self.infer_expression_type(&assign_stmt.value));
                            // struct value conversion handled below
                            // 当右值实际是结构体指针但目标变量是结构体值时，加载值
                            let final_val = if right_actual_type.ends_with('*') && var_type.starts_with("%struct.") {
                                self.generate_type_conversion(&value_val, &right_actual_type, &var_type)
                            } else if right_actual_type != var_type {
                                self.generate_type_conversion(&value_val, &right_actual_type, &var_type)
                            } else {
                                value_val.clone()
                            };
                            self.emit(&format!("    store {} %{}, {}* %{}", var_type, final_val, var_type, alloca));
                        }
                    }
                    Expr::MemberAccess(member) => {
                        let field_name = &member.member;
                        // 关键：对于结构体字段赋值，必须用原始 alloca 指针做 GEP
                        let (ptr_val, actual_type, field_offset, field_llvm_type) =
                            if let Expr::Identifier(ident) = &*member.object {
                                if let Some(alloca) = self.variables.get(&ident.name) {
                                    let var_type = self.variable_types.get(&ident.name).cloned().unwrap_or_else(|| "i64".to_string());
                                    if var_type.starts_with("%struct.") {
                                        let (off, ftype) = self.calculate_field_offset_and_type(&var_type, field_name);
                                        (alloca.clone(), var_type, off, ftype)
                                    } else {
                                        let ov = self.generate_expression(&member.object)?;
                                        let (off, ftype) = self.calculate_field_offset_and_type(&var_type, field_name);
                                        (ov, var_type, off, ftype)
                                    }
                                } else {
                                    let ov = self.generate_expression(&member.object)?;
                                    let at = self.infer_expression_type(&member.object);
                                    let (off, ftype) = self.calculate_field_offset_and_type(&at, field_name);
                                    (ov, at, off, ftype)
                                }
                            } else {
                                let ov = self.generate_expression(&member.object)?;
                                let at = self.variable_types.get(&ov).cloned().unwrap_or_else(|| self.infer_expression_type(&member.object));
                                let (off, ftype) = self.calculate_field_offset_and_type(&at, field_name);
                                (ov, at, off, ftype)
                            };

                        let ptr_as_i8 = if actual_type.starts_with("%struct.") {
                            let cast = self.new_label("struct_to_i8");
                            let base_type = actual_type.trim_end_matches('*');
                            self.emit(&format!("    %{} = bitcast {}* %{} to i8*", cast, base_type, ptr_val));
                            cast
                        } else {
                            ptr_val.clone()
                        };
                        
                        let gep = self.new_label("assign_gep");
                        self.emit(&format!("    %{} = getelementptr i8, i8* %{}, i32 {}", gep, ptr_as_i8, field_offset));
                        let typed_ptr = self.new_label("assign_typed");
                        self.emit(&format!("    %{} = bitcast i8* %{} to {}*", typed_ptr, gep, field_llvm_type));
                        
                        let right_type = self.infer_expression_type(&assign_stmt.value);
                        if field_llvm_type.starts_with("%struct.") {
                            let struct_val = if right_type.starts_with("%struct.") && right_type.ends_with('*') {
                                value_val  // 是指针，直接用于 store
                            } else if right_type == "i64" {
                                let struct_ptr = self.new_label("struct_ptr");
                                self.emit(&format!("    %{} = inttoptr i64 %{} to {}*", struct_ptr, value_val, field_llvm_type));
                                struct_ptr
                            } else {
                                value_val  // 结构体值，直接用于 store
                            };
                            // 根据值类型选择正确的 store 格式
                            if right_type.ends_with('*') || right_type == "i64" {
                                // 指针类型值
                                self.emit(&format!("    store {}* %{}, {}* %{}", field_llvm_type, struct_val, field_llvm_type, typed_ptr));
                            } else {
                                // 值类型：store 结构体值本身
                                self.emit(&format!("    store {} %{}, {}* %{}", field_llvm_type, struct_val, field_llvm_type, typed_ptr));
                            }
                        } else {
                            let final_val = if right_type != field_llvm_type {
                                self.generate_type_conversion(&value_val, &right_type, &field_llvm_type)
                            } else { value_val };
                            self.emit(&format!("    store {} %{}, {}* %{}", field_llvm_type, final_val, field_llvm_type, typed_ptr));
                        }
                    }
                    _ => {}
                }
            }
            Stmt::StructDef(_struct_def) => {
                // 结构体定义已在 register_struct_layout 中处理
                // 这里不需要生成任何IR代码
            }
            Stmt::EnumDef(_enum_def) => {
                // 枚举定义已在 register_enum_values 中处理
                // 这里不需要生成任何IR代码
            }
            Stmt::Constant(const_def) => {
                // 常量定义已在 register_constant 中处理
                // 但如果有初始化表达式，需要生成代码
                if let Ok(_) = self.evaluate_const_expr(&const_def.value) {
                    // 常量值已经注册
                }
            }
            Stmt::Break(_) => {
                // 跳转到循环结束标签
                if let Some((break_label, _)) = self.loop_label_stack.last() {
                    self.emit(&format!("    br label %L{}", break_label));
                } else {
                    return Err(CodegenError::new("break语句只能在循环中使用"));
                }
            }
            Stmt::Continue(_) => {
                // 跳转到循环 continue 标签（递增变量后重新检查条件）
                if let Some((_, continue_label)) = self.loop_label_stack.last() {
                    self.emit(&format!("    br label %L{}", continue_label));
                } else {
                    return Err(CodegenError::new("continue语句只能在循环中使用"));
                }
            }
            Stmt::Match(_) => {
                return Err(CodegenError::unsupported_feature("模式匹配语句(Match)"));
            }
            Stmt::Block(block) => {
                // 块语句：递归生成块内语句
                self.generate_block(block)?;
            }
            Stmt::TypeAlias(_) => {
                // 类型别名：不需要生成IR代码
            }
            Stmt::Try(_) => {
                return Err(CodegenError::unsupported_feature("异常处理语句(Try/Catch)"));
            }
            Stmt::Throw(_) => {
                return Err(CodegenError::unsupported_feature("抛出异常语句(Throw)"));
            }
            _ => {
                return Err(CodegenError::new("不支持的语句类型"));
            }
        }
        Ok(())
    }

    /**
     * 生成变量声明语句
     */
    fn generate_let_stmt(&mut self, let_stmt: &LetStmt) -> Result<(), CodegenError> {
        let var_name = let_stmt.name.clone();

        let struct_name = if let Some(type_annotation) = &let_stmt.type_annotation {
            match type_annotation {
                Type::Custom(n) | Type::Struct(n) => Some(n.clone()),
                _ => None,
            }
        } else { None };

        let var_type = if let Some(initializer) = &let_stmt.initializer {
            self.infer_expression_type(initializer)
        } else if let Some(type_annotation) = &let_stmt.type_annotation {
            self.type_to_llvm_type(type_annotation)
        } else { "i64".to_string() };

        let alloca = self.new_label(&var_name);
        let final_var_type = if let Some(s_name) = &struct_name {
            self.llvm_type_for_named_struct(s_name)
        } else {
            var_type.clone()
        };

        // stored_var_type 直接使用 final_var_type，不加 * 后缀
        // alloca %struct.T 创建 %struct.T*，变量的"值类型"是 %struct.T
        let stored_var_type = final_var_type.clone();

        if let Some(initializer) = &let_stmt.initializer {
            let expr_val = self.generate_expression(initializer)?;
            let actual_type = self.variable_types.get(&expr_val).cloned().unwrap_or_else(|| self.infer_expression_type(initializer));

            if final_var_type.starts_with("%struct.") && actual_type.starts_with("%struct.") && actual_type.ends_with('*') {
                // RHS 返回的是结构体指针（如 sret 调用返回的 agg_slot）
                // 需要从指针加载结构体值，再存入新的 alloca
                let loaded_val = self.new_label("letval");
                let final_struct_name = if actual_type.ends_with('*') {
                    &actual_type[..actual_type.len()-1]  // 去掉末尾的 *
                } else {
                    &actual_type
                };
                self.emit(&format!("    %{} = load {}, {} %{}", loaded_val, final_struct_name, actual_type, expr_val));
                self.emit(&format!("    %{} = alloca {}, align 8", alloca, final_var_type));
                self.emit(&format!("    store {} %{}, {}* %{}", final_var_type, loaded_val, final_var_type, alloca));
            } else if final_var_type.starts_with("%struct.") {
                self.emit(&format!("    %{} = alloca {}, align 8", alloca, final_var_type));
                let final_val = if actual_type != final_var_type {
                    self.generate_type_conversion(&expr_val, &actual_type, &final_var_type)
                } else { expr_val };
                self.emit(&format!("    store {} %{}, {}* %{}", final_var_type, final_val, final_var_type, alloca));
            } else {
                self.emit(&format!("    %{} = alloca {}, align 8", alloca, final_var_type));
                let final_val = if actual_type != final_var_type {
                    self.generate_type_conversion(&expr_val, &actual_type, &final_var_type)
                } else { expr_val };
                self.emit(&format!("    store {} %{}, {}* %{}", final_var_type, final_val, final_var_type, alloca));
            }
        } else {
            self.emit(&format!("    %{} = alloca {}, align 8", alloca, final_var_type));
        }

        self.variables.insert(var_name.clone(), alloca);
        self.variable_types.insert(var_name, stored_var_type);
        Ok(())
    }

    /**
     * 生成返回语句
     */
    fn generate_return_stmt(&mut self, ret_stmt: &ReturnStmt) -> Result<(), CodegenError> {
        if let Some(expr) = &ret_stmt.value {
            let expr_val = self.generate_expression(expr)?;
            // 使用函数声明中定义的返回类型，而不是从表达式推断
            let return_type = self.current_function_return_type.clone();
            
            if self.current_function_return_is_aggregate {
                let _expr_type = self.infer_expression_type(expr);
                let agg_result_ptr = self.new_label("agg_ret_ptr");
                self.emit(&format!("    %{} = bitcast {}* %agg.result to {}*", agg_result_ptr, return_type, return_type));
                if self.variable_types.get(&expr_val).map(|t| t.ends_with('*')).unwrap_or(false) {
                    self.emit(&format!("    call void @llvm.memcpy.p0i8.p0i8.i64(i8* %{}, i8* %{}, i64 {}, i1 false)", agg_result_ptr, expr_val, self.compute_struct_size_from_type(&return_type)));
                } else {
                    self.emit(&format!("    store {} %{}, {}* %{}", return_type, expr_val, return_type, agg_result_ptr));
                }
                self.emit("    ret void");
            } else if return_type != "void" {
                let expr_type = self.infer_expression_type(expr);
                let final_val = if expr_type != return_type {
                    self.generate_type_conversion(&expr_val, &expr_type, &return_type)
                } else {
                    expr_val
                };
                self.emit(&format!("    ret {} %{}", return_type, final_val));
            } else {
                self.emit("    ret void");
            }
        } else {
            self.emit("    ret void");
        }
        Ok(())
    }

    /**
     * 生成if语句
     */
    fn generate_if_stmt(&mut self, if_stmt: &IfStmt) -> Result<(), CodegenError> {
        // 处理第一个分支
        if let Some(first_branch) = if_stmt.branches.first() {
            let cond_val = self.generate_expression(&first_branch.condition)?;
            let cond_type = self.infer_expression_type(&first_branch.condition);
            
            // 将条件转换为 i1 类型（br i1 需要）
            let cond_for_br = if cond_type == "i64" {
                let bool_cond = self.new_label("bool");
                self.emit(&format!("    %{} = icmp ne i64 %{}, 0", bool_cond, cond_val));
                bool_cond
            } else {
                cond_val
            };
            
            let then_label = self.label_counter;
            self.label_counter += 1;
            let else_label = self.label_counter;
            self.label_counter += 1;
            let end_label = self.label_counter;
            self.label_counter += 1;
            
            self.emit(&format!("    br i1 %{}, label %L{}, label %L{}", cond_for_br, then_label, else_label));
            
            // 生成then分支
            self.emit(&format!("L{}:", then_label));
            match &*first_branch.body {
                Stmt::Block(block) => self.generate_block(block)?,
                _ => return Err(CodegenError::new("If语句的body必须是BlockStmt")),
            }
            self.emit(&format!("    br label %L{}", end_label));

            // 生成else分支
            self.emit(&format!("L{}:", else_label));
            if let Some(else_block) = &if_stmt.else_branch {
                match &**else_block {
                    Stmt::Block(block) => self.generate_block(block)?,
                    Stmt::If(nested_if) => self.generate_if_stmt(nested_if)?,
                    _ => return Err(CodegenError::new("Else语句的body必须是BlockStmt或IfStmt")),
                }
            }
            self.emit(&format!("    br label %L{}", end_label));
            
            // 生成结束标签
            self.emit(&format!("L{}:", end_label));
        }
        Ok(())
    }

    /**
     * 生成循环语句
     */
    fn generate_loop_stmt(&mut self, loop_stmt: &LoopStmt) -> Result<(), CodegenError> {
        match loop_stmt.kind {
            LoopKind::While => {
                if let Some(condition) = &loop_stmt.condition {
                    let loop_start = self.label_counter;
                    self.label_counter += 1;
                    let loop_body = self.label_counter;
                    self.label_counter += 1;
                    let loop_end = self.label_counter;
                    self.label_counter += 1;
                    // continue 跳转到 loop_start（重新检查条件）
                    let loop_continue = loop_start;

                    // 压入循环标签栈
                    self.loop_label_stack.push((loop_end, loop_continue));

                    self.emit(&format!("    br label %L{}", loop_start));

                    // 生成循环开始标签
                    self.emit(&format!("L{}:", loop_start));
                    let cond_val = self.generate_expression(condition)?;
                    let cond_type = self.infer_expression_type(condition);

                    // 将条件转换为 i1 类型（br i1 需要）
                    let cond_for_br = if cond_type == "i64" {
                        let bool_cond = self.new_label("bool");
                        self.emit(&format!("    %{} = icmp ne i64 %{}, 0", bool_cond, cond_val));
                        bool_cond
                    } else {
                        cond_val
                    };

                    self.emit(&format!("    br i1 %{}, label %L{}, label %L{}", cond_for_br, loop_body, loop_end));

                    // 生成循环体
                    self.emit(&format!("L{}:", loop_body));
                    match &*loop_stmt.body {
                        Stmt::Block(block) => self.generate_block(block)?,
                        _ => return Err(CodegenError::new("循环语句的body必须是BlockStmt")),
                    }
                    self.emit(&format!("    br label %L{}", loop_start));

                    // 生成循环结束标签（不添加terminator，让后续代码继续生成）
                    self.emit(&format!("L{}:", loop_end));

                    // 弹出循环标签栈
                    self.loop_label_stack.pop();
                }
            }
            LoopKind::Counted => {
                // 计数循环: 循环 i 从 start 到 end { body }
                if let Some(counter) = &loop_stmt.counter {
                    let loop_start = self.label_counter;
                    self.label_counter += 1;
                    let loop_body = self.label_counter;
                    self.label_counter += 1;
                    let loop_end = self.label_counter;
                    self.label_counter += 1;
                    let loop_continue = self.label_counter;
                    self.label_counter += 1;

                    // 压入循环标签栈
                    self.loop_label_stack.push((loop_end, loop_continue));

                    // 生成循环变量的 alloca
                    let var_name = counter.variable.clone();
                    let var_alloca = self.new_label(&format!("{}_alloca", var_name));
                    self.emit(&format!("    %{} = alloca i64, align 8", var_alloca));

                    // 初始化循环变量
                    let start_val = self.generate_expression(&counter.start)?;
                    self.emit(&format!("    store i64 %{}, i64* %{}", start_val, var_alloca));
                    self.variables.insert(var_name.clone(), var_alloca.clone());
                    self.variable_types.insert(var_name.clone(), "i64".to_string());

                    // 生成结束值
                    let end_val = self.generate_expression(&counter.end)?;

                    // 跳转到条件检查
                    self.emit(&format!("    br label %L{}", loop_start));

                    // 条件检查标签
                    self.emit(&format!("L{}:", loop_start));
                    let i_val = self.new_label("i_cur");
                    self.emit(&format!("    %{} = load i64, i64* %{}", i_val, var_alloca));
                    let cond = self.new_label("loop_cond");
                    self.emit(&format!("    %{} = icmp sle i64 %{}, %{}", cond, i_val, end_val));
                    self.emit(&format!("    br i1 %{}, label %L{}, label %L{}", cond, loop_body, loop_end));

                    // 循环体
                    self.emit(&format!("L{}:", loop_body));
                    match &*loop_stmt.body {
                        Stmt::Block(block) => self.generate_block(block)?,
                        _ => return Err(CodegenError::new("计数循环的body必须是BlockStmt")),
                    }

                    // continue标签（递增循环变量后跳回条件检查）
                    self.emit(&format!("L{}:", loop_continue));
                    let i_next = self.new_label("i_next");
                    self.emit(&format!("    %{} = load i64, i64* %{}", i_next, var_alloca));
                    let i_inc = self.new_label("i_inc");
                    self.emit(&format!("    %{} = add i64 %{}, 1", i_inc, i_next));
                    self.emit(&format!("    store i64 %{}, i64* %{}", i_inc, var_alloca));
                    self.emit(&format!("    br label %L{}", loop_start));

                    // 循环结束
                    self.emit(&format!("L{}:", loop_end));
                    // continue point handled by parent block

                    // 弹出循环标签栈
                    self.loop_label_stack.pop();
                }
            }
            LoopKind::For => {
                // 遍历循环: 遍历 变量 取自 列表 { body }
                if let Some(iter_list) = &loop_stmt.iterator {
                    let loop_start = self.label_counter;
                    self.label_counter += 1;
                    let loop_body = self.label_counter;
                    self.label_counter += 1;
                    let loop_end = self.label_counter;
                    self.label_counter += 1;
                    let loop_continue = self.label_counter;
                    self.label_counter += 1;

                    // 压入循环标签栈
                    self.loop_label_stack.push((loop_end, loop_continue));

                    // 获取列表指针
                    let list_val = self.generate_expression(iter_list)?;

                    // 获取列表长度
                    let list_len = self.new_label("list_len");
                    self.emit(&format!("    %{} = call i64 @rt_list_len(i8* %{})", list_len, list_val));

                    // 循环索引变量
                    let idx_alloca = self.new_label("idx_alloca");
                    self.emit(&format!("    %{} = alloca i64, align 8", idx_alloca));
                    self.emit(&format!("    store i64 0, i64* %{}", idx_alloca));

                    // 如果有循环变量名，创建 alloca
                    if let Some(counter) = &loop_stmt.counter {
                        let var_name = counter.variable.clone();
                        let var_alloca = self.new_label(&format!("{}_alloca", var_name));
                        self.emit(&format!("    %{} = alloca i64, align 8", var_alloca));
                        self.variables.insert(var_name.clone(), var_alloca);
                        self.variable_types.insert(var_name, "i64".to_string());
                    }

                    // 跳转到条件检查
                    self.emit(&format!("    br label %L{}", loop_start));

                    // 条件检查
                    self.emit(&format!("L{}:", loop_start));
                    let idx_val = self.new_label("idx");
                    self.emit(&format!("    %{} = load i64, i64* %{}", idx_val, idx_alloca));
                    let cond = self.new_label("for_cond");
                    self.emit(&format!("    %{} = icmp slt i64 %{}, %{}", cond, idx_val, list_len));
                    self.emit(&format!("    br i1 %{}, label %L{}, label %L{}", cond, loop_body, loop_end));

                    // 循环体
                    self.emit(&format!("L{}:", loop_body));

                    // 获取当前元素
                    let elem = self.new_label("elem");
                    self.emit(&format!("    %{} = call i8* @rt_list_get(i8* %{}, i64 %{})", elem, list_val, idx_val));

                    // 设置循环变量为当前元素
                    if let Some(counter) = &loop_stmt.counter {
                        let var_name = counter.variable.clone();
                        if let Some(var_alloca) = self.variables.get(&var_name).cloned() {
                            let elem_val = self.new_label("elem_val");
                            self.emit(&format!("    %{} = ptrtoint i8* %{} to i64", elem_val, elem));
                            self.emit(&format!("    store i64 %{}, i64* %{}", elem_val, var_alloca));
                        }
                    }

                    match &*loop_stmt.body {
                        Stmt::Block(block) => self.generate_block(block)?,
                        _ => return Err(CodegenError::new("遍历循环的body必须是BlockStmt")),
                    }

                    // continue标签
                    self.emit(&format!("L{}:", loop_continue));
                    let idx_next = self.new_label("idx_next");
                    let idx_inc = self.new_label("idx_inc");
                    self.emit(&format!("    %{} = load i64, i64* %{}", idx_next, idx_alloca));
                    self.emit(&format!("    %{} = add i64 %{}, 1", idx_inc, idx_next));
                    self.emit(&format!("    store i64 %{}, i64* %{}", idx_inc, idx_alloca));
                    self.emit(&format!("    br label %L{}", loop_start));

                    // 循环结束
                    self.emit(&format!("L{}:", loop_end));
                    // continue point handled by parent block

                    // 弹出循环标签栈
                    self.loop_label_stack.pop();
                }
            }
            LoopKind::Infinite => {
                let loop_body = self.label_counter;
                self.label_counter += 1;
                let loop_end = self.label_counter;
                self.label_counter += 1;
                let loop_start = loop_body;

                // 压入循环标签栈
                self.loop_label_stack.push((loop_end, loop_start));

                self.emit(&format!("    br label %L{}", loop_body));
                self.emit(&format!("L{}:", loop_body));
                match &*loop_stmt.body {
                    Stmt::Block(block) => self.generate_block(block)?,
                    _ => return Err(CodegenError::new("无限循环的body必须是BlockStmt")),
                }
                self.emit(&format!("    br label %L{}", loop_body));
                // 循环结束标签即跳出目标，后续代码直接在标签后生成
                self.emit(&format!("L{}:", loop_end));

                // 弹出循环标签栈
                self.loop_label_stack.pop();
            }
        }
        Ok(())
    }

    /**
     * 生成表达式
     */
    fn generate_expression(&mut self, expr: &Expr) -> Result<String, CodegenError> {
        match expr {
            Expr::Identifier(ident) => {
                let var_name = ident.name.clone();
                if let Some(alloca) = self.variables.get(&var_name).cloned() {
                    let var_type = self.variable_types.get(&var_name)
                        .cloned()
                        .unwrap_or_else(|| "i64".to_string());

                    if var_type.starts_with("%struct.") {
                        // 变量类型是 %struct.T（值类型），alloca 是 %struct.T*
                        // 剥离可能被污染的 * 后缀（某些路径可能错误地存入 %struct.T*）
                        let clean_type = var_type.trim_end_matches('*').to_string();
                        let load = self.new_label("id");
                        self.emit(&format!("    %{} = load {}, {}* %{}", load, clean_type, clean_type, alloca));
                        self.variable_types.insert(load.clone(), clean_type.clone());
                        Ok(load)
                    } else if var_type == "i8*" {
                        let load = self.new_label("id");
                        self.emit(&format!("    %{} = load i8*, i8** %{}", load, alloca));
                        self.variable_types.insert(load.clone(), var_type);
                        Ok(load)
                    } else {
                        let load = self.new_label("id");
                        self.emit(&format!("    %{} = load {}, {}* %{}", load, var_type, var_type, alloca));
                        self.variable_types.insert(load.clone(), var_type);
                        Ok(load)
                    }
                } else {
                    // 对于枚举变体，生成一个整数值
                    // 先尝试从动态注册的枚举值中查找
                    let enum_value = if let Some(&val) = self.enum_values.get(ident.name.as_str()) {
                        val
                    } else {
                        // 回退到硬编码的枚举值映射
                        match ident.name.as_str() {
                            "None" | "Init" | "Void" => 0,
                            "Lexing" | "Func" | "Int" => 1,
                            "Parsing" | "Params" | "Float" => 2,
                            "Semantic" | "Body" | "Ptr" => 3,
                            "Codegen" | "Expr" => 4,
                            "Linking" | "Label" => 5,
                            "Done" => 6,
                            "Error" => 7,
                            "Kw" => 0,
                            "Id" => 1,
                            "Num" => 2,
                            "Str" => 3,
                            "Sym" => 4,
                            "End" => 5,
                            "Err" => 6,
                            "Prog" => 0,
                            "Var" => 1,
                            "Ret" => 2,
                            "If" => 3,
                            "While" => 4,
                            "Call" => 5,
                            "BinOp" => 6,
                            _ => 0,
                        }
                    };
                    let load = self.new_label("enum");
                    self.emit(&format!("    %{} = add i64 0, {}", load, enum_value));
                    // 记录枚举值的类型为 i64
                    self.variable_types.insert(load.clone(), "i64".to_string());
                    Ok(load)
                }
            }
            Expr::Literal(lit) => {
                self.generate_literal_expr(lit)
            }
            Expr::Binary(binary) => {
                self.generate_binary_expr(binary)
            }
            Expr::Unary(unary) => {
                self.generate_unary_expr(unary)
            }
            Expr::Call(call) => {
                self.generate_call_expr(call)
            }
            Expr::MemberAccess(member) => {
                // 检查是否是模块间函数调用（如 utils::版本()）
                // 这里需要特殊处理，因为模块间函数调用会被解析为 MemberAccess + Call
                // 但在生成表达式时，我们只看到 MemberAccess，Call 会在后续处理
                
                // 获取对象表达式（模块名）
                let object_expr = &member.object;
                let member_name = &member.member;
                
                // 检查对象是否是标识符（模块名、枚举名还是变量名）
                if let Expr::Identifier(module_ident) = &**object_expr {
                    let id_name = &module_ident.name;
                    let translated_id = self.translate_func_name(id_name);
                    // 检查是否是枚举值访问（如 错误级别.错误）-> 解析为整数
                    if self.enum_values.contains_key(member_name) {
                        let enum_val = self.enum_values.get(member_name).copied().unwrap_or(0);
                        let result_val = self.new_label("enum");
                        self.emit(&format!("    %{} = add i64 0, {}", result_val, enum_val));
                        self.variable_types.insert(result_val.clone(), "i64".to_string());
                        return Ok(result_val);
                    }
                    // 如果标识符是一个已知变量，则这是结构体字段访问，不是模块访问
                    if self.variables.contains_key(&translated_id) || self.variables.contains_key(id_name) {
                        // 这是一个结构体字段访问，继续执行下面的代码
                    } else {
                        // 这是一个模块间的成员访问，返回模块名::成员名的组合
                        let full_name = format!("{}::{}", id_name, member_name);
                        // 翻译函数名（处理中文）
                        let translated_name = self.translate_func_name(&full_name);
                        // 对于模块间函数调用，我们需要在 Call 表达式中处理
                        // 这里只是返回函数名，供 Call 表达式使用
                        return Ok(translated_name);
                    }
                }
                {
                    // 获取字段名
                    let field_name = &member.member;

                    // 检查是否是列表方法（只有当对象是列表类型时才调用列表方法）
                    let object_type = self.infer_expression_type(&member.object);
                    
                    // 检查字段名是否存在于结构体布局中（区分结构体字段和列表方法）
                    let is_struct_field = self.struct_field_layouts.values().any(|fields| {
                        fields.iter().any(|(name, _, _)| name == field_name)
                    });
                    
                    // 只有当对象是 i8* 且字段名不是结构体字段时才认为是列表方法
                    let is_list_method = object_type == "i8*" && !is_struct_field && matches!(field_name.as_str(), "长度" | "追加" | "获取");

                    if is_list_method {
                        // 列表方法处理
                        // 生成对象表达式（列表指针 i8*）
                        let object_val = self.generate_expression(&member.object)?;
                        
                        // 获取对象类型
                        let object_type = self.infer_expression_type(&member.object);
                        
                        // 如果是 i64，需要转换为 i8*
                        let ptr_val = if object_type == "i8*" {
                            object_val
                        } else {
                            let ptr = self.new_label("list_ptr");
                            self.emit(&format!("    %{} = inttoptr i64 %{} to i8*", ptr, object_val));
                            ptr
                        };
                        
                        match field_name.as_str() {
                            "长度" => {
                                // 调用 rt_list_len，返回 i64
                                let result = self.new_label("len");
                                self.emit(&format!("    %{} = call i64 @rt_list_len(i8* %{})
", result, ptr_val));
                                Ok(result)
                            }
                            _ => {
                                // 其他方法返回指针值
                                Ok(ptr_val)
                            }
                        }
                    } else {
                        let object_val = self.generate_expression(&member.object)?;
                        let obj_type = self.infer_expression_type(&member.object);
                        let actual_type = self.variable_types.get(&object_val).cloned().unwrap_or(obj_type.clone());

                        let ptr_val = if actual_type.ends_with('*') {
                            object_val
                        } else if obj_type == "i64" && !actual_type.starts_with("%struct.") {
                            let ptr = self.new_label("ptr");
                            self.emit(&format!("    %{} = inttoptr i64 %{} to i8*", ptr, object_val));
                            ptr
                        } else if actual_type.starts_with("%struct.") {
                            // actual_type 是 %struct.T（值类型），创建临时 alloca 获取指针用于 GEP
                            let tmp_alloca = self.new_label("field_ptr");
                            self.emit(&format!("    %{} = alloca {}, align 8", tmp_alloca, actual_type));
                            self.emit(&format!("    store {} %{}, {}* %{}", actual_type, object_val, actual_type, tmp_alloca));
                            tmp_alloca
                        } else {
                            object_val
                        };

                        let obj_type_for_offset = if actual_type.starts_with("%struct.") && actual_type.ends_with('*') {
                            actual_type[0..actual_type.len()-1].to_string()
                        } else if actual_type.starts_with("%struct.") {
                            actual_type.clone()
                        } else {
                            obj_type
                        };
                        let (field_offset, field_llvm_type) = self.calculate_field_offset_and_type(&obj_type_for_offset, field_name);

                        let result = self.new_label("member");
                        self.emit(&format!("    %{} = getelementptr i8, i8* %{}, i32 {}",
                            result, ptr_val, field_offset));

                        let result_ptr = self.new_label("member_ptr");
                        self.emit(&format!("    %{} = bitcast i8* %{} to {}*", result_ptr, result, field_llvm_type));
                        
                        if field_llvm_type.starts_with("%struct.") {
                            // 返回指针，供需要指针的调用者（如 struct field assignment）
                            self.variable_types.insert(result_ptr.clone(), format!("{}*", field_llvm_type));
                            Ok(result_ptr)
                        } else {
                            let result_val = self.new_label("member_val");
                            self.emit(&format!("    %{} = load {}, {}* %{}", result_val, field_llvm_type, field_llvm_type, result_ptr));
                            self.variable_types.insert(result_val.clone(), field_llvm_type);
                            Ok(result_val)
                        }
                    }
                }
            }
            Expr::Grouped(expr) => {
                self.generate_expression(expr)
            }
            Expr::Await(await_expr) => {
                // Await 表达式：生成等待异步操作的代码
                // 简化实现：直接生成被等待的表达式
                let inner_val = self.generate_expression(&await_expr.expr)?;
                // TODO: 实现完整的异步运行时支持
                Ok(inner_val)
            }
            Expr::ListLiteral(list) => {
                // 创建列表
                let list_ptr = self.new_label("list");
                self.emit(&format!("    %{} = call i8* @rt_list_new()", list_ptr));
                
                // 添加元素
                for elem in &list.elements {
                    let elem_val = self.generate_expression(elem)?;
                    // 根据元素类型决定如何处理
                    let elem_type = self.infer_expression_type(elem);
                    
                    if elem_type == "i8*" {
                        // 字符串类型，直接使用
                        self.emit(&format!("    call void @rt_list_append(i8* %{}, i8* %{})
", list_ptr, elem_val));
                    } else if elem_type.starts_with("%struct.") {
                        // 结构体类型，需要先创建 alloca 存储，然后使用地址
                        let elem_addr = self.new_label("elem_addr");
                        self.emit(&format!("    %{} = alloca {}, align 8", elem_addr, elem_type));
                        self.emit(&format!("    store {} %{}, {}* %{}", elem_type, elem_val, elem_type, elem_addr));
                        self.emit(&format!("    call void @rt_list_append(i8* %{}, i8* %{})
", list_ptr, elem_addr));
                    } else {
                        // 其他类型（如 i64），转换为指针
                        let elem_ptr = self.new_label("elem_ptr");
                        self.emit(&format!("    %{} = inttoptr {} %{} to i8*", elem_ptr, elem_type, elem_val));
                        self.emit(&format!("    call void @rt_list_append(i8* %{}, i8* %{})
", list_ptr, elem_ptr));
                    }
                }
                
                // 返回列表指针 (i8*)
                Ok(list_ptr)
            }
            Expr::ListComprehension(comp) => {
                // 列表推导式: [x * 2 for x in list]
                // 生成代码：
                // 1. 创建新列表
                // 2. 获取原列表长度
                // 3. 循环遍历原列表
                // 4. 对每个元素应用输出表达式
                // 5. 可选：应用条件过滤
                
                // 创建新列表
                let result_list = self.new_label("result_list");
                self.emit(&format!("    %{} = call i8* @rt_list_new()", result_list));
                
                // 获取原列表
                let src_list = self.generate_expression(&comp.iterable)?;
                
                // 获取原列表长度
                let src_len = self.new_label("src_len");
                self.emit(&format!("    %{} = call i64 @rt_list_len(i8* %{})
", src_len, src_list));
                
                // 循环变量
                let i_alloca = self.new_label("i_alloca");
                self.emit(&format!("    %{} = alloca i64", i_alloca));
                self.emit(&format!("    store i64 0, i64* %{}", i_alloca));
                
                // 循环开始标签
                let loop_start = self.label_counter;
                self.label_counter += 1;
                let loop_body = self.label_counter;
                self.label_counter += 1;
                let loop_end = self.label_counter;
                self.label_counter += 1;
                
                self.emit(&format!("    br label %L{}", loop_start));
                self.emit(&format!("L{}:", loop_start));
                
                // 检查循环条件: i < len
                let i_val = self.new_label("i_val");
                self.emit(&format!("    %{} = load i64, i64* %{}", i_val, i_alloca));
                let cond = self.new_label("cond");
                self.emit(&format!("    %{} = icmp slt i64 %{}, %{}", cond, i_val, src_len));
                self.emit(&format!("    br i1 %{}, label %L{}, label %L{}", cond, loop_body, loop_end));
                
                // 循环体
                self.emit(&format!("L{}:", loop_body));
                
                // 获取当前元素
                let elem = self.new_label("elem");
                self.emit(&format!("    %{} = call i8* @rt_list_get(i8* %{}, i64 %{})
", elem, src_list, i_val));
                
                // 将元素转换为 i64 并存储到迭代变量
                let elem_val = self.new_label("elem_val");
                self.emit(&format!("    %{} = ptrtoint i8* %{} to i64", elem_val, elem));
                
                // 存储迭代变量
                let var_alloca = self.new_label(&format!("var_{}", comp.var_name));
                self.emit(&format!("    %{} = alloca i64", var_alloca));
                self.emit(&format!("    store i64 %{}, i64* %{}", elem_val, var_alloca));
                
                // 记录迭代变量
                let translated_var = comp.var_name.clone();
                self.variables.insert(translated_var.clone(), var_alloca);
                self.variable_types.insert(translated_var, "i64".to_string());
                
                // 生成输出表达式
                let output_val = self.generate_expression(&comp.output)?;
                let output_type = self.infer_expression_type(&comp.output);
                
                // 条件过滤
                if let Some(cond_expr) = &comp.condition {
                    // 生成条件表达式
                    let cond_result = self.generate_expression(cond_expr)?;
                    let cond_type = self.infer_expression_type(cond_expr);
                    
                    // 将条件转换为 i1 类型（br i1 需要）
                    let cond_for_br = if cond_type == "i64" {
                        let bool_cond = self.new_label("bool");
                        self.emit(&format!("    %{} = icmp ne i64 %{}, 0", bool_cond, cond_result));
                        bool_cond
                    } else {
                        cond_result
                    };
                    
                    // 条件跳转标签
                    let do_append = self.label_counter;
                    self.label_counter += 1;
                    let skip_append = self.label_counter;
                    self.label_counter += 1;
                    
                    // 检查条件：为真则添加，为假则跳过
                    self.emit(&format!("    br i1 %{}, label %L{}, label %L{}", cond_for_br, do_append, skip_append));
                    
                    // 添加元素
                    self.emit(&format!("L{}:", do_append));
                    
                    // 添加到结果列表
                    self.append_to_list(&result_list, &output_val, &output_type);
                    
                    // 跳过添加后的继续点
                    let after_append = self.label_counter;
                    self.label_counter += 1;
                    self.emit(&format!("    br label %L{}", after_append));
                    
                    // 跳过添加
                    self.emit(&format!("L{}:", skip_append));
                    self.emit(&format!("    br label %L{}", after_append));
                    
                    // 继续循环
                    self.emit(&format!("L{}:", after_append));
                } else {
                    // 无条件过滤，直接添加到结果列表
                    self.append_to_list(&result_list, &output_val, &output_type);
                }
                
                // 递增循环变量
                let i_next = self.new_label("i_next");
                self.emit(&format!("    %{} = add i64 %{}, 1", i_next, i_val));
                self.emit(&format!("    store i64 %{}, i64* %{}", i_next, i_alloca));
                
                // 跳回循环开始
                self.emit(&format!("    br label %L{}", loop_start));
                
                // 循环结束
                self.emit(&format!("L{}:", loop_end));
                self.emit("    unreachable");

                // 返回结果列表
                Ok(result_list)
            }
            // Match表达式暂不支持，生成默认值
            Expr::Lambda(lambda) => {
                // Lambda 表达式：生成函数指针引用
                // 注意：Lambda 的完整函数定义延迟到模块级别生成，不能嵌套在函数体内
                let lambda_func = self.new_label("lambda");

                // 捕获自由变量名列表（用于生成闭包时传递捕获变量）
                let captured_names: Vec<String> = lambda.captured_vars.iter().map(|v| v.name.clone()).collect();

                // 将 lambda 的函数定义推迟到模块级别
                // 恢复 label_counter 锚点，生成临时 IR 收集 lambda 定义
                let saved_ir = std::mem::take(&mut self.ir);
                let saved_label_counter = self.label_counter;
                let saved_variables = self.variables.clone();

                // 为 lambda 生成独立的函数定义
                self.ir = String::new();
                self.variables.clear();

                // 将捕获的变量作为函数参数传入
                if captured_names.is_empty() {
                    self.emit(&format!("define internal i64 @{}() {{\n", lambda_func));
                } else {
                    let params_str = captured_names.iter()
                        .map(|n| format!("i64 %cap_{}", Self::sanitize_identifier(n)))
                        .collect::<Vec<_>>()
                        .join(", ");
                    self.emit(&format!("define internal i64 @{}({}) {{\n", lambda_func, params_str));

                    // 将捕获变量映射到参数
                    for (_i, name) in captured_names.iter().enumerate() {
                        let safe_name = Self::sanitize_identifier(name);
                        let alloca = format!("%cap_{}", safe_name);
                        let _load_val = self.new_label("cap_load");
                        self.emit(&format!("    {} = alloca i64", alloca));
                        self.emit(&format!("    store i64 %cap_{}, i64* {}", safe_name, alloca));
                        self.variables.insert(name.clone(), alloca);
                        self.variable_types.insert(name.clone(), "i64".to_string());
                    }
                }

                let body_val = self.generate_expression(&lambda.body)?;
                self.emit(&format!("    ret i64 {}\n}}\n", body_val));

                // 收集 lambda 定义
                let lambda_def = std::mem::take(&mut self.ir);
                self.lambda_defs.push(lambda_def);

                // 恢复原来的代码生成状态
                self.ir = saved_ir;
                self.label_counter = saved_label_counter;
                self.variables = saved_variables;

                // 在当前函数中发出函数指针引用
                let func_ptr = self.new_label("lambda_ptr");
                if captured_names.is_empty() {
                    self.emit(&format!("    %{} = ptrtoint i64 ()* @{} to i64", func_ptr, lambda_func));
                } else {
                    self.emit(&format!("    %{} = ptrtoint i64 ({})* @{} to i64", func_ptr,
                        captured_names.iter().map(|_| "i64").collect::<Vec<_>>().join(", "),
                        lambda_func));
                }

                Ok(func_ptr)
            }
            Expr::IndexAccess(index_access) => {
                // 索引访问: object[index]
                let object_val = self.generate_expression(&index_access.object)?;
                let index_val = self.generate_expression(&index_access.index)?;
                
                // 获取对象表达式的实际类型（从 variable_types 中查找）
                let object_type = self.variable_types.get(&object_val)
                    .cloned()
                    .unwrap_or_else(|| self.infer_expression_type(&index_access.object));

                if object_type == "i8*" {
                    // 区分字符串索引（char access）和列表索引（element access）
                    // 先依据结构体中声明的字段类型判断是否为列表，再回退到名字启发式
                    let is_list = match &*index_access.object {
                        Expr::MemberAccess(member) => {
                            let field_name = &member.member;
                            let declared_list = self.list_typed_fields.contains(field_name);
                            let name_hint = field_name == "tokens" || field_name == "children" ||
                                field_name == "items" || field_name == "errors" ||
                                field_name.ends_with("列表");
                            declared_list || name_hint
                        }
                        Expr::Identifier(ident) => {
                            // variable name ending in "s" might be a list, but be conservative
                            let name = &ident.name;
                            name.ends_with("s") || name.contains("列表")
                        }
                        _ => false,
                    };
                    if is_list {
                        let elem = self.new_label("list_elem");
                        self.emit(&format!("    %{} = call i8* @rt_list_get(i8* %{}, i64 %{})", elem, object_val, index_val));
                        self.variable_types.insert(elem.clone(), "i8*".to_string());
                        Ok(elem)
                    } else {
                        let char_ptr = self.new_label("char_ptr");
                        self.emit(&format!("    %{} = call i8* @rt_string_char_at(i8* %{}, i64 %{})", char_ptr, object_val, index_val));
                        let char_code = self.new_label("char_code");
                        self.emit(&format!("    %{} = call i64 @rt_char_to_code(i8* %{})", char_code, char_ptr));
                        self.variable_types.insert(char_code.clone(), "i64".to_string());
                        Ok(char_code)
                    }
                } else if object_type == "i64" {
                    let ptr = self.new_label("idx_ptr");
                    self.emit(&format!("    %{} = inttoptr i64 %{} to i8*", ptr, object_val));
                    let char_ptr = self.new_label("idx_char");
                    self.emit(&format!("    %{} = call i8* @rt_string_char_at(i8* %{}, i64 %{})", char_ptr, ptr, index_val));
                    let char_code = self.new_label("idx_code");
                    self.emit(&format!("    %{} = call i64 @rt_char_to_code(i8* %{})", char_code, char_ptr));
                    self.variable_types.insert(char_code.clone(), "i64".to_string());
                    Ok(char_code)
                } else {
                    Err(CodegenError::new(&format!("不支持的类型索引访问: {}", object_type)))
                }
            }
            // 所有 Expr 变体已在上方匹配
        }
    }

    /**
     * 生成字面量表达式
     */
    fn generate_literal_expr(&mut self, lit: &LiteralExpr) -> Result<String, CodegenError> {
        match &lit.kind {
            LiteralKind::Integer(value) => {
                let label = self.new_label("int");
                self.emit(&format!("    %{} = add i64 0, {}", label, value));
                // 记录整数临时变量的类型
                self.variable_types.insert(label.clone(), "i64".to_string());
                Ok(label)
            }
            LiteralKind::Float(value) => {
                let label = self.new_label("float");
                self.emit(&format!("    %{} = fadd double 0.0, {}", label, value));
                // 记录浮点数临时变量的类型
                self.variable_types.insert(label.clone(), "double".to_string());
                Ok(label)
            }
            LiteralKind::String(value) => {
                let str_const_label = format!("str_{}", self.string_const_counter);
                self.string_const_counter += 1;
                let escaped = self.escape_string_for_llvm(&value);
                let byte_len = value.as_bytes().len();
                self.string_constants.push(format!("@str_constant_{} = private constant [{} x i8] c\"{}\\00\"", str_const_label, byte_len + 1, escaped));
                let label = self.new_label("str");
                self.emit(&format!("    %{} = call i8* @rt_str_new(i8* getelementptr inbounds ([{} x i8], [{} x i8]* @str_constant_{}, i32 0, i32 0))", 
                    label, byte_len + 1, byte_len + 1, str_const_label));
                // 记录字符串临时变量的类型
                self.variable_types.insert(label.clone(), "i8*".to_string());
                Ok(label)
            }
            LiteralKind::Boolean(value) => {
                let label = self.new_label("bool");
                self.emit(&format!("    %{} = add i64 0, {}", label, if *value { 1 } else { 0 }));
                // 记录布尔临时变量的类型
                self.variable_types.insert(label.clone(), "i1".to_string());
                Ok(label)
            }
            LiteralKind::Char(_) => {
                let label = self.new_label("char");
                self.emit(&format!("    %{} = add i64 0, 0", label));
                // 记录字符临时变量的类型
                self.variable_types.insert(label.clone(), "i64".to_string());
                Ok(label)
            }
        }
    }

    /**
     * 生成二元表达式
     */
    fn generate_binary_expr(&mut self, binary: &BinaryExpr) -> Result<String, CodegenError> {
        let left_val = self.generate_expression(&binary.left)?;
        let right_val = self.generate_expression(&binary.right)?;
        let result = self.new_label("binop");
        
        match binary.op {
            BinaryOp::Assign => {
                // 赋值操作：将右值存储回左边的变量槽
                // 左操作数通常是标识符（变量名）或成员访问（结构体字段）
                match &*binary.left {
                    Expr::Identifier(ident) => {
                        let var_name = ident.name.clone();
                        if let Some(alloca) = self.variables.get(&var_name).cloned() {
                            let var_type = self.variable_types.get(&var_name)
                                .cloned()
                                .unwrap_or_else(|| {
                                    let inferred_type = self.infer_expression_type(&binary.right);
                                    // 更新变量类型
                                    self.variable_types.insert(var_name.clone(), inferred_type.clone());
                                    inferred_type
                                });

                            // 获取右值的实际类型（从 variable_types 中查找，比 infer 更准确）
                            let right_actual_type = self.variable_types.get(&right_val)
                                .cloned()
                                .unwrap_or_else(|| self.infer_expression_type(&binary.right));



                            // 当右值实际是结构体指针（如 sret 调用返回的 agg_slot）但目标变量是结构体值时，
                            // 需要从指针加载结构体值
                            let final_val = if right_actual_type.ends_with('*') && var_type.starts_with("%struct.") {
                                self.generate_type_conversion(&right_val, &right_actual_type, &var_type)
                            } else if right_actual_type != var_type {
                                self.generate_type_conversion(&right_val, &right_actual_type, &var_type)
                            } else {
                                right_val.clone()
                            };

                            // 生成 store 指令将结果写回变量槽
                            self.emit(&format!("    store {} %{}, {}* %{}", var_type, final_val, var_type, alloca));
                        }
                    }
                    Expr::MemberAccess(member) => {
                        let field_name = &member.member;
                        // 关键：对于结构体字段赋值，必须用原始 alloca 指针做 GEP
                        let (ptr_val, actual_type, field_offset, field_llvm_type) =
                            if let Expr::Identifier(ident) = &*member.object {
                                if let Some(alloca) = self.variables.get(&ident.name) {
                                    let var_type = self.variable_types.get(&ident.name).cloned().unwrap_or_else(|| "i64".to_string());
                                    if var_type.starts_with("%struct.") {
                                        let (off, ftype) = self.calculate_field_offset_and_type(&var_type, field_name);
                                        (alloca.clone(), var_type, off, ftype)
                                    } else {
                                        let ov = self.generate_expression(&member.object)?;
                                        let (off, ftype) = self.calculate_field_offset_and_type(&var_type, field_name);
                                        (ov, var_type, off, ftype)
                                    }
                                } else {
                                    let ov = self.generate_expression(&member.object)?;
                                    let at = self.infer_expression_type(&member.object);
                                    let (off, ftype) = self.calculate_field_offset_and_type(&at, field_name);
                                    (ov, at, off, ftype)
                                }
                            } else {
                                let ov = self.generate_expression(&member.object)?;
                                let at = self.variable_types.get(&ov).cloned().unwrap_or_else(|| self.infer_expression_type(&member.object));
                                let (off, ftype) = self.calculate_field_offset_and_type(&at, field_name);
                                (ov, at, off, ftype)
                            };

                        let ptr_as_i8 = if actual_type.starts_with("%struct.") {
                            let cast = self.new_label("struct_to_i8");
                            let base_type = actual_type.trim_end_matches('*');
                            self.emit(&format!("    %{} = bitcast {}* %{} to i8*", cast, base_type, ptr_val));
                            cast
                        } else {
                            ptr_val.clone()
                        };
                        
                        let gep = self.new_label("assign_gep");
                        self.emit(&format!("    %{} = getelementptr i8, i8* %{}, i32 {}", gep, ptr_as_i8, field_offset));
                        let typed_ptr = self.new_label("assign_typed");
                        self.emit(&format!("    %{} = bitcast i8* %{} to {}*", typed_ptr, gep, field_llvm_type));
                        
                        let right_type = self.infer_expression_type(&binary.right);
                        if field_llvm_type.starts_with("%struct.") {
                            if right_type.ends_with('*') || right_type == "i64" {
                                self.emit(&format!("    store {}* %{}, {}* %{}", field_llvm_type, right_val, field_llvm_type, typed_ptr));
                            } else {
                                self.emit(&format!("    store {} %{}, {}* %{}", field_llvm_type, right_val, field_llvm_type, typed_ptr));
                            }
                        } else {
                            let final_val = if right_type != field_llvm_type {
                                self.generate_type_conversion(&right_val, &right_type, &field_llvm_type)
                            } else { right_val.clone() };
                            self.emit(&format!("    store {} %{}, {}* %{}", field_llvm_type, final_val, field_llvm_type, typed_ptr));
                        }
                    }
                    _ => {}
                }
                // 返回右值
                Ok(right_val)
            }
            BinaryOp::AddAssign | BinaryOp::SubAssign | BinaryOp::MulAssign | BinaryOp::DivAssign | BinaryOp::RemAssign => {
                // 复合赋值：先执行运算，然后将结果写回变量
                let op_result = self.new_label("compound_op");
                let op_type = match binary.op {
                    BinaryOp::AddAssign => BinaryOp::Add,
                    BinaryOp::SubAssign => BinaryOp::Sub,
                    BinaryOp::MulAssign => BinaryOp::Mul,
                    BinaryOp::DivAssign => BinaryOp::Div,
                    BinaryOp::RemAssign => BinaryOp::Rem,
                    _ => unreachable!(),
                };
                
                // 生成运算
                let left_type = self.infer_expression_type(&binary.left);
                let right_type = self.infer_expression_type(&binary.right);
                
                match op_type {
                    BinaryOp::Add => {
                        if left_type == "i8*" || right_type == "i8*" {
                            let l_val = if left_type != "i8*" { self.generate_type_conversion(&left_val, &left_type, "i8*") } else { left_val.clone() };
                            let r_val = if right_type != "i8*" { self.generate_type_conversion(&right_val, &right_type, "i8*") } else { right_val.clone() };
                            self.emit(&format!("    %{} = call i8* @rt_str_concat(i8* %{}, i8* %{})", op_result, l_val, r_val));
                            self.variable_types.insert(op_result.clone(), "i8*".to_string());
                        } else {
                            self.emit(&format!("    %{} = add i64 %{}, %{}", op_result, left_val, right_val));
                            self.variable_types.insert(op_result.clone(), "i64".to_string());
                        }
                    }
                    BinaryOp::Sub => {
                        if left_type == "double" || right_type == "double" {
                            let l_val = if left_type != "double" { self.generate_type_conversion(&left_val, &left_type, "double") } else { left_val.clone() };
                            let r_val = if right_type != "double" { self.generate_type_conversion(&right_val, &right_type, "double") } else { right_val.clone() };
                            self.emit(&format!("    %{} = fsub double %{}, %{}", op_result, l_val, r_val));
                            self.variable_types.insert(op_result.clone(), "double".to_string());
                        } else {
                            self.emit(&format!("    %{} = sub i64 %{}, %{}", op_result, left_val, right_val));
                            self.variable_types.insert(op_result.clone(), "i64".to_string());
                        }
                    }
                    BinaryOp::Mul => {
                        if left_type == "double" || right_type == "double" {
                            let l_val = if left_type != "double" { self.generate_type_conversion(&left_val, &left_type, "double") } else { left_val.clone() };
                            let r_val = if right_type != "double" { self.generate_type_conversion(&right_val, &right_type, "double") } else { right_val.clone() };
                            self.emit(&format!("    %{} = fmul double %{}, %{}", op_result, l_val, r_val));
                            self.variable_types.insert(op_result.clone(), "double".to_string());
                        } else {
                            self.emit(&format!("    %{} = mul i64 %{}, %{}", op_result, left_val, right_val));
                            self.variable_types.insert(op_result.clone(), "i64".to_string());
                        }
                    }
                    BinaryOp::Div => {
                        if left_type == "double" || right_type == "double" {
                            let l_val = if left_type != "double" { self.generate_type_conversion(&left_val, &left_type, "double") } else { left_val.clone() };
                            let r_val = if right_type != "double" { self.generate_type_conversion(&right_val, &right_type, "double") } else { right_val.clone() };
                            self.emit(&format!("    %{} = fdiv double %{}, %{}", op_result, l_val, r_val));
                            self.variable_types.insert(op_result.clone(), "double".to_string());
                        } else {
                            self.emit(&format!("    %{} = sdiv i64 %{}, %{}", op_result, left_val, right_val));
                            self.variable_types.insert(op_result.clone(), "i64".to_string());
                        }
                    }
                    BinaryOp::Rem => {
                        self.emit(&format!("    %{} = srem i64 %{}, %{}", op_result, left_val, right_val));
                        self.variable_types.insert(op_result.clone(), "i64".to_string());
                    }
                    _ => {}
                }
                
                // 将结果写回变量
                match &*binary.left {
                    Expr::Identifier(ident) => {
                        let var_name = ident.name.clone();
                        if let Some(alloca) = self.variables.get(&var_name).cloned() {
                            let var_type = self.variable_types.get(&op_result).cloned().unwrap_or("i64".to_string());
                            self.emit(&format!("    store {} %{}, {}* %{}", var_type, op_result, var_type, alloca));
                            self.variable_types.insert(var_name, var_type);
                        }
                    }
                    _ => {}
                }
                
                Ok(op_result)
            }
            BinaryOp::Add => {
                // 检查操作数类型：如果是字符串(i8*)则使用字符串拼接
                let left_type = self.infer_expression_type(&binary.left);
                let right_type = self.infer_expression_type(&binary.right);

                if left_type == "i8*" || right_type == "i8*" {
                    // 字符串拼接：调用 rt_str_concat
                    // 确保两个操作数都是 i8*
                    let l_val = if left_type != "i8*" {
                        let conv = self.generate_type_conversion(&left_val, &left_type, "i8*");
                        conv
                    } else {
                        left_val.clone()
                    };
                    let r_val = if right_type != "i8*" {
                        let conv = self.generate_type_conversion(&right_val, &right_type, "i8*");
                        conv
                    } else {
                        right_val.clone()
                    };
                    self.emit(&format!("    %{} = call i8* @rt_str_concat(i8* %{}, i8* %{})", result, l_val, r_val));
                    self.variable_types.insert(result.clone(), "i8*".to_string());
                } else {
                    self.emit(&format!("    %{} = add i64 %{}, %{}", result, left_val, right_val));
                    self.variable_types.insert(result.clone(), "i64".to_string());
                }
                Ok(result)
            }
            BinaryOp::Sub => {
                let left_type = self.infer_expression_type(&binary.left);
                let right_type = self.infer_expression_type(&binary.right);
                if left_type == "double" || right_type == "double" {
                    let l_val = if left_type != "double" { self.generate_type_conversion(&left_val, &left_type, "double") } else { left_val.clone() };
                    let r_val = if right_type != "double" { self.generate_type_conversion(&right_val, &right_type, "double") } else { right_val.clone() };
                    self.emit(&format!("    %{} = fsub double %{}, %{}", result, l_val, r_val));
                    self.variable_types.insert(result.clone(), "double".to_string());
                } else {
                    self.emit(&format!("    %{} = sub i64 %{}, %{}", result, left_val, right_val));
                    self.variable_types.insert(result.clone(), "i64".to_string());
                }
                Ok(result)
            }
            BinaryOp::Mul => {
                let left_type = self.infer_expression_type(&binary.left);
                let right_type = self.infer_expression_type(&binary.right);
                if left_type == "double" || right_type == "double" {
                    let l_val = if left_type != "double" { self.generate_type_conversion(&left_val, &left_type, "double") } else { left_val.clone() };
                    let r_val = if right_type != "double" { self.generate_type_conversion(&right_val, &right_type, "double") } else { right_val.clone() };
                    self.emit(&format!("    %{} = fmul double %{}, %{}", result, l_val, r_val));
                    self.variable_types.insert(result.clone(), "double".to_string());
                } else {
                    self.emit(&format!("    %{} = mul i64 %{}, %{}", result, left_val, right_val));
                    self.variable_types.insert(result.clone(), "i64".to_string());
                }
                Ok(result)
            }
            BinaryOp::Div => {
                let left_type = self.infer_expression_type(&binary.left);
                let right_type = self.infer_expression_type(&binary.right);
                if left_type == "double" || right_type == "double" {
                    let l_val = if left_type != "double" { self.generate_type_conversion(&left_val, &left_type, "double") } else { left_val.clone() };
                    let r_val = if right_type != "double" { self.generate_type_conversion(&right_val, &right_type, "double") } else { right_val.clone() };
                    self.emit(&format!("    %{} = fdiv double %{}, %{}", result, l_val, r_val));
                    self.variable_types.insert(result.clone(), "double".to_string());
                } else {
                    self.emit(&format!("    %{} = sdiv i64 %{}, %{}", result, left_val, right_val));
                    self.variable_types.insert(result.clone(), "i64".to_string());
                }
                Ok(result)
            }
            BinaryOp::Rem => {
                self.emit(&format!("    %{} = srem i64 %{}, %{}", result, left_val, right_val));
                self.variable_types.insert(result.clone(), "i64".to_string());
                Ok(result)
            }
            BinaryOp::Eq => {
                // 优先从实际值的 variable_types 获取类型（比 infer_expression_type 更准确）
                let left_type = self.variable_types.get(&left_val).cloned()
                    .unwrap_or_else(|| self.infer_expression_type(&binary.left));
                let right_type = self.variable_types.get(&right_val).cloned()
                    .unwrap_or_else(|| self.infer_expression_type(&binary.right));
                if left_type == "i8*" || right_type == "i8*" {
                    let l_val = if left_type != "i8*" { self.generate_type_conversion(&left_val, &left_type, "i8*") } else { left_val.clone() };
                    let r_val = if right_type != "i8*" { self.generate_type_conversion(&right_val, &right_type, "i8*") } else { right_val.clone() };
                    let tmp = self.new_label("tmp"); self.emit(&format!("    %{} = call i64 @rt_str_eq(i8* %{}, i8* %{})", tmp, l_val, r_val));
                    self.emit(&format!("    %{} = icmp ne i64 %{}, 0", result, tmp));
                } else if left_type == "double" || right_type == "double" {
                    let l_val = if left_type != "double" { self.generate_type_conversion(&left_val, &left_type, "double") } else { left_val.clone() };
                    let r_val = if right_type != "double" { self.generate_type_conversion(&right_val, &right_type, "double") } else { right_val.clone() };
                    self.emit(&format!("    %{} = fcmp oeq double %{}, %{}", result, l_val, r_val));
                } else if left_type.starts_with("%struct.") || right_type.starts_with("%struct.") {
                    // 结构体比较：转换为指针比较
                    let l_ptr = if left_type.starts_with("%struct.") {
                        let conv = self.new_label("left_ptr");
                        self.emit(&format!("    %{} = alloca {}, align 8", conv, left_type));
                        self.emit(&format!("    store {} %{}, {}* %{}", left_type, left_val, left_type, conv));
                        let ptr = self.new_label("left_ptr_cast");
                        self.emit(&format!("    %{} = bitcast {}* %{} to i8*", ptr, left_type, conv));
                        ptr
                    } else if left_type == "i64" {
                        // 整数转指针
                        let ptr = self.new_label("left_ptr");
                        self.emit(&format!("    %{} = inttoptr i64 %{} to i8*", ptr, left_val));
                        ptr
                    } else {
                        left_val.clone()
                    };
                    let r_ptr = if right_type.starts_with("%struct.") {
                        let conv = self.new_label("right_ptr");
                        self.emit(&format!("    %{} = alloca {}, align 8", conv, right_type));
                        self.emit(&format!("    store {} %{}, {}* %{}", right_type, right_val, right_type, conv));
                        let ptr = self.new_label("right_ptr_cast");
                        self.emit(&format!("    %{} = bitcast {}* %{} to i8*", ptr, right_type, conv));
                        ptr
                    } else if right_type == "i64" {
                        // 整数转指针
                        let ptr = self.new_label("right_ptr");
                        self.emit(&format!("    %{} = inttoptr i64 %{} to i8*", ptr, right_val));
                        ptr
                    } else {
                        right_val.clone()
                    };
                    self.emit(&format!("    %{} = icmp eq i8* %{}, %{}", result, l_ptr, r_ptr));
                } else {
                    self.emit(&format!("    %{} = icmp eq i64 %{}, %{}", result, left_val, right_val));
                }
                self.variable_types.insert(result.clone(), "i1".to_string());
                Ok(result)
            }
            BinaryOp::Ne => {
                let left_type = self.variable_types.get(&left_val).cloned()
                    .unwrap_or_else(|| self.infer_expression_type(&binary.left));
                let right_type = self.variable_types.get(&right_val).cloned()
                    .unwrap_or_else(|| self.infer_expression_type(&binary.right));
                if left_type == "i8*" || right_type == "i8*" {
                    let l_val = if left_type != "i8*" { self.generate_type_conversion(&left_val, &left_type, "i8*") } else { left_val.clone() };
                    let r_val = if right_type != "i8*" { self.generate_type_conversion(&right_val, &right_type, "i8*") } else { right_val.clone() };
                    let tmp = self.new_label("tmp"); self.emit(&format!("    %{} = call i64 @rt_str_ne(i8* %{}, i8* %{})", tmp, l_val, r_val));
                    self.emit(&format!("    %{} = icmp ne i64 %{}, 0", result, tmp));
                } else if left_type == "double" || right_type == "double" {
                    let l_val = if left_type != "double" { self.generate_type_conversion(&left_val, &left_type, "double") } else { left_val.clone() };
                    let r_val = if right_type != "double" { self.generate_type_conversion(&right_val, &right_type, "double") } else { right_val.clone() };
                    self.emit(&format!("    %{} = fcmp one double %{}, %{}", result, l_val, r_val));
                } else if left_type.starts_with("%struct.") || right_type.starts_with("%struct.") {
                    // 结构体比较：转换为指针比较
                    let l_ptr = if left_type.starts_with("%struct.") {
                        let conv = self.new_label("left_ptr");
                        self.emit(&format!("    %{} = alloca {}, align 8", conv, left_type));
                        self.emit(&format!("    store {} %{}, {}* %{}", left_type, left_val, left_type, conv));
                        let ptr = self.new_label("left_ptr_cast");
                        self.emit(&format!("    %{} = bitcast {}* %{} to i8*", ptr, left_type, conv));
                        ptr
                    } else if left_type == "i64" {
                        // 整数转指针
                        let ptr = self.new_label("left_ptr");
                        self.emit(&format!("    %{} = inttoptr i64 %{} to i8*", ptr, left_val));
                        ptr
                    } else {
                        left_val.clone()
                    };
                    let r_ptr = if right_type.starts_with("%struct.") {
                        let conv = self.new_label("right_ptr");
                        self.emit(&format!("    %{} = alloca {}, align 8", conv, right_type));
                        self.emit(&format!("    store {} %{}, {}* %{}", right_type, right_val, right_type, conv));
                        let ptr = self.new_label("right_ptr_cast");
                        self.emit(&format!("    %{} = bitcast {}* %{} to i8*", ptr, right_type, conv));
                        ptr
                    } else if right_type == "i64" {
                        // 整数转指针
                        let ptr = self.new_label("right_ptr");
                        self.emit(&format!("    %{} = inttoptr i64 %{} to i8*", ptr, right_val));
                        ptr
                    } else {
                        right_val.clone()
                    };
                    self.emit(&format!("    %{} = icmp ne i8* %{}, %{}", result, l_ptr, r_ptr));
                } else {
                    self.emit(&format!("    %{} = icmp ne i64 %{}, %{}", result, left_val, right_val));
                }
                self.variable_types.insert(result.clone(), "i1".to_string());
                Ok(result)
            }
            BinaryOp::Lt => {
                let left_type = self.infer_expression_type(&binary.left);
                let right_type = self.infer_expression_type(&binary.right);
                if left_type == "i8*" || right_type == "i8*" {
                    let l_val = if left_type != "i8*" { self.generate_type_conversion(&left_val, &left_type, "i8*") } else { left_val.clone() };
                    let r_val = if right_type != "i8*" { self.generate_type_conversion(&right_val, &right_type, "i8*") } else { right_val.clone() };
                    let tmp = self.new_label("tmp"); self.emit(&format!("    %{} = call i64 @rt_str_lt(i8* %{}, i8* %{})", tmp, l_val, r_val));
                    self.emit(&format!("    %{} = icmp ne i64 %{}, 0", result, tmp));
                } else if left_type == "double" || right_type == "double" {
                    let l_val = if left_type != "double" { self.generate_type_conversion(&left_val, &left_type, "double") } else { left_val.clone() };
                    let r_val = if right_type != "double" { self.generate_type_conversion(&right_val, &right_type, "double") } else { right_val.clone() };
                    self.emit(&format!("    %{} = fcmp olt double %{}, %{}", result, l_val, r_val));
                } else {
                    self.emit(&format!("    %{} = icmp slt i64 %{}, %{}", result, left_val, right_val));
                }
                self.variable_types.insert(result.clone(), "i1".to_string());
                Ok(result)
            }
            BinaryOp::Le => {
                let left_type = self.infer_expression_type(&binary.left);
                let right_type = self.infer_expression_type(&binary.right);
                if left_type == "i8*" || right_type == "i8*" {
                    let l_val = if left_type != "i8*" { self.generate_type_conversion(&left_val, &left_type, "i8*") } else { left_val.clone() };
                    let r_val = if right_type != "i8*" { self.generate_type_conversion(&right_val, &right_type, "i8*") } else { right_val.clone() };
                    let tmp = self.new_label("tmp"); self.emit(&format!("    %{} = call i64 @rt_str_le(i8* %{}, i8* %{})", tmp, l_val, r_val));
                    self.emit(&format!("    %{} = icmp ne i64 %{}, 0", result, tmp));
                } else if left_type == "double" || right_type == "double" {
                    let l_val = if left_type != "double" { self.generate_type_conversion(&left_val, &left_type, "double") } else { left_val.clone() };
                    let r_val = if right_type != "double" { self.generate_type_conversion(&right_val, &right_type, "double") } else { right_val.clone() };
                    self.emit(&format!("    %{} = fcmp ole double %{}, %{}", result, l_val, r_val));
                } else {
                    self.emit(&format!("    %{} = icmp sle i64 %{}, %{}", result, left_val, right_val));
                }
                self.variable_types.insert(result.clone(), "i1".to_string());
                Ok(result)
            }
            BinaryOp::Gt => {
                let left_type = self.infer_expression_type(&binary.left);
                let right_type = self.infer_expression_type(&binary.right);
                if left_type == "i8*" || right_type == "i8*" {
                    let l_val = if left_type != "i8*" { self.generate_type_conversion(&left_val, &left_type, "i8*") } else { left_val.clone() };
                    let r_val = if right_type != "i8*" { self.generate_type_conversion(&right_val, &right_type, "i8*") } else { right_val.clone() };
                    let tmp = self.new_label("tmp"); self.emit(&format!("    %{} = call i64 @rt_str_gt(i8* %{}, i8* %{})", tmp, l_val, r_val));
                    self.emit(&format!("    %{} = icmp ne i64 %{}, 0", result, tmp));
                } else if left_type == "double" || right_type == "double" {
                    let l_val = if left_type != "double" { self.generate_type_conversion(&left_val, &left_type, "double") } else { left_val.clone() };
                    let r_val = if right_type != "double" { self.generate_type_conversion(&right_val, &right_type, "double") } else { right_val.clone() };
                    self.emit(&format!("    %{} = fcmp ogt double %{}, %{}", result, l_val, r_val));
                } else {
                    self.emit(&format!("    %{} = icmp sgt i64 %{}, %{}", result, left_val, right_val));
                }
                self.variable_types.insert(result.clone(), "i1".to_string());
                Ok(result)
            }
            BinaryOp::Ge => {
                let left_type = self.infer_expression_type(&binary.left);
                let right_type = self.infer_expression_type(&binary.right);
                if left_type == "i8*" || right_type == "i8*" {
                    let l_val = if left_type != "i8*" { self.generate_type_conversion(&left_val, &left_type, "i8*") } else { left_val.clone() };
                    let r_val = if right_type != "i8*" { self.generate_type_conversion(&right_val, &right_type, "i8*") } else { right_val.clone() };
                    let tmp = self.new_label("tmp"); self.emit(&format!("    %{} = call i64 @rt_str_ge(i8* %{}, i8* %{})", tmp, l_val, r_val));
                    self.emit(&format!("    %{} = icmp ne i64 %{}, 0", result, tmp));
                } else if left_type == "double" || right_type == "double" {
                    let l_val = if left_type != "double" { self.generate_type_conversion(&left_val, &left_type, "double") } else { left_val.clone() };
                    let r_val = if right_type != "double" { self.generate_type_conversion(&right_val, &right_type, "double") } else { right_val.clone() };
                    self.emit(&format!("    %{} = fcmp oge double %{}, %{}", result, l_val, r_val));
                } else {
                    self.emit(&format!("    %{} = icmp sge i64 %{}, %{}", result, left_val, right_val));
                }
                self.variable_types.insert(result.clone(), "i1".to_string());
                Ok(result)
            }
            BinaryOp::And => {
                // 逻辑与操作：使用 i1 类型
                self.emit(&format!("    %{} = and i1 %{}, %{}", result, left_val, right_val));
                self.variable_types.insert(result.clone(), "i1".to_string());
                Ok(result)
            }
            BinaryOp::Or => {
                // 逻辑或操作：使用 i1 类型
                self.emit(&format!("    %{} = or i1 %{}, %{}", result, left_val, right_val));
                self.variable_types.insert(result.clone(), "i1".to_string());
                Ok(result)
            }
            BinaryOp::BitAnd => {
                self.emit(&format!("    %{} = and i64 %{}, %{}", result, left_val, right_val));
                self.variable_types.insert(result.clone(), "i64".to_string());
                Ok(result)
            }
            BinaryOp::BitOr => {
                self.emit(&format!("    %{} = or i64 %{}, %{}", result, left_val, right_val));
                self.variable_types.insert(result.clone(), "i64".to_string());
                Ok(result)
            }
            BinaryOp::BitXor => {
                self.emit(&format!("    %{} = xor i64 %{}, %{}", result, left_val, right_val));
                self.variable_types.insert(result.clone(), "i64".to_string());
                Ok(result)
            }
            BinaryOp::Shl => {
                self.emit(&format!("    %{} = shl i64 %{}, %{}", result, left_val, right_val));
                self.variable_types.insert(result.clone(), "i64".to_string());
                Ok(result)
            }
            BinaryOp::Shr => {
                self.emit(&format!("    %{} = ashr i64 %{}, %{}", result, left_val, right_val));
                self.variable_types.insert(result.clone(), "i64".to_string());
                Ok(result)
            }
            BinaryOp::Hash => {
                // 哈希运算：简单返回左值
                Ok(left_val)
            }
        }
    }

    /**
     * 生成一元表达式
     */
    fn generate_unary_expr(&mut self, unary: &UnaryExpr) -> Result<String, CodegenError> {
        let expr_val = self.generate_expression(&unary.operand)?;
        let result = self.new_label("unop");
        
        match unary.op {
            UnaryOp::Neg => {
                self.emit(&format!("    %{} = sub i64 0, %{}", result, expr_val));
            }
            UnaryOp::Not => {
                self.emit(&format!("    %{} = xor i64 %{}, 1", result, expr_val));
            }
            UnaryOp::BitNot => {
                self.emit(&format!("    %{} = xor i64 %{}, -1", result, expr_val));
            }
        }
        
        Ok(result)
    }

    /**
     * 判断函数返回类型
     * 返回 Some("i8*") 表示返回指针，Some("i64") 表示返回整数，None 表示未知
     */
    fn get_func_return_type(&self, func_name: &str) -> Option<String> {
        // 首先检查用户定义函数签名
        if let Some((_, return_type)) = self.user_functions.get(func_name) {
            return Some(return_type.clone());
        }
        
        // 然后检查外部函数签名
        if let Some((_, return_type)) = self.extern_functions.get(func_name) {
            return Some(return_type.clone());
        }
        
        // 如果带参数类型的函数名找不到，尝试查找不带参数类型的版本
        // 函数名格式可能是：base_name_i8ptr_i64_i64
        // 使用与 simple_func_name 相同的逻辑提取基础函数名
        if func_name.contains('_') {
            let parts: Vec<&str> = func_name.split('_').collect();
            let type_suffixes = ["i64", "i8ptr", "double", "void"];
            let mut result_parts = Vec::new();
            for part in parts {
                if type_suffixes.contains(&part) {
                    continue;
                }
                result_parts.push(part);
            }
            let simple_func_name = result_parts.join("_");
            self.match_builtin_return_type(&simple_func_name)
        } else {
            self.match_builtin_return_type(func_name)
        }
    }
    
    fn match_builtin_return_type(&self, func_name: &str) -> Option<String> {
        match func_name {
            // 返回 i8* 的函数
            "rt_list_new" | "列表" => Some("i8*".to_string()),
            "rt_list_get" | "列表获取" => Some("i8*".to_string()),
            "rt_str_new" => Some("i8*".to_string()),
            "rt_str_concat" | "rt_string_concat" | "文本拼接" => Some("i8*".to_string()),
            "rt_string_substring" => Some("i8*".to_string()),
            "rt_string_slice" => Some("i8*".to_string()),
            "rt_readline" | "读取行" => Some("i8*".to_string()),
            "rt_malloc" => Some("i8*".to_string()),
            "str_contains" => Some("i8*".to_string()),
            // 返回 i64 的函数
            "rt_list_len" | "列表长度" => Some("i64".to_string()),
            "rt_string_len" | "文本长度" => Some("i64".to_string()),
            "rt_utf8_byte_length" | "utf8字节长度" => Some("i64".to_string()),
            "rt_string_indexOf" | "文本查找" => Some("i64".to_string()),
            "rt_is_utf8_leader" | "是utf8首字节" => Some("i64".to_string()),
            "rt_is_utf8_continuation" | "是utf8续字节" => Some("i64".to_string()),
            "rt_string_char_at" | "文本取字符" => Some("i8*".to_string()),
            "rt_char_to_code" | "字符编码" => Some("i64".to_string()),
            "rt_code_to_char" | "编码转字符" => Some("i8*".to_string()),
            "rt_str_to_int" | "文本转整数" => Some("i64".to_string()),
            "rt_int_to_str" | "整数转文本" => Some("i8*".to_string()),
            "rt_string_fromChar" => Some("i8*".to_string()),
            "argv" => Some("i8*".to_string()),
            "argc" | "参数个数" => Some("i64".to_string()),
            "file_read" | "读取文件" => Some("i8*".to_string()),
            "file_write" | "写入文件" => Some("i32".to_string()),
            "file_exists" | "文件存在" => Some("i32".to_string()),
            "exec_cmd" | "执行命令" => Some("i32".to_string()),
            "print_int" | "打印整数" => Some("i64".to_string()),
            "str_to_int" => Some("i64".to_string()),
            // 无返回的函数
            "rt_list_append" | "列表追加" => None,
            "rt_list_set" | "列表设置" => None,
            "rt_print" | "打印" => None,
            "rt_println" | "打印行" => None,
            "rt_error" | "报错" => None,
            "rt_free" => None,
            _ => None,
        }
    }

    /**
     * 判断函数参数类型
     * 返回参数的类型列表
     */
    fn get_func_param_types(&self, func_name: &str) -> Vec<String> {
        // 首先检查用户定义函数签名
        if let Some((param_types, _)) = self.user_functions.get(func_name) {
            return param_types.clone();
        }
        
        // 然后检查外部函数签名
        if let Some((param_types, _)) = self.extern_functions.get(func_name) {
            return param_types.clone();
        }
        
        // 如果带参数类型的函数名找不到，尝试查找不带参数类型的版本
        // 函数名格式可能是：base_name_i8ptr_i64_i64
        // 需要逐步去掉参数类型后缀来查找基础函数名
        let mut current_name = func_name;
        loop {
            if let Some((param_types, _)) = self.user_functions.get(current_name) {
                return param_types.clone();
            }
            if let Some((param_types, _)) = self.extern_functions.get(current_name) {
                return param_types.clone();
            }
            // 尝试去掉最后一个下划线及其后面的内容
            if let Some(underscore_pos) = current_name.rfind('_') {
                current_name = &current_name[..underscore_pos];
            } else {
                break;
            }
        }
        
        // 使用简化后的函数名进行内置函数匹配
        match current_name {
            "rt_list_append" => vec!["i8*".to_string(), "i8*".to_string()],
            "rt_list_set" => vec!["i8*".to_string(), "i64".to_string(), "i8*".to_string()],
            "rt_list_get" => vec!["i8*".to_string(), "i64".to_string()],
            "rt_list_len" => vec!["i8*".to_string()],
            "rt_str_concat" => vec!["i8*".to_string(), "i8*".to_string()],
            "rt_string_concat" => vec!["i8*".to_string(), "i8*".to_string()],
            "rt_string_substring" | "rt_string_indexOf" | "文本查找" => vec!["i8*".to_string(), "i8*".to_string()],
            "rt_string_slice" => vec!["i8*".to_string(), "i64".to_string(), "i64".to_string()],
            "rt_string_len" => vec!["i8*".to_string()],
            // UTF-8 helper functions
            "rt_utf8_byte_length" => vec!["i64".to_string()],
            "rt_is_utf8_leader" => vec!["i64".to_string()],
            "rt_is_utf8_continuation" => vec!["i64".to_string()],
            "rt_str_new" => vec!["i8*".to_string()],
            "rt_print" => vec!["i8*".to_string()],
            "rt_println" => vec!["i8*".to_string()],
            "rt_error" => vec!["i8*".to_string()],
            "rt_malloc" => vec!["i64".to_string()],
            "rt_free" => vec!["i8*".to_string()],
            "print_int" => vec!["i64".to_string()],
            "str_to_int" => vec!["i8*".to_string()],
            "rt_str_to_int" => vec!["i8*".to_string()],
            "rt_string_fromChar" => vec!["i64".to_string()],
            "rt_int_to_str" => vec!["i64".to_string()],
            "rt_float_to_str" => vec!["double".to_string()],
            // 命令行参数函数
            "argv" => vec!["i64".to_string()],
            "argc" => vec![],
            // 文件操作函数
            "file_read" => vec!["i8*".to_string()],
            "file_write" => vec!["i8*".to_string(), "i8*".to_string()],
            "file_exists" => vec!["i8*".to_string()],
            "exec_cmd" => vec!["i8*".to_string()],
            "删除文件" => vec!["i8*".to_string()],
            _ => vec![],
        }
    }

    /**
     * 生成函数调用表达式
     */
    fn generate_call_expr(&mut self, call: &CallExpr) -> Result<String, CodegenError> {
        // 获取函数名和是否间接调用
        let (func_name, is_indirect, is_func_local_var) = match &*call.function {
            Expr::Identifier(ident) => {
                let def_name = self.translate_def_name(&ident.name);
                // 检查是否是内置函数（如列表操作函数）
                let is_builtin = matches!(ident.name.as_str(), 
                    "列表追加" | "列表获取" | "列表长度" | "列表设置" | "rt_list_append" | "rt_list_get" | "rt_list_len" | "rt_list_set"
                );
                // 检查是否包含列表操作相关的Unicode字符
                let has_list_chars = ident.name.chars().any(|c| c == '列' || c == '表' || c == '追' || c == '加' || c == '取' || c == '长' || c == '设' || c == '置');
                // 检查是否确实是局部变量（而非全局函数名）
                // 注意：user_functions 中存储的是带模块名前缀的函数名，所以需要生成完整函数名来检查
                let arg_types: Vec<String> = call.arguments
                    .iter()
                    .map(|arg| self.infer_expression_type(arg))
                    .collect();
                let arg_count = call.arguments.len();
                
                let mut full_func_name = String::new();
                
                for (name, (params, _)) in &self.user_functions {
                    if params.len() == arg_count {
                        let name_lower = name.to_lowercase();
                        let def_name_lower = def_name.to_lowercase();
                        
                        if name_lower.contains(&def_name_lower) || def_name_lower.contains(&name_lower) {
                            full_func_name = name.clone();
                            break;
                        }
                    }
                }
                
                if full_func_name.is_empty() {
                    if is_builtin {
                        // 内置函数直接使用原始函数名
                        full_func_name = def_name.clone();
                    } else {
                        // 生成参数类型后缀，与函数定义时保持一致
                        let sanitized_types: Vec<String> = arg_types
                            .iter()
                            .map(|t| {
                                let mut simplified = t.clone();
                                if simplified.starts_with("%struct.") {
                                    simplified = simplified.trim_start_matches("%struct.").to_string();
                                }
                                simplified.replace("*", "ptr").replace("%", "struct_").replace(".", "_")
                            })
                            .collect();
                        let param_suffix = sanitized_types.join("_");
                        if param_suffix.is_empty() {
                            full_func_name = def_name.clone();
                        } else {
                            full_func_name = format!("{}_{}", def_name, param_suffix);
                        }
                    }
                }
                let is_local_var = !is_builtin && !has_list_chars && (
                    self.variables.contains_key(&def_name)
                    || self.variables.contains_key(&ident.name)
                );
                if is_local_var && !self.user_functions.contains_key(&full_func_name)
                    && !self.extern_functions.contains_key(&full_func_name)
                {
                    (def_name, true, true)
                } else {
                    (full_func_name, false, false)
                }
            }
            Expr::MemberAccess(member) => {
                // 处理 XY 的方法调用语法：object.method(args)
                let obj_type = self.infer_expression_type(&member.object);
                if obj_type == "i8*" {
                    // 列表方法调用：直接生成对应的 C runtime 调用
                    let list_val = self.generate_expression(&member.object)?;
                    let mut all_args = vec![list_val];
                    for arg in &call.arguments {
                        all_args.push(self.generate_expression(arg)?);
                    }
                    let result = self.new_label("call");
                    match member.member.as_str() {
                        "追加" | "append" => {
                            if all_args.len() >= 2 {
                                let a1_type = self.variable_types.get(&all_args[1]).cloned().unwrap_or("i8*".to_string());
                                let a1 = if a1_type != "i8*" { self.generate_type_conversion(&all_args[1], &a1_type, "i8*") } else { all_args[1].clone() };
                                self.emit(&format!("    call void @rt_list_append(i8* %{}, i8* %{})", all_args[0], a1));
                            }
                            return Ok(result);
                        }
                        "长度" | "length" => {
                            self.emit(&format!("    %{} = call i64 @rt_list_len(i8* %{})", result, all_args[0]));
                            self.variable_types.insert(result.clone(), "i64".to_string());
                            return Ok(result);
                        }
                        "获取" | "get" => {
                            if all_args.len() >= 2 {
                                let a1_type = self.variable_types.get(&all_args[1]).cloned().unwrap_or("i64".to_string());
                                let a1 = if a1_type != "i64" { self.generate_type_conversion(&all_args[1], &a1_type, "i64") } else { all_args[1].clone() };
                                self.emit(&format!("    %{} = call i8* @rt_list_get(i8* %{}, i64 %{})", result, all_args[0], a1));
                                self.variable_types.insert(result.clone(), "i8*".to_string());
                            }
                            return Ok(result);
                        }
                        _ => {}
                    }
                }
                let val = self.generate_expression(&call.function)?;
                (val, true, false)
            }
            _ => {
                let val = self.generate_expression(&call.function)?;
                (val, true, false)
            }
        };

        // 生成参数表达式
        let mut args = Vec::new();
        for arg in &call.arguments {
            let arg_val = self.generate_expression(arg)?;
            args.push(arg_val);
        }

        let result = self.new_label("call");

        if is_indirect {
            // 间接调用：将函数指针转换为函数类型后调用
            eprintln!("DEBUG: is_indirect=true, func_name={}", func_name);
            // 获取参数类型列表
            let arg_types: Vec<String> = call.arguments.iter()
                .map(|arg| self.infer_expression_type(arg))
                .collect();
            
            // 转换参数并生成参数类型字符串
            let converted_args: Vec<String> = args.iter().enumerate()
                .map(|(i, a)| {
                    let arg_type = if i < arg_types.len() {
                        &arg_types[i]
                    } else { "i64" };
                    if arg_type == "i8*" {
                        let conv = self.new_label("arg_conv");
                        self.emit(&format!("    %{} = ptrtoint i8* %{} to i64", conv, a));
                        format!("i64 %{}", conv)
                    } else if arg_type == "double" {
                        let conv = self.new_label("arg_conv");
                        self.emit(&format!("    %{} = fptosi double %{} to i64", conv, a));
                        format!("i64 %{}", conv)
                    } else if arg_type.starts_with("%struct.") {
                        // 结构体类型参数：转为指针
                        let conv = self.new_label("arg_conv");
                        let struct_addr = self.new_label("struct_addr");
                        self.emit(&format!("    %{} = alloca {}, align 8", struct_addr, arg_type));
                        self.emit(&format!("    store {} %{}, {}* %{}", arg_type, a, arg_type, struct_addr));
                        self.emit(&format!("    %{} = ptrtoint {}* %{} to i64", conv, arg_type, struct_addr));
                        format!("i64 %{}", conv)
                    } else {
                        format!("i64 %{}", a)
                    }
                })
                .collect();
            
            // 构建函数指针类型签名
            let param_types_str = arg_types.iter()
                .map(|t| {
                    if t == "i8*" || t == "double" || t.starts_with("%struct.") {
                        "i64".to_string()
                    } else {
                        t.clone()
                    }
                })
                .collect::<Vec<String>>()
                .join(", ");
            let func_ptr_type = if param_types_str.is_empty() {
                "i64 ()*".to_string()
            } else {
                format!("i64 ({})*", param_types_str)
            };
            
            // func_name 可能是 i8*（指针）或 i64（整数值）
            let func_int = if is_func_local_var || func_name.starts_with("id_") {
                let var_type = self.variable_types.get(&func_name)
                    .cloned()
                    .unwrap_or_else(|| "i64".to_string());
                eprintln!("DEBUG: func_name {} is local SSA value with type {}", func_name, var_type);
                if var_type == "i8*" {
                    let conv = self.new_label("func_to_int");
                    self.emit(&format!("    %{} = ptrtoint i8* %{} to i64", conv, func_name));
                    conv
                } else {
                    func_name
                }
            } else if func_name.contains("member_val") || func_name.starts_with('%') {
                let var_type = self.variable_types.get(&func_name)
                    .cloned()
                    .unwrap_or_else(|| "i64".to_string());
                eprintln!("DEBUG: func_name {} has type {}", func_name, var_type);
                if var_type == "i8*" {
                    let conv = self.new_label("func_to_int");
                    self.emit(&format!("    %{} = ptrtoint i8* %{} to i64", conv, func_name));
                    conv
                } else {
                    func_name
                }
            } else {
                let conv = self.new_label("func_to_int");
                self.emit(&format!("    %{} = ptrtoint i8* @{} to i64", conv, func_name));
                conv
            };
            let func_ptr = self.new_label("func_ptr");
            self.emit(&format!("    %{} = inttoptr i64 %{} to {}", func_ptr, func_int, func_ptr_type));
            self.emit(&format!("    %{} = call i64 %{}({})", result, func_ptr, converted_args.join(", ")));
            self.variable_types.insert(result.clone(), "i64".to_string());
            return Ok(result);
        }

        // 特殊处理内置函数（先检查是否以简单函数名开头，处理带参数类型后缀的情况）
        // 函数名格式可能是：base_name_i8ptr_i64_i64
        let simple_func_name = if func_name.contains('_') {
            let parts: Vec<&str> = func_name.split('_').collect();
            let type_suffixes = ["i64", "i8ptr", "double", "void"];
            let mut result_parts = Vec::new();
            for part in parts {
                if type_suffixes.contains(&part) {
                    // 如果是类型后缀，跳过
                    continue;
                }
                result_parts.push(part);
            }
            if result_parts.is_empty() {
                func_name.clone()
            } else {
                result_parts.join("_")
            }
        } else {
            func_name.clone()
        };

        if simple_func_name == "print" || simple_func_name == "rt_print" {
            // 打印函数 - 需要类型转换
            if !args.is_empty() {
                // 从实际值的 variable_types 获取类型（比 infer 更准确）
                let actual_type = self.variable_types.get(&args[0]).cloned()
                    .unwrap_or_else(|| self.infer_arg_type(&call.arguments[0]));
                // 如果不是 i8*，需要转换
                if actual_type != "i8*" {
                    let converted_val = self.generate_type_conversion(&args[0], &actual_type, "i8*");
                    self.emit(&format!("    call void @rt_print(i8* %{})", converted_val));
                } else {
                    self.emit(&format!("    call void @rt_print(i8* %{})", args[0]));
                }
            }
            Ok(result)
        } else if simple_func_name == "println" {
            // 打印行函数 - 需要类型转换
            if !args.is_empty() {
                let actual_type = self.variable_types.get(&args[0]).cloned()
                    .unwrap_or_else(|| self.infer_arg_type(&call.arguments[0]));
                if actual_type != "i8*" {
                    let converted_val = self.generate_type_conversion(&args[0], &actual_type, "i8*");
                    self.emit(&format!("    call void @rt_println(i8* %{})", converted_val));
                } else {
                    self.emit(&format!("    call void @rt_println(i8* %{})", args[0]));
                }
            }
            Ok(result)
        } else if simple_func_name == "error" {
            // 报错函数 - 需要类型转换
            if !args.is_empty() {
                let actual_type = self.variable_types.get(&args[0]).cloned()
                    .unwrap_or_else(|| self.infer_arg_type(&call.arguments[0]));
                if actual_type != "i8*" {
                    let converted_val = self.generate_type_conversion(&args[0], &actual_type, "i8*");
                    self.emit(&format!("    call void @rt_error(i8* %{})", converted_val));
                } else {
                    self.emit(&format!("    call void @rt_error(i8* %{})", args[0]));
                }
            }
            Ok(result)
        } else if simple_func_name == "rt_list_new" {
            // 列表创建函数，返回 i8*
            self.emit(&format!("    %{} = call i8* @rt_list_new()", result));
            self.variable_types.insert(result.clone(), "i8*".to_string());
            Ok(result)
        } else if simple_func_name == "rt_list_append" {
            // 列表追加函数
            if args.len() >= 2 {
                let a0_type = self.variable_types.get(&args[0]).cloned()
                    .unwrap_or_else(|| self.infer_arg_type(&call.arguments[0]));
                let a0 = if a0_type != "i8*" { self.generate_type_conversion(&args[0], &a0_type, "i8*") } else { args[0].clone() };
                let a1_type = self.variable_types.get(&args[1]).cloned()
                    .unwrap_or_else(|| self.infer_arg_type(&call.arguments[1]));
                let a1 = if a1_type != "i8*" { self.generate_type_conversion(&args[1], &a1_type, "i8*") } else { args[1].clone() };
                self.emit(&format!("    call void @rt_list_append(i8* %{}, i8* %{})", a0, a1));
            }
            Ok(result)
        } else if simple_func_name == "rt_list_len" {
            // 列表长度函数
            if !args.is_empty() {
                let arg_type = self.variable_types.get(&args[0]).cloned()
                    .unwrap_or_else(|| self.infer_arg_type(&call.arguments[0]));
                let arg_val = if arg_type != "i8*" {
                    self.generate_type_conversion(&args[0], &arg_type, "i8*")
                } else { args[0].clone() };
                self.emit(&format!("    %{} = call i64 @rt_list_len(i8* %{})", result, arg_val));
            } else {
                self.emit(&format!("    %{} = call i64 @rt_list_len(i8* null)", result));
            }
            self.variable_types.insert(result.clone(), "i64".to_string());
            Ok(result)
        } else if simple_func_name == "rt_list_get" {
            // 列表获取函数
            if args.len() >= 2 {
                let a0_type = self.variable_types.get(&args[0]).cloned()
                    .unwrap_or_else(|| self.infer_arg_type(&call.arguments[0]));
                let a0 = if a0_type != "i8*" { self.generate_type_conversion(&args[0], &a0_type, "i8*") } else { args[0].clone() };
                let a1_type = self.variable_types.get(&args[1]).cloned()
                    .unwrap_or_else(|| self.infer_arg_type(&call.arguments[1]));
                let a1 = if a1_type != "i64" { self.generate_type_conversion(&args[1], &a1_type, "i64") } else { args[1].clone() };
                self.emit(&format!("    %{} = call i8* @rt_list_get(i8* %{}, i64 %{})", result, a0, a1));
            } else if args.len() == 1 {
                let a0_type = self.variable_types.get(&args[0]).cloned()
                    .unwrap_or_else(|| self.infer_arg_type(&call.arguments[0]));
                let a0 = if a0_type != "i8*" { self.generate_type_conversion(&args[0], &a0_type, "i8*") } else { args[0].clone() };
                self.emit(&format!("    %{} = call i8* @rt_list_get(i8* %{}, i64 0)", result, a0));
            } else {
                self.emit(&format!("    %{} = call i8* @rt_list_get(i8* null, i64 0)", result));
            }
            self.variable_types.insert(result.clone(), "i8*".to_string());
            Ok(result)
        } else if simple_func_name == "rt_str_new" {
            // 字符串创建函数，返回 i8*
            if !args.is_empty() {
                self.emit(&format!("    %{} = call i8* @rt_str_new(i8* %{})", result, args[0]));
            } else {
                self.emit(&format!("    %{} = call i8* @rt_str_new(i8* null)", result));
            }
            self.variable_types.insert(result.clone(), "i8*".to_string());
            Ok(result)
        } else if simple_func_name == "rt_str_concat" || simple_func_name == "rt_string_concat" {
            // 字符串拼接函数，返回 i8*
            if args.len() >= 2 {
                self.emit(&format!("    %{} = call i8* @rt_str_concat(i8* %{}, i8* %{})", result, args[0], args[1]));
            } else if args.len() == 1 {
                self.emit(&format!("    %{} = call i8* @rt_str_concat(i8* %{}, i8* null)", result, args[0]));
            } else {
                self.emit(&format!("    %{} = call i8* @rt_str_concat(i8* null, i8* null)", result));
            }
            self.variable_types.insert(result.clone(), "i8*".to_string());
            Ok(result)
        } else if simple_func_name == "rt_string_len" {
            // 字符串长度函数
            if !args.is_empty() {
                self.emit(&format!("    %{} = call i64 @rt_string_len(i8* %{})", result, args[0]));
            } else {
                self.emit(&format!("    %{} = call i64 @rt_string_len(i8* null)", result));
            }
            self.variable_types.insert(result.clone(), "i64".to_string());
            Ok(result)
        } else if simple_func_name == "print_int" {
            // 打印整数函数
            if !args.is_empty() {
                self.emit(&format!("    %{} = call i64 @print_int(i64 %{})", result, args[0]));
            } else {
                self.emit(&format!("    %{} = call i64 @print_int(i64 0)", result));
            }
            Ok(result)
        } else if simple_func_name == "print_float" {
            // 打印浮点数函数
            if !args.is_empty() {
                self.emit(&format!("    %{} = call i64 @print_float(double %{})", result, args[0]));
            } else {
                self.emit(&format!("    %{} = call i64 @print_float(double 0.0)", result));
            }
            Ok(result)
        } else {
            // 普通函数调用：参数需要添加类型和百分号前缀
            // 检查是否有预定义的参数类型
            // 优先使用完整的 func_name（带参数类型后缀）来获取函数签名信息
            let param_types = self.get_func_param_types(&func_name);
            let return_type = self.get_func_return_type(&func_name).unwrap_or_else(|| "i64".to_string());

            // 生成参数列表，包含类型转换
            let mut converted_args = Vec::new();
            for (i, arg) in args.iter().enumerate() {
                // 获取期望的参数类型
                let expected_type = if i < param_types.len() {
                    param_types[i].clone()
                } else {
                    "i64".to_string()
                };

                // 对于 struct 参数，传递指针而非值
                if expected_type.starts_with("%struct.") {
                    // 查找原始 alloca 指针（通过变量名）
                    if let Expr::Identifier(ident) = &call.arguments[i] {
                        if let Some(alloca) = self.variables.get(&ident.name) {
                            converted_args.push(format!("{}* %{}", expected_type, alloca));
                            continue;
                        }
                    }
                    // 检查 arg 的实际类型：如果已经是指针，直接使用
                    let actual_type = self.variable_types.get(arg).cloned()
                        .unwrap_or_else(|| self.infer_arg_type(&call.arguments[i]));
                    if actual_type.ends_with('*') {
                        // arg 已经是指针，直接使用
                        converted_args.push(format!("{}* %{}", expected_type, arg));
                        continue;
                    }
                    // 否则需要 alloca + store 获取指针
                    let tmp = self.new_label("arg_ptr");
                    self.emit(&format!("    %{} = alloca {}, align 8", tmp, expected_type));
                    self.emit(&format!("    store {} %{}, ptr %{}", expected_type, arg, tmp));
                    converted_args.push(format!("{}* %{}", expected_type, tmp));
                    continue;
                }

                // 获取实际参数类型
                let actual_type = self.variable_types.get(arg)
                    .cloned()
                    .unwrap_or_else(|| self.infer_arg_type(&call.arguments[i]));

                if actual_type != expected_type {
                    let converted_val = self.generate_type_conversion(arg, &actual_type, &expected_type);
                    converted_args.push(format!("{} %{}", expected_type, converted_val));
                } else {
                    converted_args.push(format!("{} %{}", expected_type, arg));
                }
            }
            
            let args_str = converted_args.join(", ");
            
            // 对于内置运行时函数，直接使用原始函数名，不添加参数类型后缀
            // 因为运行时函数的声明是不带参数类型后缀的
            // 检查是否是内置函数（rt_、print、str_、file_、exec_、arg、argv、argc、init_args、error）
            let is_builtin = CodeGenerator::is_builtin_func(&simple_func_name);
            let call_func_name = if is_builtin {
                simple_func_name.clone()
            } else {
                func_name.clone()
            };

            if self.is_aggregate_llvm_type(&return_type) {
                let agg_slot = self.new_label("agg_slot");
                self.emit(&format!("    %{} = alloca {}, align 8", agg_slot, return_type));
                let mut call_args = vec![format!("{}* sret({}) %{}", return_type, return_type, agg_slot)];
                call_args.extend(converted_args.clone());
                self.emit(&format!("    call void @{}({})", call_func_name, call_args.join(", ")));
                self.variable_types.insert(agg_slot.clone(), format!("{}*", return_type));
                Ok(agg_slot)
            } else if return_type == "void" {
                self.emit(&format!("    call void @{}({})", call_func_name, converted_args.join(", ")));
                Ok(result)
            } else {
                self.emit(&format!("    %{} = call {} @{}({})", result, return_type, call_func_name, args_str));
                // 记录临时变量的类型，用于后续类型推断
                self.variable_types.insert(result.clone(), return_type.clone());
                Ok(result)
            }
        }
    }

    /**
     * 推断参数的实际类型
     */
    fn infer_arg_type(&self, arg: &Expr) -> String {
        match arg {
            Expr::Identifier(ident) => {
                let var_name = ident.name.clone();
                self.variable_types.get(&var_name)
                    .or_else(|| self.variable_types.get(&ident.name))
                    .cloned()
                    .unwrap_or_else(|| self.infer_expression_type(arg))
            }
            Expr::IndexAccess(index_access) => {
                let object_type = self.infer_expression_type(&index_access.object);
                if object_type == "i8*" {
                    // 区分列表/字符串索引：列表元素类型推断为 i8*，字符串索引为字符 i64
                    let is_list = match &*index_access.object {
                        Expr::MemberAccess(member) => {
                            let field_name = &member.member;
                            let declared_list = self.list_typed_fields.contains(field_name);
                            let name_hint = field_name == "tokens" || field_name == "children" ||
                                field_name == "items" || field_name == "errors" ||
                                field_name.ends_with("列表");
                            declared_list || name_hint
                        }
                        Expr::Identifier(ident) => {
                            ident.name.ends_with("s") || ident.name.contains("列表")
                        }
                        _ => false,
                    };
                    if is_list {
                        "i8*".to_string()
                    } else {
                        "i64".to_string()
                    }
                } else {
                    self.infer_expression_type(arg)
                }
            }
            _ => self.infer_expression_type(arg),
        }
    }

    /**
     * 生成类型转换代码
     * i64 -> i8*: 调用 rt_int_to_str
     * i8* -> i64: 调用 rt_str_to_int
     */
    fn generate_type_conversion(&mut self, val: &str, from_type: &str, to_type: &str) -> String {
        // 如果源类型是 void，说明函数没有返回值，不应该进行类型转换
        // 返回一个默认值（根据目标类型）
        if from_type == "void" {
            let result = self.new_label("default");
            if to_type == "i8*" {
                // 返回空指针
                self.emit(&format!("    %{} = inttoptr i64 0 to i8*", result));
            } else if to_type == "i64" {
                // 返回 0
                self.emit(&format!("    %{} = add i64 0, 0", result));
            } else if to_type == "double" {
                // 返回 0.0
                self.emit(&format!("    %{} = fadd double 0.0, 0.0", result));
            } else {
                // 其他类型，返回默认值
                self.emit(&format!("    %{} = add {} 0, 0", result, to_type));
            }
            return result;
        }
        
        if from_type == to_type {
            return val.to_string();
        }
        
        let result = self.new_label("conv");
        
        if from_type == "i64" && to_type == "i8*" {
            // 整数转字符串（用于打印等场景）
            self.emit(&format!("    %{} = call i8* @rt_int_to_str(i64 %{})", result, val));
        } else if from_type == "i8*" && to_type == "i64" {
            // 指针转整数
            self.emit(&format!("    %{} = ptrtoint i8* %{} to i64", result, val));
        } else if from_type == "i64" && to_type == "double" {
            // 整数转浮点
            self.emit(&format!("    %{} = sitofp i64 %{} to double", result, val));
        } else if from_type == "double" && to_type == "i64" {
            // 浮点转整数
            self.emit(&format!("    %{} = fptosi double %{} to i64", result, val));
        } else if from_type == "double" && to_type == "i8*" {
            // 浮点数转字符串
            self.emit(&format!("    %{} = call i8* @rt_float_to_str(double %{})", result, val));
        } else if from_type == "i8*" && to_type == "double" {
            // 字符串转浮点数
            self.emit(&format!("    %{} = call double @rt_str_to_double(i8* %{})", result, val));
        } else if from_type.starts_with("%struct.") && to_type == "i64" {
            // 结构体转整数：先获取指针，再转为整数
            let ptr = self.new_label("struct_ptr");
            self.emit(&format!("    %{} = alloca {}, align 8", ptr, from_type));
            self.emit(&format!("    store {} %{}, {}* %{}", from_type, val, from_type, ptr));
            self.emit(&format!("    %{} = ptrtoint {}* %{} to i64", result, from_type, ptr));
        } else if from_type == "i64" && to_type.starts_with("%struct.") {
            // 整数转结构体：先转为指针，再解引用
            let ptr = self.new_label("struct_ptr");
            self.emit(&format!("    %{} = inttoptr i64 %{} to {}*", ptr, val, to_type));
            self.emit(&format!("    %{} = load {}, {}* %{}", result, to_type, to_type, ptr));
        } else if from_type.starts_with("%struct.") && to_type == "i8*" {
            // 结构体转指针：用 rt_malloc 堆分配，避免栈指针失效
            let size = self.compute_struct_size_from_type(&from_type);
            let heap_ptr = self.new_label("heap");
            self.emit(&format!("    %{} = call i8* @rt_malloc(i64 {})", heap_ptr, size));
            let typed_ptr = self.new_label("typed_ptr");
            self.emit(&format!("    %{} = bitcast i8* %{} to {}*", typed_ptr, heap_ptr, from_type));
            self.emit(&format!("    store {} %{}, {}* %{}", from_type, val, from_type, typed_ptr));
            return heap_ptr;  // 直接返回已转换的 i8*，无需额外 bitcast
        } else if from_type == "i8*" && to_type.starts_with("%struct.") {
            // 指针转结构体：先转换为结构体指针，再加载结构体值
            let struct_ptr = self.new_label("struct_ptr");
            self.emit(&format!("    %{} = bitcast i8* %{} to {}*", struct_ptr, val, to_type));
            self.emit(&format!("    %{} = load {}, {}* %{}", result, to_type, to_type, struct_ptr));
        } else if from_type.ends_with('*') && to_type.starts_with("%struct.") {
            // 结构体指针转结构体值：直接加载
            self.emit(&format!("    %{} = load {}, {} %{}", result, to_type, from_type, val));
        } else if from_type.starts_with("%struct.") && to_type.ends_with('*') {
            // 结构体值转结构体指针：alloca + store，返回指针
            let ptr = self.new_label("conv_ptr");
            self.emit(&format!("    %{} = alloca {}, align 8", ptr, from_type));
            self.emit(&format!("    store {} %{}, {}* %{}", from_type, val, from_type, ptr));
            return ptr;  // 直接返回 alloca 指针，不需要再加 bitcast
        } else if from_type == "i1" && (to_type == "i64" || to_type == "i32" || to_type == "i8") {
            // i1 转整数类型：用 zext（不能用 bitcast）
            self.emit(&format!("    %{} = zext i1 %{} to {}", result, val, to_type));
        } else if from_type == "i64" && to_type == "i1" {
            // 整数转 i1：用 trunc
            self.emit(&format!("    %{} = trunc i64 %{} to i1", result, val));
        } else if from_type == "i1" && to_type == "i8*" {
            // i1 转指针：先 zext 再 inttoptr
            let zext_val = self.new_label("bool_ext");
            self.emit(&format!("    %{} = zext i1 %{} to i64", zext_val, val));
            self.emit(&format!("    %{} = inttoptr i64 %{} to i8*", result, zext_val));
        } else if from_type == "i1" && to_type == "double" {
            // i1 转浮点：先 zext 再 sitofp
            let zext_val = self.new_label("bool_ext");
            self.emit(&format!("    %{} = zext i1 %{} to i64", zext_val, val));
            self.emit(&format!("    %{} = sitofp i64 %{} to double", result, zext_val));
        } else {
            // 其他情况，直接使用原值（可能需要 bitcast）
            self.emit(&format!("    %{} = bitcast {} %{} to {}", result, from_type, val, to_type));
        }
        
        result
    }

    /**
     * 将 AST 类型转换为 LLVM 类型字符串
     */
    fn type_to_llvm_type(&self, ty: &Type) -> String {
        match ty {
            Type::Int | Type::Long | Type::Bool | Type::Char => "i64".to_string(),
            Type::Float | Type::Double => "double".to_string(),
            Type::String => "i8*".to_string(),
            Type::Void => "void".to_string(),
            Type::Pointer => "i8*".to_string(),
            Type::List(_) => "i8*".to_string(),
            Type::Array(_) => "i8*".to_string(),
            Type::Optional(_) => "i8*".to_string(),
            Type::Custom(name) => self.llvm_type_for_named_struct(name),
            Type::Struct(name) => self.llvm_type_for_named_struct(name),
            Type::Function(_, _) => "i8*".to_string(),
            Type::Future(_) => "i8*".to_string(),
            Type::Any => "i8*".to_string(),
            Type::Unknown => "i64".to_string(),
            Type::TypeVar(_) => "i64".to_string(),
        }
    }

    /**
     * 推断表达式类型
     */
    fn infer_expression_type(&self, expr: &Expr) -> String {
        match expr {
            Expr::Identifier(ident) => {
                let var_name = ident.name.clone();
                let ty = self.variable_types.get(&var_name)
                    .cloned()
                    .unwrap_or_else(|| "i64".to_string());
                // 剥离可能被污染的 * 后缀（防御性）
                if ty.starts_with("%struct.") { ty.trim_end_matches('*').to_string() } else { ty }
            },
            Expr::Literal(lit) => {
                match &lit.kind {
                    LiteralKind::Integer(_) => "i64".to_string(),
                    LiteralKind::Float(_) => "double".to_string(),
                    LiteralKind::String(_) => "i8*".to_string(),
                    LiteralKind::Boolean(_) => "i64".to_string(),
                    LiteralKind::Char(_) => "i64".to_string(),
                }
            }
            Expr::Binary(binary) => {
                // 根据二元操作的类型返回不同的类型
                match binary.op {
                    // 比较操作和逻辑操作返回 i1 类型
                    BinaryOp::Eq | BinaryOp::Ne |
                    BinaryOp::Lt | BinaryOp::Le |
                    BinaryOp::Gt | BinaryOp::Ge |
                    BinaryOp::And | BinaryOp::Or => "i1".to_string(),
                    // Add 操作：检查是否涉及字符串
                    BinaryOp::Add => {
                        let left_type = self.infer_expression_type(&binary.left);
                        let right_type = self.infer_expression_type(&binary.right);
                        if left_type == "i8*" || right_type == "i8*" {
                            "i8*".to_string()  // 字符串拼接返回字符串
                        } else {
                            "i64".to_string()
                        }
                    }
                    // 其他二元操作返回 i64 类型
                    _ => "i64".to_string(),
                }
            }
            Expr::Unary(_) => "i64".to_string(),
            Expr::Call(call) => {
                // 检查函数名来确定返回类型
                let base_func_name = match &*call.function {
                    Expr::Identifier(ident) => {
                        self.translate_func_name(&ident.name)
                    }
                    _ => {
                        return "i8*".to_string(); // 表达式调用默认返回指针
                    }
                };

                // 先检查内置函数的返回类型（如 rt_list_new → i8*）
                // 直接通过原始函数名查表，比复杂的模块前缀匹配更可靠
                let orig_func_name = match &*call.function {
                    Expr::Identifier(ident) => ident.name.clone(),
                    _ => String::new(),
                };
                if !orig_func_name.is_empty() {
                    if let Some(ret_type) = self.get_func_return_type(&orig_func_name) {
                        return ret_type;
                    }
                }
                // 也检查翻译后的函数名
                if let Some(ret_type) = self.get_func_return_type(&base_func_name) {
                    return ret_type;
                }
                if let Some(ret_type) = self.get_func_return_type(&base_func_name) {
                    ret_type
                } else if !self.module_name.is_empty() {
                    // 尝试查找带模块名前缀的函数返回类型
                    let func_name_with_module = format!("{}_module_{}", self.module_name, base_func_name);
                    if let Some(ret_type) = self.get_func_return_type(&func_name_with_module) {
                        ret_type
                    } else {
                        // 对于无返回函数（如 rt_list_append），返回 i64
                        if base_func_name.contains("rt_list_append") || base_func_name.contains("rt_print") || base_func_name.contains("rt_error") {
                            "i64".to_string()
                        } else {
                            // 尝试从 user_functions 中查找参数数量匹配的函数
                            let _arg_count = call.arguments.len();
                            let mut found_return_type = None;
                            let search_name = format!("{}_module_{}", self.module_name, base_func_name);
                            for (name, (_params, ret_type)) in &self.user_functions {
                                if name.starts_with(&search_name) {
                                    found_return_type = Some(ret_type.clone());
                                    break;
                                }
                            }
                            found_return_type.unwrap_or_else(|| "i64".to_string())
                        }
                    }
                } else {
                    // 对于无返回函数（如 rt_list_append），返回 i64
                    if base_func_name.contains("rt_list_append") || base_func_name.contains("rt_print") || base_func_name.contains("rt_error") {
                        "i64".to_string()
                    } else {
                        // 尝试从 user_functions 中查找参数数量匹配的函数
                        let _arg_count = call.arguments.len();
                        let mut found_return_type = None;
                        for (name, (_params, ret_type)) in &self.user_functions {
                            if name.starts_with(&base_func_name) {
                                found_return_type = Some(ret_type.clone());
                                break;
                            }
                        }
                        found_return_type.unwrap_or_else(|| "i64".to_string())
                    }
                }
            }
            Expr::MemberAccess(member) => {
                let field_name = &member.member;
                let obj_type = self.infer_expression_type(&member.object);
                let field_type = self.infer_member_type(&obj_type, field_name);
                if field_type.starts_with("%struct.") {
                    "i64".to_string()
                } else {
                    field_type
                }
            }
            Expr::Grouped(expr) => self.infer_expression_type(expr),
            Expr::Await(await_expr) => self.infer_expression_type(&await_expr.expr),
            Expr::ListLiteral(_) => "i8*".to_string(),
            Expr::ListComprehension(_) => "i8*".to_string(),
            Expr::Lambda(_) => "i8*".to_string(),
            Expr::IndexAccess(index_access) => {
                let object_type = self.infer_expression_type(&index_access.object);
                if object_type == "i8*" {
                    // 列表元素类型推断为 i8*，字符串索引返回字符 i64
                    let is_list = match &*index_access.object {
                        Expr::MemberAccess(member) => {
                            let field_name = &member.member;
                            let declared_list = self.list_typed_fields.contains(field_name);
                            let name_hint = field_name == "tokens" || field_name == "children" ||
                                field_name == "items" || field_name == "errors" ||
                                field_name.ends_with("列表");
                            declared_list || name_hint
                        }
                        Expr::Identifier(ident) => {
                            ident.name.ends_with("s") || ident.name.contains("列表")
                        }
                        _ => false,
                    };
                    if is_list { "i8*".to_string() } else { "i64".to_string() }
                } else {
                    "i64".to_string()
                }
            }
            _ => "i64".to_string(),
        }
    }

    /**
     * 翻译类型
     */
    fn translate_type(&self, ty: &Type) -> String {
        match ty {
            Type::Int | Type::Long => "i64".to_string(),
            Type::Float | Type::Double => "double".to_string(),
            Type::String => "i8*".to_string(),
            Type::Bool | Type::Char => "i64".to_string(),
            Type::Void => "void".to_string(),
            Type::Pointer => "i8*".to_string(),
            Type::List(_) | Type::Array(_) | Type::Optional(_) => "i8*".to_string(),
            Type::Custom(name) | Type::Struct(name) => {
                let base_name = if name.contains("::") {
                    name.split("::").last().unwrap()
                } else {
                    name
                };
                if self.enum_types.contains(base_name) {
                    "i64".to_string()
                } else {
                    self.llvm_type_for_named_struct(name)
                }
            },
            Type::Function(_, _) => "i8*".to_string(),
            Type::Future(_) => "i8*".to_string(),
            Type::Any => "i8*".to_string(),
            Type::Unknown => "i64".to_string(),
            Type::TypeVar(_) => "i64".to_string(),
        }
    }

    /**
     * 翻译函数名（处理中文函数名）
     * 注意：运行时函数（如 rt_list_new, rt_list_append）保持原名不被哈希
     */
    fn translate_func_name(&self, name: &str) -> String {
        self.translate_func_name_internal(name, true)
    }

    /// 翻译函数定义名（不使用外部函数哈希）
    fn translate_def_name(&self, name: &str) -> String {
        self.translate_func_name_internal(name, false)
    }

    /// 翻译函数名，is_call 表示是否是函数调用（true=调用，false=定义）
    fn translate_func_name_internal(&self, name: &str, is_call: bool) -> String {
        // 对于模块间函数调用，将 :: 替换为 __ 以避免 LLVM IR 语法错误
        let sanitized_name = name.replace("::", "__");
        if name.contains("::") {
            // 包含模块前缀的函数名，也需要处理中文字符
            let def_hash_name = self.generate_hash_name(&sanitized_name, "");
            let extern_hash_name = self.generate_hash_name(&sanitized_name, "__extern__");
            if is_call && self.user_functions.contains_key(&def_hash_name) {
                def_hash_name
            } else if is_call && self.extern_functions.contains_key(&extern_hash_name) {
                extern_hash_name
            } else {
                def_hash_name
            }
        } else {
            // 中文函数名翻译为有效的 LLVM 标识符
            match name {
                "主" | "主函数" | "main" => "xy_main".to_string(),
                "打印" => "print".to_string(),
                "打印行" => "println".to_string(),
                "打印整数" => "print_int".to_string(),
                "打印浮点数" => "print_float".to_string(),
                "报错" => "error".to_string(),
                "版本" => "version".to_string(),
                "加" => "add".to_string(),
                "减" => "sub".to_string(),
                "乘" => "mul".to_string(),
                "除" => "div".to_string(),
                "读取行" => "read_line".to_string(),
                "新建列表" => "rt_list_new".to_string(),
                "列表追加" => "rt_list_append".to_string(),
                "列表获取" => "rt_list_get".to_string(),
                "列表长度" => "rt_list_len".to_string(),
                "文本长度" => "rt_string_len".to_string(),
                "列表" => "rt_list_new".to_string(),
                "整数转字符" => "rt_string_fromChar".to_string(),
                // ========== 运行时函数映射（保持原名，不哈希）==========
                // 列表操作函数
                "rt_list_new" => "rt_list_new".to_string(),
                "rt_list_append" => "rt_list_append".to_string(),
                "rt_list_get" => "rt_list_get".to_string(),
                "rt_list_set" => "rt_list_set".to_string(),
                "rt_list_len" => "rt_list_len".to_string(),
                // 字符串操作函数
                "rt_str_new" => "rt_str_new".to_string(),
                "rt_str_concat" => "rt_str_concat".to_string(),
                "rt_string_len" => "rt_string_len".to_string(),
                "rt_string_concat" => "rt_string_concat".to_string(),
                "str_concat" => "rt_str_concat".to_string(),
                // 内存管理函数
                "rt_malloc" => "rt_malloc".to_string(),
                "rt_free" => "rt_free".to_string(),
                // 输入输出函数
                "rt_print" => "rt_print".to_string(),
                "rt_println" => "rt_println".to_string(),
                "print_int" => "print_int".to_string(),
                "print_float" => "print_float".to_string(),
                "rt_error" => "rt_error".to_string(),
                // ========== V2 编译器函数映射（映射到 ASCII 安全名称） ==========
                // 注意：不再为特定函数名提供映射，统一使用哈希名生成机制
                // 这样可以确保函数定义和调用使用相同的名称
                // V2 运行时函数映射
                "列表添加" => "rt_list_append".to_string(),
                "列表设置" => "rt_list_set".to_string(),
                // V2 参数读取函数
                "获取参数" => "argv".to_string(),
                "获取参数个数" => "argc".to_string(),
                "取参数" => "argv".to_string(),
                "取参数个数" => "argc".to_string(),
                "命令行参数" => "argv".to_string(),
                // V2 文件操作
                "读取文件" => "file_read".to_string(),
                "写入文件" => "file_write".to_string(),
                "文件存在" => "file_exists".to_string(),
                "执行命令" => "exec_cmd".to_string(),
                // V2 文本操作
                "文本切片" | "文本截取" => "rt_string_substring".to_string(),
                "文本获取字符" => "rt_string_char_at".to_string(),
                "文本包含" => "str_contains".to_string(),
                "文本查找" => "rt_string_indexOf".to_string(),
                "文本转整数" => "rt_str_to_int".to_string(),
                "整数转文本" => "rt_int_to_str".to_string(),
                "详细输出" => "rt_print".to_string(),
                "字符编码" => "rt_char_to_code".to_string(),
                "编码转字符" => "rt_code_to_char".to_string(),
                // UTF-8 helper functions
                "utf8字节长度" => "rt_utf8_byte_length".to_string(),
                "是utf8首字节" => "rt_is_utf8_leader".to_string(),
                "是utf8续字节" => "rt_is_utf8_continuation".to_string(),
                _ => {
                    // 使用固定作用域确保定义和调用使用相同的哈希名
                    let def_hash_name = self.generate_hash_name(name, "");
                    let extern_hash_name = self.generate_hash_name(name, "__extern__");
                    if is_call && self.user_functions.contains_key(&def_hash_name) {
                        def_hash_name
                    } else if is_call {
                        let mut found_name = String::new();
                        for (func_name, _) in &self.user_functions {
                            if func_name.contains(&def_hash_name) {
                                found_name = func_name.clone();
                                break;
                            }
                        }
                        if !found_name.is_empty() {
                            found_name
                        } else if self.extern_functions.contains_key(&extern_hash_name) {
                            extern_hash_name
                        } else {
                            def_hash_name
                        }
                    } else if self.extern_functions.contains_key(&extern_hash_name) {
                        extern_hash_name
                    } else {
                        def_hash_name
                    }
                }
            }
        }
    }

    /**
     * 为中文标识符生成有效的 LLVM 名称（哈希形式）
     * 包含函数作用域以确保全局唯一性
     */
    fn generate_hash_name(&self, name: &str, scope: &str) -> String {
        // 使用 Unicode 编码替代纯哈希，避免不同中文函数名产生哈希冲突
        // 格式: fn_U<codepoint>_U<codepoint>..._<scope_hash>
        let mut encoded = String::from("fn");
        for ch in name.chars() {
            if ch.is_ascii() && ch.is_alphanumeric() {
                encoded.push(ch);
            } else if ch == '_' {
                encoded.push('_');
            } else {
                // 非ASCII字符使用 Unicode 码点编码，确保不同名称产生不同前缀
                encoded.push_str(&format!("u{:x}", ch as u32));
            }
        }
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        scope.hash(&mut hasher);
        // 如果设置了模块名，将其包含在哈希计算中，确保不同模块的相同函数名生成不同的哈希
        if !self.module_name.is_empty() {
            self.module_name.hash(&mut hasher);
        }
        let hash = hasher.finish();
        format!("{}_{:x}", encoded, hash)
    }

    /**
     * 计算 LLVM IR 字符串字面量解析后的实际字节长度
     * LLVM 在解析 c"..." 时会处理转义序列（如 \n, \t, \\ 等）
     */
    #[allow(dead_code)]
    fn calculate_llvm_string_length(&self, escaped: &str) -> usize {
        let mut len = 0;
        let chars: Vec<char> = escaped.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '\\' && i + 1 < chars.len() {
                // 处理转义序列
                match chars[i + 1] {
                    'n' => { len += 1; i += 2; }  // \n -> 换行符
                    't' => { len += 1; i += 2; }  // \t -> 制表符
                    '\\' => { len += 1; i += 2; } // \\ -> \
                    '"' => { len += 1; i += 2; }  // \" -> "
                    _ => { len += 2; i += 2; }    // 其他转义，当作两个字符
                }
            } else {
                len += chars[i].len_utf8();
                i += 1;
            }
        }
        len
    }

    /**
     * 计算字段偏移
     * 在已注册的结构体布局中查找字段偏移量
     * 如果未找到，按默认顺序计算（假设每个字段8字节）
     */
    #[allow(dead_code)]
    fn calculate_field_offset(&self, field_name: &str) -> i32 {
        for (_struct_name, fields) in &self.struct_field_layouts {
            for (name, offset, _) in fields {
                if name == field_name {
                    return *offset;
                }
            }
        }
        0
    }

    fn calculate_field_offset_and_type(&self, struct_name: &str, field_name: &str) -> (i32, String) {
        let mut lookup_names = Vec::new();

        // 脱去 %struct. 前缀和 * 后缀（防御被污染的指针类型）
        let cleaned = struct_name
            .trim_start_matches("%struct.")
            .trim_end_matches('*');
        let base_name = cleaned;
        
        lookup_names.push(base_name.to_string());
        lookup_names.push(self.translate_struct_name(struct_name));
        
        for lookup_name in &lookup_names {
            if let Some(fields) = self.struct_field_layouts.get(lookup_name) {
                for (name, offset, llvm_type) in fields {
                    if name == field_name {
                        return (*offset, llvm_type.clone());
                    }
                }
            }
        }
        
        // 全局搜索所有已注册的 struct 布局（跨模块兼容）
        for (_struct_name, fields) in &self.struct_field_layouts {
            for (name, offset, llvm_type) in fields {
                if name == field_name {
                    return (*offset, llvm_type.clone());
                }
            }
        }
        (0, "i64".to_string())
    }

    fn is_builtin_func(name: &str) -> bool {
        name.starts_with("rt_") || 
        name.starts_with("print") || 
        name.starts_with("str_") || 
        name.starts_with("file_") || 
        name.starts_with("exec_") || 
        name.starts_with("arg") ||
        name == "init_args" ||
        name == "error"
    }

    /**
     * 推断成员类型
     * 在已注册的结构体布局中查找字段的LLVM类型
     * 如果未找到，默认返回i64
     */
    fn infer_member_type(&self, struct_name: &str, field_name: &str) -> String {
        let mut lookup_names = Vec::new();

        // 脱去前缀和后缀
        let cleaned = struct_name
            .trim_start_matches("%struct.")
            .trim_end_matches('*');
        let base_name = cleaned;
        
        lookup_names.push(base_name.to_string());
        lookup_names.push(self.translate_struct_name(struct_name));
        
        for lookup_name in &lookup_names {
            if let Some(fields) = self.struct_field_layouts.get(lookup_name) {
                for (name, _, llvm_type) in fields {
                    if name == field_name {
                        return llvm_type.clone();
                    }
                }
            }
        }
        
        // 如果按 struct 名没找到，搜索所有已注册的 struct 布局（跨模块兼容）
        for (_struct_name, fields) in &self.struct_field_layouts {
            for (name, _, llvm_type) in fields {
                if name == field_name {
                    return llvm_type.clone();
                }
            }
        }
        match field_name {
            "位置" | "长度" | "行号" | "列号" | "开始位置" | "结束位置" |
            "当前字符" | "当前行号" | "当前列号" | "当前位置" |
            "pos" | "count" | "tokenCount" | "nodeCount" | "funcCount" |
            "tempCount" | "labelCount" | "stringConstCount" | "indent" |
            "id" | "kind" | "line" | "intValue" |
            "状态" | "起始位置" | "是否错误" | "已恢复" | "跳过Token数" | "恢复点" |
            "激活" | "循环层级" | "错误计数" | "警告计数" | "层级" | "可变" |
            "已初始化" | "作用域层级" | "父作用域" | "全局作用域" | "当前作用域" |
            "当前函数返回类型" | "functionIndexCounter" | "currentFunctionIndex" => "i64".to_string(),
            "文本" | "名称" | "名字" | "值" | "内容" | "数据" |
            "函数名" | "返回类型" | "参数类型" | "字段名" | "字段类型" |
            "左" | "右" | "条件" | "then分支" | "else分支" |
            "对象" | "成员" | "参数" | "参数列表" | "语句" | "语句列表" |
            "表达式" | "表达式列表" | "声明" | "声明列表" |
            "函数" | "函数列表" | "变量" | "变量列表" |
            "结构体" | "结构体列表" | "字段" | "字段列表" |
            "body" | "params" | "args" | "name" | "type" | "value" |
            "children" | "items" | "elements" => "i8*".to_string(),
            _ => "i64".to_string(),
        }
    }

    fn function_return_signature(&self, ty: &Type) -> (String, bool, String) {
        if self.is_aggregate_type(ty) {
            let llvm_type = self.translate_type(ty);
            ("void".to_string(), true, llvm_type)
        } else {
            let llvm_type = self.translate_type(ty);
            (llvm_type.clone(), false, llvm_type)
        }
    }

    fn is_aggregate_type(&self, ty: &Type) -> bool {
        matches!(ty, Type::Struct(_) | Type::Custom(_))
    }

    fn is_aggregate_llvm_type(&self, ty: &str) -> bool {
        ty.starts_with("%struct.") || ty.starts_with("%") && self.struct_field_layouts.contains_key(&ty.trim_start_matches('%').to_string())
    }

    fn llvm_type_for_named_struct(&self, name: &str) -> String {
        format!("%struct.{}", self.translate_struct_name(name))
    }

    fn translate_struct_name(&self, name: &str) -> String {
        let cleaned_name = name.trim_start_matches('%').trim_start_matches("struct.");
        let base_name = if cleaned_name.contains("::") {
            cleaned_name.split("::").last().unwrap()
        } else {
            cleaned_name
        };
        let mut encoded = String::from("fn");
        for ch in base_name.chars() {
            if ch.is_ascii() && ch.is_alphanumeric() {
                encoded.push(ch);
            } else if ch == '_' {
                encoded.push('_');
            } else {
                encoded.push_str(&format!("u{:x}", ch as u32));
            }
        }
        // 使用确定性哈希（不能用 DefaultHasher，它每次运行随机种子不同）
        // 简单 FNV-1a 64-bit hash，确保相同输入始终产生相同输出
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in base_name.as_bytes() {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{}_{:x}", encoded, hash)
    }

    fn compute_struct_size_from_type(&self, ty: &str) -> i64 {
        let structural_name = ty.trim_start_matches('%').trim_start_matches("struct.");
        if let Some(size) = self.struct_field_layouts.get(structural_name) {
            if let Some((_, last_offset, _)) = size.last() {
                return (last_offset + 8) as i64;
            }
        }
        self.compute_struct_size(structural_name) as i64
    }

    /**
     * 净化 LLVM 标识符，将非 ASCII 字符替换为 Unicode 码点
     * LLVM IR 只接受 ASCII 标识符，中文字符必须转换
     */
    fn sanitize_identifier(prefix: &str) -> String {
        let mut result = String::new();
        for ch in prefix.chars() {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '-' {
                result.push(ch);
            } else {
                // 将非 ASCII 字符替换为 u{hex} 形式
                result.push_str(&format!("u{:x}", ch as u32));
            }
        }
        // 确保不以数字开头（LLVM 不允许）
        if result.starts_with(|c: char| c.is_ascii_digit()) {
            result.insert(0, 'v');
        }
        result
    }

    /**
     * 生成新标签
     */
    fn new_label(&mut self, prefix: &str) -> String {
        let safe_prefix = Self::sanitize_identifier(prefix);
        let label = format!("{}_{}", safe_prefix, self.label_counter);
        self.label_counter += 1;
        label
    }

    /**
     * 将元素添加到列表中，处理不同类型的元素
     * list_ptr: 列表指针（i8*）
     * elem_val: 元素值
     * elem_type: 元素类型
     */
    fn append_to_list(&mut self, list_ptr: &str, elem_val: &str, elem_type: &str) {
        if elem_type == "i8*" {
            self.emit(&format!("    call void @rt_list_append(i8* %{}, i8* %{})
", list_ptr, elem_val));
        } else if elem_type.starts_with("%struct.") {
            let elem_addr = self.new_label("elem_addr");
            self.emit(&format!("    %{} = alloca {}, align 8", elem_addr, elem_type));
            self.emit(&format!("    store {} %{}, {}* %{}", elem_type, elem_val, elem_type, elem_addr));
            self.emit(&format!("    call void @rt_list_append(i8* %{}, i8* %{})
", list_ptr, elem_addr));
        } else {
            let elem_ptr = self.new_label("elem_ptr");
            self.emit(&format!("    %{} = inttoptr {} %{} to i8*", elem_ptr, elem_type, elem_val));
            self.emit(&format!("    call void @rt_list_append(i8* %{}, i8* %{})
", list_ptr, elem_ptr));
        }
    }

    /**
     * 输出IR代码
     */
    fn emit(&mut self, code: &str) {
        self.ir.push_str(code);
        self.ir.push_str("\n");
    }
}

/**
 * 生成IR代码
 */
pub fn generate_ir(module: &Module) -> Result<String, CodegenError> {
    let mut generator = CodeGenerator::new();
    generator.generate(module)
}

pub fn generate_ir_with_module_name(module: &Module, module_name: &str) -> Result<String, CodegenError> {
    let mut generator = CodeGenerator::with_module_name(module_name);
    generator.generate(module)
}
