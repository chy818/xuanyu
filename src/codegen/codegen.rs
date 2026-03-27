/**
 * @file codegen.rs
 * @brief CCAS 代码生成器
 * @description 将 AST 转换为 LLVM IR 代码
 * 
 * 功能:
 * - 模块定义生成
 * - 函数定义和调用
 * - 表达式 IR 生成
 * - 控制流 IR 生成
 * - RISC-V RV64GC 目标支持
 * - FFI 外部函数声明
 * - 常量定义
 * - 枚举类型
 */

use crate::ast::*;
use crate::error::CodegenError;

/**
 * LLVM IR 类型映射
 * 规范要求：
 * - 整数 → i64
 * - 浮点 → double (LLVM 中 f64 对应 double)
 * - 布尔 → i1
 * - 文本 → i8*
 * - 指针 → i8*
 * - 空 → void
 */
fn type_to_llvm(ty: &Type) -> &'static str {
    match ty {
        Type::Int => "i64",      // 规范：整数为 i64
        Type::Long => "i64",
        Type::Float => "double", // LLVM 中 f64 对应 double
        Type::Double => "double",
        Type::Bool => "i64",
        Type::String => "i8*",
        Type::Char => "i8",
        Type::Void => "void",
        Type::Pointer => "i8*",  // 规范：指针
        Type::List(_) => "i8*",     // 列表是指针
        Type::Optional(_) => "i64",
        Type::Array(_) => "i64",
        Type::Struct(_) => "i64",  // 结构体实例存储为 i64（指针值）
        Type::Unknown => "i64",     // 未知类型暂时用 i64 代替
        Type::TypeVar(_) => "i64",  // 类型变量暂时用 i64 代替
        Type::Custom(name) => {
            match name.as_str() {
                _ => "i64",
            }
        }
    }
}

/**
 * 代码生成器
 */
pub struct CodeGenerator {
    ir_output: String,
    indent: usize,
    label_counter: usize,
    /// Lambda 函数计数器
    lambda_counter: usize,
    /// 变量名到 SSA 值的映射
    variables: std::collections::HashMap<String, String>,
    /// 变量名到类型的映射
    variable_types: std::collections::HashMap<String, String>,
    /// 字符串常量表
    string_constants: std::collections::HashMap<String, (String, usize)>,
    /// 闭包变量映射：变量名 -> Lambda 函数名
    closures: std::collections::HashMap<String, String>,
    /// 当前函数名（用于尾调用优化）
    current_function_name: Option<String>,
    /// 当前函数参数列表
    current_function_params: Vec<String>,
    /// 函数入口标签
    entry_label: Option<String>,
    /// 是否处于尾调用位置
    in_tail_position: bool,
}

impl CodeGenerator {
    pub fn new() -> Self {
        Self {
            ir_output: String::new(),
            indent: 0,
            label_counter: 0,
            lambda_counter: 0,
            variables: std::collections::HashMap::new(),
            variable_types: std::collections::HashMap::new(),
            string_constants: std::collections::HashMap::new(),
            closures: std::collections::HashMap::new(),
            current_function_name: None,
            current_function_params: Vec::new(),
            entry_label: None,
            in_tail_position: false,
        }
    }

    /**
     * 计算结构体字段偏移
     * 简化处理：使用哈希表存储结构体字段信息
     */
    fn calculate_field_offset(&self, field_name: &str) -> i32 {
        // 常见的 Token 字段偏移（以 8 字节为单位）
        match field_name {
            "类型" | "type" => 0,
            "字面量" | "literal" => 8,
            "行" | "line" => 16,
            "列" | "column" => 24,
            _ => 0, // 默认偏移
        }
    }

