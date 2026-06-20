/**
 * @file codegen.rs
 * @brief 代码生成模块
 * @description 负责将AST转换为LLVM IR
 */

use std::collections::HashMap;
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
    /// 循环 break/continue 标签栈（break_label, continue_label）
    loop_label_stack: Vec<(usize, usize)>,
    /// 结构体字段偏移映射（结构体名 -> [(字段名, 偏移量, LLVM类型)]）
    struct_field_layouts: HashMap<String, Vec<(String, i32, String)>>,
    /// 枚举值映射（枚举成员名 -> 整数值）
    enum_values: HashMap<String, i64>,
    /// 是否已经生成了运行时声明
    runtime_declarations_emitted: bool,
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
            string_constants: Vec::new(),
            extern_functions: HashMap::new(),
            user_functions: HashMap::new(),
            current_function_name: String::new(),
            current_function_return_type: String::new(),
            loop_label_stack: Vec::new(),
            struct_field_layouts: HashMap::new(),
            enum_values: HashMap::new(),
            runtime_declarations_emitted: false,
        }
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
        self.loop_label_stack.clear();
        self.struct_field_layouts.clear();
        self.enum_values.clear();

        // 注册结构体字段布局（在生成函数之前）
        for struct_def in &module.structs {
            self.register_struct_layout(struct_def);
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
        for func in &module.functions {
            let func_name = self.translate_def_name(&func.name);
            let return_type = self.translate_type(&func.return_type);
            let param_types: Vec<String> = func.params
                .iter()
                .map(|param| self.translate_type(&param.param_type))
                .collect();
            self.user_functions.insert(func_name, (param_types, return_type));
        }

        // 生成函数定义
        let mut has_xy_main = false;
        for func in &module.functions {
            self.generate_function(func)?;
            if func.name == "主" || func.name == "主函数" {
                has_xy_main = true;
            }
        }

        // 如果存在 XY 主函数，生成 C 兼容的 main 包装器
        if has_xy_main {
            self.emit("\ndefine i32 @main(i32 %argc, i8** %argv) {");
            self.emit("    call void @init_args(i32 %argc, i8** %argv)");
            self.emit("    %argc_ext = sext i32 %argc to i64");
            self.emit("    %argv_i8 = bitcast i8** %argv to i8*");
            self.emit("    %result = call i64 @xy_main(i64 %argc_ext, i8* %argv_i8)");
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

        Ok(self.ir.clone())
    }

    /// 修复生成IR中的空基本块和无终止符的基本块
    fn fix_empty_blocks(&mut self) {
        let lines: Vec<String> = self.ir.lines().map(|s| s.to_string()).collect();
        let mut result: Vec<String> = Vec::new();
        let mut needs_terminator = false;
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

            if is_label(trimmed) {
                // 如果上一个基本块没有终止符，添加br到当前标签
                if needs_terminator {
                    let prev_label = trimmed; // Use current label as target
                    result.push(format!("    br label %{}", prev_label));
                    needs_terminator = false;
                }
                result.push(line.clone());
                i += 1;
                // Skip empty lines after label
                while i < lines.len() && lines[i].trim().is_empty() {
                    result.push(lines[i].clone());
                    i += 1;
                }
                // Check what follows
                if i >= lines.len() {
                    result.push("    unreachable".to_string());
                } else {
                    let next = lines[i].trim();
                    if is_label(next) || next == "}" {
                        result.push("    unreachable".to_string());
                    }
                }
                needs_terminator = false;
            } else if trimmed == "}" || trimmed.starts_with("define ") || trimmed.starts_with("declare ") {
                if needs_terminator {
                    result.push("    unreachable".to_string());
                    needs_terminator = false;
                }
                result.push(line.clone());
                i += 1;
            } else {
                if is_terminator(trimmed) {
                    needs_terminator = false;
                } else if !trimmed.is_empty() && !trimmed.starts_with(';') && !trimmed.starts_with('@') && !trimmed.starts_with("declare") && !trimmed.starts_with("define") && !trimmed.starts_with("entry:") {
                    needs_terminator = true;
                }
                result.push(line.clone());
                i += 1;
            }
        }
        if needs_terminator {
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
    }

    /**
     * 注册结构体字段布局
     */
    fn register_struct_layout(&mut self, struct_def: &StructDefinition) {
        let struct_name = struct_def.name.clone();
        let mut fields = Vec::new();
        let mut offset = 0i32;

        for field in &struct_def.fields {
            // 确定字段的LLVM类型和大小
            let (llvm_type, field_size) = match &field.field_type {
                Type::Int | Type::Long | Type::Bool | Type::Char => ("i64", 8),
                Type::Float | Type::Double => ("double", 8),
                Type::String | Type::Pointer | Type::Any => ("i8*", 8),
                Type::List(_) | Type::Array(_) | Type::Optional(_) | Type::Future(_) => ("i8*", 8),
                Type::Function(_, _) => ("i8*", 8),
                Type::Struct(s_name) | Type::Custom(s_name) => {
                    let size = self.compute_struct_size(s_name);
                    ("i64", size)
                },
                Type::Void | Type::Unknown | Type::TypeVar(_) => ("i64", 8),
            };
            fields.push((field.name.clone(), offset, llvm_type.to_string()));
            offset += field_size;
        }
        self.struct_field_layouts.insert(struct_name, fields);
    }

    /// 计算结构体的总大小（字节数），用于正确计算嵌套结构体偏移
    fn compute_struct_size(&self, struct_name: &str) -> i32 {
        if let Some(fields) = self.struct_field_layouts.get(struct_name) {
            if let Some((_, last_offset, _)) = fields.last() {
                return last_offset + 8;
            }
        }
        // 未注册的结构体：根据名称猜测大小
        // Token 有6个字段 = 48字节, AST节点有7个字段 = 56字节, Lexer ~112字节
        match struct_name {
            "Token" => 48,       // 6 fields × 8 bytes
            "AST节点" | "ASTNode" => 56,  // 7 fields × 8 bytes
            "Lexer" => 120,      // 10 fields: 9×8 + 48(Token) = 120
            "Parser" => 80,      // 9 fields × 8 + extra
            "Sema" => 120,       // ~13 fields × 8 + inline structs
            "Codegen" => 160,    // ~20 fields × 8
            "符号" => 48,        // 6 fields
            "作用域" => 32,      // 4 fields
            "循环状态" => 16,    // 2 fields
            "循环上下文" => 32,  // 4 fields
            "ErrorRecovery" => 24, // 3 fields
            _ => {
                // 对于未知结构体，保守估计80字节
                eprintln!("[codegen] 未知结构体大小: {}, 默认80字节", struct_name);
                80
            }
        }
    }

    /**
     * 注册枚举成员值
     */
    fn register_enum_values(&mut self, enum_def: &EnumDefinition) {
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
            let var_name = self.translate_func_name(&const_def.name);
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
        // 翻译函数名（处理中文函数名）- 使用定义名以避免与外部声明冲突
        let func_name = self.translate_def_name(&func.name);
        
        // 设置当前函数名和返回类型，用于生成唯一的变量名和正确的返回语句
        self.current_function_name = func_name.clone();
        self.current_function_return_type = self.translate_type(&func.return_type);
        
        // 生成函数签名
        let return_type = self.current_function_return_type.clone();
        let param_types: Vec<String> = func.params
            .iter()
            .map(|param| self.translate_type(&param.param_type))
            .collect();
        
        let params_str = param_types.join(", ");
        self.emit(&format!("define {} @{}({}) {{\n", return_type, func_name, params_str));
        
        // 处理函数参数
        for (i, param) in func.params.iter().enumerate() {
            let param_name = self.translate_func_name(&param.name);
            let param_type = self.translate_type(&param.param_type);
            let alloca = self.new_label(&param_name);
            
            self.emit(&format!("    %{} = alloca {}, align 8", alloca, param_type));
            self.emit(&format!("    store {} %{}, {}* %{}", param_type, i, param_type, alloca));
            
            // 记录变量
            self.variables.insert(param_name.clone(), alloca);
            self.variable_types.insert(param_name, param_type);
        }
        
        // 记录生成函数体前的 IR 长度
        let _ir_len_before_body = self.ir.len();
        
        // 生成函数体
        self.generate_block(&func.body)?;

        // 始终在函数末尾生成返回语句
        if return_type != "void" {
            if return_type == "i8*" || return_type == "double" {
                self.emit(&format!("    ret {} null", return_type));
            } else {
                self.emit(&format!("    ret {} 0", return_type));
            }
        } else {
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
                        let var_name = self.translate_func_name(&ident.name);
                        if let Some(alloca) = self.variables.get(&var_name).cloned() {
                            let var_type = self.variable_types.get(&var_name)
                                .cloned()
                                .unwrap_or_else(|| "i64".to_string());
                            let right_type = self.infer_expression_type(&assign_stmt.value);
                            let final_val = if right_type != var_type {
                                self.generate_type_conversion(&value_val, &right_type, &var_type)
                            } else {
                                value_val.clone()
                            };
                            self.emit(&format!("    store {} %{}, {}* %{}", var_type, final_val, var_type, alloca));
                        }
                    }
                    Expr::MemberAccess(member) => {
                        // 结构体字段赋值（统一i64存储）
                        let object_val = self.generate_expression(&member.object)?;
                        let field_name = &member.member;
                        let field_offset = self.calculate_field_offset(field_name);
                        let obj_type = self.infer_expression_type(&member.object);
                        let ptr_val = if obj_type == "i64" {
                            let ptr = self.new_label("assign_field_ptr");
                            self.emit(&format!("    %{} = inttoptr i64 %{} to i8*", ptr, object_val));
                            ptr
                        } else { object_val };
                        let gep = self.new_label("assign_gep");
                        self.emit(&format!("    %{} = getelementptr i8, i8* %{}, i32 {}", gep, ptr_val, field_offset));
                        let typed_ptr = self.new_label("assign_typed");
                        self.emit(&format!("    %{} = bitcast i8* %{} to i64*", typed_ptr, gep));
                        let right_type = self.infer_expression_type(&assign_stmt.value);
                        let final_val = if right_type != "i64" {
                            self.generate_type_conversion(&value_val, &right_type, "i64")
                        } else { value_val };
                        self.emit(&format!("    store i64 %{}, i64* %{}", final_val, typed_ptr));
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
                // Match 语句暂不生成代码（简化处理）
            }
            Stmt::Block(block) => {
                // 块语句：递归生成块内语句
                self.generate_block(block)?;
            }
            Stmt::TypeAlias(_) => {
                // 类型别名：不需要生成IR代码
            }
            Stmt::Try(_) => {
                // Try 语句暂不生成代码（简化处理）
            }
            Stmt::Throw(_) => {
                // Throw 语句暂不生成代码（简化处理）
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
        let var_name = self.translate_func_name(&let_stmt.name);

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
        if let Some(s_name) = struct_name {
            let struct_size = self.compute_struct_size(&s_name);
            // 堆分配结构体，避免栈上分配导致的悬垂指针问题
            let heap_ptr = self.new_label(&format!("{}_heap", var_name));
            self.emit(&format!("    %{} = call i8* @rt_malloc(i64 {})", heap_ptr, struct_size));
            // 存储堆指针到 alloca 槽中（兼容现有变量查找逻辑）
            let ptr_store = self.new_label(&format!("{}_ref", var_name));
            self.emit(&format!("    %{} = alloca i8*, align 8", ptr_store));
            self.emit(&format!("    store i8* %{}, i8** %{}", heap_ptr, ptr_store));
            self.variables.insert(var_name.clone(), ptr_store);
            self.variable_types.insert(var_name, "i8*".to_string());
        } else {
            self.emit(&format!("    %{} = alloca {}, align 8", alloca, var_type));
            if let Some(initializer) = &let_stmt.initializer {
                let expr_val = self.generate_expression(initializer)?;
                let expr_type = self.infer_expression_type(initializer);
                let final_val = if expr_type != var_type {
                    self.generate_type_conversion(&expr_val, &expr_type, &var_type)
                } else { expr_val };
                self.emit(&format!("    store {} %{}, {}* %{}", var_type, final_val, var_type, alloca));
            }
            self.variables.insert(var_name.clone(), alloca);
            self.variable_types.insert(var_name, var_type);
        }
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
            
            // 如果返回类型不是 void，可能需要类型转换
            if return_type != "void" {
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
                    let var_name = self.translate_func_name(&counter.variable);
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
                        let var_name = self.translate_func_name(&counter.variable);
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
                        let var_name = self.translate_func_name(&counter.variable);
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
                self.emit(&format!("L{}:", loop_end));
                self.emit("    unreachable");

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
                // 翻译变量名（处理中文变量名）
                let translated_name = self.translate_func_name(&ident.name);
                // 查找变量的 SSA 值和类型
                if let Some(alloca) = self.variables.get(&translated_name).cloned() {
                    let var_type = self.variable_types.get(&translated_name)
                        .cloned()
                        .unwrap_or_else(|| "i64".to_string());
                    let load = self.new_label("id");
                    
                    // 根据类型选择正确的加载指令
                    if var_type == "i8*" {
                        // 列表/指针类型：从 i8** 加载
                        self.emit(&format!("    %{} = load i8*, i8** %{}", load, alloca));
                    } else {
                        // 其他类型
                        self.emit(&format!("    %{} = load {}, {}* %{}", load, var_type, var_type, alloca));
                    }
                    Ok(load)
                } else if let Some(alloca) = self.variables.get(&ident.name).cloned() {
                    // 尝试原始名称（处理枚举变体等未翻译的名称）
                    let var_type = self.variable_types.get(&ident.name)
                        .cloned()
                        .unwrap_or_else(|| "i64".to_string());
                    let load = self.new_label("id");
                    
                    if var_type == "i8*" {
                        self.emit(&format!("    %{} = load i8*, i8** %{}", load, alloca));
                    } else {
                        self.emit(&format!("    %{} = load {}, {}* %{}", load, var_type, var_type, alloca));
                    }
                    Ok(load)
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

                    // 检查是否是列表方法
                    let is_list_method = matches!(field_name.as_str(), "长度" | "追加" | "获取");

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
                        // 结构体字段访问：统一以 i64 加载（8字节），需要时通过 inttoptr 转换为指针
                        let object_val = self.generate_expression(&member.object)?;
                        let field_offset = self.calculate_field_offset(field_name);
                        let obj_type = self.infer_expression_type(&member.object);

                        let ptr_val = if obj_type == "i64" {
                            let ptr = self.new_label("ptr");
                            self.emit(&format!("    %{} = inttoptr i64 %{} to i8*", ptr, object_val));
                            ptr
                        } else {
                            object_val
                        };

                        // GEP 到字段位置
                        let result = self.new_label("member");
                        self.emit(&format!("    %{} = getelementptr i8, i8* %{}, i32 {}",
                            result, ptr_val, field_offset));

                        // 统一作为 i64 加载
                        let result_ptr = self.new_label("member_ptr");
                        self.emit(&format!("    %{} = bitcast i8* %{} to i64*", result_ptr, result));
                        let result_val = self.new_label("member_val");
                        self.emit(&format!("    %{} = load i64, i64* %{}", result_val, result_ptr));
                        self.variable_types.insert(result_val.clone(), "i64".to_string());

                        Ok(result_val)
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
                    let elem_ptr = self.new_label("elem_ptr");
                    
                    if elem_type == "i8*" {
                        // 字符串类型，直接使用
                        self.emit(&format!("    call void @rt_list_append(i8* %{}, i8* %{})
", list_ptr, elem_val));
                    } else {
                        // 其他类型，转换为指针
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
                let translated_var = self.translate_func_name(&comp.var_name);
                self.variables.insert(translated_var.clone(), var_alloca);
                self.variable_types.insert(translated_var, "i64".to_string());
                
                // 生成输出表达式
                let output_val = self.generate_expression(&comp.output)?;
                
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
                    let output_ptr = self.new_label("output_ptr");
                    self.emit(&format!("    %{} = inttoptr i64 %{} to i8*", output_ptr, output_val));
                    self.emit(&format!("    call void @rt_list_append(i8* %{}, i8* %{})
", result_list, output_ptr));
                    
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
                    let output_ptr = self.new_label("output_ptr");
                    self.emit(&format!("    %{} = inttoptr i64 %{} to i8*", output_ptr, output_val));
                    self.emit(&format!("    call void @rt_list_append(i8* %{}, i8* %{})
", result_list, output_ptr));
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
                // Lambda表达式：生成一个函数指针
                let lambda_func = self.new_label("lambda");

                // 生成函数定义
                self.emit(&format!("    define internal i64 @{}() {{\n", lambda_func));

                // 生成函数体
                let body_val = self.generate_expression(&lambda.body)?;

                // 返回值
                self.emit(&format!("        ret i64 %{}\n", body_val));
                self.emit("    }\n");

                // 获取函数指针
                let func_ptr = self.new_label("func_ptr");
                self.emit(&format!("    %{} = inttoptr i64 ptrtoint (i64 ()* @{} to i64) to i8*", func_ptr, lambda_func));

                Ok(func_ptr)
            }
            Expr::IndexAccess(index_access) => {
                // 索引访问: object[index]
                let object_val = self.generate_expression(&index_access.object)?;
                let index_val = self.generate_expression(&index_access.index)?;
                let object_type = self.infer_expression_type(&index_access.object);

                if object_type == "i8*" {
                    // 字符串或列表索引访问
                    // 先尝试字符串索引（默认行为）
                    let char_ptr = self.new_label("char_ptr");
                    self.emit(&format!("    %{} = call i8* @rt_string_char_at(i8* %{}, i64 %{})", char_ptr, object_val, index_val));
                    let char_code = self.new_label("char_code");
                    self.emit(&format!("    %{} = call i64 @rt_char_to_code(i8* %{})", char_code, char_ptr));
                    self.variable_types.insert(char_code.clone(), "i64".to_string());
                    Ok(char_code)
                } else if object_type == "i64" {
                    // 整数类型的索引访问：先将 i64 转换为 i8* 指针，再进行索引
                    // 这通常用于模拟指针算术
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
            _ => {
                Err(CodegenError::new("不支持的表达式类型"))
            }
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
                let label = self.new_label("str");
                let escaped = self.escape_string_for_llvm(&value);
                let byte_len = value.as_bytes().len();
                self.string_constants.push(format!("@str_constant_{} = private constant [{} x i8] c\"{}\\00\"", label, byte_len + 1, escaped));
                self.emit(&format!("    %{} = call i8* @rt_str_new(i8* getelementptr inbounds ([{} x i8], [{} x i8]* @str_constant_{}, i32 0, i32 0))", 
                    label, byte_len + 1, byte_len + 1, label));
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
                        let var_name = self.translate_func_name(&ident.name);
                        // 查找变量的 SSA 分配槽
                        if let Some(alloca) = self.variables.get(&var_name).cloned() {
                            // 获取变量类型，如果未知则从右值推断
                            let var_type = self.variable_types.get(&var_name)
                                .cloned()
                                .unwrap_or_else(|| {
                                    // 从右值推断类型
                                    let inferred_type = self.infer_expression_type(&binary.right);
                                    // 更新变量类型
                                    self.variable_types.insert(var_name.clone(), inferred_type.clone());
                                    inferred_type
                                });

                            // 可能需要类型转换
                            let right_type = self.infer_expression_type(&binary.right);
                            let final_val = if right_type != var_type {
                                self.generate_type_conversion(&right_val, &right_type, &var_type)
                            } else {
                                right_val.clone()
                            };

                            // 生成 store 指令将结果写回变量槽
                            self.emit(&format!("    store {} %{}, {}* %{}", var_type, final_val, var_type, alloca));
                        }
                    }
                    Expr::MemberAccess(member) => {
                        // 结构体字段赋值: struct.field = value（统一i64存储）
                        let object_val = self.generate_expression(&member.object)?;
                        let field_name = &member.member;
                        let obj_type = self.infer_expression_type(&member.object);
                        let field_offset = self.calculate_field_offset(field_name);

                        let ptr_val = if obj_type == "i64" {
                            let ptr = self.new_label("assign_ptr");
                            self.emit(&format!("    %{} = inttoptr i64 %{} to i8*", ptr, object_val));
                            ptr
                        } else {
                            object_val
                        };

                        let field_ptr = self.new_label("field_ptr");
                        self.emit(&format!("    %{} = getelementptr i8, i8* %{}, i32 {}",
                            field_ptr, ptr_val, field_offset));

                        // 统一转换为 i64* 并存储
                        let typed_ptr = self.new_label("typed_field_ptr");
                        self.emit(&format!("    %{} = bitcast i8* %{} to i64*", typed_ptr, field_ptr));

                        // 右值转换为 i64
                        let right_type = self.infer_expression_type(&binary.right);
                        let final_val = if right_type != "i64" {
                            self.generate_type_conversion(&right_val, &right_type, "i64")
                        } else {
                            right_val.clone()
                        };

                        self.emit(&format!("    store i64 %{}, i64* %{}", final_val, typed_ptr));
                    }
                    _ => {}
                }
                // 返回右值
                Ok(right_val)
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
                let left_type = self.infer_expression_type(&binary.left);
                let right_type = self.infer_expression_type(&binary.right);
                if left_type == "i8*" || right_type == "i8*" {
                    let l_val = if left_type != "i8*" { self.generate_type_conversion(&left_val, &left_type, "i8*") } else { left_val.clone() };
                    let r_val = if right_type != "i8*" { self.generate_type_conversion(&right_val, &right_type, "i8*") } else { right_val.clone() };
                    let tmp = self.new_label("tmp"); self.emit(&format!("    %{} = call i64 @rt_str_eq(i8* %{}, i8* %{})", tmp, l_val, r_val));
                    self.emit(&format!("    %{} = icmp ne i64 %{}, 0", result, tmp));
                } else if left_type == "double" || right_type == "double" {
                    let l_val = if left_type != "double" { self.generate_type_conversion(&left_val, &left_type, "double") } else { left_val.clone() };
                    let r_val = if right_type != "double" { self.generate_type_conversion(&right_val, &right_type, "double") } else { right_val.clone() };
                    self.emit(&format!("    %{} = fcmp oeq double %{}, %{}", result, l_val, r_val));
                } else {
                    self.emit(&format!("    %{} = icmp eq i64 %{}, %{}", result, left_val, right_val));
                }
                self.variable_types.insert(result.clone(), "i1".to_string());
                Ok(result)
            }
            BinaryOp::Ne => {
                let left_type = self.infer_expression_type(&binary.left);
                let right_type = self.infer_expression_type(&binary.right);
                if left_type == "i8*" || right_type == "i8*" {
                    let l_val = if left_type != "i8*" { self.generate_type_conversion(&left_val, &left_type, "i8*") } else { left_val.clone() };
                    let r_val = if right_type != "i8*" { self.generate_type_conversion(&right_val, &right_type, "i8*") } else { right_val.clone() };
                    let tmp = self.new_label("tmp"); self.emit(&format!("    %{} = call i64 @rt_str_ne(i8* %{}, i8* %{})", tmp, l_val, r_val));
                    self.emit(&format!("    %{} = icmp ne i64 %{}, 0", result, tmp));
                } else if left_type == "double" || right_type == "double" {
                    let l_val = if left_type != "double" { self.generate_type_conversion(&left_val, &left_type, "double") } else { left_val.clone() };
                    let r_val = if right_type != "double" { self.generate_type_conversion(&right_val, &right_type, "double") } else { right_val.clone() };
                    self.emit(&format!("    %{} = fcmp one double %{}, %{}", result, l_val, r_val));
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
        
        match func_name {
            // 返回 i8* 的函数
            "rt_list_new" => Some("i8*".to_string()),
            "rt_list_get" => Some("i8*".to_string()),
            "rt_str_new" => Some("i8*".to_string()),
            "rt_str_concat" => Some("i8*".to_string()),
            "rt_string_concat" => Some("i8*".to_string()),
            "rt_string_substring" => Some("i8*".to_string()),
            "rt_string_slice" => Some("i8*".to_string()),
            "rt_readline" => Some("i8*".to_string()),
            "rt_malloc" => Some("i8*".to_string()),
            "str_contains" => Some("i8*".to_string()),
            // 返回 i64 的函数
            "rt_list_len" => Some("i64".to_string()),
            "rt_string_len" => Some("i64".to_string()),
            "rt_string_char_at" => Some("i8*".to_string()),
            "rt_char_to_code" => Some("i64".to_string()),
            "argv" => Some("i8*".to_string()),
            "argc" => Some("i64".to_string()),
            "file_read" => Some("i8*".to_string()),
            "file_write" => Some("i32".to_string()),
            "file_exists" => Some("i32".to_string()),
            "exec_cmd" => Some("i32".to_string()),
            "print_int" => Some("i64".to_string()),
            "str_to_int" => Some("i64".to_string()),
            // 无返回的函数
            "rt_list_append" => None, // void
            "rt_list_set" => None,    // void
            "rt_print" => None,       // void
            "rt_println" => None,     // void
            "rt_error" => None,       // void
            "rt_free" => None,        // void
            _ => None,                // 未知
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
        
        match func_name {
            "rt_list_append" => vec!["i8*".to_string(), "i8*".to_string()],
            "rt_list_set" => vec!["i8*".to_string(), "i64".to_string(), "i8*".to_string()],
            "rt_list_get" => vec!["i8*".to_string(), "i64".to_string()],
            "rt_list_len" => vec!["i8*".to_string()],
            "rt_str_concat" => vec!["i8*".to_string(), "i8*".to_string()],
            "rt_string_concat" => vec!["i8*".to_string(), "i8*".to_string()],
            "rt_string_substring" => vec!["i8*".to_string(), "i64".to_string(), "i64".to_string()],
            "rt_string_slice" => vec!["i8*".to_string(), "i64".to_string(), "i64".to_string()],
            "rt_string_len" => vec!["i8*".to_string()],
            "rt_str_new" => vec!["i8*".to_string()],
            "rt_print" => vec!["i8*".to_string()],
            "rt_println" => vec!["i8*".to_string()],
            "rt_error" => vec!["i8*".to_string()],
            "rt_malloc" => vec!["i64".to_string()],
            "rt_free" => vec!["i8*".to_string()],
            "print_int" => vec!["i64".to_string()],
            "str_to_int" => vec!["i8*".to_string()],
            _ => vec![],
        }
    }

    /**
     * 生成函数调用表达式
     */
    fn generate_call_expr(&mut self, call: &CallExpr) -> Result<String, CodegenError> {
        // 获取函数名
        let (func_name, is_indirect) = match &*call.function {
            Expr::Identifier(ident) => {
                let translated = self.translate_func_name(&ident.name);
                // 检查是否确实是局部变量（而非全局函数名）
                let is_local_var = self.variables.contains_key(&translated)
                    || self.variables.contains_key(&ident.name);
                if is_local_var && !self.user_functions.contains_key(&translated)
                    && !self.extern_functions.contains_key(&translated)
                {
                    // 是局部变量，生成间接调用
                    let var_val = self.generate_expression(&call.function)?;
                    (var_val, true)
                } else {
                    (translated, false)
                }
            }
            _ => {
                let val = self.generate_expression(&call.function)?;
                (val, true)
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
            // 间接调用：将所有参数统一转为 i64
            let converted_args: Vec<String> = args.iter().enumerate()
                .map(|(i, a)| {
                    let arg_type = if i < call.arguments.len() {
                        self.infer_expression_type(&call.arguments[i])
                    } else { "i64".to_string() };
                    if arg_type == "i8*" {
                        let conv = self.new_label("arg_conv");
                        self.emit(&format!("    %{} = ptrtoint i8* %{} to i64", conv, a));
                        format!("i64 %{}", conv)
                    } else if arg_type == "double" {
                        let conv = self.new_label("arg_conv");
                        self.emit(&format!("    %{} = fptosi double %{} to i64", conv, a));
                        format!("i64 %{}", conv)
                    } else {
                        format!("i64 %{}", a)
                    }
                })
                .collect();
            self.emit(&format!("    %{} = call i64 %{}({})", result, func_name, converted_args.join(", ")));
            self.variable_types.insert(result.clone(), "i64".to_string());
            return Ok(result);
        }

        // 特殊处理内置函数
        if func_name == "print" {
            // 打印函数 - 需要类型转换
            if !args.is_empty() {
                // 获取参数的实际类型
                let actual_type = self.infer_arg_type(&call.arguments[0]);
                // 如果不是 i8*，需要转换
                if actual_type != "i8*" {
                    let converted_val = self.generate_type_conversion(&args[0], &actual_type, "i8*");
                    self.emit(&format!("    call void @rt_print(i8* %{})", converted_val));
                } else {
                    self.emit(&format!("    call void @rt_print(i8* %{})", args[0]));
                }
            }
            Ok(result)
        } else if func_name == "println" {
            // 打印行函数 - 需要类型转换
            if !args.is_empty() {
                // 获取参数的实际类型
                let actual_type = self.infer_arg_type(&call.arguments[0]);
                // 如果不是 i8*，需要转换
                if actual_type != "i8*" {
                    let converted_val = self.generate_type_conversion(&args[0], &actual_type, "i8*");
                    self.emit(&format!("    call void @rt_println(i8* %{})", converted_val));
                } else {
                    self.emit(&format!("    call void @rt_println(i8* %{})", args[0]));
                }
            }
            Ok(result)
        } else if func_name == "error" {
            // 报错函数 - 需要类型转换
            if !args.is_empty() {
                // 获取参数的实际类型
                let actual_type = self.infer_arg_type(&call.arguments[0]);
                // 如果不是 i8*，需要转换
                if actual_type != "i8*" {
                    let converted_val = self.generate_type_conversion(&args[0], &actual_type, "i8*");
                    self.emit(&format!("    call void @rt_error(i8* %{})", converted_val));
                } else {
                    self.emit(&format!("    call void @rt_error(i8* %{})", args[0]));
                }
            }
            Ok(result)
        } else if func_name == "rt_list_new" {
            // 列表创建函数，返回 i8*
            self.emit(&format!("    %{} = call i8* @rt_list_new()", result));
            Ok(result)
        } else if func_name == "rt_list_append" {
            // 列表追加函数
            if args.len() >= 2 {
                let a0_type = self.infer_arg_type(&call.arguments[0]);
                let a0 = if a0_type != "i8*" { self.generate_type_conversion(&args[0], &a0_type, "i8*") } else { args[0].clone() };
                let a1_type = self.infer_arg_type(&call.arguments[1]);
                let a1 = if a1_type != "i8*" { self.generate_type_conversion(&args[1], &a1_type, "i8*") } else { args[1].clone() };
                self.emit(&format!("    call void @rt_list_append(i8* %{}, i8* %{})", a0, a1));
            }
            Ok(result)
        } else if func_name == "rt_list_len" {
            // 列表长度函数
            if !args.is_empty() {
                let arg_type = self.infer_arg_type(&call.arguments[0]);
                let arg_val = if arg_type != "i8*" {
                    self.generate_type_conversion(&args[0], &arg_type, "i8*")
                } else { args[0].clone() };
                self.emit(&format!("    %{} = call i64 @rt_list_len(i8* %{})", result, arg_val));
            } else {
                self.emit(&format!("    %{} = call i64 @rt_list_len(i8* null)", result));
            }
            Ok(result)
        } else if func_name == "rt_list_get" {
            // 列表获取函数
            if args.len() >= 2 {
                let a0_type = self.infer_arg_type(&call.arguments[0]);
                let a0 = if a0_type != "i8*" { self.generate_type_conversion(&args[0], &a0_type, "i8*") } else { args[0].clone() };
                let a1_type = self.infer_arg_type(&call.arguments[1]);
                let a1 = if a1_type != "i64" { self.generate_type_conversion(&args[1], &a1_type, "i64") } else { args[1].clone() };
                self.emit(&format!("    %{} = call i8* @rt_list_get(i8* %{}, i64 %{})", result, a0, a1));
                self.variable_types.insert(result.clone(), "i8*".to_string());
            } else if args.len() == 1 {
                let a0_type = self.infer_arg_type(&call.arguments[0]);
                let a0 = if a0_type != "i8*" { self.generate_type_conversion(&args[0], &a0_type, "i8*") } else { args[0].clone() };
                self.emit(&format!("    %{} = call i8* @rt_list_get(i8* %{}, i64 0)", result, a0));
            } else {
                self.emit(&format!("    %{} = call i8* @rt_list_get(i8* null, i64 0)", result));
            }
            Ok(result)
        } else if func_name == "rt_str_new" {
            // 字符串创建函数，返回 i8*
            if !args.is_empty() {
                self.emit(&format!("    %{} = call i8* @rt_str_new(i8* %{})", result, args[0]));
            } else {
                self.emit(&format!("    %{} = call i8* @rt_str_new(i8* null)", result));
            }
            Ok(result)
        } else if func_name == "rt_str_concat" || func_name == "rt_string_concat" {
            // 字符串拼接函数，返回 i8*
            if args.len() >= 2 {
                self.emit(&format!("    %{} = call i8* @rt_str_concat(i8* %{}, i8* %{})", result, args[0], args[1]));
            } else if args.len() == 1 {
                self.emit(&format!("    %{} = call i8* @rt_str_concat(i8* %{}, i8* null)", result, args[0]));
            } else {
                self.emit(&format!("    %{} = call i8* @rt_str_concat(i8* null, i8* null)", result));
            }
            Ok(result)
        } else if func_name == "rt_string_len" {
            // 字符串长度函数
            if !args.is_empty() {
                self.emit(&format!("    %{} = call i64 @rt_string_len(i8* %{})", result, args[0]));
            } else {
                self.emit(&format!("    %{} = call i64 @rt_string_len(i8* null)", result));
            }
            Ok(result)
        } else if func_name == "print_int" {
            // 打印整数函数
            if !args.is_empty() {
                self.emit(&format!("    %{} = call i64 @print_int(i64 %{})", result, args[0]));
            } else {
                self.emit(&format!("    %{} = call i64 @print_int(i64 0)", result));
            }
            Ok(result)
        } else if func_name == "print_float" {
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
                
                // 获取实际参数类型（从变量类型映射中推断）
                let actual_type = self.infer_arg_type(&call.arguments[i]);
                
                // 如果类型不匹配，生成转换代码
                if actual_type != expected_type {
                    let converted_val = self.generate_type_conversion(arg, &actual_type, &expected_type);
                    converted_args.push(format!("{} %{}", expected_type, converted_val));
                } else {
                    converted_args.push(format!("{} %{}", expected_type, arg));
                }
            }
            
            let args_str = converted_args.join(", ");

            if return_type == "void" {
                self.emit(&format!("    call void @{}({})", func_name, args_str));
                Ok(result)
            } else {
                self.emit(&format!("    %{} = call {} @{}({})", result, return_type, func_name, args_str));
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
        // 使用通用的表达式类型推断，避免默认返回 i64 造成类型不匹配
        self.infer_expression_type(arg)
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
            // 整数转指针（用于字符串/指针兼容）
            self.emit(&format!("    %{} = inttoptr i64 %{} to i8*", result, val));
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
            Type::Custom(_name) => {
                // 自定义类型，可能是结构体或枚举
                // 暂时当作指针处理
                "i8*".to_string()
            },
            Type::Struct(_) => "i8*".to_string(),
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
                // 从变量类型映射中查找实际类型
                let var_name = self.translate_func_name(&ident.name);
                self.variable_types.get(&var_name)
                    .cloned()
                    .unwrap_or_else(|| "i64".to_string())
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
                let func_name = match &*call.function {
                    Expr::Identifier(ident) => {
                        self.translate_func_name(&ident.name)
                    }
                    _ => {
                        return "i8*".to_string(); // 表达式调用默认返回指针
                    }
                };

                // 使用 get_func_return_type 来确定返回类型
                if let Some(ret_type) = self.get_func_return_type(&func_name) {
                    ret_type
                } else {
                    // 对于无返回函数（如 rt_list_append），返回 i64
                    if func_name.contains("rt_list_append") || func_name.contains("rt_print") || func_name.contains("rt_error") {
                        "i64".to_string()
                    } else {
                        "i8*".to_string()  // 默认返回 i8* 更安全
                    }
                }
            }
            Expr::MemberAccess(member) => {
                // 检查是否是列表方法
                let field_name = &member.member;
                if field_name == "长度" {
                    "i64".to_string()
                } else {
                    // 结构体字段统一为 i64（8字节加载）
                    "i64".to_string()
                }
            }
            Expr::Grouped(expr) => self.infer_expression_type(expr),
            Expr::Await(await_expr) => self.infer_expression_type(&await_expr.expr),
            Expr::ListLiteral(_) => "i8*".to_string(),
            Expr::ListComprehension(_) => "i8*".to_string(),
            Expr::Lambda(_) => "i8*".to_string(),
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
            Type::Custom(_) | Type::Struct(_) => "i8*".to_string(),
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
        // 对于模块间函数调用，保留模块名::函数名的格式
        if name.contains("::") {
            name.to_string()
        } else {
            // 中文函数名翻译为有效的 LLVM 标识符
            match name {
                "主" => "xy_main".to_string(),
                "主函数" => "xy_main".to_string(),
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
                "tokenize" => "tokenize".to_string(),
                "parser分析Tokens" => "parser_parse_tokens".to_string(),
                "parserParse" => "parserParse".to_string(),
                "semaInit" => "semaInit".to_string(),
                "sema分析AST" => "sema_analyze_ast".to_string(),
                "semaAnalyze" => "semaAnalyze".to_string(),
                "codegenInit" => "codegenInit".to_string(),
                "codegenRun" => "codegenRun".to_string(),
                "codegenGenerate" => "codegenGenerate".to_string(),
                "codegenRunMain" => "codegenRunMain".to_string(),
                "codegen获取IR" => "codegen_get_ir".to_string(),
                "codegen遍历AST" => "codegen_traverse_ast".to_string(),
                // V2 运行时函数映射
                "列表添加" => "rt_list_append".to_string(),
                "列表设置" => "rt_list_set".to_string(),
                // V2 参数读取函数
                "获取参数" => "argv".to_string(),
                "获取参数个数" => "argc".to_string(),
                // V2 文件操作
                "读取文件" => "file_read".to_string(),
                "写入文件" => "file_write".to_string(),
                "文件存在" => "file_exists".to_string(),
                "执行命令" => "exec_cmd".to_string(),
                // V2 文本操作
                "文本切片" => "rt_string_substring".to_string(),
                "文本获取字符" => "rt_string_char_at".to_string(),
                "文本包含" => "str_contains".to_string(),
                "文本转整数" => "rt_str_to_int".to_string(),
                "整数转文本" => "rt_int_to_str".to_string(),
                "详细输出" => "rt_print".to_string(),
                "字符编码" => "rt_char_to_code".to_string(),
                _ => {
                    // 使用固定作用域确保定义和调用使用相同的哈希名
                    let def_hash_name = self.generate_hash_name(name, "");
                    let extern_hash_name = self.generate_hash_name(name, "__extern__");
                    // 先检查是否是已定义的用户函数（定义优先于外部声明）
                    if is_call && self.user_functions.contains_key(&def_hash_name) {
                        def_hash_name
                    } else if is_call && self.extern_functions.contains_key(&extern_hash_name) {
                        // 是外部函数调用，使用外部作用域的哈希名
                        extern_hash_name
                    } else {
                        // 函数定义或非外部函数调用，使用固定作用域生成哈希名
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
        let hash = hasher.finish();
        format!("{}_{:x}", encoded, hash)
    }

    /**
     * 计算 LLVM IR 字符串字面量解析后的实际字节长度
     * LLVM 在解析 c"..." 时会处理转义序列（如 \n, \t, \\ 等）
     */
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
    fn calculate_field_offset(&self, field_name: &str) -> i32 {
        // 在所有已注册的结构体布局中查找
        for (_struct_name, fields) in &self.struct_field_layouts {
            for (name, offset, _) in fields {
                if name == field_name {
                    return *offset;
                }
            }
        }
        // 如果未找到，默认偏移为0（兼容未注册的结构体）
        0
    }

    /**
     * 推断成员类型
     * 在已注册的结构体布局中查找字段的LLVM类型
     * 如果未找到，默认返回i64
     */
    fn infer_member_type(&self, field_name: &str) -> String {
        // 先在已注册的结构体布局中查找
        for (_struct_name, fields) in &self.struct_field_layouts {
            for (name, _, llvm_type) in fields {
                if name == field_name {
                    return llvm_type.clone();
                }
            }
        }
        // 如果未找到，根据字段命名模式推断
        // 常见的整数字段名
        match field_name {
            // 整数字段（位置/索引/计数类）
            "位置" | "长度" | "行号" | "列号" | "开始位置" | "结束位置" |
            "当前字符" | "当前行号" | "当前列号" | "当前位置" |
            "pos" | "count" | "tokenCount" | "nodeCount" | "funcCount" |
            "tempCount" | "labelCount" | "stringConstCount" | "indent" |
            "id" | "kind" | "line" | "intValue" |
            "状态" | "起始位置" | "是否错误" | "已恢复" | "跳过Token数" | "恢复点" |
            "激活" | "循环层级" | "错误计数" | "警告计数" | "层级" | "可变" |
            "已初始化" | "作用域层级" | "父作用域" | "全局作用域" | "当前作用域" |
            "当前函数返回类型" | "functionIndexCounter" | "currentFunctionIndex" => "i64".to_string(),
            // 默认为指针类型（文本/列表/指针字段）
            _ => "i8*".to_string(),
        }
    }

    /**
     * 生成新标签
     */
    fn new_label(&mut self, prefix: &str) -> String {
        let label = format!("{}_{}", prefix, self.label_counter);
        self.label_counter += 1;
        label
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
