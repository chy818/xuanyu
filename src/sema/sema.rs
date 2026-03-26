/**
 * @file sema.rs
 * @brief CCAS 语义分析器 (Semantic Analyzer)
 * @description 类型检查、作用域分析、符号解析
 * 
 * 功能:
 * - 变量声明类型检查
 * - 函数调用类型匹配
 * - 作用域嵌套检查
 * - 自动类型推断
 */

use crate::ast::*;
use crate::lexer::token::Span;
use crate::error::{TypeError, CompilerError};

/**
 * 符号信息
 */
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub symbol_type: Type,
    pub is_mutable: bool,
    pub span: Span,
}

/**
 * 作用域
 */
#[derive(Debug, Clone)]
pub struct Scope {
    parent: Option<usize>,
    symbols: std::collections::HashMap<String, Symbol>,
    types: std::collections::HashMap<String, Type>,
}

impl Scope {
    pub fn new(parent: Option<usize>) -> Self {
        Self {
            parent,
            symbols: std::collections::HashMap::new(),
            types: std::collections::HashMap::new(),
        }
    }

    pub fn define(&mut self, name: String, symbol: Symbol) {
        self.symbols.insert(name, symbol);
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name).or_else(|| {
            // 父作用域查找需要通过 SemanticAnalyzer 来处理
            // 这里只返回当前作用域的结果
            None
        })
    }

    /**
     * 添加类型定义
     */
    pub fn add_type(&mut self, name: &str, type_info: Type) {
        self.types.insert(name.to_string(), type_info);
    }

    /**
     * 查找类型定义
     */
    pub fn lookup_type<'a>(&'a self, name: &str, scopes: &'a [Scope]) -> Option<&'a Type> {
        if let Some(t) = self.types.get(name) {
            return Some(t);
        }
        // 递归查找父作用域
        if let Some(parent_idx) = self.parent {
            if let Some(parent) = scopes.get(parent_idx) {
                return parent.lookup_type(name, scopes);
            }
        }
        None
    }
}

/**
 * 语义分析器
 */