    /**
     * 转义 LLVM 标识符
     * 规范要求：
     * - ASCII 字母数字下划线：直接输出
     * - 非 ASCII：转义并加双引号
     */
    fn escape_llvm_ident(&self, name: &str) -> String {
        // 检查是否全是 ASCII 字母数字下划线
        if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            name.to_string()
        } else {
            // 转义内部字符
            let escaped = name
                .replace('\\', "\\\\")
                .replace('"', "\\\"");
            format!("\"{}\"", escaped)
        }
    }

    /**
     * 生成 LLVM 函数声明名（不带 @ 前缀）
     * 中文函数名需要用引号包裹
     */
    fn emit_func_decl(&self, name: &str) -> String {
        if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            name.to_string()
        } else {
            format!("\"{}\"", name)
        }
    }

    /**
     * 翻译函数名（兼容内置函数）
     * 优先使用已知映射，否则使用 escape_llvm_ident
     */
    fn translate_func_name(&self, name: &str) -> String {
        // 已知内置函数映射
        let known_translation = match name {
            // 内置函数 - 映射到运行时库的实际函数名
            "打印" => Some("打印"),       // 运行时库中的函数名
            "打印整数" => Some("打印整数"),
            "打印浮点" => Some("打印浮点"),
            "打印布尔" => Some("打印布尔"),
            "新建列表" => Some("rt_list_new"),
            "列表追加" => Some("rt_list_append"),
            "列表获取" => Some("rt_list_get"),
            "列表长度" => Some("rt_list_len"),
            "文本长度" => Some("rt_string_len"),
            "文本获取字符" => Some("rt_string_char_at"),
            "字符编码" => Some("rt_char_to_code"),
            // 兼容旧名称 - 映射到新的运行时函数
            "创建列表" => Some("rt_list_new"),
            "列表添加" => Some("rt_list_append"),
            "读取行" => Some("rt_readline"),
            "文本转整数" => Some("str_to_int"),
            "整数转文本" => Some("int_to_str"),
            "文本拼接" => Some("str_concat"),
            "文本切片" => Some("str_slice"),
            "文本包含" => Some("str_contains"),
            "参数个数" => Some("argc"),
            "获取参数" => Some("argv"),
            // 控制台输入函数
            "输入整数" => Some("输入整数"),
            "输入文本" => Some("输入文本"),
            // 文件 I/O 函数
            "文件读取" => Some("文件读取"),
            "文件写入" => Some("文件写入"),
            "文件存在" => Some("文件存在"),
            "文件删除" => Some("文件删除"),
            // 系统命令函数
            "执行命令" => Some("exec_cmd"),
            "命令输出" => Some("cmd_output"),
            // 词法分析器用户函数
            "是空格" => Some("is_space"),
            "检查空格" => Some("check_space"),
            "是数字" => Some("is_digit"),
            "检查数字" => Some("check_digit"),
            "是字母" => Some("is_alpha"),
            "检查字母" => Some("check_alpha"),
            "是关键字" => Some("is_keyword"),
            "扫描数字" => Some("scan_digit"),
            "是字母数字" => Some("is_alnum"),
            "扫描标识符" => Some("scan_identifier"),
            "词法分析" => Some("lex"),
            "主" => Some("主"),  // 不翻译，保留中文用于 IR
            _ => None,
        };
        
        if let Some(translation) = known_translation {
            return translation.to_string();
        }
        
        // 未知的函数名，使用转义
        self.escape_llvm_ident(name)
    }

    fn emit(&mut self, line: &str) {
        let indent_str = "  ".repeat(self.indent);
        self.ir_output.push_str(&indent_str);
        self.ir_output.push_str(line);
        self.ir_output.push('\n');
    }

    fn emit_label(&mut self, label_name: &str) {
        // 移除前面的空行（只移除连续的空行，保留一个换行）
        while self.ir_output.ends_with("\n\n") {
            self.ir_output.pop();
        }
        self.ir_output.push_str(label_name);
        self.ir_output.push_str(":\n");
    }

    fn new_label(&mut self, prefix: &str) -> String {
        let label = format!("{}.{}", prefix, self.label_counter);
        self.label_counter += 1;
        label
    }

    /**
     * 推断表达式类型
     * 用于在代码生成时确定表达式的 LLVM 类型
     */
    fn infer_expression_type(&self, expr: &Expr) -> String {
        match expr {
            Expr::Literal(lit) => {
                // 根据字面量类型推断
                match &lit.kind {
                    LiteralKind::Integer(_) => "i64".to_string(),
                    LiteralKind::Float(_) => "double".to_string(),
                    LiteralKind::Boolean(_) => "i64".to_string(),
                    LiteralKind::String(_) => "i8*".to_string(),
                    LiteralKind::Char(_) => "i32".to_string(),
                }
            }
            Expr::Identifier(ident) => {
                // 查找变量类型 - 使用翻译后的名称
                let translated_name = self.translate_func_name(&ident.name);
                if let Some(var_type) = self.variable_types.get(&translated_name) {
                    var_type.clone()
                } else if let Some(var_type) = self.variable_types.get(&ident.name) {
                    var_type.clone()
                } else {
                    // 默认假设为整数
                    "i64".to_string()
                }
            }
            Expr::Binary(binary) => {
                // 二元运算结果类型
                let left_type = self.infer_expression_type(&binary.left);
                let right_type = self.infer_expression_type(&binary.right);
                
                // 比较运算返回布尔类型
                match binary.op {
                    BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Gt | BinaryOp::Lt | BinaryOp::Ge | BinaryOp::Le => "i1".to_string(),
                    _ => {
                        // 其他运算：如果任一操作数是浮点，结果为浮点
                        if left_type == "double" || right_type == "double" {
                            "double".to_string()
                        } else {
                            "i64".to_string()
                        }
                    }
                }
            }
            Expr::Call(call) => {
                // 函数调用返回类型
                let func_name = match &*call.function {
                    Expr::Identifier(ident) => ident.name.clone(),
                    _ => "unknown".to_string(),
                };
                
                // 内置函数返回类型
                match func_name.as_str() {
                    "打印" | "print" => "void".to_string(),
                    "整数转文本" | "int_to_str" => "i8*".to_string(),
                    "文本转整数" | "str_to_int" => "i64".to_string(),
                    "文本长度" | "str_len" => "i64".to_string(),
                    "文本切片" | "str_slice" => "i8*".to_string(),
                    _ => "i64".to_string(), // 默认用户函数返回 i64
                }
            }
            Expr::MemberAccess(_) => {
                // 成员访问：所有字段都存储为 i64
                "i64".to_string()
            }
            Expr::Unary(unary) => {
                // 一元运算：继承操作数类型
                self.infer_expression_type(&unary.operand)
            }
            Expr::ListLiteral(_) => {
                // 列表类型：返回 i8* 指针
                "i8*".to_string()
            }
            Expr::IndexAccess(index) => {
                // 推断对象类型
                let obj_type = self.infer_expression_type(&index.object);
                if obj_type == "i8*" {
                    // 字符串索引：返回 i64（字节值）
                    "i64".to_string()
                } else {
                    // 列表索引：返回 i64
                    "i64".to_string()
                }
            }
            Expr::ListComprehension(_) => {
                // 列表推导式：返回列表类型 i8*
                "i8*".to_string()
            }
            Expr::Lambda(_) => {
                // Lambda 表达式：返回函数指针类型 i8*
                "i8*".to_string()
            }
            Expr::Grouped(expr) => {
                // 括号表达式：继承内部表达式类型
                self.infer_expression_type(expr)
            }
        }
    }

    /**
     * 加载并解析导入模块
     * 递归处理模块导入，收集所有函数定义
     */
    fn load_imported_module(&self, module_path: &str, processed_modules: &mut std::collections::HashSet<String>) -> Result<Module, CodegenError> {
        // 去除引号
        let path = module_path.trim_matches('"');
        
        // 检查是否已处理过该模块
        if processed_modules.contains(path) {
            return Ok(Module {
                imports: vec![],
                functions: vec![],
                structs: vec![],
                enums: vec![],
                type_aliases: vec![],
                constants: vec![],
                extern_functions: vec![],
                span: crate::lexer::token::Span::dummy(),
            });
        }
        
        // 标记为已处理
        processed_modules.insert(path.to_string());
        
        // 读取模块文件
        let source = std::fs::read_to_string(path)
            .map_err(|e| CodegenError {
                code: "C001".to_string(),
                message: format!("无法加载模块 '{}': {}", path, e),
            })?;
        
        // 词法分析
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize()
            .map_err(|e| CodegenError {
                code: "C002".to_string(),
                message: format!("词法分析错误: {}", e.message),
            })?;
        
        // 语法分析
        let ast = crate::parser::parse(tokens)
            .map_err(|e| CodegenError {
                code: "C003".to_string(),
                message: format!("语法分析错误: {}", e.message),
            })?;
        
        Ok(ast)
    }

    /**
     * 递归收集模块及其导入的所有函数
     * 注意：使用 collected_names 在整个递归过程中去重
     */
    fn collect_all_functions(&self, module: &Module, collected_names: &mut std::collections::HashSet<String>, processed_modules: &mut std::collections::HashSet<String>) -> Result<Vec<Function>, CodegenError> {
        let mut all_functions = Vec::new();
        
        // 先处理导入模块
        for import in &module.imports {
            let imported_module = self.load_imported_module(&import.module_path, processed_modules)?;

            // 递归收集导入模块的函数
            let imported_functions = self.collect_all_functions(&imported_module, collected_names, processed_modules)?;

            // 直接添加递归返回的函数（递归调用已经去重并添加到 collected_names 了）
            all_functions.extend(imported_functions);
        }

        // 添加当前模块的函数（去重）
        for func in &module.functions {
            if !collected_names.contains(&func.name) {
                collected_names.insert(func.name.clone());
                all_functions.push(func.clone());
            }
        }

        Ok(all_functions)
    }

    /**
     * 生成模块
     */
    pub fn generate(&mut self, module: &Module) -> Result<String, CodegenError> {
        // 清空字符串常量表，为新文件做准备
        self.string_constants.clear();
        
        // 收集所有函数（包括导入模块的函数）
        let mut collected_names = std::collections::HashSet::new();
        let mut processed_modules = std::collections::HashSet::new();
        let all_functions = self.collect_all_functions(module, &mut collected_names, &mut processed_modules)?;
        
        // 查找主函数并记录其返回类型
        // 优先查找 "主" 函数，如果没有则查找 "main" 函数
        let mut main_func_return_type: Option<Type> = None;
        let mut has_main_func = false;  // 用户是否定义了 main 函数
        
        for func in &all_functions {
            if func.name == "主" {
                main_func_return_type = Some(func.return_type.clone());
                break;
            }
        }
        
        // 如果没有 "主" 函数，检查是否有 "main" 函数
        if main_func_return_type.is_none() {
            for func in &all_functions {
                if func.name == "main" {
                    main_func_return_type = Some(func.return_type.clone());
                    has_main_func = true;
                    break;
                }
            }
        }
        
        // 生成内置函数声明
        self.emit("; ==================== 内置函数 ====================");
        self.emit("");
        
        // 运行时库函数 - 规范要求
        // 打印函数 (使用 ASCII 函数名以避免 LLVM IR 兼容性问题)
        self.emit("declare void @print(i8*)");
        
        // 列表操作
        self.emit("declare i8* @rt_list_new()");
        self.emit("declare void @rt_list_append(i8*, i8*)");
        self.emit("declare i8* @rt_list_get(i8*, i64)");
        self.emit("declare i64 @rt_list_len(i8*)");
        
        // 文本操作
        self.emit("declare i64 @rt_string_len(i8*)");
        
        // 控制台输入函数
        self.emit("declare i64 @\"输入整数\"()");
        self.emit("declare i8* @\"输入文本\"()");
        
        // 打印整数/浮点/布尔函数 (ASCII 函数名)
        self.emit("declare void @print_int(i64)");
        self.emit("declare void @print_float(double)");
        self.emit("declare void @print_bool(i1)");
        
        // 类型转换函数
        self.emit("declare i64 @str_to_int(i8*)");
        self.emit("declare i8* @int_to_str(i64)");
        
        // 列表函数（旧名称兼容）
        self.emit("declare i8* @create_list(i64)");
        self.emit("declare i64 @list_add(i8*, i64)");
        self.emit("declare i64 @list_get(i8*, i64)");
        self.emit("declare i64 @list_len(i8*)");
        
        // 文件 I/O 函数 (ASCII 函数名)
        self.emit("declare i8* @file_read(i8*)");
        self.emit("declare i32 @file_write(i8*, i8*)");
        self.emit("declare i32 @file_exists(i8*)");
        self.emit("declare i32 @file_delete(i8*)");
        
        // 系统命令函数
        self.emit("declare i32 @exec_cmd(i8*)");
        self.emit("declare i8* @cmd_output(i8*)");
        
        // 命令行参数函数
        self.emit("declare i64 @argc()");
        self.emit("declare i8* @argv(i64)");
        
        // 字符串函数
        self.emit("declare i8* @str_concat(i8*, i8*)");
        self.emit("declare i8* @str_slice(i8*, i64, i64)");
        self.emit("declare i8* @str_contains(i8*, i8*)");
        
        self.emit("");
        self.emit("; ==================== 用户函数 ====================");
        self.emit("");

        // 生成每个函数的 IR（包括导入模块的函数）
        for func in all_functions.iter() {
            self.generate_function(func)?;
        }

        // 生成入口点包装函数（仅当用户定义了 "主" 函数时）
        // 如果用户定义了 "main" 函数，则不需要生成包装函数
        if let Some(ret_type) = main_func_return_type {
            if !has_main_func {
                // 用户定义了 "主" 函数，需要生成 main 包装
                self.emit("");
                self.emit("; ==================== 入口点包装 ====================");
                self.emit("");
                self.generate_main_wrapper(&ret_type);
            }
        }

        // 生成外部函数声明 (FFI)
        if !module.extern_functions.is_empty() {
            self.emit("");
            self.emit("; ==================== 外部函数声明 ====================");
            self.emit("");
            for ext_func in &module.extern_functions {
                self.generate_extern_function(ext_func)?;
            }
        }

        // 生成常量定义
        if !module.constants.is_empty() {
            self.emit("");
            self.emit("; ==================== 常量定义 ====================");
            self.emit("");
            for constant in &module.constants {
                self.generate_global_constant(constant)?;
            }
        }

        // 在用户函数之后生成字符串常量
        self.emit("");
        self.emit("; ==================== 字符串常量 ====================");
        self.emit("");
        self.generate_string_constants();

        Ok(self.ir_output.clone())
    }
    
    /**
     * 生成 main 入口点包装函数
     * 规范要求：
     * - 若 主 返回 空: call @"主"(); ret i32 0
     * - 若 主 返回 整数: %ret = call @"主"(); ret i32 (i32 %ret)
     * - 若 主 返回其他: 编译错误（在此阶段不处理）
     */
    fn generate_main_wrapper(&mut self, return_type: &Type) {
        self.emit("define i32 @main() {");
        
        match return_type {
            Type::Void => {
                // 情形 A: 返回空
                self.emit("  call void @\"主\"()");
                self.emit("  ret i32 0");
            }
            Type::Int | Type::Long => {
                // 情形 B: 返回整数
                let llvm_type = type_to_llvm(return_type);
                self.emit(&format!("  %ret = call {} @\"主\"()", llvm_type));
                // 将 i64 截断为 i32
                self.emit("  %trunc = trunc i64 %ret to i32");
                self.emit("  ret i32 %trunc");
            }
            _ => {
                // 其他类型：暂不处理
                self.emit("  ret i32 0");
            }
        }
        
        self.emit("}");
    }

    /**
     * 生成外部函数声明 (FFI)
     * 外部函数声明生成 LLVM declare 语句
     */
    fn generate_extern_function(&mut self, ext_func: &ExternFunction) -> Result<(), CodegenError> {
        let ret_type = type_to_llvm(&ext_func.return_type);
        
        // 生成参数列表
        let mut param_strs = Vec::new();
        for param in &ext_func.params {
            let param_type = type_to_llvm(&param.param_type);
            param_strs.push(format!("{}", param_type));
        }
        let params_str = param_strs.join(", ");
        
        // 确定函数名：使用链接名（如果有）或翻译后的名字
        let func_name = if let Some(link_name) = &ext_func.link_name {
            link_name.clone()
        } else {
            self.translate_func_name(&ext_func.name)
        };
        
        self.emit(&format!("declare {} @{} ({})", ret_type, func_name, params_str));
        
        // 注册到变量映射
        self.variables.insert(ext_func.name.clone(), func_name);
        self.variable_types.insert(ext_func.name.clone(), ret_type.to_string());
        
        Ok(())
    }

    /**
     * 生成全局常量定义
     */
    fn generate_global_constant(&mut self, constant: &ConstantDef) -> Result<(), CodegenError> {
        let llvm_type = type_to_llvm(&constant.const_type);
        let const_name = self.escape_llvm_ident(&constant.name);
        
        // 生成常量值
        let const_value = self.generate_expression(&constant.value)?;
        
        // 生成全局常量声明
        self.emit(&format!("@{} = constant {} {}", const_name, llvm_type, format!("%{}", const_value)));
        
        Ok(())
    }

    /**
     * 生成字符串常量
     */
    fn generate_string_constants(&mut self) {
        // 将 HashMap 转换为 Vec 以避免借用问题
        let constants: Vec<(String, String, usize)> = self.string_constants
            .iter()
            .map(|(k, &(ref v, len))| (k.clone(), v.clone(), len))
            .collect();
        
        for (label, content, array_size) in constants {
            // 使用存储的长度（已经包含 null 终止符）
            self.emit(&format!("@{} = private constant [{} x i8] c\"{}\"",
                label, array_size, content));
        }
    }

    /**
     * 生成函数
     */
    fn generate_function(&mut self, func: &Function) -> Result<(), CodegenError> {
        // 函数头 - 生成函数签名
        let ret_type = type_to_llvm(&func.return_type);

        // 重置函数作用域的变量映射和计数器
        // （文档建议：每个函数作用域独立计数）
        self.variables.clear();
        self.variable_types.clear();
        self.label_counter = 0;
        self.closures.clear();

        // 保存当前函数信息（用于尾调用优化）
        let func_name = func.name.clone();
        self.current_function_name = Some(func_name.clone());
        self.current_function_params = func.params.iter()
            .map(|p| self.translate_func_name(&p.name))
            .collect();

        // 生成参数列表
        let mut param_strs = Vec::new();
        let mut translated_param_names = Vec::new();
        for param in &func.params {
            let param_type = type_to_llvm(&param.param_type);
            let translated_name = self.translate_func_name(&param.name);
            translated_param_names.push(translated_name.clone());
            param_strs.push(format!("{} %{}", param_type, translated_name));
        }
        let params_str = param_strs.join(", ");

        // 翻译函数名
        let llvm_func_name = self.translate_func_name(&func_name);

        // 根据函数名是否包含非ASCII字符决定格式
        let func_def = if func_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            // ASCII 函数名：直接使用
            format!("define {} @{} ({}) {{", ret_type, llvm_func_name, params_str)
        } else {
            // 非ASCII 函数名：使用引号包裹
            format!("define {} @\"{}\" ({}) {{", ret_type, func_name, params_str)
        };
        self.emit(&func_def);
        self.emit(&format!("; 函数: {}", func.name));

        // 创建函数入口标签（用于尾调用优化）
        let entry = self.new_label("entry");
        self.entry_label = Some(entry.clone());
        self.emit(&format!("{}:", entry));

        // 为每个参数创建分配指令
        for (param, translated_name) in func.params.iter().zip(translated_param_names.iter()) {
            let param_type = type_to_llvm(&param.param_type);
            let alloca = self.new_label("v");  // 文档建议：使用 %v_0, %v_1 格式
            self.emit(&format!("%{} = alloca {}", alloca, param_type));
            self.emit(&format!("store {} %{}, {}* %{}", param_type, translated_name, param_type, alloca));
            // 使用翻译后的名字作为 key，以便在表达式引用时能找到
            self.variables.insert(translated_name.clone(), alloca);
            self.variable_types.insert(translated_name.clone(), param_type.to_string());
        }

        // 检查函数是否有显式返回语句
        let has_return = func.body.statements.iter().any(|stmt| {
            matches!(stmt, Stmt::Return(_))
        });

        // 函数体语句
        for stmt in &func.body.statements {
            // 每个语句开始时重置尾调用位置标记
            self.in_tail_position = false;
            self.generate_statement(stmt)?;
        }

        // 如果没有返回语句，添加默认返回
        if !has_return {
            if func.return_type == Type::Void {
                self.emit("ret void");
            } else {
                self.emit("ret i64 0");
            }
        }

        self.emit("}");

        // 清除函数信息
        self.current_function_name = None;
        self.current_function_params.clear();
        self.entry_label = None;
        self.in_tail_position = false;

        Ok(())
    }

    /**
     * 生成语句
     */
    fn generate_statement(&mut self, stmt: &Stmt) -> Result<(), CodegenError> {
        match stmt {
            Stmt::Let(let_stmt) => {
                self.generate_let_statement(let_stmt)
            }
            Stmt::Return(return_stmt) => {
                self.generate_return_statement(return_stmt)
            }
            Stmt::Expr(expr_stmt) => {
                self.generate_expression(&expr_stmt.expr)?;
                Ok(())
            }
            Stmt::If(if_stmt) => {
                self.generate_if_statement(if_stmt)
            }
            Stmt::Loop(loop_stmt) => {
                self.generate_loop_statement(loop_stmt)
            }
            Stmt::Block(block_stmt) => {
                self.generate_block_statement(block_stmt)
            }
            Stmt::Assignment(assign_stmt) => {
                self.generate_assignment_statement(assign_stmt)
            }
            Stmt::Break(_) | Stmt::Continue(_) => {
                Ok(()) // TODO: 实现 break/continue
            }
            Stmt::StructDef(_) => {
                Ok(()) // TODO: 实现结构体定义生成
            }
            Stmt::EnumDef(_) => {
                Ok(()) // TODO: 实现枚举定义生成
            }
            Stmt::TypeAlias(_) => {
                Ok(()) // TODO: 实现类型别名生成
            }
            Stmt::Constant(constant) => {
                self.generate_constant_statement(constant)
            }
            Stmt::Match(match_stmt) => {
                self.generate_match_statement(match_stmt)
            }
        }
    }

    /**
     * 生成模式匹配语句
     * 模式匹配编译为一系列条件分支
     */
    fn generate_match_statement(&mut self, match_stmt: &MatchStmt) -> Result<(), CodegenError> {
        // 生成要匹配的值
        let _subject_value = self.generate_expression(&match_stmt.subject)?;
        
        // 为每个分支生成标签
        let end_label = self.new_label("match_end");
        
        // 生成每个分支
        for arm in &match_stmt.arms {
            match &arm.pattern {
                MatchPattern::EnumVariant { variant_name: _, fields, .. } => {
                    // 创建分支标签
                    let arm_label = self.new_label("match_arm");
                    
                    // TODO: 实际应该检查枚举标签匹配
                    // 这里简化为直接生成分支体
                    self.emit_label(&arm_label);
                    
                    // 将捕获的字段绑定到变量
                    for field in fields {
                        // 简化：生成一个虚拟的变量绑定
                        let field_alloca = self.new_label("field");
                        self.emit(&format!("%{} = alloca i64", field_alloca));
                        self.variables.insert(field.binding_name.clone(), field_alloca);
                        self.variable_types.insert(field.binding_name.clone(), "i64".to_string());
                    }
                    
                    // 生成分支体
                    self.generate_statement(&arm.body)?;
                    
                    // 跳转到结束
                    self.emit(&format!("br label %{}", end_label));
                }
                MatchPattern::Wildcard => {
                    // 默认分支
                    let arm_label = self.new_label("match_default");
                    self.emit_label(&arm_label);
                    self.generate_statement(&arm.body)?;
                    self.emit(&format!("br label %{}", end_label));
                }
            }
        }
        
        // 结束标签
        self.emit_label(&end_label);
        
        Ok(())
    }

    /**
     * 生成常量定义语句
     * 常量在编译期求值，生成全局常量
     */
    fn generate_constant_statement(&mut self, constant: &ConstantDef) -> Result<(), CodegenError> {
        // 记录常量到变量映射（常量作为全局常量处理）
        let _const_value = self.generate_expression(&constant.value)?;
        
        // 常量存储为全局变量
        let llvm_type = type_to_llvm(&constant.const_type);
        let const_name = self.escape_llvm_ident(&constant.name);
        
        // 在模块级别生成常量定义（在 generate 函数中处理）
        // 这里只记录常量名映射
        self.variables.insert(const_name.clone(), format!("@{}", const_name));
        self.variable_types.insert(const_name, llvm_type.to_string());
        
        Ok(())
    }

    /**
     * 生成变量声明语句
     */
    fn generate_let_statement(&mut self, let_stmt: &LetStmt) -> Result<(), CodegenError> {
        // 获取变量类型
        let var_type = let_stmt.type_annotation
            .as_ref()
            .map(|t| type_to_llvm(t))
            .unwrap_or("i64");

        // 分配局部变量
        let alloca = self.new_label("alloca");
        self.emit(&format!("%{} = alloca {}", alloca, var_type));

        // 如果有初始化值
        if let Some(init) = &let_stmt.initializer {
            let value = self.generate_expression(init)?;

            // 获取初始化表达式的类型
            let init_type = self.infer_expression_type(init);

            // 检测是否是 Lambda 表达式并记录闭包
            if matches!(init, Expr::Lambda(_)) {
                // Lambda 表达式：value 是闭包指针 (i64)
                let closure_name = format!("closure_{}", let_stmt.name);
                self.closures.insert(closure_name.clone(), closure_name.clone());
                self.variables.insert(closure_name.clone(), alloca.clone());
                self.variable_types.insert(closure_name, "i64".to_string());
            }

            // 类型匹配处理
            if var_type == "i8*" && init_type == "i8*" {
                // 列表类型：存储 i8* 指针
                self.emit(&format!("store i8* %{}, i8** %{}", value, alloca));
            } else if var_type == init_type {
                // 类型匹配
                self.emit(&format!("store {} %{}, {}* %{}", var_type, value, var_type, alloca));
            } else {
                // 类型不匹配，尝试转换
                self.emit(&format!("store {} %{}, {}* %{}", init_type, value, var_type, alloca));
            }
        }

        // 记录变量及其类型 - 使用翻译后的名字
        let translated_name = self.translate_func_name(&let_stmt.name);
        self.variables.insert(translated_name.clone(), alloca);
        self.variable_types.insert(translated_name, var_type.to_string());

        Ok(())
    }

    /**
     * 生成返回语句（支持尾调用优化）
     */
    fn generate_return_statement(&mut self, return_stmt: &ReturnStmt) -> Result<(), CodegenError> {
        if let Some(value) = &return_stmt.value {
            // 检查是否是尾递归调用
            if self.in_tail_position {
                if let Expr::Call(call_expr) = value {
                    // 检查是否是调用当前函数
                    if let Expr::Identifier(ident) = &*call_expr.function {
                        if let Some(current_func) = &self.current_function_name {
                            let called_func = &ident.name;
                            // 翻译函数名进行比较
                            let translated_called = self.translate_func_name(called_func);
                            let translated_current = self.translate_func_name(current_func);

                            if translated_called == translated_current {
                                // 检测到尾递归！将调用转换为参数更新 + 跳转
                                self.emit("; 尾递归优化：跳转回入口");
                                self.emit(&format!("br label %{}", self.entry_label.as_ref().unwrap()));
                                return Ok(());
                            }
                        }
                    }
                }
            }

            // 如果不是尾递归，正常生成返回
            let result = self.generate_expression(value)?;
            let ret_type = type_to_llvm(&Type::Int); // 简化
            self.emit(&format!("ret {} %{}", ret_type, result));
        } else {
            self.emit("ret void");
        }
        Ok(())
    }

    /**
     * 生成 if 语句
     */
    fn generate_if_statement(&mut self, if_stmt: &IfStmt) -> Result<(), CodegenError> {
        // 生成条件
        if let Some(branch) = if_stmt.branches.first() {
            let cond_result = self.generate_expression(&branch.condition)?;

            // 创建标签
            let then_label = self.new_label("then");
            let else_label = self.new_label("else");
            let end_label = self.new_label("ifend");

            // 条件分支：先比较条件是否非零，得到 i1
            let cond_i1 = self.new_label("cond_bool");
            self.emit(&format!("%{} = icmp ne i64 %{}, 0", cond_i1, cond_result));
            self.emit(&format!("br i1 %{}, label %{}, label %{}",
                cond_i1, then_label, else_label));

            // then 分支
            self.emit_label(&then_label);
            self.generate_statement(&branch.body)?;

            // 跳转到结束
            self.emit(&format!("br label %{}", end_label));

            // else 分支
            self.emit_label(&else_label);
            if let Some(else_body) = &if_stmt.else_branch {
                self.generate_statement(else_body)?;
            }

            // else 分支也需要跳转到结束
            self.emit(&format!("br label %{}", end_label));

            // 结束标签
            self.emit_label(&end_label);

            // if-else 结构结束后处于尾位置（因为 if 和 else 分支都会跳到这里）
            self.in_tail_position = true;
        }

        Ok(())
    }

    /**
     * 生成循环语句
     * while 循环结构:
     *   loop_start:
     *     条件判断
     *     br cond, loop_body, loop_end
     *   loop_body:
     *     循环体
     *     br loop_start
     *   loop_end:
     */
    fn generate_loop_statement(&mut self, loop_stmt: &LoopStmt) -> Result<(), CodegenError> {
        let loop_start = self.new_label("loop");
        let loop_body = self.new_label("loopbody");
        let loop_end = self.new_label("loopend");

        // 跳到循环条件判断
        self.emit(&format!("br label %{}", loop_start));

        // 循环条件判断入口
        self.emit_label(&loop_start);

        // 生成循环条件 (如果有)
        if let Some(cond) = &loop_stmt.condition {
            let cond_result = self.generate_expression(cond)?;
            // 条件为真跳到循环体，为假跳到循环结束
            self.emit(&format!("br i1 %{}, label %{}, label %{}", 
                cond_result, loop_body, loop_end));
        } else {
            // 无限循环，直接跳到循环体
            self.emit(&format!("br label %{}", loop_body));
        }

        // 循环体入口
        self.emit_label(&loop_body);

        // 循环体处于尾位置（如果最后是 return递归 可以优化）
        self.in_tail_position = true;

        // 生成循环体
        self.generate_statement(&loop_stmt.body)?;

        // 循环体执行完后，跳回条件判断（这不是尾调用优化的位置）
        self.in_tail_position = false;
        self.emit(&format!("br label %{}", loop_start));

        // 循环结束标签
        self.emit_label(&loop_end);

        // 循环结束后处于尾位置
        self.in_tail_position = true;

        Ok(())
    }

    /**
     * 生成块语句
     */
    fn generate_block_statement(&mut self, block_stmt: &BlockStmt) -> Result<(), CodegenError> {
        for stmt in &block_stmt.statements {
            self.generate_statement(stmt)?;
        }
        // 块语句结束后处于尾位置
        if !block_stmt.statements.is_empty() {
            self.in_tail_position = true;
        }
        Ok(())
    }

    /**
     * 生成赋值语句
     */
    fn generate_assignment_statement(&mut self, assign_stmt: &AssignmentStmt) -> Result<(), CodegenError> {
        // 生成值表达式
        let value = self.generate_expression(&assign_stmt.value)?;
        
        // 获取目标变量名并更新映射
        if let Expr::Identifier(ident) = &assign_stmt.target {
            // 翻译变量名（处理中文变量名）
            let translated_name = self.translate_func_name(&ident.name);
            if let Some(alloca) = self.variables.get(&translated_name).cloned() {
                // 获取变量类型
                let var_type = self.variable_types.get(&translated_name)
                    .cloned()
                    .unwrap_or_else(|| "i64".to_string());
                // 存储到已有变量
                self.emit(&format!("store {} %{}, {}* %{}", var_type, value, var_type, alloca));
            } else {
                // 获取值类型（从表达式推断）
                let var_type = "i64".to_string(); // 默认类型
                // 新变量，分配空间
                let new_alloca = self.new_label("alloca");
                self.emit(&format!("%{} = alloca {}", new_alloca, var_type));
                self.emit(&format!("store {} %{}, {}* %{}", var_type, value, var_type, new_alloca));
                self.variables.insert(translated_name.clone(), new_alloca);
                self.variable_types.insert(translated_name.clone(), var_type);
            }
        } else {
            // 目标不是标识符，生成注释
            self.emit(&format!("; 赋值目标不是标识符"));
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
                        self.emit(&format!("%{} = load i8*, i8** %{}", load, alloca));
                    } else {
                        // 其他类型
                        self.emit(&format!("%{} = load {}, {}* %{}", load, var_type, var_type, alloca));
                    }
                    Ok(load)
                } else if let Some(alloca) = self.variables.get(&ident.name).cloned() {
                    // 尝试原始名称（处理枚举变体等未翻译的名称）
                    let var_type = self.variable_types.get(&ident.name)
                        .cloned()
                        .unwrap_or_else(|| "i64".to_string());
                    let load = self.new_label("id");
                    
                    if var_type == "i8*" {
                        self.emit(&format!("%{} = load i8*, i8** %{}", load, alloca));
                    } else {
                        self.emit(&format!("%{} = load {}, {}* %{}", load, var_type, var_type, alloca));
                    }
                    Ok(load)
                } else {
                    // 对于枚举变体，生成一个整数值
                    // 枚举成员按定义顺序编号：第一个成员为 0，第二个为 1，以此类推
                    let enum_value = match ident.name.as_str() {
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
                    };
                    let load = self.new_label("enum");
                    self.emit(&format!("%{} = add i64 0, {}", load, enum_value));
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
                        self.emit(&format!("%{} = inttoptr i64 %{} to i8*", ptr, object_val));
                        ptr
                    };
                    
                    match field_name.as_str() {
                        "长度" => {
                            // 调用 rt_list_len，返回 i64
                            let result = self.new_label("len");
                            self.emit(&format!("%{} = call i64 @rt_list_len(i8* %{})", result, ptr_val));
                            Ok(result)
                        }
                        _ => {
                            // 其他方法返回指针值
                            Ok(ptr_val)
                        }
                    }
                } else {
                    // 结构体字段访问
                    let object_val = self.generate_expression(&member.object)?;
                    
                    // 计算字段偏移
                    let field_offset = self.calculate_field_offset(field_name);
                    
                    // 将 i64 值转换为指针
                    let ptr_val = self.new_label("id");
                    self.emit(&format!("%{} = inttoptr i64 %{} to i8*", ptr_val, object_val));
                    
                    // 生成 GEP 指令获取字段指针
                    let result = self.new_label("member");
                    self.emit(&format!("%{} = getelementptr i8, i8* %{}, i32 {}", 
                        result, ptr_val, field_offset));
                    
                    // 将指针转换为 i64 指针
                    let result_ptr = self.new_label("member_ptr");
                    self.emit(&format!("%{} = bitcast i8* %{} to i64*", result_ptr, result));
                    
                    // 加载字段值
                    let result_val = self.new_label("member_val");
                    self.emit(&format!("%{} = load i64, i64* %{}", result_val, result_ptr));
                    
                    Ok(result_val)
                }
            }
            Expr::Grouped(expr) => {
                self.generate_expression(expr)
            }
            Expr::ListLiteral(list) => {
                // 创建列表
                let list_ptr = self.new_label("list");
                self.emit(&format!("%{} = call i8* @rt_list_new()", list_ptr));
                
                // 添加元素
                for elem in &list.elements {
                    let elem_val = self.generate_expression(elem)?;
                    // 根据元素类型决定如何处理
                    let elem_type = self.infer_expression_type(elem);
                    let elem_ptr = self.new_label("elem_ptr");
                    
                    if elem_type == "i8*" {
                        // 字符串类型，直接使用
                        self.emit(&format!("call void @rt_list_append(i8* %{}, i8* %{})", list_ptr, elem_val));
                    } else {
                        // 其他类型，转换为指针
                        self.emit(&format!("%{} = inttoptr {} %{} to i8*", elem_ptr, elem_type, elem_val));
                        self.emit(&format!("call void @rt_list_append(i8* %{}, i8* %{})", list_ptr, elem_ptr));
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
                self.emit(&format!("%{} = call i8* @rt_list_new()", result_list));
                
                // 获取原列表
                let src_list = self.generate_expression(&comp.iterable)?;
                
                // 获取原列表长度
                let src_len = self.new_label("src_len");
                self.emit(&format!("%{} = call i64 @rt_list_len(i8* %{})", src_len, src_list));
                
                // 循环变量
                let i_alloca = self.new_label("i_alloca");
                self.emit(&format!("%{} = alloca i64", i_alloca));
                self.emit(&format!("store i64 0, i64* %{}", i_alloca));
                
                // 循环开始标签
                let loop_start = self.label_counter;
                self.label_counter += 1;
                let loop_body = self.label_counter;
                self.label_counter += 1;
                let loop_end = self.label_counter;
                self.label_counter += 1;
                
                self.emit(&format!("br label %L{}", loop_start));
                self.emit(&format!("L{}:", loop_start));
                
                // 检查循环条件: i < len
                let i_val = self.new_label("i_val");
                self.emit(&format!("%{} = load i64, i64* %{}", i_val, i_alloca));
                let cond = self.new_label("cond");
                self.emit(&format!("%{} = icmp slt i64 %{}, %{}", cond, i_val, src_len));
                self.emit(&format!("br i1 %{}, label %L{}, label %L{}", cond, loop_body, loop_end));
                
                // 循环体
                self.emit(&format!("L{}:", loop_body));
                
                // 获取当前元素
                let elem = self.new_label("elem");
                self.emit(&format!("%{} = call i8* @rt_list_get(i8* %{}, i64 %{})", elem, src_list, i_val));
                
                // 将元素转换为 i64 并存储到迭代变量
                let elem_val = self.new_label("elem_val");
                self.emit(&format!("%{} = ptrtoint i8* %{} to i64", elem_val, elem));
                
                // 存储迭代变量
                let var_alloca = self.new_label(&format!("var_{}", comp.var_name));
                self.emit(&format!("%{} = alloca i64", var_alloca));
                self.emit(&format!("store i64 %{}, i64* %{}", elem_val, var_alloca));
                
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
                    
                    // 条件跳转标签
                    let do_append = self.label_counter;
                    self.label_counter += 1;
                    let skip_append = self.label_counter;
                    self.label_counter += 1;
                    
                    // 检查条件：为真则添加，为假则跳过
                    self.emit(&format!("br i1 %{}, label %L{}, label %L{}", cond_result, do_append, skip_append));
                    
                    // 添加元素
                    self.emit(&format!("L{}:", do_append));
                    
                    // 添加到结果列表
                    let output_ptr = self.new_label("output_ptr");
                    self.emit(&format!("%{} = inttoptr i64 %{} to i8*", output_ptr, output_val));
                    self.emit(&format!("call void @rt_list_append(i8* %{}, i8* %{})", result_list, output_ptr));
                    
                    // 跳过添加后的继续点
                    let after_append = self.label_counter;
                    self.label_counter += 1;
                    self.emit(&format!("br label %L{}", after_append));
                    
                    // 跳过添加
                    self.emit(&format!("L{}:", skip_append));
                    self.emit(&format!("br label %L{}", after_append));
                    
                    // 继续循环
                    self.emit(&format!("L{}:", after_append));
                } else {
                    // 无条件过滤，直接添加到结果列表
                    let output_ptr = self.new_label("output_ptr");
                    self.emit(&format!("%{} = inttoptr i64 %{} to i8*", output_ptr, output_val));
                    self.emit(&format!("call void @rt_list_append(i8* %{}, i8* %{})", result_list, output_ptr));
                }
                
                // 递增循环变量
                let i_next = self.new_label("i_next");
                self.emit(&format!("%{} = add i64 %{}, 1", i_next, i_val));
                self.emit(&format!("store i64 %{}, i64* %{}", i_next, i_alloca));
                self.emit(&format!("br label %L{}", loop_start));
                
                // 循环结束
                self.emit(&format!("L{}:", loop_end));
                
                // 返回结果列表
                Ok(result_list)
            }
            Expr::Lambda(lambda) => {
                // Lambda 表达式：生成完整闭包
                let lambda_id = self.lambda_counter;
                self.lambda_counter += 1;

                let lambda_func_name = format!("lambda_{}_func", lambda_id);
                let closure_var_name = format!("closure_{}", lambda_id);
                let captured_count = lambda.captured_vars.len();

                // ===== 第一步：生成 Lambda 函数 =====
                // Lambda 函数签名: i64 (i8*)
                // 第一个参数是闭包上下文的 i8* 指针
                self.emit(&format!("define i64 @{}(i8* %ctx) {{", lambda_func_name));

                // 在函数入口保存当前上下文
                self.emit("entry:");

                // 为参数分配空间并注册
                for param in &lambda.params {
                    let param_type = type_to_llvm(&param.param_type);
                    let param_alloca = self.new_label("param");
                    self.emit(&format!("%{} = alloca {}", param_alloca, param_type));
                    let translated_name = self.translate_func_name(&param.name);
                    self.variables.insert(translated_name.clone(), param_alloca.clone());
                    self.variable_types.insert(translated_name, param_type.to_string());
                }

                // ===== 从闭包上下文加载捕获的变量 =====
                let mut captured_offset = 16;
                for captured_var in &lambda.captured_vars {
                    let captured_type = type_to_llvm(&captured_var.var_type);
                    let captured_alloca = self.new_label("captured");
                    self.emit(&format!("%{} = alloca {}", captured_alloca, captured_type));

                    // 计算捕获变量的 GEP
                    let gep = self.new_label("gep");
                    self.emit(&format!("%{} = getelementptr i8, i8* %ctx, i64 {}", gep, captured_offset));

                    // 转换为目标类型指针并加载
                    let ptr_cast = self.new_label("ptr_cast");
                    self.emit(&format!("%{} = bitcast i8* %{} to {}*", ptr_cast, gep, captured_type));
                    let loaded = self.new_label("loaded");
                    self.emit(&format!("%{} = load {}, {}* %{}", loaded, captured_type, captured_type, ptr_cast));

                    // 存储到局部变量
                    self.emit(&format!("store {} %{}, {}* %{}", captured_type, loaded, captured_type, captured_alloca));

                    // 注册到变量表
                    self.variables.insert(captured_var.name.clone(), captured_alloca.clone());
                    self.variable_types.insert(captured_var.name.clone(), captured_type.to_string());

                    // 更新偏移量
                    let type_size = if captured_type == "double" { 8 } else { 8 };
                    captured_offset += type_size;
                }

                // 生成函数体
                let body_result = self.generate_expression(&lambda.body)?;

                // 生成返回指令
                self.emit(&format!("ret i64 %{}", body_result));
                self.emit("}");

                // ===== 第二步：创建闭包结构 =====

                // 计算闭包大小：16 字节头 + 每个捕获变量 8 字节
                let closure_size = 16 + captured_count * 8;
                let size_label = self.new_label("closure_size");
                self.emit(&format!("%{} = mul i64 1, {}", size_label, closure_size));

                // 分配闭包内存
                let malloc_label = self.new_label("malloc");
                self.emit(&format!("%{} = call i8* @malloc(i64 %{})", malloc_label, size_label));

                // 存储函数指针到闭包
                let func_ptr_int = self.new_label("func_ptr_int");
                self.emit(&format!("%{} = ptrtoint i64 (i8*)* @{} to i64", func_ptr_int, lambda_func_name));
                let func_ptr_ptr = self.new_label("func_ptr_ptr");
                self.emit(&format!("%{} = inttoptr i64 %{} to i8*", func_ptr_ptr, func_ptr_int));

                // 存储函数指针到偏移 0
                let gep0 = self.new_label("gep0");
                self.emit(&format!("%{} = getelementptr i8, i8* %{}, i64 0", gep0, malloc_label));
                let ptr_cast0 = self.new_label("ptr_cast0");
                self.emit(&format!("%{} = bitcast i8* %{} to i8**", ptr_cast0, gep0));
                self.emit(&format!("store i8* %{}, i8** %{}", func_ptr_ptr, ptr_cast0));

                // 存储捕获变量数量到偏移 8
                let gep8 = self.new_label("gep8");
                self.emit(&format!("%{} = getelementptr i8, i8* %{}, i64 8", gep8, malloc_label));
                let ptr_cast8 = self.new_label("ptr_cast8");
                self.emit(&format!("%{} = bitcast i8* %{} to i64*", ptr_cast8, gep8));
                self.emit(&format!("store i64 {}, i64* %{}", captured_count, ptr_cast8));

                // ===== 第三步：复制捕获变量的当前值到闭包 =====
                let mut copy_offset = 16;
                for captured_var in &lambda.captured_vars {
                    // 获取外部变量的当前值
                    if let Some(var_ssa) = self.variables.get(&captured_var.name).cloned() {
                        let var_type = type_to_llvm(&captured_var.var_type);

                        // 加载当前值
                        let load_inst = self.new_label("captured_val");
                        self.emit(&format!("%{} = load {}, {}* %{}", load_inst, var_type, var_type, var_ssa));

                        // 计算目标位置
                        let dest_gep = self.new_label("dest_gep");
                        self.emit(&format!("%{} = getelementptr i8, i8* %{}, i64 {}", dest_gep, malloc_label, copy_offset));
                        let dest_ptr = self.new_label("dest_ptr");
                        self.emit(&format!("%{} = bitcast i8* %{} to {}*", dest_ptr, dest_gep, var_type));

                        // 存储值
                        self.emit(&format!("store {} %{}, {}* %{}", var_type, load_inst, var_type, dest_ptr));
                    }
                    copy_offset += 8;
                }

                // 返回闭包指针（作为 i64）
                let result = self.new_label("closure_result");
                self.emit(&format!("%{} = ptrtoint i8* %{} to i64", result, malloc_label));

                // 注册闭包变量
                self.variables.insert(closure_var_name.clone(), result.clone());
                self.variable_types.insert(closure_var_name, "i64".to_string());

                Ok(result)
            }
            Expr::IndexAccess(index) => {
                // 生成对象表达式
                let obj_val = self.generate_expression(&index.object)?;

                // 推断对象类型
                let obj_type = self.infer_expression_type(&index.object);

                // 生成索引表达式
                let idx_val = self.generate_expression(&index.index)?;

                // 根据对象类型生成不同的索引代码
                if obj_type == "i8*" {
                    // 字符串索引：使用 getelementptr 获取字节
                    let ptr = self.new_label("str_ptr");
                    self.emit(&format!("%{} = getelementptr i8, i8* %{}, i64 %{}", ptr, obj_val, idx_val));
                    let byte_val = self.new_label("byte");
                    self.emit(&format!("%{} = load i8, i8* %{}", byte_val, ptr));
                    // 零扩展为 i64
                    let result = self.new_label("char_val");
                    self.emit(&format!("%{} = zext i8 %{} to i64", result, byte_val));
                    Ok(result)
                } else {
                    // 列表索引：调用 rt_list_get
                    // 将对象值转换为指针
                    let obj_ptr = self.new_label("obj_ptr");
                    self.emit(&format!("%{} = inttoptr i64 %{} to i8*", obj_ptr, obj_val));

                    // 调用列表获取函数
                    let result = self.new_label("elem");
                    self.emit(&format!("%{} = call i8* @rt_list_get(i8* %{}, i64 %{})", result, obj_ptr, idx_val));

                    // 将结果转换为 i64
                    let result_val = self.new_label("elem_val");
                    self.emit(&format!("%{} = ptrtoint i8* %{} to i64", result_val, result));
                    Ok(result_val)
                }
            }
        }
    }

    /**
     * 生成字面量表达式
     */
    fn generate_literal_expr(&mut self, lit: &LiteralExpr) -> Result<String, CodegenError> {
        let result = self.new_label("lit");
        
        match &lit.kind {
            LiteralKind::Integer(n) => {
                self.emit(&format!("%{} = add i64 0, {}", result, n));
            }
            LiteralKind::Float(f) => {
                let bits = f.to_bits();
                self.emit(&format!("%{} = fadd double 0.0, 0x{:x}", result, bits));
            }
            LiteralKind::String(s) => {
                // LLVM IR 中 c"..." 格式自动包含 null 终止符
                let escaped = s.replace("\\", "\\\\").replace("\"", "\\\"");
                // 字符串长度（不含终止符）
                let str_len = escaped.len();
                // 生成唯一的字符串常量标签
                let str_label = format!("str_{}", self.label_counter);
                self.label_counter += 1;
                // 存储字符串和长度（长度 = 实际字符数，因为 c"..." 会自动加 null）
                self.string_constants.insert(str_label.clone(), (escaped.clone(), str_len));
                // 在函数内部生成 getelementptr 引用
                let gep = format!("%{} = getelementptr [{} x i8], [{} x i8]* @{}, i32 0, i32 0", 
                    result, str_len, str_len, str_label);
                self.emit(&gep);
            }
            LiteralKind::Char(c) => {
                let value = *c as i32;
                self.emit(&format!("%{} = add i8 0, {}", result, value));
            }
            LiteralKind::Boolean(b) => {
                let value = if *b { 1 } else { 0 };
                self.emit(&format!("%{} = add i64 0, {}", result, value));
            }
        }

        Ok(result)
    }

    /**
     * 生成二元表达式
     */
    fn generate_binary_expr(&mut self, binary: &BinaryExpr) -> Result<String, CodegenError> {
        let left = self.generate_expression(&binary.left)?;
        let right = self.generate_expression(&binary.right)?;
        let result = self.new_label("binop");

        // 判断是否是比较操作
        let _is_comparison = matches!(binary.op, 
            BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Gt | BinaryOp::Lt | BinaryOp::Ge | BinaryOp::Le);

        // 分离比较运算和其他运算的处理
        let llvm_op = match binary.op {
            BinaryOp::Add => ("add", "i64"),
            BinaryOp::Sub => ("sub", "i64"),
            BinaryOp::Mul => ("mul", "i64"),
            BinaryOp::Div => ("sdiv", "i64"),
            BinaryOp::Rem => ("srem", "i64"),
            BinaryOp::And => {
                // 逻辑与：
                // 1. 将操作数与0比较得到 i1 (非零为真)
                // 2. 执行逻辑与运算 (i1)
                // 3. 将结果转换回 i64
                let left_cond = self.new_label("left_cond");
                let right_cond = self.new_label("right_cond");

                self.emit(&format!("%{} = icmp ne i64 %{}, 0", left_cond, left));
                self.emit(&format!("%{} = icmp ne i64 %{}, 0", right_cond, right));

                let and_result = self.new_label("and_result");
                self.emit(&format!("%{} = and i1 %{}, %{}", and_result, left_cond, right_cond));
                self.emit(&format!("%{} = zext i1 %{} to i64", result, and_result));
                return Ok(result);
            }
            BinaryOp::Or => {
                // 逻辑或：
                // 1. 将操作数与0比较得到 i1
                // 2. 执行逻辑或运算 (i1)
                // 3. 将结果转换回 i64
                let left_cond = self.new_label("left_cond");
                let right_cond = self.new_label("right_cond");

                self.emit(&format!("%{} = icmp ne i64 %{}, 0", left_cond, left));
                self.emit(&format!("%{} = icmp ne i64 %{}, 0", right_cond, right));

                let or_result = self.new_label("or_result");
                self.emit(&format!("%{} = or i1 %{}, %{}", or_result, left_cond, right_cond));
                self.emit(&format!("%{} = zext i1 %{} to i64", result, or_result));
                return Ok(result);
            }
            BinaryOp::Eq => ("icmp eq", "i64"),
            BinaryOp::Ne => ("icmp ne", "i64"),
            BinaryOp::Gt => ("icmp sgt", "i64"),
            BinaryOp::Lt => ("icmp slt", "i64"),
            BinaryOp::Ge => ("icmp sge", "i64"),
            BinaryOp::Le => ("icmp sle", "i64"),
            BinaryOp::BitAnd => ("and", "i64"),
            BinaryOp::BitOr => ("or", "i64"),
            BinaryOp::BitXor => ("xor", "i64"),
            BinaryOp::Shl => ("shl", "i64"),
            BinaryOp::Shr => ("lshr", "i64"),
            BinaryOp::Hash => ("xor", "i64"),
            BinaryOp::Assign => ("add", "i64"),
        };

        // 处理赋值运算符
        if binary.op == BinaryOp::Assign {
            // 赋值表达式: target = value
            // 左侧应该是标识符
            if let Expr::Identifier(ident) = &*binary.left {
                // 翻译变量名（处理中文变量名）
                let translated_name = self.translate_func_name(&ident.name);
                if let Some(alloca) = self.variables.get(&translated_name).cloned() {
                    // 获取变量类型
                    let var_type = self.variable_types.get(&translated_name)
                        .cloned()
                        .unwrap_or_else(|| "i64".to_string());
                    // 存储到已有变量
                    self.emit(&format!("store {} %{}, {}* %{}", var_type, right, var_type, alloca));
                } else {
                    // 新变量，分配空间
                    let var_type = "i64".to_string();
                    let new_alloca = self.new_label("alloca");
                    self.emit(&format!("%{} = alloca {}", new_alloca, var_type));
                    self.emit(&format!("store {} %{}, {}* %{}", var_type, right, var_type, new_alloca));
                    self.variables.insert(translated_name.clone(), new_alloca);
                    self.variable_types.insert(translated_name.clone(), var_type);
                }
            }
            // 返回右值作为结果
            return Ok(right);
        }

        // 检查是否是比较操作
        let is_comparison = matches!(binary.op,
            BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Gt | BinaryOp::Lt | BinaryOp::Ge | BinaryOp::Le);

        // 生成运算指令
        if is_comparison {
            // 比较运算：先生成 i1 结果，再转换为 i64 (0或1)
            let cmp_result = self.new_label("cmp");
            self.emit(&format!("%{} = {} i64 %{}, %{}", cmp_result, llvm_op.0, left, right));
            self.emit(&format!("%{} = zext i1 %{} to i64", result, cmp_result));
        } else {
            // 算术运算：结果类型是 i64
            self.emit(&format!("%{} = {} {} %{}, %{}", result, llvm_op.0, llvm_op.1, left, right));
        }

        Ok(result)
    }

    /**
     * 生成一元表达式
     */
    fn generate_unary_expr(&mut self, unary: &UnaryExpr) -> Result<String, CodegenError> {
        let operand = self.generate_expression(&unary.operand)?;
        let result = self.new_label("unop");

        match unary.op {
            UnaryOp::Neg => {
                // 负数：0 - operand
                self.emit(&format!("%{} = sub i64 0, %{}", result, operand));
            }
            UnaryOp::Not => {
                // 逻辑非：
                // 1. 比较 operand != 0，得到 i1 (1 if non-zero, 0 if zero)
                // 2. Xor with 1 来取反
                // 3. Zext i1 to i64
                let cmp_result = self.new_label("cmp");
                let xor_result = self.new_label("xor");
                self.emit(&format!("%{} = icmp ne i64 %{}, 0", cmp_result, operand));
                self.emit(&format!("%{} = xor i1 %{}, 1", xor_result, cmp_result));
                self.emit(&format!("%{} = zext i1 %{} to i64", result, xor_result));
            }
            UnaryOp::BitNot => {
                // 按位非
                self.emit(&format!("%{} = xor i64 %{}, -1", result, operand));
            }
        }

        Ok(result)
    }

    /**
     * 生成函数调用表达式
     */
    fn generate_call_expr(&mut self, call: &CallExpr) -> Result<String, CodegenError> {
        // 获取函数名
        let func_name = match &*call.function {
            Expr::Identifier(ident) => ident.name.clone(),
            _ => "unknown".to_string(),
        };

        // 检查是否是闭包调用
        let closure_name = format!("closure_{}", func_name);
        let is_closure_call = self.closures.contains_key(&closure_name);

        // 检查是否是内置函数
        let is_builtin_print = func_name == "打印" || func_name == "print";
        let is_builtin_to_int = func_name == "文本转整数" || func_name == "str_to_int";
        let is_builtin_to_str = func_name == "整数转文本" || func_name == "int_to_str";
        
        // 检查是否是列表函数
        let is_list_create = func_name == "创建列表" || func_name == "create_list";
        let is_list_add = func_name == "列表添加" || func_name == "list_add";
        let is_list_get = func_name == "列表获取" || func_name == "list_get";
        let is_list_len = func_name == "列表长度" || func_name == "list_len";
        let is_list_constructor = func_name == "列表";
        let is_list_func = is_list_create || is_list_add || is_list_get || is_list_len || is_list_constructor;
        
        // 检查是否是控制台输入函数
        let is_input_int = func_name == "输入整数";
        let is_input_text = func_name == "输入文本";
        let _is_input_func = is_input_int || is_input_text;
        
        // 检查是否是字符串函数
        let is_str_len = func_name == "文本长度" || func_name == "str_len";
        let is_str_concat = func_name == "文本拼接" || func_name == "str_concat";
        let is_str_slice = func_name == "文本切片" || func_name == "str_slice";
        let is_str_contains = func_name == "文本包含" || func_name == "str_contains";
        let is_str_func = is_str_len || is_str_concat || is_str_slice || is_str_contains;
        
        // 检查是否是命令行参数函数
        let is_arg_count = func_name == "参数个数" || func_name == "argc";
        let is_arg_get = func_name == "获取参数" || func_name == "argv";
        let is_arg_func = is_arg_count || is_arg_get;
        
        // 将中文函数名映射到英文 LLVM 函数名，并生成正确格式
        let llvm_func_call_name = match func_name.as_str() {
            // 内置函数
            "打印" => "print".to_string(),
            "打印整数" => "print_int".to_string(),
            "打印浮点" => "print_float".to_string(),
            "打印布尔" => "print_bool".to_string(),
            "文本转整数" => "str_to_int".to_string(),
            "整数转文本" => "int_to_str".to_string(),
            "创建列表" => "create_list".to_string(),
            "列表" => "rt_list_new".to_string(),  // 列表构造函数
            "列表添加" => "list_add".to_string(),
            "列表获取" => "list_get".to_string(),
            "列表长度" => "list_len".to_string(),
            // 控制台输入函数
            "输入整数" => "\"输入整数\"".to_string(),
            "输入文本" => "\"输入文本\"".to_string(),
            // 文本函数
            "文本长度" => "rt_string_len".to_string(),
            "文本拼接" => "str_concat".to_string(),
            "文本切片" => "str_slice".to_string(),
            "文本包含" => "str_contains".to_string(),
            // 命令行参数函数
            "参数个数" => "argc".to_string(),
            "获取参数" => "argv".to_string(),
            // 系统命令函数
            "执行命令" => "exec_cmd".to_string(),
            "命令输出" => "cmd_output".to_string(),
            // 词法分析器用户函数
            "是空格" => "is_space".to_string(),
            "检查空格" => "check_space".to_string(),
            "是数字" => "is_digit".to_string(),
            "检查数字" => "check_digit".to_string(),
            "是字母" => "is_alpha".to_string(),
            "检查字母" => "check_alpha".to_string(),
            "是关键字" => "is_keyword".to_string(),
            "扫描数字" => "scan_digit".to_string(),
            "是字母数字" => "is_alnum".to_string(),
            "扫描标识符" => "scan_identifier".to_string(),
            "词法分析" => "lex".to_string(),
            "主" => "主".to_string(),
            "文件存在" => "file_exists".to_string(),
            "文件读取" => "file_read".to_string(),
            "文件写入" => "file_write".to_string(),
            "文件删除" => "file_delete".to_string(),
            _ => func_name.clone(),
        };
        
        // 区分内置函数和用户函数
        // 内置函数使用 ASCII 名称，用户函数使用 emit_func_decl 转义
        let is_builtin = matches!(func_name.as_str(), 
            "打印" | "打印整数" | "打印浮点" | "打印布尔" |
            "文本转整数" | "整数转文本" | "创建列表" | "列表添加" | "列表获取" | "列表长度" |
            "输入整数" | "输入文本" |
            "文本长度" | "文本拼接" | "文本切片" | "文本包含" |
            "参数个数" | "获取参数" |
            "文件读取" | "文件写入" | "文件存在" | "文件删除" |
            "执行命令" | "命令输出" |
            "是空格" | "检查空格" | "是数字" | "检查数字" | "是字母" | "检查字母" |
            "是关键字" | "扫描数字" | "是字母数字" | "扫描标识符" | "词法分析"
        );
        
        // 检查是否是文件 I/O 函数
        let is_file_read = func_name == "文件读取";
        let is_file_write = func_name == "文件写入";
        let is_file_exists = func_name == "文件存在";
        let is_file_delete = func_name == "文件删除";
        let is_file_func = is_file_read || is_file_write || is_file_exists || is_file_delete;
        
        // 检查是否是系统命令函数
        let is_exec_cmd = func_name == "执行命令";
        let is_cmd_output = func_name == "命令输出";
        let _is_sys_cmd_func = is_exec_cmd || is_cmd_output;
        
        // 获取 LLVM 函数引用名（用于 call 指令）
        // 所有非ASCII函数名都需要用引号包裹
        let llvm_func_ref = if llvm_func_call_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            llvm_func_call_name.clone()
        } else {
            // 非ASCII函数名需要用引号包裹
            self.emit_func_decl(&llvm_func_call_name)
        };

        // 生成参数
        // 对于打印函数，需要根据参数类型选择正确的打印函数
        let mut actual_print_func = llvm_func_call_name.clone();
        let mut args = Vec::new();
        
        for (idx, arg) in call.arguments.iter().enumerate() {
            let arg_val = self.generate_expression(arg)?;
            if is_str_slice {
                // 文本切片需要 (i8*, i64, i64)
                // idx=0: 字符串 (i8*), idx=1: 起始位置 (i64), idx=2: 结束位置 (i64)
                if idx == 0 {
                    args.push(format!("i8* %{}", arg_val));
                } else {
                    args.push(format!("i64 %{}", arg_val));
                }
            } else if is_builtin_print {
                // 打印函数：根据参数类型选择正确的打印函数
                // 检测参数类型（简化：通过表达式推断）
                let arg_type = self.infer_expression_type(arg);
                match arg_type.as_str() {
                    "i64" | "i32" => {
                        // 整数类型：使用 print_int
                        actual_print_func = "print_int".to_string();
                        args.push(format!("i64 %{}", arg_val));
                    }
                    "double" | "float" => {
                        // 浮点类型：使用 print_float
                        actual_print_func = "print_float".to_string();
                        args.push(format!("double %{}", arg_val));
                    }
                    "i1" | "bool" => {
                        // 布尔类型：使用 print_bool
                        actual_print_func = "print_bool".to_string();
                        args.push(format!("i1 %{}", arg_val));
                    }
                    _ => {
                        // 默认：字符串指针类型
                        args.push(format!("i8* %{}", arg_val));
                    }
                }
            } else if is_builtin_to_int || is_file_func || is_str_func {
                // 文本转整数、文件函数、字符串函数接受 i8* 字符串指针
                args.push(format!("i8* %{}", arg_val));
            } else if is_builtin_to_str {
                // 整数转文本接受 i64
                args.push(format!("i64 %{}", arg_val));
            } else if is_list_func {
                // 列表函数参数类型
                if is_list_create {
                    // 创建列表接受初始容量 (i64)
                    args.push(format!("i64 %{}", arg_val));
                } else if is_list_add {
                    // 列表添加: (列表指针, 元素值)
                    if idx == 0 {
                        args.push(format!("i8* %{}", arg_val));  // 列表指针
                    } else {
                        args.push(format!("i64 %{}", arg_val));  // 元素值
                    }
                } else if is_list_get {
                    // 列表获取: (列表指针, 索引)
                    if idx == 0 {
                        args.push(format!("i8* %{}", arg_val));  // 列表指针
                    } else {
                        args.push(format!("i64 %{}", arg_val));  // 索引
                    }
                } else if is_list_len {
                    // 列表长度: (列表指针)
                    args.push(format!("i8* %{}", arg_val));
                } else {
                    args.push(format!("i8* %{}", arg_val));
                }
            } else if is_arg_func {
                // 参数相关函数
                args.push(format!("i64 %{}", arg_val));
            } else if is_exec_cmd {
                // 执行命令接受 i8* 字符串指针
                args.push(format!("i8* %{}", arg_val));
            } else if is_cmd_output {
                // 命令输出接受 i8* 字符串指针
                args.push(format!("i8* %{}", arg_val));
            } else {
                // 用户函数：根据参数实际类型生成
                let arg_type = self.infer_expression_type(arg);
                args.push(format!("{} %{}", arg_type, arg_val));
            }
        }
        
        // 更新打印函数名（使用实际选择的打印函数）
        let _final_func_name = if is_builtin_print {
            actual_print_func
        } else {
            llvm_func_call_name
        };

        let result = self.new_label("call");
        
        // 判断返回类型
        let (ret_type, is_void_call) = if is_builtin_print {
            // 打印函数返回 void
            ("void".to_string(), true)
        } else if is_builtin_to_str || is_list_create || is_str_concat || is_str_slice || is_arg_get || is_file_read || is_cmd_output || is_input_text {
            // 整数转文本、创建列表、字符串拼接/切片、获取参数、文件读取、命令输出、输入文本返回 i8*
            ("i8*".to_string(), false)
        } else if is_str_len || is_file_write || is_arg_count || is_file_exists || is_file_delete || is_exec_cmd || is_input_int || is_list_len || is_list_get || is_str_contains {
            // 字符串长度、文件写入、参数个数、文件存在、文件删除、执行命令、输入整数、列表长度、列表获取、字符串包含返回 i64
            ("i64".to_string(), false)
        } else if !is_builtin {
            // 用户函数（非内置函数）返回 i64（假设）
            ("i64".to_string(), false)
        } else {
            // 其他内置函数返回 i64
            ("i64".to_string(), false)
        };
        
        // 生成函数调用
        if is_closure_call {
            // 闭包调用：从闭包中提取函数指针并调用
            // 获取闭包指针（从 closure_xxx 变量）
            let closure_ptr = if let Some(closure_ssa) = self.variables.get(&closure_name) {
                closure_ssa.clone()
            } else {
                // 尝试直接使用函数名作为变量
                func_name.clone()
            };

            // 从闭包中提取函数指针（偏移 0 处）
            let closure_val = self.new_label("closure_val");
            self.emit(&format!("%{} = load i64, i64* %{}", closure_val, closure_ptr));

            let closure_int_ptr = self.new_label("closure_int_ptr");
            self.emit(&format!("%{} = inttoptr i64 %{} to i8*", closure_int_ptr, closure_val));

            let func_ptr_ptr = self.new_label("func_ptr_ptr");
            self.emit(&format!("%{} = getelementptr i8, i8* %{}, i64 0", func_ptr_ptr, closure_int_ptr));

            let func_ptr_ptr_cast = self.new_label("func_ptr_ptr_cast");
            self.emit(&format!("%{} = bitcast i8* %{} to i64 (i8*)*", func_ptr_ptr_cast, func_ptr_ptr));

            let func_ptr = self.new_label("func_ptr");
            self.emit(&format!("%{} = load i64 (i8*)*, i64 (i8*)* %{}", func_ptr, func_ptr_ptr_cast));

            // 构建闭包调用的参数列表：闭包上下文 + 用户参数
            let mut closure_args = vec![format!("i8* %{}", closure_int_ptr)];
            closure_args.extend(args.iter().map(|a| a.clone()));

            let args_str = closure_args.join(", ");
            let result = self.new_label("closure_call");
            self.emit(&format!("%{} = call i64 %{}({})", result, func_ptr, args_str));

            Ok(result)
        } else if args.is_empty() {
            if is_void_call {
                self.emit(&format!("call {} @{}({})", ret_type, llvm_func_ref, ""));
                self.emit(&format!("%{} = add i64 0, 0", result));
            } else {
                self.emit(&format!("%{} = call {} @{}({})", result, ret_type, llvm_func_ref, ""));
            }
            Ok(result)
        } else {
            let args_str = args.join(", ");

            if is_builtin_to_str || is_list_create || is_str_concat || is_str_slice || is_arg_get || is_file_read || is_cmd_output || is_input_text {
                // 整数转文本、创建列表、字符串拼接/切片、获取参数、文件读取、命令输出、输入文本返回 i8*
                self.emit(&format!("%{} = call i8* @{}({})", result, llvm_func_ref, args_str));
            } else if is_str_len || is_file_write || is_arg_count || is_file_exists || is_file_delete || is_exec_cmd || is_input_int || is_list_len || is_list_get || is_str_contains {
                // 字符串长度、文件写入、参数个数、文件存在、文件删除、执行命令、输入整数、列表长度、列表获取、字符串包含返回 i64
                self.emit(&format!("%{} = call i64 @{}({})", result, llvm_func_ref, args_str));
            } else if is_builtin_print {
                // 打印函数返回 void
                self.emit(&format!("call void @{}({})", llvm_func_ref, args_str));
                // 为保持 SSA 形式，生成一个虚拟返回值
                self.emit(&format!("%{} = add i64 0, 0", result));
            } else if !is_builtin {
                // 用户函数（非内置函数）
                self.emit(&format!("%{} = call i64 @{}({})", result, llvm_func_ref, args_str));
            } else {
                // 其他内置函数返回 i64
                self.emit(&format!("%{} = call i64 @{}({})", result, llvm_func_ref, args_str));
            }

            Ok(result)
        }
    }
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/**
 * 代码生成入口函数
 */
pub fn generate_ir(module: &Module) -> Result<String, CodegenError> {
    let mut generator = CodeGenerator::new();
    generator.generate(module)
}
