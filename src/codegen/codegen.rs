/**
 * @file codegen.rs
 * @brief 代码生成模块
 * @description 负责将AST转换为LLVM IR
 */

use std::collections::HashMap;

use crate::ast::*;
use crate::error::CodegenError;

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
        }
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

        // 生成运行时库函数声明
        self.emit_runtime_declarations();

        // 生成用户定义的外部函数声明
        for extern_func in &module.extern_functions {
            self.generate_extern_function(extern_func)?;
        }

        // 预先收集用户函数签名（用于类型推断）
        for func in &module.functions {
            let func_name = self.translate_func_name(&func.name);
            let return_type = self.translate_type(&func.return_type);
            let param_types: Vec<String> = func.params
                .iter()
                .map(|param| self.translate_type(&param.param_type))
                .collect();
            self.user_functions.insert(func_name, (param_types, return_type));
        }

        // 生成函数定义
        for func in &module.functions {
            self.generate_function(func)?;
        }

        // 在所有函数定义之后添加字符串常量定义
        for constant in &self.string_constants {
            self.ir.push_str(constant);
            self.ir.push('\n');
        }

        Ok(self.ir.clone())
    }

    /**
     * 生成运行时库函数声明
     */
    fn emit_runtime_declarations(&mut self) {
        // 内存管理
        self.emit("declare i8* @rt_malloc(i64)");
        self.emit("declare void @rt_free(i8*)");
        
        // 字符串处理
        self.emit("declare i8* @rt_str_new(i8*)");
        self.emit("declare i8* @rt_str_concat(i8*, i8*)");
        self.emit("declare i64 @rt_str_len(i8*)");
        
        // 列表操作
        self.emit("declare i8* @rt_list_new()");
        self.emit("declare void @rt_list_append(i8*, i8*)");
        self.emit("declare i64 @rt_list_len(i8*)");
        self.emit("declare i8* @rt_list_get(i8*, i64)");
        
        // 打印函数
        self.emit("declare void @rt_print(i8*)");
        self.emit("declare void @rt_println(i8*)");
        self.emit("declare i64 @print_int(i64)");
        self.emit("declare i64 @print_float(double)");
        
        // 类型转换函数
        self.emit("declare i8* @rt_int_to_str(i64)");
        self.emit("declare i64 @rt_str_to_int(i8*)");
        self.emit("declare i8* @rt_float_to_str(double)");
        self.emit("declare double @rt_str_to_double(i8*)");
        
        // 错误处理
        self.emit("declare void @rt_error(i8*)");
    }

    /**
     * 生成用户定义的外部函数声明
     * 外部 函数 函数名(参数列表) -> 返回类型
     */
    fn generate_extern_function(&mut self, extern_func: &ExternFunction) -> Result<(), CodegenError> {
        // 翻译函数名（处理中文函数名）
        let func_name = self.translate_func_name(&extern_func.name);
        
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
        // 翻译函数名（处理中文函数名）
        let func_name = self.translate_func_name(&func.name);
        
        // 设置当前函数名，用于生成唯一的变量名
        self.current_function_name = func_name.clone();
        
        // 生成函数签名
        let return_type = self.translate_type(&func.return_type);
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
        let ir_len_before_body = self.ir.len();
        
        // 生成函数体
        self.generate_block(&func.body)?;
        
        // 检查函数体中是否有返回语句（只检查新增的 IR）
        let body_ir = &self.ir[ir_len_before_body..];
        let has_return = body_ir.contains("ret");
        
        // 生成返回语句（如果没有显式返回）
        if return_type != "void" {
            if !has_return {
                self.emit(&format!("    ret {} 0", return_type));
            }
        } else {
            if !has_return {
                self.emit("    ret void");
            }
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
        
        if let Some(initializer) = &let_stmt.initializer {
            let expr_val = self.generate_expression(initializer)?;
            let var_type = self.infer_expression_type(initializer);
            
            let alloca = self.new_label(&var_name);
            self.emit(&format!("    %{} = alloca {}, align 8", alloca, var_type));
            self.emit(&format!("    store {} %{}, {}* %{}", var_type, expr_val, var_type, alloca));
            
            // 记录变量
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
            let return_type = self.infer_expression_type(expr);
            self.emit(&format!("    ret {} %{}", return_type, expr_val));
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
            
            let then_label = self.label_counter;
            self.label_counter += 1;
            let else_label = self.label_counter;
            self.label_counter += 1;
            let end_label = self.label_counter;
            self.label_counter += 1;
            
            self.emit(&format!("    br i1 %{}, label %L{}, label %L{}", cond_val, then_label, else_label));
            
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
                    Stmt::If(nested_if) => self.generate_if_stmt(nested_if)?,  // 支持嵌套的if（否则若）
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
                    
                    self.emit(&format!("    br label %L{}", loop_start));
                    
                    // 生成循环开始标签
                    self.emit(&format!("L{}:", loop_start));
                    let cond_val = self.generate_expression(condition)?;
                    self.emit(&format!("    br i1 %{}, label %L{}, label %L{}", cond_val, loop_body, loop_end));
                    
                    // 生成循环体
                    self.emit(&format!("L{}:", loop_body));
                    match &*loop_stmt.body {
                        Stmt::Block(block) => self.generate_block(block)?,
                        _ => return Err(CodegenError::new("循环语句的body必须是BlockStmt")),
                    }
                    self.emit(&format!("    br label %L{}", loop_start));
                    
                    // 生成循环结束标签
                    self.emit(&format!("L{}:", loop_end));
                }
            }
            _ => {
                return Err(CodegenError::new("只支持While循环"));
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
                
                // 检查对象是否是标识符（模块名）
                if let Expr::Identifier(module_ident) = &**object_expr {
                    // 这是一个模块间的成员访问，返回模块名::成员名的组合
                    let module_name = &module_ident.name;
                    let full_name = format!("{}::{}", module_name, member_name);
                    
                    // 翻译函数名（处理中文）
                    let translated_name = self.translate_func_name(&full_name);
                    
                    // 对于模块间函数调用，我们需要在 Call 表达式中处理
                    // 这里只是返回函数名，供 Call 表达式使用
                    Ok(translated_name)
                } else {
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
                        // 结构体字段访问
                        let object_val = self.generate_expression(&member.object)?;

                        // 计算字段偏移
                        let field_offset = self.calculate_field_offset(field_name);

                        // 根据字段名推断成员类型
                        let ptr_type = self.infer_member_type(field_name);

                        // 获取对象类型
                        let obj_type = self.infer_expression_type(&member.object);

                        // 将对象值转换为 i8* 指针（仅当类型为 i64 时才需要转换）
                        let ptr_val = if obj_type == "i64" {
                            // 需要转换
                            let ptr = self.new_label("ptr");
                            self.emit(&format!("    %{} = inttoptr i64 %{} to i8*", ptr, object_val));
                            ptr
                        } else {
                            // 已经是 i8* 类型，直接使用
                            object_val
                        };

                        // 生成 GEP 指令获取字段指针
                        let result = self.new_label("member");
                        self.emit(&format!("    %{} = getelementptr i8, i8* %{}, i32 {}",
                            result, ptr_val, field_offset));

                        // 将指针转换为正确类型的指针
                        let result_ptr = self.new_label("member_ptr");
                        self.emit(&format!("    %{} = bitcast i8* %{} to {}*", result_ptr, result, ptr_type));

                        // 加载字段值
                        let result_val = self.new_label("member_val");
                        self.emit(&format!("    %{} = load {}, {}* %{}", result_val, ptr_type, ptr_type, result_ptr));
                        
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
                    
                    // 条件跳转标签
                    let do_append = self.label_counter;
                    self.label_counter += 1;
                    let skip_append = self.label_counter;
                    self.label_counter += 1;
                    
                    // 检查条件：为真则添加，为假则跳过
                    self.emit(&format!("    br i1 %{}, label %L{}, label %L{}", cond_result, do_append, skip_append));
                    
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
                let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
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
                // 左操作数通常是标识符（变量名）
                match &*binary.left {
                    Expr::Identifier(ident) => {
                        let var_name = self.translate_func_name(&ident.name);
                        // 查找变量的 SSA 分配槽
                        if let Some(alloca) = self.variables.get(&var_name).cloned() {
                            let var_type = self.variable_types.get(&var_name)
                                .cloned()
                                .unwrap_or_else(|| "i64".to_string());
                            // 生成 store 指令将结果写回变量槽
                            self.emit(&format!("    store {} %{}, {}* %{}", var_type, right_val, var_type, alloca));
                        }
                    }
                    _ => {}
                }
                // 返回右值
                Ok(right_val)
            }
            BinaryOp::Add => {
                self.emit(&format!("    %{} = add i64 %{}, %{}", result, left_val, right_val));
                self.variable_types.insert(result.clone(), "i64".to_string());
                Ok(result)
            }
            BinaryOp::Sub => {
                self.emit(&format!("    %{} = sub i64 %{}, %{}", result, left_val, right_val));
                self.variable_types.insert(result.clone(), "i64".to_string());
                Ok(result)
            }
            BinaryOp::Mul => {
                self.emit(&format!("    %{} = mul i64 %{}, %{}", result, left_val, right_val));
                self.variable_types.insert(result.clone(), "i64".to_string());
                Ok(result)
            }
            BinaryOp::Div => {
                self.emit(&format!("    %{} = sdiv i64 %{}, %{}", result, left_val, right_val));
                self.variable_types.insert(result.clone(), "i64".to_string());
                Ok(result)
            }
            BinaryOp::Rem => {
                self.emit(&format!("    %{} = srem i64 %{}, %{}", result, left_val, right_val));
                self.variable_types.insert(result.clone(), "i64".to_string());
                Ok(result)
            }
            BinaryOp::Eq => {
                self.emit(&format!("    %{} = icmp eq i64 %{}, %{}", result, left_val, right_val));
                self.variable_types.insert(result.clone(), "i1".to_string());
                Ok(result)
            }
            BinaryOp::Ne => {
                self.emit(&format!("    %{} = icmp ne i64 %{}, %{}", result, left_val, right_val));
                self.variable_types.insert(result.clone(), "i1".to_string());
                Ok(result)
            }
            BinaryOp::Lt => {
                self.emit(&format!("    %{} = icmp slt i64 %{}, %{}", result, left_val, right_val));
                self.variable_types.insert(result.clone(), "i1".to_string());
                Ok(result)
            }
            BinaryOp::Le => {
                self.emit(&format!("    %{} = icmp sle i64 %{}, %{}", result, left_val, right_val));
                self.variable_types.insert(result.clone(), "i1".to_string());
                Ok(result)
            }
            BinaryOp::Gt => {
                self.emit(&format!("    %{} = icmp sgt i64 %{}, %{}", result, left_val, right_val));
                self.variable_types.insert(result.clone(), "i1".to_string());
                Ok(result)
            }
            BinaryOp::Ge => {
                self.emit(&format!("    %{} = icmp sge i64 %{}, %{}", result, left_val, right_val));
                self.variable_types.insert(result.clone(), "i1".to_string());
                Ok(result)
            }
            BinaryOp::And => {
                self.emit(&format!("    %{} = and i64 %{}, %{}", result, left_val, right_val));
                self.variable_types.insert(result.clone(), "i1".to_string());
                Ok(result)
            }
            BinaryOp::Or => {
                self.emit(&format!("    %{} = or i64 %{}, %{}", result, left_val, right_val));
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
            // 返回 i64 的函数
            "rt_list_len" => Some("i64".to_string()),
            "rt_string_len" => Some("i64".to_string()),
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
        // 获取函数名（特殊处理：函数名是标识符时直接使用，不生成变量加载）
        let func_name = match &*call.function {
            Expr::Identifier(ident) => {
                // 直接翻译函数名，不查找变量
                self.translate_func_name(&ident.name)
            }
            _ => {
                // 复杂表达式（如模块访问），生成表达式
                self.generate_expression(&call.function)?
            }
        };

        // 生成参数表达式
        let mut args = Vec::new();
        for arg in &call.arguments {
            let arg_val = self.generate_expression(arg)?;
            args.push(arg_val);
        }

        // 生成调用指令
        let result = self.new_label("call");

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
            // 列表追加函数，无返回值
            if args.len() >= 2 {
                self.emit(&format!("    call void @rt_list_append(i8* %{}, i8* %{})", args[0], args[1]));
            }
            Ok(result)
        } else if func_name == "rt_list_len" {
            // 列表长度函数
            if !args.is_empty() {
                self.emit(&format!("    %{} = call i64 @rt_list_len(i8* %{})", result, args[0]));
            } else {
                self.emit(&format!("    %{} = call i64 @rt_list_len(i8* null)", result));
            }
            Ok(result)
        } else if func_name == "rt_list_get" {
            // 列表获取函数，返回 i8*
            if args.len() >= 2 {
                self.emit(&format!("    %{} = call i8* @rt_list_get(i8* %{}, i64 %{})", result, args[0], args[1]));
            } else if args.len() == 1 {
                self.emit(&format!("    %{} = call i8* @rt_list_get(i8* %{}, i64 0)", result, args[0]));
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
        match arg {
            Expr::Identifier(ident) => {
                // 从变量类型映射中获取
                let var_name = self.translate_func_name(&ident.name);
                self.variable_types.get(&var_name).cloned().unwrap_or_else(|| "i64".to_string())
            }
            Expr::Literal(lit) => {
                match &lit.kind {
                    LiteralKind::Integer(_) => "i64".to_string(),
                    LiteralKind::Float(_) => "double".to_string(),
                    LiteralKind::String(_) => "i8*".to_string(),
                    LiteralKind::Boolean(_) => "i1".to_string(),
                    _ => "i64".to_string(),
                }
            }
            Expr::Call(call_expr) => {
                // 函数调用的返回类型
                let func_name = match &*call_expr.function {
                    Expr::Identifier(ident) => self.translate_func_name(&ident.name),
                    _ => "unknown".to_string(),
                };
                self.get_func_return_type(&func_name).unwrap_or_else(|| "i64".to_string())
            }
            _ => "i64".to_string(),
        }
    }

    /**
     * 生成类型转换代码
     * i64 -> i8*: 调用 rt_int_to_str
     * i8* -> i64: 调用 rt_str_to_int
     */
    fn generate_type_conversion(&mut self, val: &str, from_type: &str, to_type: &str) -> String {
        if from_type == to_type {
            return val.to_string();
        }
        
        let result = self.new_label("conv");
        
        if from_type == "i64" && to_type == "i8*" {
            // 整数转字符串
            self.emit(&format!("    %{} = call i8* @rt_int_to_str(i64 %{})", result, val));
        } else if from_type == "i8*" && to_type == "i64" {
            // 字符串转整数
            self.emit(&format!("    %{} = call i64 @rt_str_to_int(i8* %{})", result, val));
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
     * 推断表达式类型
     */
    fn infer_expression_type(&self, expr: &Expr) -> String {
        match expr {
            Expr::Identifier(_) => "i64".to_string(),
            Expr::Literal(lit) => {
                match &lit.kind {
                    LiteralKind::Integer(_) => "i64".to_string(),
                    LiteralKind::Float(_) => "double".to_string(),
                    LiteralKind::String(_) => "i8*".to_string(),
                    LiteralKind::Boolean(_) => "i64".to_string(),
                    LiteralKind::Char(_) => "i64".to_string(),
                }
            }
            Expr::Binary(_) => "i64".to_string(),
            Expr::Unary(_) => "i64".to_string(),
            Expr::Call(call) => {
                // 检查函数名来确定返回类型
                let func_name = match &*call.function {
                    Expr::Identifier(ident) => {
                        self.translate_func_name(&ident.name)
                    }
                    _ => {
                        return "i64".to_string(); // 默认返回 i64
                    }
                };

                // 使用 get_func_return_type 来确定返回类型
                if let Some(ret_type) = self.get_func_return_type(&func_name) {
                    ret_type
                } else {
                    // 无返回的函数（如 rt_list_append）
                    "i64".to_string()
                }
            }
            Expr::MemberAccess(member) => {
                // 检查是否是列表方法
                let field_name = &member.member;
                if field_name == "长度" {
                    "i64".to_string()
                } else {
                    "i8*".to_string()
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
            Type::Int => "i64".to_string(),
            Type::Float => "double".to_string(),
            Type::String => "i8*".to_string(),
            Type::Bool => "i64".to_string(),
            Type::Void => "void".to_string(),
            Type::Pointer => "i8*".to_string(),
            Type::Function(_, _) => "i8*".to_string(),
            _ => "i64".to_string(),
        }
    }

    /**
     * 翻译函数名（处理中文函数名）
     * 注意：运行时函数（如 rt_list_new, rt_list_append）保持原名不被哈希
     */
    fn translate_func_name(&self, name: &str) -> String {
        // 对于模块间函数调用，保留模块名::函数名的格式
        if name.contains("::") {
            name.to_string()
        } else {
            // 中文函数名翻译为有效的 LLVM 标识符
            match name {
                "主" => "main".to_string(),
                "主函数" => "main".to_string(),
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
                "新建列表" => "list_new".to_string(),
                "列表追加" => "list_append".to_string(),
                "列表获取" => "list_get".to_string(),
                "文本长度" => "str_len".to_string(),
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
                _ => {
                    // 对于未映射的中文标识符，生成哈希名称（包含函数作用域）
                    self.generate_hash_name(name, &self.current_function_name)
                }
            }
        }
    }

    /**
     * 为中文标识符生成有效的 LLVM 名称（哈希形式）
     * 包含函数作用域以确保全局唯一性
     */
    fn generate_hash_name(&self, name: &str, scope: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        // 将函数作用域和变量名组合在一起进行哈希
        scope.hash(&mut hasher);
        name.hash(&mut hasher);
        let hash = hasher.finish();
        format!("fn_{:x}", hash)
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
     */
    fn calculate_field_offset(&self, _field_name: &str) -> i32 {
        // 简化实现：假设所有字段都是8字节
        0
    }

    /**
     * 推断成员类型
     */
    fn infer_member_type(&self, _field_name: &str) -> String {
        // 简化实现：默认返回i64
        "i64".to_string()
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
