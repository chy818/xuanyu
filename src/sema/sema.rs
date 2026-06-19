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
use crate::error::TypeError;

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
 * 函数签名
 * 存储函数的参数类型和返回类型
 */
#[derive(Debug, Clone)]
struct FunctionSignature {
    /// 参数类型列表
    param_types: Vec<Type>,
    /// 返回类型
    return_type: Type,
    /// 是否是泛型函数
    is_generic: bool,
    /// 类型参数名称列表
    type_params: Vec<String>,
}

/**
 * 语义分析器
 */
pub struct SemanticAnalyzer {
    scopes: Vec<Scope>,
    errors: Vec<TypeError>,
    import_stack: Vec<String>,
    /// 当前模块的目录（用于解析相对导入路径）
    current_module_dir: Option<String>,
    /// 当前泛型函数的类型变量映射
    /// 例如: T -> 整数, U -> 文本
    type_var_bindings: std::collections::HashMap<String, Type>,
    /// 函数签名表
    /// 存储所有已定义函数的签名信息
    function_signatures: std::collections::HashMap<String, FunctionSignature>,
    /// 结构体定义表
    /// 存储所有已定义结构体的字段信息
    struct_definitions: std::collections::HashMap<String, crate::ast::StructDefinition>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let mut scopes = Vec::new();
        scopes.push(Scope::new(None)); // 全局作用域
        Self {
            scopes,
            errors: Vec::new(),
            import_stack: Vec::new(),
            current_module_dir: None,
            type_var_bindings: std::collections::HashMap::new(),
            function_signatures: std::collections::HashMap::new(),
            struct_definitions: std::collections::HashMap::new(),
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
     * 添加类型变量绑定
     */
    fn add_type_var_binding(&mut self, type_var: String, concrete_type: Type) {
        self.type_var_bindings.insert(type_var, concrete_type);
    }

    /**
     * 查找类型变量绑定
     */
    fn lookup_type_var(&self, type_var: &str) -> Option<Type> {
        self.type_var_bindings.get(type_var).cloned()
    }

    /**
     * 清除类型变量绑定（进入泛型函数时调用）
     */
    fn clear_type_vars(&mut self) {
        self.type_var_bindings.clear();
    }

    /**
     * 替换类型中的类型变量为具体类型
     */
    #[allow(dead_code)]
    fn substitute_type_vars(&self, type_hint: &Type) -> Type {
        match type_hint {
            Type::TypeVar(name) => {
                self.lookup_type_var(name).unwrap_or_else(|| type_hint.clone())
            }
            Type::List(elem) => {
                Type::List(Box::new(self.substitute_type_vars(elem)))
            }
            Type::Optional(inner) => {
                Type::Optional(Box::new(self.substitute_type_vars(inner)))
            }
            Type::Array(elem) => {
                Type::Array(Box::new(self.substitute_type_vars(elem)))
            }
            Type::Function(param_types, return_type) => {
                let substituted_params: Vec<Type> = param_types.iter()
                    .map(|t| self.substitute_type_vars(t))
                    .collect();
                let substituted_return = Box::new(self.substitute_type_vars(return_type));
                Type::Function(substituted_params, substituted_return)
            }
            _ => type_hint.clone(),
        }
    }

    /**
     * 使用给定的类型映射替换类型变量
     * @param type_hint 要替换的类型
     * @param type_subst 类型变量到具体类型的映射
     */
    fn substitute_type_with_map(&self, type_hint: &Type, type_subst: &std::collections::HashMap<String, Type>) -> Type {
        match type_hint {
            Type::TypeVar(name) => {
                type_subst.get(name).cloned().unwrap_or_else(|| type_hint.clone())
            }
            Type::List(elem) => {
                Type::List(Box::new(self.substitute_type_with_map(elem, type_subst)))
            }
            Type::Optional(inner) => {
                Type::Optional(Box::new(self.substitute_type_with_map(inner, type_subst)))
            }
            Type::Array(elem) => {
                Type::Array(Box::new(self.substitute_type_with_map(elem, type_subst)))
            }
            Type::Function(param_types, return_type) => {
                let substituted_params: Vec<Type> = param_types.iter()
                    .map(|t| self.substitute_type_with_map(t, type_subst))
                    .collect();
                let substituted_return = Box::new(self.substitute_type_with_map(return_type, type_subst));
                Type::Function(substituted_params, substituted_return)
            }
            _ => type_hint.clone(),
        }
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

        // 处理顶级变量定义
        for var_stmt in &module.variables {
            self.analyze_let_statement(var_stmt)?;
        }

        // 收集所有函数声明到全局作用域
        for func in &module.functions {
            self.define_symbol(
                func.name.clone(),
                func.return_type.clone(),
                false,
                func.span,
            );
            
            // 存储函数签名（用于泛型函数调用的类型推断）
            let param_types: Vec<Type> = func.params.iter()
                .map(|p| p.param_type.clone())
                .collect();
            let signature = FunctionSignature {
                param_types,
                return_type: func.return_type.clone(),
                is_generic: !func.type_params.is_empty(),
                type_params: func.type_params.iter().map(|tp| tp.name.clone()).collect(),
            };
            self.function_signatures.insert(func.name.clone(), signature);
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

        // 保存当前的模块目录
        let previous_dir = self.current_module_dir.take();

        // 尝试加载模块文件（传递当前模块目录用于相对路径解析）
        let resolved_path = self.resolve_module_path(module_path, previous_dir.as_deref())?;
        let imported_module = self.load_module(module_path, previous_dir.as_deref())?;

        // 更新当前模块目录为被导入模块的目录
        let imported_dir = std::path::Path::new(&resolved_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string());
        self.current_module_dir = imported_dir;

        // 递归分析导入的模块
        self.analyze_module(&imported_module)?;

        // 从导入栈中移除
        self.import_stack.pop();

        // 恢复之前的模块目录
        self.current_module_dir = previous_dir;

        // 将导入的符号添加到当前作用域
        for func in &imported_module.functions {
            self.define_symbol(
                func.name.clone(),
                func.return_type.clone(),
                false,
                func.span.clone(),
            );
            
            // 同时存储函数签名（用于类型推断）
            let param_types: Vec<Type> = func.params.iter()
                .map(|p| p.param_type.clone())
                .collect();
            let signature = FunctionSignature {
                param_types,
                return_type: func.return_type.clone(),
                is_generic: !func.type_params.is_empty(),
                type_params: func.type_params.iter().map(|tp| tp.name.clone()).collect(),
            };
            self.function_signatures.insert(func.name.clone(), signature);
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
     * 解析模块文件路径
     * 尝试多个可能的位置
     */
    fn resolve_module_path(&self, module_path: &str, current_file: Option<&str>) -> Result<String, Vec<TypeError>> {
        let path = module_path.trim_matches('"');

        // 尝试1: 直接作为绝对/相对路径
        if std::path::Path::new(path).exists() {
            return Ok(path.to_string());
        }

        // 尝试2: 相对于当前模块目录
        if let Some(current) = current_file {
            let current_dir = std::path::Path::new(current)
                .parent()
                .map(|p| p.to_string_lossy().to_string());

            if let Some(dir) = current_dir {
                let relative_path = format!("{}/{}", dir, path);
                if std::path::Path::new(&relative_path).exists() {
                    return Ok(relative_path);
                }
            }
        }

        // 尝试3: 相对于项目根目录 (当前工作目录)
        // 检查是否是 stdlib 模块
        if !path.starts_with("stdlib/") && !path.starts_with("./") && !path.starts_with("../") {
            let stdlib_path = format!("stdlib/{}", path);
            if std::path::Path::new(&stdlib_path).exists() {
                return Ok(stdlib_path);
            }
        }

        // 尝试4: 直接使用提供的路径
        if std::path::Path::new(path).exists() {
            return Ok(path.to_string());
        }

        // 所有尝试都失败
        Err(vec![TypeError {
            code: "E0428".to_string(),
            message: format!("无法找到模块 '{}' (已尝试相对于当前模块目录和 stdlib 目录)", path),
            span: Span::dummy(),
        }])
    }

    /**
     * 加载模块文件
     */
    fn load_module(&self, module_path: &str, current_file: Option<&str>) -> Result<Module, Vec<TypeError>> {
        // 解析模块路径
        let resolved_path = self.resolve_module_path(module_path, current_file)?;

        // 读取模块文件（使用 UTF-8 编码）
        let source = std::fs::read(&resolved_path)
            .map_err(|e| vec![TypeError {
                code: "E0428".to_string(),
                message: format!("无法加载模块 '{}': {}", resolved_path, e),
                span: Span::dummy(),
            }])?;
        let source = String::from_utf8(source)
            .map_err(|_e| vec![TypeError {
                code: "E0428".to_string(),
                message: format!("模块不是有效的 UTF-8 编码: {}", resolved_path),
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

        // 如果是泛型函数，注册类型参数
        let is_generic = !func.type_params.is_empty();
        if is_generic {
            self.clear_type_vars();
            for type_param in &func.type_params {
                self.add_type_var_binding(
                    type_param.name.clone(),
                    Type::TypeVar(type_param.name.clone()),
                );
            }
        }

        // 构建类型参数名集合，用于快速查找
        let type_param_names: std::collections::HashSet<String> =
            func.type_params.iter().map(|tp| tp.name.clone()).collect();

        // 处理返回类型中的类型参数
        let return_type = self.substitute_custom_to_typevar(&func.return_type, &type_param_names);

        // 添加函数参数到作用域
        // 注意：XY编译器自展时会遇到前向引用问题（如 Parser 类型在函数定义之后才定义）
        // 因此我们跳过参数类型的验证，只注册参数名
        for param in &func.params {
            // 检查参数类型是否是自定义类型且名称与类型参数匹配
            let param_type = self.substitute_custom_to_typevar(&param.param_type, &type_param_names);

            self.define_symbol(
                param.name.clone(),
                param_type.clone(),
                true,  // 函数参数默认可变，支持函数式编程风格
                Span::dummy(),
            );
        }

        // 检查返回类型是否有效
        match &return_type {
            Type::TypeVar(_name) => {
                // 类型变量是允许的
            }
            Type::Unknown => {
                // Unknown 是允许的，表示尚未推断
            }
            _ => {
                // 其他类型是有效的
            }
        }

        // 分析函数体
        for stmt in &func.body.statements {
            self.analyze_statement(stmt)?;
        }

        // 退出函数作用域
        self.exit_scope();

        // 如果是泛型函数，清除类型变量绑定
        if is_generic {
            self.clear_type_vars();
        }

        Ok(())
    }

    /**
     * 将 Custom("T") 替换为 TypeVar("T") 如果 T 在类型参数列表中
     */
    fn substitute_custom_to_typevar(&self, type_hint: &Type, type_param_names: &std::collections::HashSet<String>) -> Type {
        match type_hint {
            Type::Custom(name) => {
                if type_param_names.contains(name) {
                    Type::TypeVar(name.clone())
                } else {
                    type_hint.clone()
                }
            }
            Type::List(elem) => {
                Type::List(Box::new(self.substitute_custom_to_typevar(elem, type_param_names)))
            }
            Type::Optional(inner) => {
                Type::Optional(Box::new(self.substitute_custom_to_typevar(inner, type_param_names)))
            }
            Type::Array(elem) => {
                Type::Array(Box::new(self.substitute_custom_to_typevar(elem, type_param_names)))
            }
            Type::Function(param_types, return_type) => {
                let substituted_params: Vec<Type> = param_types.iter()
                    .map(|t| self.substitute_custom_to_typevar(t, type_param_names))
                    .collect();
                let substituted_return = Box::new(self.substitute_custom_to_typevar(return_type, type_param_names));
                Type::Function(substituted_params, substituted_return)
            }
            _ => type_hint.clone(),
        }
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
            Stmt::Try(try_stmt) => {
                // 分析异常处理
                self.analyze_try_statement(try_stmt)
            }
            Stmt::Throw(throw_stmt) => {
                // 分析抛出异常
                self.analyze_throw_statement(throw_stmt)
            }
        }
    }

    /**
     * 分析 try-catch-finally 语句
     */
    fn analyze_try_statement(&mut self, try_stmt: &TryStmt) -> Result<Type, Vec<TypeError>> {
        // 分析 try 块
        self.enter_scope();
        for stmt in &try_stmt.try_block.statements {
            self.analyze_statement(stmt)?;
        }
        self.exit_scope();

        // 分析 catch 子句
        for catch_clause in &try_stmt.catch_clauses {
            self.enter_scope();
            // 将异常变量添加到作用域
            self.define_symbol(
                catch_clause.var_name.clone(),
                Type::Custom("异常".to_string()),
                false,
                catch_clause.span.clone(),
            );
            // 分析 catch 块
            for stmt in &catch_clause.body.statements {
                self.analyze_statement(stmt)?;
            }
            self.exit_scope();
        }

        // 分析 finally 块
        if let Some(ref finally_block) = try_stmt.finally_block {
            self.enter_scope();
            for stmt in &finally_block.statements {
                self.analyze_statement(stmt)?;
            }
            self.exit_scope();
        }

        Ok(Type::Void)
    }

    /**
     * 分析 throw 语句
     */
    fn analyze_throw_statement(&mut self, throw_stmt: &ThrowStmt) -> Result<Type, Vec<TypeError>> {
        // 分析异常表达式
        let exception_type = self.analyze_expression(&throw_stmt.exception)?;
        
        // 验证抛出的是异常类型
        // 简化实现：接受任何类型
        
        Ok(exception_type)
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
        self.define_symbol("列表获取".to_string(), Type::Any, false, Span::dummy());
        self.define_symbol("列表长度".to_string(), Type::Int, false, Span::dummy());
        
        // 添加列表函数的签名（用于类型推断）
        self.function_signatures.insert("列表获取".to_string(), FunctionSignature {
            param_types: vec![Type::List(Box::new(Type::Any)), Type::Int],
            return_type: Type::Any,
            is_generic: false,
            type_params: vec![],
        });
        
        // 控制台输入函数
        self.define_symbol("输入整数".to_string(), Type::Int, false, Span::dummy());
        self.define_symbol("输入文本".to_string(), Type::String, false, Span::dummy());
        
        // 字符串函数
        self.define_symbol("文本长度".to_string(), Type::Int, false, Span::dummy());
        self.define_symbol("文本拼接".to_string(), Type::String, false, Span::dummy());
        self.define_symbol("文本切片".to_string(), Type::String, false, Span::dummy());
        self.define_symbol("文本包含".to_string(), Type::Int, false, Span::dummy());
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
        // 存储结构体定义
        self.struct_definitions.insert(struct_def.name.clone(), struct_def.clone());
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
     * 支持类型推断：如果没有显式类型标注，从初始化表达式推断类型
     */
    fn analyze_let_statement(&mut self, let_stmt: &LetStmt) -> Result<(), Vec<TypeError>> {
        // 分析初始化表达式并推断类型
        let inferred_type = if let Some(init) = &let_stmt.initializer {
            let init_type = self.analyze_expression(init)?;
            Some(init_type)
        } else {
            None
        };

        // 检查类型标注与初始化值类型是否兼容
        if let Some(_init) = &let_stmt.initializer {
            if let Some(type_annotation) = &let_stmt.type_annotation {
                if let Some(ref init_type) = inferred_type {
                    if !init_type.can_cast_to(type_annotation) {
                        self.error(
                            format!("类型不匹配: 期望 {:?}, 但找到 {:?}", type_annotation, init_type),
                            let_stmt.span,
                        );
                    }
                }
            }
        }

        // 确定最终变量类型
        // 优先级: 显式类型标注 > 推断类型 > 默认类型
        let var_type = if let Some(type_annotation) = &let_stmt.type_annotation {
            type_annotation.clone()
        } else if let Some(ref init_type) = inferred_type {
            init_type.clone()
        } else {
            Type::Unknown
        };

        // 定义符号
        self.define_symbol(
            let_stmt.name.clone(),
            var_type,
            let_stmt.is_mutable,
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
     * 检查：1. 变量是否已定义 2. 变量是否可变 3. 赋值类型兼容性
     */
    fn analyze_assignment_statement(&mut self, assign_stmt: &AssignmentStmt) -> Result<Type, Vec<TypeError>> {
        // 检查左值
        if let Expr::Identifier(ident) = &assign_stmt.target {
            let symbol_info = self.lookup_symbol(&ident.name).cloned();
            
            if symbol_info.is_none() {
                return Err(vec![TypeError {
                    code: "CCAS-T001".to_string(),
                    message: format!("未定义的变量: {}", ident.name),
                    span: ident.span,
                }]);
            }
            
            // 获取符号信息的副本
            let sym = symbol_info.unwrap();
            
            // 检查变量是否可变
            if !sym.is_mutable {
                return Err(vec![TypeError {
                    code: "CCAS-T002".to_string(),
                    message: format!("变量 '{}' 是不可变的，不能重新赋值。使用 '定义 可变 {}' 声明可变变量", 
                        ident.name, ident.name),
                    span: ident.span,
                }]);
            }
            
            // 检查赋值类型兼容性
            let value_type = self.analyze_expression(&assign_stmt.value)?;
            if sym.symbol_type != Type::Unknown && value_type != Type::Unknown {
                if !self.is_type_compatible(&sym.symbol_type, &value_type) {
                    return Err(vec![TypeError {
                        code: "CCAS-T003".to_string(),
                        message: format!("赋值类型不匹配: 变量 '{}' 类型为 {:?}, 但赋值类型为 {:?}", 
                            ident.name, sym.symbol_type, value_type),
                        span: assign_stmt.value.span(),
                    }]);
                }
            }
        } else {
            // 非标识符的左值（如数组索引、结构体字段等）
            self.analyze_expression(&assign_stmt.target)?;
        }

        Ok(Type::Void)
    }

    /**
     * 检查类型兼容性
     * 判断 source_type 是否可以赋值给 target_type
     */
    fn is_type_compatible(&self, target_type: &Type, source_type: &Type) -> bool {
        match (target_type, source_type) {
            // 相同类型直接兼容
            (t1, t2) if t1 == t2 => true,

            // Unknown 类型可以接受任何类型（用于自展）
            (Type::Unknown, _) | (_, Type::Unknown) => true,

            // Any 类型可以接受或转换为任何类型（支持异构列表）
            (Type::Any, _) | (_, Type::Any) => true,

            // 整数类型兼容（包括不同位宽）
            (Type::Int, Type::Int) => true,
            (Type::Long, Type::Int) => true,
            (Type::Int, Type::Long) => true,
            (Type::Long, Type::Long) => true,

            // 浮点类型兼容
            (Type::Float, Type::Float) => true,
            (Type::Double, Type::Float) => true,
            (Type::Float, Type::Double) => true,
            (Type::Double, Type::Double) => true,

            // 列表类型协变：如果元素类型兼容，则列表类型兼容
            // 例如：List(Int) 可以赋值给 List(Any)
            (Type::List(target_elem), Type::List(source_elem)) => {
                self.is_type_compatible(target_elem, source_elem)
            }

            // Optional 类型协变
            (Type::Optional(target_inner), Type::Optional(source_inner)) => {
                self.is_type_compatible(target_inner, source_inner)
            }

            // 其他情况不兼容
            _ => false,
        }
    }

    /**
     * 验证表达式
     */
    fn analyze_expression(&mut self, expr: &Expr) -> Result<Type, Vec<TypeError>> {
        match expr {
            Expr::Identifier(ident) => {
                // 首先检查是否是类型变量（泛型参数）
                if let Some(type_var) = self.lookup_type_var(&ident.name) {
                    return Ok(type_var);
                }

                // 其次查找变量符号
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
                // 分析对象表达式
                let object_type = self.analyze_expression(&member.object)?;
                let member_name = &member.member;

                // 确定成员类型
                let result = match &object_type {
                    Type::Struct(struct_name) | Type::Custom(struct_name) => {
                        if let Some(struct_def) = self.struct_definitions.get(struct_name) {
                            let mut found = false;
                            let mut field_type = Type::Any;
                            for field in &struct_def.fields {
                                if &field.name == member_name {
                                    field_type = field.field_type.clone();
                                    found = true;
                                    break;
                                }
                            }
                            if found {
                                field_type
                            } else {
                                Type::Any
                            }
                        } else {
                            Type::Any
                        }
                    }
                    Type::List(_) => {
                        // 处理列表方法
                        if member_name == "获取" || member_name == "append" || member_name == "set" || member_name == "len" {
                            Type::Any
                        } else {
                            Type::Any
                        }
                    }
                    _ => Type::Any,
                };

                // 设置成员类型（使用内部可变性）
                member.set_member_type(result.clone());
                Ok(result)
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
                // 索引访问返回元素类型
                match obj_type {
                    Type::List(elem_type) => Ok(*elem_type),
                    Type::String => Ok(Type::Int),  // 字符串索引返回字符编码
                    Type::Array(elem_type) => Ok(*elem_type),
                    _ => Ok(Type::Any),  // 默认返回 Any 类型
                }
            }
            Expr::Await(await_expr) => {
                // 分析被等待的表达式
                let inner_type = self.analyze_expression(&await_expr.expr)?;
                // 如果内部类型是 Future<T>，返回 T
                match inner_type {
                    Type::Future(t) => Ok(*t),
                    _ => Ok(inner_type),
                }
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
            Expr::Lambda(lambda) => {
                // Lambda 表达式分析
                // 1. 进入 Lambda 作用域
                self.enter_scope();

                // 2. 注册参数为局部变量
                let mut param_names = Vec::new();
                let mut param_types = Vec::new();
                for param in &lambda.params {
                    self.define_symbol(
                        param.name.clone(),
                        param.param_type.clone(),
                        false,
                        lambda.span
                    );
                    param_names.push(param.name.clone());
                    param_types.push(param.param_type.clone());
                }

                // 3. 分析函数体，收集自由变量（外部变量）
                let mut captured_vars = Vec::new();
                self.collect_free_variables(&lambda.body, &param_names, &mut captured_vars)?;

                // 4. 分析函数体获取返回类型
                let body_type = self.analyze_expression(&lambda.body)?;

                // 5. 退出作用域
                self.exit_scope();

                // 6. 更新 Lambda 的捕获变量列表
                let mut lambda_mut = lambda.clone();
                lambda_mut.captured_vars = captured_vars.clone();

                // 7. 返回函数类型
                Ok(Type::Function(param_types, Box::new(body_type)))
            }
        }
    }

    /**
     * 收集表达式中的自由变量
     * free_vars: 已知的自由变量集合
     * captured: 收集到的被捕获变量
     */
    fn collect_free_variables(
        &self,
        expr: &Expr,
        param_names: &[String],
        captured: &mut Vec<CapturedVar>,
    ) -> Result<(), Vec<TypeError>> {
        match expr {
            Expr::Identifier(ident) => {
                // 检查是否是 lambda 参数
                if !param_names.contains(&ident.name) {
                    // 检查变量是否存在于符号表中
                    if let Some(_symbol) = self.lookup_symbol(&ident.name) {
                        // 变量存在，需要捕获（因为 lambda 是独立函数，无法访问外部栈帧）
                        // 检查是否已经捕获
                        if !captured.iter().any(|v| v.name == ident.name) {
                            // 获取变量类型
                            let var_type = self.analyze_identifier_type(&ident.name)?;
                            captured.push(CapturedVar {
                                name: ident.name.clone(),
                                var_type,
                            });
                        }
                    }
                    // 如果变量不存在于符号表中，可能是全局函数，不需要捕获
                }
                Ok(())
            }
            Expr::Lambda(inner_lambda) => {
                // 嵌套 Lambda：参数不能被外层捕获
                let mut inner_param_names = param_names.to_vec();
                for param in &inner_lambda.params {
                    inner_param_names.push(param.name.clone());
                }
                // 分析内部 Lambda 的体
                self.collect_free_variables(&inner_lambda.body, &inner_param_names, captured)
            }
            Expr::Binary(binary) => {
                self.collect_free_variables(&binary.left, param_names, captured)?;
                self.collect_free_variables(&binary.right, param_names, captured)
            }
            Expr::Unary(unary) => {
                self.collect_free_variables(&unary.operand, param_names, captured)
            }
            Expr::Call(call) => {
                self.collect_free_variables(&call.function, param_names, captured)?;
                for arg in &call.arguments {
                    self.collect_free_variables(arg, param_names, captured)?;
                }
                Ok(())
            }
            Expr::MemberAccess(member) => {
                self.collect_free_variables(&member.object, param_names, captured)
            }
            Expr::ListLiteral(list) => {
                for elem in &list.elements {
                    self.collect_free_variables(elem, param_names, captured)?;
                }
                Ok(())
            }
            Expr::IndexAccess(index) => {
                self.collect_free_variables(&index.object, param_names, captured)?;
                self.collect_free_variables(&index.index, param_names, captured)
            }
            Expr::ListComprehension(comp) => {
                // 列表推导式有特殊作用域
                self.collect_free_variables(&comp.output, param_names, captured)?;
                self.collect_free_variables(&comp.iterable, param_names, captured)?;
                if let Some(cond) = &comp.condition {
                    self.collect_free_variables(cond, param_names, captured)?;
                }
                Ok(())
            }
            Expr::Grouped(inner) => {
                self.collect_free_variables(inner, param_names, captured)
            }
            Expr::Await(await_expr) => {
                // Await 表达式：收集被等待表达式中的自由变量
                self.collect_free_variables(&await_expr.expr, param_names, captured)
            }
            Expr::Literal(_) => Ok(()),
        }
    }

    /**
     * 分析标识符类型（不记录到符号表）
     */
    fn analyze_identifier_type(&self, name: &str) -> Result<Type, Vec<TypeError>> {
        // 首先查找变量符号
        if let Some(symbol) = self.lookup_symbol(name) {
            return Ok(symbol.symbol_type.clone());
        }
        // 检查是否是类型名
        if self.lookup_type(name).is_some() {
            return Ok(Type::Custom(name.to_string()));
        }
        // 返回 Unknown
        Ok(Type::Unknown)
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
                // Any 类型可以与任何数值类型运算
                if left_type == Type::Any || right_type == Type::Any {
                    // 如果有一个是 Any，返回另一个类型（或 Int 作为默认）
                    if left_type == Type::Any && right_type == Type::Any {
                        return Ok(Type::Int);
                    }
                    return Ok(if left_type == Type::Any { right_type } else { left_type });
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
            // 赋值：需要检查左值是否可变
            BinaryOp::Assign => {
                // 检查左值是否是标识符
                if let Expr::Identifier(ident) = binary.left.as_ref() {
                    let symbol_info = self.lookup_symbol(&ident.name).cloned();
                    
                    if symbol_info.is_none() {
                        return Err(vec![TypeError {
                            code: "CCAS-T001".to_string(),
                            message: format!("未定义的变量: {}", ident.name),
                            span: ident.span,
                        }]);
                    }
                    
                    let sym = symbol_info.unwrap();
                    
                    // 检查变量是否可变
                    if !sym.is_mutable {
                        return Err(vec![TypeError {
                            code: "CCAS-T002".to_string(),
                            message: format!("变量 '{}' 是不可变的，不能重新赋值。使用 '定义 可变 {}' 声明可变变量", 
                                ident.name, ident.name),
                            span: ident.span,
                        }]);
                    }
                    
                    // 检查赋值类型兼容性
                    if sym.symbol_type != Type::Unknown && right_type != Type::Unknown {
                        if !self.is_type_compatible(&sym.symbol_type, &right_type) {
                            return Err(vec![TypeError {
                                code: "CCAS-T003".to_string(),
                                message: format!("赋值类型不匹配: 变量 '{}' 类型为 {:?}, 但赋值类型为 {:?}", 
                                    ident.name, sym.symbol_type, right_type),
                                span: binary.right.span(),
                            }]);
                        }
                    }
                }
                
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
                    return Ok(Type::List(Box::new(Type::Int)));
                }
                "整数" => {
                    return Ok(Type::Int);
                }
                "文本" => {
                    return Ok(Type::String);
                }
                "打印" | "print" => {
                    return Ok(Type::Void);
                }
                "列表获取" | "列表长度" | "列表添加" => {
                    return Ok(Type::Any);
                }
                _ => {}
            }

            // 处理带有显式类型参数的泛型调用
            if !call.type_args.is_empty() {
                // 查找函数签名
                if let Some(signature) = self.function_signatures.get(&ident.name).cloned() {
                    if signature.is_generic {
                        // 创建类型参数映射
                        let mut type_subst: std::collections::HashMap<String, Type> = std::collections::HashMap::new();
                        for (i, type_param) in signature.type_params.iter().enumerate() {
                            if i < call.type_args.len() {
                                type_subst.insert(type_param.clone(), call.type_args[i].clone());
                            }
                        }

                        // 替换返回类型中的类型变量
                        let return_type = self.substitute_type_with_map(&signature.return_type, &type_subst);
                        return Ok(return_type);
                    }
                }

                // 没有找到函数签名，使用第一个类型参数作为返回类型
                let return_type = call.type_args[0].clone();
                return Ok(return_type);
            }

            // 先查找函数签名表（用于获取正确的返回类型）
            if let Some(signature) = self.function_signatures.get(&ident.name) {
                return Ok(signature.return_type.clone());
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

        // 处理 MemberAccess 表达式
        if let Expr::MemberAccess(member) = &*call.function {
            // 分析对象表达式
            let object_type = self.analyze_expression(&member.object)?;
            let member_name = &member.member;

            // 处理列表方法
            if let Type::List(_) = object_type {
                if member_name == "获取" {
                    // 列表.获取() 应该返回列表元素类型，暂时返回 Any
                    return Ok(Type::Any);
                } else if member_name == "append" {
                    return Ok(Type::Void);
                } else if member_name == "len" {
                    return Ok(Type::Int);
                }
            }
        }

        // 简化返回 Any，由调用者决定具体类型
        Ok(Type::Any)
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

        // 获取成员名称
        let member_name = &member.member;

        // 检查是否是结构体类型
        match &object_type {
            Type::Struct(struct_name) | Type::Custom(struct_name) => {
                // 查找结构体定义
                if let Some(struct_def) = self.struct_definitions.get(struct_name) {
                    // 查找字段类型
                    for field in &struct_def.fields {
                        if &field.name == member_name {
                            return Ok(field.field_type.clone());
                        }
                    }
                    // 字段未找到，返回 Any
                    return Ok(Type::Any);
                }
                // 结构体定义未找到，返回 Any
                Ok(Type::Any)
            }
            _ => {
                // 非结构体类型，返回 Any
                Ok(Type::Any)
            }
        }
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
        // Any 类型可以接受或转换为任何类型（支持异构列表）
        // 这个规则必须在最前面，确保优先匹配
        if matches!(target, Type::Any) {
            return true;
        }
        if matches!(self, Type::Any) {
            return true;
        }

        match (self, target) {
            (Type::Int, Type::Long) => true,
            (Type::Int, Type::Float) => true,
            (Type::Int, Type::Double) => true,
            (Type::Long, Type::Float) => true,
            (Type::Long, Type::Double) => true,
            (Type::Float, Type::Double) => true,
            // Unknown 类型可以接受任何类型
            (_, Type::Unknown) => true,
            (Type::Unknown, _) => true,
            // 函数类型兼容：参数和返回类型数量相同即可
            (Type::Function(params_s, ret_s), Type::Function(params_t, ret_t)) => {
                // 如果目标参数类型包含 Unknown 或为空，则接受任意参数
                // 如果目标返回类型是 Unknown，则接受任何返回类型
                let params_ok = if params_t.is_empty() || params_t.iter().any(|t| *t == Type::Unknown) {
                    true
                } else if params_s.len() == params_t.len() {
                    params_s.iter().zip(params_t.iter()).all(|(s, t)| {
                        if *t == Type::Unknown {
                            true
                        } else {
                            s.can_cast_to(t)
                        }
                    })
                } else {
                    false
                };
                let ret_ok = if **ret_t == Type::Unknown {
                    true
                } else {
                    ret_s.can_cast_to(ret_t)
                };
                params_ok && ret_ok
            }
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