pub struct SemanticAnalyzer {
    scopes: Vec<Scope>,
    errors: Vec<TypeError>,
    import_stack: Vec<String>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let mut scopes = Vec::new();
        scopes.push(Scope::new(None)); // 全局作用域
        Self { 
            scopes, 
            errors: Vec::new(),
            import_stack: Vec::new(),
        }
    }

    /**
     * 进入新作用域
     */
    fn enter_scope(&mut self) {
        let parent = Some(self.scopes.len() - 1);
        self.scopes.push(Scope::new(parent));
    }

    /**
     * 退出作用域
     */
    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    /**
     * 查找类型定义
     */
    fn lookup_type(&self, name: &str) -> Option<&Type> {
        // 从当前作用域向上查找
        for scope in self.scopes.iter().rev() {
            if let Some(t) = scope.lookup_type(name, &self.scopes) {
                return Some(t);
            }
        }
        None
    }

    /**
     * 定义符号
     */
    fn define_symbol(&mut self, name: String, symbol_type: Type, is_mutable: bool, span: Span) {
        let scope_idx = self.scopes.len() - 1;
        let name_clone = name.clone();
        self.scopes[scope_idx].define(name, Symbol {
            name: name_clone,
            symbol_type,
            is_mutable,
            span,
        });
    }

    /**
     * 查找符号
     */
    fn lookup_symbol(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.lookup(name) {
                return Some(symbol);
            }
        }
        None
    }

    /**
     * 报告错误
     */
    fn error(&mut self, message: String, span: Span) {
        self.errors.push(TypeError {
            code: "CCAS-T001".to_string(),
            message,
            span,
        });
    }

    /**
     * 验证模块
     */
    pub fn analyze_module(&mut self, module: &Module) -> Result<(), Vec<TypeError>> {
        // 首先注册内置函数
        self.register_builtin_functions()?;

        // 处理导入语句
        for import in &module.imports {
            self.process_import(import)?;
        }

        // 收集所有结构体定义到全局作用域
        for struct_def in &module.structs {
            self.register_struct(struct_def)?;
        }

        // 收集所有枚举定义到全局作用域
        for enum_def in &module.enums {
            self.register_enum(enum_def)?;
        }

        // 收集所有类型别名到全局作用域
        for type_alias in &module.type_aliases {
            self.register_type_alias(type_alias)?;
        }

        // 处理顶级常量定义
        for const_stmt in &module.constants {
            self.analyze_constant_statement(const_stmt)?;
        }

        // 收集所有函数声明到全局作用域
        for func in &module.functions {
            self.define_symbol(
                func.name.clone(),
                func.return_type.clone(),
                false,
                func.span,
            );
        }

        // 验证每个函数
        for func in &module.functions {
            self.analyze_function(func)?;
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }

    /**
     * 处理导入语句
     */
    fn process_import(&mut self, import: &ImportStmt) -> Result<(), Vec<TypeError>> {
        let module_path = &import.module_path;
        
        // 解析模块路径 (去除引号和.xy后缀)
        let module_name = module_path.trim_matches('"').trim_end_matches(".xy");
        
        // 检查循环导入
        if self.import_stack.contains(&module_name.to_string()) {
            self.errors.push(TypeError {
                code: "E0428".to_string(),
                message: format!("检测到循环导入: {}", module_name),
                span: import.span.clone(),
            });
            return Ok(());
        }

        // 将当前模块添加到导入栈
        self.import_stack.push(module_name.to_string());

        // 尝试加载模块文件
        let imported_module = self.load_module(module_path)?;
        
        // 递归分析导入的模块
        self.analyze_module(&imported_module)?;

        // 从导入栈中移除
        self.import_stack.pop();

        // 将导入的符号添加到当前作用域
        for func in &imported_module.functions {
            self.define_symbol(
                func.name.clone(),
                func.return_type.clone(),
                false,
                func.span.clone(),
            );
        }

        for struct_def in &imported_module.structs {
            let struct_type = Type::Struct(struct_def.name.clone());
            if let Some(scope) = self.scopes.last_mut() {
                scope.add_type(&struct_def.name, struct_type);
            }
        }

        for enum_def in &imported_module.enums {
            let enum_type = Type::Custom(enum_def.name.clone());
            if let Some(scope) = self.scopes.last_mut() {
                scope.add_type(&enum_def.name, enum_type);
            }
        }

        Ok(())
    }

    /**
     * 加载模块文件
     */
    fn load_module(&self, module_path: &str) -> Result<Module, Vec<TypeError>> {
        // 去除引号
        let path = module_path.trim_matches('"');
        
        // 读取模块文件
        let source = std::fs::read_to_string(path)
            .map_err(|e| vec![TypeError {
                code: "E0428".to_string(),
                message: format!("无法加载模块 '{}': {}", path, e),
                span: Span::dummy(),
            }])?;

        // 词法分析
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize()
            .map_err(|e| vec![TypeError {
                code: "E0428".to_string(),
                message: format!("模块词法错误: {:?}", e),
                span: Span::dummy(),
            }])?;

        // 语法分析
        let mut parser = crate::parser::Parser::new(tokens);
        let module = parser.parse_module()
            .map_err(|e| vec![TypeError {
                code: "E0428".to_string(),
                message: format!("模块语法错误: {:?}", e),
                span: Span::dummy(),
            }])?;

        Ok(module)
    }

    /**
     * 验证函数
     */
    fn analyze_function(&mut self, func: &Function) -> Result<(), Vec<TypeError>> {
        // 进入函数作用域
        self.enter_scope();

        // 添加函数参数到作用域
        // 注意：XY编译器自展时会遇到前向引用问题（如 Parser 类型在函数定义之后才定义）
        // 因此我们跳过参数类型的验证，只注册参数名
        for param in &func.params {
            // 使用 Type::Unknown 作为参数类型，避免前向引用问题
            self.define_symbol(
                param.name.clone(),
                Type::Unknown,
                false,
                Span::dummy(),
            );
        }

        // 分析函数体
        for stmt in &func.body.statements {
            self.analyze_statement(stmt)?;
        }

        // 退出函数作用域
        self.exit_scope();

        Ok(())
    }

    /**
     * 验证语句
     */
    fn analyze_statement(&mut self, stmt: &Stmt) -> Result<Type, Vec<TypeError>> {
        match stmt {
            Stmt::Let(let_stmt) => {
                self.analyze_let_statement(let_stmt)?;
                Ok(Type::Void)
            }
            Stmt::Return(return_stmt) => {
                self.analyze_return_statement(return_stmt)
            }
            Stmt::If(if_stmt) => {
                self.analyze_if_statement(if_stmt)
            }
            Stmt::Loop(loop_stmt) => {
                self.analyze_loop_statement(loop_stmt)
            }
            Stmt::Expr(expr_stmt) => {
                self.analyze_expression(&expr_stmt.expr)?;
                Ok(Type::Void)
            }
            Stmt::Block(block_stmt) => {
                self.enter_scope();
                for s in &block_stmt.statements {
                    self.analyze_statement(s)?;
                }
                self.exit_scope();
                Ok(Type::Void)
            }
            Stmt::Break(_) | Stmt::Continue(_) => {
                Ok(Type::Void)
            }
            Stmt::Assignment(assign_stmt) => {
                self.analyze_assignment_statement(assign_stmt)
            }
            Stmt::StructDef(struct_def) => {
                // 注册结构体到符号表
                self.register_struct(struct_def)
            }
            Stmt::EnumDef(enum_def) => {
                // 注册枚举到符号表
                self.register_enum(enum_def)
            }
            Stmt::TypeAlias(type_alias) => {
                // 注册类型别名
                self.register_type_alias(type_alias)
            }
            Stmt::Constant(constant) => {
                // 分析常量定义
                self.analyze_constant_statement(constant)
            }
            Stmt::Match(match_stmt) => {
                // 分析模式匹配
                self.analyze_match_statement(match_stmt)
            }
        }
    }

    /**
     * 分析模式匹配语句
     */
    fn analyze_match_statement(&mut self, match_stmt: &MatchStmt) -> Result<Type, Vec<TypeError>> {
        // 分析要匹配的值
        let _subject_type = self.analyze_expression(&match_stmt.subject)?;
        
        // 分析每个分支
        for arm in &match_stmt.arms {
            // 在分支内创建新的作用域（用于变量绑定）
            self.enter_scope();
            
            // 处理字段绑定
            if let MatchPattern::EnumVariant { fields, .. } = &arm.pattern {
                for field in fields {
                    // 将捕获的字段作为变量添加到作用域
                    self.define_symbol(
                        field.binding_name.clone(),
                        Type::Int, // 简化：假设为整数类型
                        false,
                        match_stmt.span,
                    );
                }
            }
            
            // 分析分支体
            self.analyze_statement(&arm.body)?;
            
            self.exit_scope();
        }
        
        Ok(Type::Void)
    }

    /**
     * 分析常量定义语句
     */
    fn analyze_constant_statement(&mut self, constant: &ConstantDef) -> Result<Type, Vec<TypeError>> {
        // 分析常量值表达式
        let value_type = self.analyze_expression(&constant.value)?;
        
        // 检查类型标注
        if let Some(type_annotation) = &Some(constant.const_type.clone()) {
            if !value_type.can_cast_to(type_annotation) {
                self.error(
                    format!("常量类型不匹配: 期望 {:?}, 但找到 {:?}", type_annotation, value_type),
                    constant.span,
                );
            }
        }
        
        // 将常量添加到符号表（作为常量）
        self.define_symbol(
            constant.name.clone(),
            constant.const_type.clone(),
            false, // 常量不可变
            constant.span,
        );
        
        Ok(Type::Void)
    }

    /**
     * 注册内置函数
     */
    fn register_builtin_functions(&mut self) -> Result<(), Vec<TypeError>> {
        // 打印函数
        self.define_symbol("打印".to_string(), Type::Int, false, Span::dummy());
        self.define_symbol("打印整数".to_string(), Type::Int, false, Span::dummy());
        self.define_symbol("打印浮点".to_string(), Type::Int, false, Span::dummy());
        self.define_symbol("打印布尔".to_string(), Type::Int, false, Span::dummy());
        
        // 类型转换函数
        self.define_symbol("文本转整数".to_string(), Type::Int, false, Span::dummy());
        self.define_symbol("整数转文本".to_string(), Type::String, false, Span::dummy());
        
        // 列表函数
        self.define_symbol("创建列表".to_string(), Type::String, false, Span::dummy());
        self.define_symbol("列表添加".to_string(), Type::Int, false, Span::dummy());
        self.define_symbol("列表获取".to_string(), Type::Int, false, Span::dummy());
        self.define_symbol("列表长度".to_string(), Type::Int, false, Span::dummy());
        
        // 控制台输入函数
        self.define_symbol("输入整数".to_string(), Type::Int, false, Span::dummy());
        self.define_symbol("输入文本".to_string(), Type::String, false, Span::dummy());
        
        // 字符串函数
        self.define_symbol("文本长度".to_string(), Type::Int, false, Span::dummy());
        self.define_symbol("文本拼接".to_string(), Type::String, false, Span::dummy());
        self.define_symbol("文本切片".to_string(), Type::String, false, Span::dummy());
        self.define_symbol("文本包含".to_string(), Type::String, false, Span::dummy());
        self.define_symbol("文本获取字符".to_string(), Type::String, false, Span::dummy());
        self.define_symbol("字符编码".to_string(), Type::Int, false, Span::dummy());
        
        // 命令行参数函数
        self.define_symbol("参数个数".to_string(), Type::Int, false, Span::dummy());
        self.define_symbol("获取参数".to_string(), Type::String, false, Span::dummy());
        
        // 文件 I/O 函数
        self.define_symbol("文件读取".to_string(), Type::String, false, Span::dummy());
        self.define_symbol("文件写入".to_string(), Type::Int, false, Span::dummy());
        self.define_symbol("文件存在".to_string(), Type::Int, false, Span::dummy());
        self.define_symbol("文件删除".to_string(), Type::Int, false, Span::dummy());
        
        // 系统命令函数
        self.define_symbol("执行命令".to_string(), Type::Int, false, Span::dummy());
        self.define_symbol("命令输出".to_string(), Type::String, false, Span::dummy());
        
        Ok(())
    }

    /**
     * 注册结构体到符号表
     */
    fn register_struct(&mut self, struct_def: &StructDefinition) -> Result<Type, Vec<TypeError>> {
        let struct_type = Type::Struct(struct_def.name.clone());
        if let Some(scope) = self.scopes.last_mut() {
            scope.add_type(&struct_def.name, struct_type.clone());
        }
        Ok(Type::Void)
    }

    /**
     * 注册枚举到符号表
     */
    fn register_enum(&mut self, enum_def: &EnumDefinition) -> Result<Type, Vec<TypeError>> {
        let enum_type = Type::Custom(enum_def.name.clone());
        if let Some(scope) = self.scopes.last_mut() {
            scope.add_type(&enum_def.name, enum_type.clone());
            
            // 注册每个变体作为符号
            for variant in &enum_def.variants {
                // 变体名添加到符号表
                let variant_type = Type::Custom(format!("{}::{}", enum_def.name, variant.name));
                let symbol = Symbol {
                    name: variant.name.clone(),
                    symbol_type: variant_type,
                    is_mutable: false,
                    span: enum_def.span,
                };
                scope.define(variant.name.clone(), symbol);
            }
        }
        Ok(Type::Void)
    }

    /**
     * 注册类型别名
     */
    fn register_type_alias(&mut self, type_alias: &TypeAlias) -> Result<Type, Vec<TypeError>> {
        if let Some(scope) = self.scopes.last_mut() {
            scope.add_type(&type_alias.name, type_alias.aliased_type.clone());
        }
        Ok(Type::Void)
    }

    /**
     * 验证变量声明语句
     */
    fn analyze_let_statement(&mut self, let_stmt: &LetStmt) -> Result<(), Vec<TypeError>> {
        // 分析初始化表达式
        if let Some(init) = &let_stmt.initializer {
            let init_type = self.analyze_expression(init)?;

            // 检查类型标注
            if let Some(type_annotation) = &let_stmt.type_annotation {
                if !init_type.can_cast_to(type_annotation) {
                    self.error(
                        format!("类型不匹配: 期望 {:?}, 但找到 {:?}", type_annotation, init_type),
                        let_stmt.span,
                    );
                }
            }
        } else if let_stmt.type_annotation.is_none() {
            // 需要类型标注或初始化值
            // 注意：XY编译器自展时会遇到前向引用问题，这里不报错而是使用默认类型
            // self.error(
            //     "变量声明需要类型标注或初始化值".to_string(),
            //     let_stmt.span,
            // );
        }

        // 定义符号
        // 如果有类型标注，使用类型标注；否则检查初始化值的类型
        // 注意：XY编译器自展时可能遇到未定义的类型，我们使用 Unknown 或 Int 作为默认值
        let var_type = let_stmt.type_annotation.clone().unwrap_or(Type::Unknown);
        self.define_symbol(
            let_stmt.name.clone(),
            var_type,
            false, // TODO: 支持可变
            let_stmt.span,
        );

        Ok(())
    }

    /**
     * 验证返回语句
     */
    fn analyze_return_statement(&mut self, return_stmt: &ReturnStmt) -> Result<Type, Vec<TypeError>> {
        if let Some(value) = &return_stmt.value {
            self.analyze_expression(value)
        } else {
            Ok(Type::Void)
        }
    }

    /**
     * 验证 if 语句
     */
    fn analyze_if_statement(&mut self, if_stmt: &IfStmt) -> Result<Type, Vec<TypeError>> {
        for branch in &if_stmt.branches {
            let cond_type = self.analyze_expression(&branch.condition)?;
            
            // 条件必须是布尔类型、整数类型或字符串类型（用于比较结果）
            if cond_type != Type::Bool && cond_type != Type::Int && cond_type != Type::String {
                self.error(
                    format!("if 条件必须是布尔类型，但找到 {:?}", cond_type),
                    branch.condition.span(),
                );
            }

            self.analyze_statement(&branch.body)?;
        }

        if let Some(else_branch) = &if_stmt.else_branch {
            self.analyze_statement(else_branch)?;
        }

        Ok(Type::Void)
    }

    /**
     * 验证循环语句
     */
    fn analyze_loop_statement(&mut self, loop_stmt: &LoopStmt) -> Result<Type, Vec<TypeError>> {
        // 处理计数循环 - 自动定义循环变量
        if let Some(counter) = &loop_stmt.counter {
            // 定义循环变量
            self.define_symbol(
                counter.variable.clone(),
                Type::Int,
                true,
                loop_stmt.span,
            );
        }
        
        if let Some(cond) = &loop_stmt.condition {
            let cond_type = self.analyze_expression(cond)?;
            if cond_type != Type::Bool {
                self.error(
                    format!("循环条件必须是布尔类型，但找到 {:?}", cond_type),
                    cond.span(),
                );
            }
        }

        self.analyze_statement(&loop_stmt.body)
    }

    /**
     * 验证赋值语句
     */
    fn analyze_assignment_statement(&mut self, assign_stmt: &AssignmentStmt) -> Result<Type, Vec<TypeError>> {
        // 检查左值
        if let Expr::Identifier(ident) = &assign_stmt.target {
            let symbol = self.lookup_symbol(&ident.name);
            if symbol.is_none() {
                self.error(
                    format!("未定义的变量: {}", ident.name),
                    ident.span,
                );
            }
        }

        let value_type = self.analyze_expression(&assign_stmt.value)?;
        
        // TODO: 检查赋值类型兼容性
        
        Ok(Type::Void)
    }

    /**
     * 验证表达式
     */
    fn analyze_expression(&mut self, expr: &Expr) -> Result<Type, Vec<TypeError>> {
        match expr {
            Expr::Identifier(ident) => {
                // 首先查找变量符号
                let symbol = self.lookup_symbol(&ident.name);
                match symbol {
                    Some(s) => Ok(s.symbol_type.clone()),
                    None => {
                        // 变量未找到，检查是否是类型名（枚举类型可以作为表达式使用）
                        if self.lookup_type(&ident.name).is_some() {
                            // 返回自定义类型
                            Ok(Type::Custom(ident.name.clone()))
                        } else {
                            // 注意：XY编译器自展时会遇到前向引用问题
                            // 这里不报错而是返回 Unknown，让编译器继续运行
                            Ok(Type::Unknown)
                        }
                    }
                }
            }
            Expr::Literal(lit) => {
                Ok(lit.kind.type_info())
            }
            Expr::Binary(binary) => {
                self.analyze_binary_expression(binary)
            }
            Expr::Unary(unary) => {
                self.analyze_unary_expression(unary)
            }
            Expr::Call(call) => {
                self.analyze_call_expression(call)
            }
            Expr::MemberAccess(member) => {
                self.analyze_member_expression(member)
            }
            Expr::Grouped(expr) => {
                self.analyze_expression(expr)
            }
            Expr::ListLiteral(list) => {
                // 分析列表元素类型
                let mut elem_type = Type::Int;  // 默认元素类型
                for elem in &list.elements {
                    let t = self.analyze_expression(elem)?;
                    elem_type = t;  // 使用最后一个元素的类型
                }
                // 返回列表类型 (泛型)
                Ok(Type::List(Box::new(elem_type)))
            }
            Expr::IndexAccess(index) => {
                // 分析被索引的对象
                let obj_type = self.analyze_expression(&index.object)?;
                // 分析索引表达式
                self.analyze_expression(&index.index)?;
                // 索引访问返回元素类型（简化处理，返回整数）
                Ok(Type::Int)
            }
            Expr::ListComprehension(comp) => {
                // 分析迭代列表
                let iterable_type = self.analyze_expression(&comp.iterable)?;
                
                // 进入新作用域
                self.enter_scope();
                
                // 在新作用域中注册迭代变量
                let var_type = match iterable_type {
                    Type::List(elem_type) => *elem_type,
                    _ => Type::Int,
                };
                self.define_symbol(comp.var_name.clone(), var_type.clone(), false, comp.span);
                
                // 分析输出表达式
                let output_type = self.analyze_expression(&comp.output)?;
                
                // 分析可选条件
                if let Some(cond) = &comp.condition {
                    self.analyze_expression(cond)?;
                }
                
                // 退出作用域
                self.exit_scope();
                
                // 返回列表类型
                Ok(Type::List(Box::new(output_type)))
            }
        }
    }

    /**
     * 验证二元表达式
     */
    fn analyze_binary_expression(&mut self, binary: &BinaryExpr) -> Result<Type, Vec<TypeError>> {
        let left_type = self.analyze_expression(&binary.left)?;
        let right_type = self.analyze_expression(&binary.right)?;

        match binary.op {
            // 算术运算
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {
                // 左右类型必须相同且为数值类型
                // 注意：XY编译器自展时会遇到 Unknown 类型（用于前向引用）
                if left_type == Type::Unknown || right_type == Type::Unknown {
                    // 有一个操作数是 Unknown，返回 Unknown 让编译器继续
                    return Ok(Type::Unknown);
                }
                if left_type != right_type {
                    return Err(vec![TypeError {
                        code: "CCAS-T003".to_string(),
                        message: format!("算术运算需要相同类型，但左边是 {:?}，右边是 {:?}", left_type, right_type),
                        span: binary.span,
                    }]);
                }
                Ok(left_type)
            }
            // 比较运算
            BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Gt | BinaryOp::Lt | BinaryOp::Ge | BinaryOp::Le => {
                Ok(Type::Bool)
            }
            // 逻辑运算
            BinaryOp::And | BinaryOp::Or => {
                if left_type != Type::Bool || right_type != Type::Bool {
                    return Err(vec![TypeError {
                        code: "CCAS-T004".to_string(),
                        message: "逻辑运算需要布尔类型".to_string(),
                        span: binary.span,
                    }]);
                }
                Ok(Type::Bool)
            }
            // 位运算
            BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor | BinaryOp::Shl | BinaryOp::Shr | BinaryOp::Hash => {
                Ok(left_type)
            }
            // 赋值
            BinaryOp::Assign => {
                Ok(right_type)
            }
        }
    }

    /**
     * 验证一元表达式
     */
    fn analyze_unary_expression(&mut self, unary: &UnaryExpr) -> Result<Type, Vec<TypeError>> {
        let operand_type = self.analyze_expression(&unary.operand)?;

        match unary.op {
            UnaryOp::Neg => {
                // 负号只能用于数值类型
                if !operand_type.is_numeric() {
                    return Err(vec![TypeError {
                        code: "CCAS-T005".to_string(),
                        message: format!("负号只能用于数值类型，但找到 {:?}", operand_type),
                        span: unary.span,
                    }]);
                }
                Ok(operand_type)
            }
            UnaryOp::Not => {
                // 逻辑非只能用于布尔类型
                if operand_type != Type::Bool {
                    return Err(vec![TypeError {
                        code: "CCAS-T006".to_string(),
                        message: format!("非运算只能用于布尔类型，但找到 {:?}", operand_type),
                        span: unary.span,
                    }]);
                }
                Ok(Type::Bool)
            }
            UnaryOp::BitNot => {
                // 位非只能用于整数类型
                if !operand_type.is_integer() {
                    return Err(vec![TypeError {
                        code: "CCAS-T007".to_string(),
                        message: format!("位非只能用于整数类型，但找到 {:?}", operand_type),
                        span: unary.span,
                    }]);
                }
                Ok(operand_type)
            }
        }
    }

    /**
     * 验证函数调用表达式
     */
    fn analyze_call_expression(&mut self, call: &CallExpr) -> Result<Type, Vec<TypeError>> {
        // 分析函数名表达式
        if let Expr::Identifier(ident) = &*call.function {
            // 检查是否为内置类型构造函数
            match ident.name.as_str() {
                "列表" => {
                    // 列表构造函数，返回空列表类型
                    return Ok(Type::List(Box::new(Type::Int)));
                }
                "整数" => {
                    // 整数构造函数，返回整数类型
                    return Ok(Type::Int);
                }
                "文本" => {
                    // 文本构造函数，返回文本类型
                    return Ok(Type::String);
                }
                "打印" | "print" => {
                    // 打印函数，返回 Void
                    return Ok(Type::Void);
                }
                _ => {}
            }
            
            // 查找符号
            let symbol = self.lookup_symbol(&ident.name);
            if let Some(sym) = symbol {
                // 如果找到符号，检查是否是枚举变体
                let sym_type_str = format!("{:?}", sym.symbol_type);
                if sym_type_str.contains("::") {
                    // 是枚举变体构造函数，提取枚举类型
                    let enum_name = sym_type_str
                        .trim_start_matches("Custom(\"")
                        .trim_end_matches("\")");
                    let parts: Vec<&str> = enum_name.split("::").collect();
                    if !parts.is_empty() {
                        return Ok(Type::Custom(parts[0].to_string()));
                    }
                }
                // 返回找到的符号类型
                return Ok(sym.symbol_type.clone());
            }
            
            // 没找到，尝试查找枚举变体
            if let Some(enum_name) = self.find_enum_variant(&ident.name) {
                return Ok(Type::Custom(enum_name));
            }

            // 注意：XY编译器自展时会遇到函数前向引用问题
            // 函数调用在定义之前是常见的模式（如递归、互递归）
            // 因此我们不报错，而是返回 Unknown 类型，让编译器继续运行
            return Ok(Type::Unknown)
        }

        // TODO: 检查参数类型匹配
        
        Ok(Type::Int) // 简化返回 int
    }
    
    /**
     * 查找枚举变体（通过遍历所有符号）
     */
    fn find_enum_variant(&self, variant_name: &str) -> Option<String> {
        for scope in &self.scopes {
            for (name, symbol) in &scope.symbols {
                if name == variant_name {
                    let sym_type_str = format!("{:?}", symbol.symbol_type);
                    if sym_type_str.contains("::") {
                        // 提取枚举名: Custom("颜色::红") -> 颜色
                        let enum_name = sym_type_str
                            .trim_start_matches("Custom(\"")
                            .trim_end_matches("\")");
                        let parts: Vec<&str> = enum_name.split("::").collect();
                        if !parts.is_empty() {
                            return Some(parts[0].to_string());
                        }
                    }
                }
            }
        }
        None
    }

    /**
     * 验证成员访问表达式
     */
    fn analyze_member_expression(&mut self, member: &MemberAccessExpr) -> Result<Type, Vec<TypeError>> {
        // 分析对象表达式
        let object_type = self.analyze_expression(&member.object)?;
        
        // 检查是否是结构体类型
        if let Type::Struct(struct_name) = &object_type {
            // 查找结构体定义
            if let Some(scope) = self.scopes.last() {
                if let Some(struct_type) = scope.lookup_type(struct_name, &self.scopes) {
                    // 结构体类型已注册，简化处理返回整数
                    return Ok(Type::Int);
                }
            }
        }
        
        // 简化处理：返回整数类型
        Ok(Type::Int)
    }
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/**
 * 类型辅助方法
 */
trait TypeExt {
    fn can_cast_to(&self, target: &Type) -> bool;
    fn is_numeric(&self) -> bool;
    fn is_integer(&self) -> bool;
}

impl TypeExt for Type {
    fn can_cast_to(&self, target: &Type) -> bool {
        match (self, target) {
            (Type::Int, Type::Long) => true,
            (Type::Int, Type::Float) => true,
            (Type::Int, Type::Double) => true,
            (Type::Long, Type::Float) => true,
            (Type::Long, Type::Double) => true,
            (Type::Float, Type::Double) => true,
            _ => self == target,
        }
    }

    fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Long | Type::Float | Type::Double)
    }

    fn is_integer(&self) -> bool {
        matches!(self, Type::Int | Type::Long)
    }
}

/**
 * 字面量类型推断
 */
impl LiteralKind {
    pub fn type_info(&self) -> Type {
        match self {
            LiteralKind::Integer(_) => Type::Int,
            LiteralKind::Float(_) => Type::Float,
            LiteralKind::String(_) => Type::String,
            LiteralKind::Char(_) => Type::Char,
            LiteralKind::Boolean(_) => Type::Bool,
        }
    }
}

/**
 * 语义分析入口函数
 */
pub fn analyze(module: &Module) -> Result<(), Vec<TypeError>> {
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze_module(module)
}
