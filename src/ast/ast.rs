/**
 * @file ast.rs
 * @brief CCAS 抽象语法树 (AST) 节点定义
 * @description 定义支持中文标识符的基础 AST 节点结构
 */

use crate::lexer::token::{Token, TokenType, Span};
use std::cell::RefCell;
use std::rc::Rc;

/**
 * AST 节点基 trait
 */
pub trait ASTNode {
    /**
     * 获取节点的位置信息
     */
    fn span(&self) -> Span;
}

/**
 * 表达式节点
 */
#[derive(Debug, Clone)]
pub enum Expr {
    /**
     * 标识符表达式
     * 例如: 用户年龄, 计算总和
     */
    Identifier(IdentifierExpr),
    
    /**
     * 字面量表达式
     */
    Literal(LiteralExpr),
    
    /**
     * 二元运算表达式
     */
    Binary(BinaryExpr),
    
    /**
     * 一元运算表达式
     */
    Unary(UnaryExpr),
    
    /**
     * 函数调用表达式
     */
    Call(CallExpr),
    
    /**
     * 成员访问表达式
     */
    MemberAccess(MemberAccessExpr),
    
    /**
     * 列表字面量表达式
     * 例如: [1, 2, 3], ["a", "b", "c"]
     */
    ListLiteral(ListLiteralExpr),
    
    /**
     * 索引访问表达式
     * 例如: 列表[0], 数组[索引]
     */
    IndexAccess(IndexAccessExpr),

    /**
     * 列表推导式
     * 例如: [x * 2 for x in 列表]
     */
    ListComprehension(ListComprehensionExpr),

    /**
     * Lambda 表达式（匿名函数）
     * 例如: 函数(x, y) => x + y
     * 或: 函数(参数: 整数) => 参数 * 2
     */
    Lambda(LambdaExpr),

    /**
     * Await 表达式
     * 用于等待异步操作完成
     * 例如: 等待 异步函数()
     * 或: 等待 future
     */
    Await(AwaitExpr),

    /**
     * 括号表达式
     */
    Grouped(Box<Expr>),
}

impl ASTNode for Expr {
    fn span(&self) -> Span {
        match self {
            Expr::Identifier(e) => e.span(),
            Expr::Literal(e) => e.span(),
            Expr::Binary(e) => e.span(),
            Expr::Unary(e) => e.span(),
            Expr::Call(e) => e.span(),
            Expr::MemberAccess(e) => e.span(),
            Expr::ListLiteral(e) => e.span(),
            Expr::IndexAccess(e) => e.span(),
            Expr::ListComprehension(e) => e.span(),
            Expr::Lambda(e) => e.span(),
            Expr::Await(e) => e.span(),
            Expr::Grouped(e) => e.span(),
        }
    }
}

/**
 * 标识符表达式
 */
#[derive(Debug, Clone)]
pub struct IdentifierExpr {
    pub name: String,
    pub span: Span,
}

impl IdentifierExpr {
    pub fn new(name: String, span: Span) -> Self {
        Self { name, span }
    }
}

impl ASTNode for IdentifierExpr {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * 字面量表达式类型
 */
#[derive(Debug, Clone)]
pub enum LiteralKind {
    /// 整数: 123, 0xFF
    Integer(i64),
    /// 浮点数: 3.14
    Float(f64),
    /// 文本: "你好"
    String(String),
    /// 字符: 'A'
    Char(char),
    /// 布尔: 真, 假
    Boolean(bool),
}

/**
 * 字面量表达式
 */
#[derive(Debug, Clone)]
pub struct LiteralExpr {
    pub kind: LiteralKind,
    pub span: Span,
}

impl LiteralExpr {
    pub fn new(kind: LiteralKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl ASTNode for LiteralExpr {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * 二元运算符
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    /// 赋值运算符
    Assign,     // 赋值 (=)
    
    /// 算术运算符
    Add,       // 加 (+)
    Sub,       // 减 (-)
    Mul,       // 乘 (*)
    Div,       // 除 (/)
    Rem,       // 取余 (%)
    
    /// 比较运算符
    Eq,        // 等于 (==)
    Ne,        // 不等于 (!=)
    Gt,        // 大于 (>)
    Lt,        // 小于 (<)
    Ge,        // 大于等于 (>=)
    Le,        // 小于等于 (<=)
    
    /// 逻辑运算符
    And,       // 与 (&&)
    Or,        // 或 (||)
    
    /// 位运算符
    BitAnd,    // 位与 (&)
    BitOr,     // 位或 (|)
    BitXor,    // 位异或 (^)
    Shl,       // 左移 (<<)
    Shr,       // 右移 (>>)
    Hash,      // 哈希运算 (#)
}

/**
 * 二元运算表达式
 */
#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub op: BinaryOp,
    pub left: Box<Expr>,
    pub right: Box<Expr>,
    pub span: Span,
}

impl BinaryExpr {
    pub fn new(op: BinaryOp, left: Box<Expr>, right: Box<Expr>, span: Span) -> Self {
        Self { op, left, right, span }
    }
}

impl ASTNode for BinaryExpr {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * 一元运算符
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// 负号 (-)
    Neg,
    /// 逻辑非 (!)
    Not,
    /// 位非 (~)
    BitNot,
}

/**
 * 一元运算表达式
 */
#[derive(Debug, Clone)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub operand: Box<Expr>,
    pub span: Span,
}

impl UnaryExpr {
    pub fn new(op: UnaryOp, operand: Box<Expr>, span: Span) -> Self {
        Self { op, operand, span }
    }
}

impl ASTNode for UnaryExpr {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * 函数调用表达式
 */
#[derive(Debug, Clone)]
pub struct CallExpr {
    pub function: Box<Expr>,
    pub arguments: Vec<Expr>,
    pub return_type: Option<Type>,  // 函数返回类型
    pub type_args: Vec<Type>,      // 泛型类型参数
    pub span: Span,
}

impl CallExpr {
    pub fn new(function: Box<Expr>, arguments: Vec<Expr>, span: Span) -> Self {
        Self {
            function,
            arguments,
            return_type: None,
            type_args: Vec::new(),
            span,
        }
    }

    pub fn new_with_type_args(function: Box<Expr>, arguments: Vec<Expr>, type_args: Vec<Type>, span: Span) -> Self {
        Self {
            function,
            arguments,
            return_type: None,
            type_args,
            span,
        }
    }
}

impl ASTNode for CallExpr {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * 成员访问表达式
 */
#[derive(Debug, Clone)]
pub struct MemberAccessExpr {
    pub object: Box<Expr>,
    pub member: String,
    pub span: Span,
    pub member_type: Rc<RefCell<Option<Type>>>,
}

impl MemberAccessExpr {
    pub fn new(object: Box<Expr>, member: String, span: Span) -> Self {
        Self { object, member, span, member_type: Rc::new(RefCell::new(None)) }
    }

    pub fn with_type(object: Box<Expr>, member: String, span: Span, member_type: Type) -> Self {
        Self { object, member, span, member_type: Rc::new(RefCell::new(Some(member_type))) }
    }

    pub fn get_member_type(&self) -> Option<Type> {
        self.member_type.borrow().clone()
    }

    pub fn set_member_type(&self, t: Type) {
        *self.member_type.borrow_mut() = Some(t);
    }
}

impl ASTNode for MemberAccessExpr {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * 列表字面量表达式
 * 例如: [1, 2, 3], ["a", "b", "c"]
 */
#[derive(Debug, Clone)]
pub struct ListLiteralExpr {
    pub elements: Vec<Expr>,
    pub span: Span,
}

impl ListLiteralExpr {
    pub fn new(elements: Vec<Expr>, span: Span) -> Self {
        Self { elements, span }
    }
}

impl ASTNode for ListLiteralExpr {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * 索引访问表达式
 * 例如: 列表[0], 数组[索引]
 */
#[derive(Debug, Clone)]
pub struct IndexAccessExpr {
    pub object: Box<Expr>,
    pub index: Box<Expr>,
    pub span: Span,
}

impl IndexAccessExpr {
    pub fn new(object: Box<Expr>, index: Box<Expr>, span: Span) -> Self {
        Self { object, index, span }
    }
}

impl ASTNode for IndexAccessExpr {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * 列表推导式
 * 例如: [x * 2 for x in 列表]
 */
#[derive(Debug, Clone)]
pub struct ListComprehensionExpr {
    /// 输出表达式
    pub output: Box<Expr>,
    /// 迭代变量名
    pub var_name: String,
    /// 迭代的列表
    pub iterable: Box<Expr>,
    /// 可选的条件过滤
    pub condition: Option<Box<Expr>>,
    pub span: Span,
}

impl ListComprehensionExpr {
    pub fn new(
        output: Box<Expr>,
        var_name: String,
        iterable: Box<Expr>,
        condition: Option<Box<Expr>>,
        span: Span,
    ) -> Self {
        Self { output, var_name, iterable, condition, span }
    }
}

impl ASTNode for ListComprehensionExpr {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * Lambda 表达式（匿名函数）
 * 例如: 函数(x, y) => x + y
 */
#[derive(Debug, Clone)]
pub struct LambdaExpr {
    pub params: Vec<FunctionParam>,          // 参数列表
    pub body: Box<Expr>,                     // 函数体表达式
    pub return_type: Option<Type>,            // 返回类型（可推断）
    pub captured_vars: Vec<CapturedVar>,      // 捕获的外部变量
    pub span: Span,
}

/**
 * 捕获的变量信息
 */
#[derive(Debug, Clone)]
pub struct CapturedVar {
    pub name: String,        // 变量名
    pub var_type: Type,      // 变量类型
}

impl LambdaExpr {
    pub fn new(params: Vec<FunctionParam>, body: Box<Expr>, span: Span) -> Self {
        Self {
            params,
            body,
            return_type: None,
            captured_vars: Vec::new(),
            span,
        }
    }

    pub fn new_with_return_type(params: Vec<FunctionParam>, body: Box<Expr>, return_type: Option<Type>, span: Span) -> Self {
        Self {
            params,
            body,
            return_type,
            captured_vars: Vec::new(),
            span,
        }
    }
}

impl ASTNode for LambdaExpr {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * Await 表达式
 * 用于等待异步操作完成并获取结果
 * 
 * 语法:
 * 等待 表达式
 * 
 * 示例:
 * 等待 异步函数()
 * 等待 future
 * 等待 获取数据()
 */
#[derive(Debug, Clone)]
pub struct AwaitExpr {
    /// 要等待的表达式 (通常是异步函数调用或 Future)
    pub expr: Box<Expr>,
    /// span 信息
    pub span: Span,
}

impl AwaitExpr {
    /**
     * 创建新的 Await 表达式
     */
    pub fn new(expr: Expr, span: Span) -> Self {
        Self {
            expr: Box::new(expr),
            span,
        }
    }

    /**
     * 获取被等待表达式的类型
     * 如果表达式类型是 Future<T>，返回 T
     */
    pub fn inner_type(&self) -> Type {
        match self.expr.as_ref() {
            Expr::Call(call) => call.return_type.clone().unwrap_or(Type::Unknown),
            _ => Type::Unknown,
        }
    }
}

impl ASTNode for AwaitExpr {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * Future 类型
 * 表示异步操作的最终结果
 */
#[derive(Debug, Clone)]
pub struct FutureType {
    /// Future 包装的内部类型
    pub inner_type: Type,
    /// 是否已完成
    pub is_completed: bool,
    /// 结果值 (如果已完成)
    pub result: Option<Box<Expr>>,
}

impl FutureType {
    pub fn new(inner_type: Type) -> Self {
        Self {
            inner_type,
            is_completed: false,
            result: None,
        }
    }

    pub fn completed(result: Expr) -> Self {
        Self {
            inner_type: Type::Unknown,
            is_completed: true,
            result: Some(Box::new(result)),
        }
    }
}

/**
 * 异步运行时上下文
 * 用于跟踪当前是否在异步函数中
 */
#[derive(Debug, Clone)]
pub struct AsyncContext {
    /// 是否在异步函数中
    pub in_async_fn: bool,
    /// 当前挂起的 await 数量
    pub pending_awaits: usize,
    /// 当前函数名
    pub current_function: Option<String>,
}

impl AsyncContext {
    pub fn new() -> Self {
        Self {
            in_async_fn: false,
            pending_awaits: 0,
            current_function: None,
        }
    }

    pub fn enter_async_fn(&mut self, fn_name: String) {
        self.in_async_fn = true;
        self.current_function = Some(fn_name);
    }

    pub fn exit_async_fn(&mut self) {
        self.in_async_fn = false;
        self.current_function = None;
    }

    pub fn increment_awaits(&mut self) {
        self.pending_awaits += 1;
    }

    pub fn decrement_awaits(&mut self) {
        if self.pending_awaits > 0 {
            self.pending_awaits -= 1;
        }
    }
}

/**
 * 语句节点
 */
#[derive(Debug, Clone)]
pub enum Stmt {
    /**
     * 表达式语句
     */
    Expr(ExprStmt),
    
    /**
     * 变量声明语句
     * 例如: 定义 用户名 = "张三"
     */
    Let(LetStmt),
    
    /**
     * 赋值语句
     * 例如: 用户名 = "李四"
     */
    Assignment(AssignmentStmt),
    
    /**
     * 返回语句
     * 例如: 返回 结果
     */
    Return(ReturnStmt),
    
    /**
     * 条件语句
     * 例如: 若 分数 大于 60 则 { 打印("及格") } 否则 { 打印("不及格") }
     */
    If(IfStmt),
    
    /**
     * 循环语句
     * 例如: 循环 { ... } 或 当 条件 { ... }
     */
    Loop(LoopStmt),
    
    /**
     * 中断语句
     * 例如: 中断 或 继续
     */
    Break(BreakStmt),
    
    /**
     * 继续语句
     */
    Continue(ContinueStmt),
    
    /**
     * 块语句
     */
    Block(BlockStmt),
    
    /**
     * 结构体定义语句
     * 例如: 结构体 用户 { 姓名: 文本, 年龄: 整数 }
     */
    StructDef(StructDefinition),
    
    /**
     * 枚举定义语句
     * 例如: 枚举 颜色 { 红, 绿, 蓝 }
     */
    EnumDef(EnumDefinition),
    
    /**
     * 类型别名语句
     * 例如: 类型 整数别名 = 整数
     */
    TypeAlias(TypeAlias),
    
    /**
     * 常量定义语句
     * 例如: 常量 最高分 = 100
     */
    Constant(ConstantDef),
    
    /**
     * 模式匹配语句
     * 例如: 匹配 值 {
     *     情况 数字(n) => n,
     *     情况 加法(a, b) => a + b,
     *     默认 => 0
     * }
     */
    Match(MatchStmt),
    
    /**
     * 异常处理语句
     * 例如: 尝试 { ... } 捕获 (e: 异常) { ... } 最终 { ... }
     */
    Try(TryStmt),
    
    /**
     * 抛出异常语句
     * 例如: 抛出 异常("错误信息")
     */
    Throw(ThrowStmt),
}

impl ASTNode for Stmt {
    fn span(&self) -> Span {
        match self {
            Stmt::Expr(e) => e.span(),
            Stmt::Let(e) => e.span(),
            Stmt::Assignment(e) => e.span(),
            Stmt::Return(e) => e.span(),
            Stmt::If(e) => e.span(),
            Stmt::Loop(e) => e.span(),
            Stmt::Break(e) => e.span(),
            Stmt::Continue(e) => e.span(),
            Stmt::Block(e) => e.span(),
            Stmt::StructDef(e) => e.span.clone(),
            Stmt::EnumDef(e) => e.span.clone(),
            Stmt::TypeAlias(e) => e.span.clone(),
            Stmt::Constant(e) => e.span.clone(),
            Stmt::Match(e) => e.span.clone(),
            Stmt::Try(e) => e.span.clone(),
            Stmt::Throw(e) => e.span.clone(),
        }
    }
}

/**
 * 表达式语句
 */
#[derive(Debug, Clone)]
pub struct ExprStmt {
    pub expr: Expr,
    pub span: Span,
}

impl ExprStmt {
    pub fn new(expr: Expr, span: Span) -> Self {
        Self { expr, span }
    }
}

impl ASTNode for ExprStmt {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * 变量声明语句
 * 例如: 定义 用户名: 文本 = "你好"
 * 例如: 定义 可变 计数: 整数 = 0
 */
#[derive(Debug, Clone)]
pub struct LetStmt {
    pub name: String,
    pub type_annotation: Option<Type>,
    pub initializer: Option<Expr>,
    /// 是否可变变量（使用 可变 关键字修饰）
    pub is_mutable: bool,
    pub span: Span,
}

impl LetStmt {
    pub fn new(name: String, type_annotation: Option<Type>, initializer: Option<Expr>, is_mutable: bool, span: Span) -> Self {
        Self { name, type_annotation, initializer, is_mutable, span }
    }
}

impl ASTNode for LetStmt {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * 赋值语句
 */
#[derive(Debug, Clone)]
pub struct AssignmentStmt {
    pub target: Expr,
    pub value: Expr,
    pub span: Span,
}

impl AssignmentStmt {
    pub fn new(target: Expr, value: Expr, span: Span) -> Self {
        Self { target, value, span }
    }
}

impl ASTNode for AssignmentStmt {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * 返回语句
 */
#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub value: Option<Expr>,
    pub span: Span,
}

impl ReturnStmt {
    pub fn new(value: Option<Expr>, span: Span) -> Self {
        Self { value, span }
    }
}

impl ASTNode for ReturnStmt {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * 条件分支
 */
#[derive(Debug, Clone)]
pub struct Branch {
    pub condition: Expr,
    pub body: Box<Stmt>,
}

/**
 * 条件语句
 */
#[derive(Debug, Clone)]
pub struct IfStmt {
    pub branches: Vec<Branch>,
    pub else_branch: Option<Box<Stmt>>,
    pub span: Span,
}

impl IfStmt {
    pub fn new(branches: Vec<Branch>, else_branch: Option<Box<Stmt>>, span: Span) -> Self {
        Self { branches, else_branch, span }
    }
}

impl ASTNode for IfStmt {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * 循环类型
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopKind {
    /// 无限循环: 循环 { ... }
    Infinite,
    /// 条件循环: 当 条件 { ... }
    While,
    /// 计数循环: 计数循环 (i 从 0 到 10) { ... }
    Counted,
    /// 遍历循环: 遍历 项 取自 集合 { ... }
    For,
}

/**
 * 循环语句
 */
#[derive(Debug, Clone)]
pub struct LoopStmt {
    pub kind: LoopKind,
    pub condition: Option<Expr>,
    pub counter: Option<CounterInit>,
    pub iterator: Option<Expr>,
    pub body: Box<Stmt>,
    pub span: Span,
}

impl LoopStmt {
    pub fn new(kind: LoopKind, condition: Option<Expr>, counter: Option<CounterInit>,
               iterator: Option<Expr>, body: Box<Stmt>, span: Span) -> Self {
        Self { kind, condition, counter, iterator, body, span }
    }
}

impl ASTNode for LoopStmt {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * 计数循环初始化
 */
#[derive(Debug, Clone)]
pub struct CounterInit {
    pub variable: String,
    pub start: Expr,
    pub end: Expr,
    pub step: Option<Expr>,
}

/**
 * 中断语句
 */
#[derive(Debug, Clone)]
pub struct BreakStmt {
    pub label: Option<String>,
    pub span: Span,
}

impl BreakStmt {
    pub fn new(label: Option<String>, span: Span) -> Self {
        Self { label, span }
    }
}

impl ASTNode for BreakStmt {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * 继续语句
 */
#[derive(Debug, Clone)]
pub struct ContinueStmt {
    pub label: Option<String>,
    pub span: Span,
}

impl ContinueStmt {
    pub fn new(label: Option<String>, span: Span) -> Self {
        Self { label, span }
    }
}

impl ASTNode for ContinueStmt {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * 块语句
 */
#[derive(Debug, Clone)]
pub struct BlockStmt {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

impl BlockStmt {
    pub fn new(statements: Vec<Stmt>, span: Span) -> Self {
        Self { statements, span }
    }
}

impl ASTNode for BlockStmt {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * 类型定义
 */
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// 整数
    Int,
    /// 长整数
    Long,
    /// 浮点数
    Float,
    /// 双精度
    Double,
    /// 布尔
    Bool,
    /// 文本 (字符串)
    String,
    /// 字符
    Char,
    /// 无返回
    Void,
    /// 指针 (用于FFI和列表)
    Pointer,
    /// 列表类型 (泛型: List<T>)
    List(Box<Type>),
    /// 或许类型
    Optional(Box<Type>),
    /// 数组
    Array(Box<Type>),
    /// 自定义类型
    Custom(String),
    /// 结构体类型 (命名)
    Struct(String),
    /// 未知类型（用于前向引用）
    Unknown,
    /// 类型变量（泛型参数）
    /// 例如: 函数 foo<T>(x: T) => x
    /// T 就是 TypeVar("T")
    TypeVar(String),
    /// 函数类型
    /// 例如: 函数(整数) => 整数
    Function(Vec<Type>, Box<Type>),
    /// Future 类型 (异步操作的结果)
    /// 例如: Future<整数> 表示返回整数的异步操作
    Future(Box<Type>),
    /// 任意类型 (用于异构列表)
    /// 可以存储任何类型的值，包括自定义结构体
    Any,
}

/**
 * 结构体字段定义
 */
#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub field_type: Type,
}

/**
 * 结构体类型定义
 */
#[derive(Debug, Clone)]
pub struct StructDefinition {
    pub name: String,
    pub fields: Vec<StructField>,
    pub span: Span,
}

/**
 * 枚举变体定义
 * 支持多字段变体：枚举 表达式 { 数字(整数), 加法(左: 节点, 右: 节点) }
 */
#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<EnumVariantField>,
}

/**
 * 枚举变体字段
 */
#[derive(Debug, Clone)]
pub struct EnumVariantField {
    pub name: Option<String>,  // 命名字段：左: 节点，或 None 表示位置参数
    pub field_type: Type,
}

/**
 * 枚举类型定义
 */
#[derive(Debug, Clone)]
pub struct EnumDefinition {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

/**
 * 类型别名定义
 */
#[derive(Debug, Clone)]
pub struct TypeAlias {
    pub name: String,
    pub aliased_type: Type,
    pub span: Span,
}

/**
 * 函数参数
 */
#[derive(Debug, Clone)]
pub struct FunctionParam {
    pub name: String,
    pub param_type: Type,
}

/**
 * 类型参数定义
 * 例如: 函数 foo<T, U>(...) 中的 T 和 U
 */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeParam {
    /// 类型参数名称
    pub name: String,
    /// 上界约束（可选），默认为 Any
    pub bound: Option<Type>,
}

impl TypeParam {
    pub fn new(name: String) -> Self {
        Self { name, bound: None }
    }

    pub fn with_bound(name: String, bound: Type) -> Self {
        Self { name: name, bound: Some(bound) }
    }
}

/**
 * 类型约束
 */
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeConstraint {
    /// 可加法约束 T: Add
    Add,
    /// 可比较约束 T: Comparable
    Comparable,
    /// 可复制约束 T: Copy
    Copy,
    /// 可显示约束 T: Display
    Display,
}

/**
 * 函数定义
 */
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    /// 泛型类型参数列表，例如 <T, U>
    pub type_params: Vec<TypeParam>,
    pub params: Vec<FunctionParam>,
    pub return_type: Type,
    pub body: BlockStmt,
    pub span: Span,
    /// 是否是异步函数
    pub is_async: bool,
    /// 是否是外部函数（仅声明，不定义）
    pub is_external: bool,
}

impl Function {
    pub fn new(name: String, params: Vec<FunctionParam>, return_type: Type,
               body: BlockStmt, span: Span) -> Self {
        Self {
            name,
            type_params: Vec::new(),
            params,
            return_type,
            body,
            span,
            is_async: false,
            is_external: false,
        }
    }

    pub fn with_type_params(name: String, type_params: Vec<TypeParam>,
                           params: Vec<FunctionParam>, return_type: Type,
                           body: BlockStmt, span: Span) -> Self {
        Self {
            name,
            type_params,
            params,
            return_type,
            body,
            span,
            is_async: false,
            is_external: false,
        }
    }

    /**
     * 创建异步函数
     */
    pub fn async_fn(name: String, params: Vec<FunctionParam>, return_type: Type,
                    body: BlockStmt, span: Span) -> Self {
        Self {
            name,
            type_params: Vec::new(),
            params,
            return_type,
            body,
            span,
            is_async: true,
            is_external: false,
        }
    }

    /**
     * 创建外部函数（仅声明，不定义）
     */
    pub fn external(name: String, params: Vec<FunctionParam>, return_type: Type, span: Span) -> Self {
        Self {
            name,
            type_params: Vec::new(),
            params,
            return_type,
            body: BlockStmt::new(Vec::new(), span),
            span,
            is_async: false,
            is_external: true,
        }
    }

    /**
     * 检查函数是否是泛型函数
     */
    pub fn is_generic(&self) -> bool {
        !self.type_params.is_empty()
    }

    /**
     * 检查函数是否是异步函数
     */
    pub fn is_async_fn(&self) -> bool {
        self.is_async
    }

    /**
     * 检查给定的类型是否是类型变量
     */
    pub fn is_type_var(&self, type_name: &str) -> bool {
        self.type_params.iter().any(|tp| tp.name == type_name)
    }
}

impl ASTNode for Function {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * 导入声明
 */
#[derive(Debug, Clone)]
pub struct ImportStmt {
    pub module_path: String,
    pub imported_items: Vec<String>,
    pub span: Span,
}

/**
 * 模块 (顶层程序单元)
 */
#[derive(Debug, Clone)]
pub struct Module {
    pub imports: Vec<ImportStmt>,
    pub functions: Vec<Function>,
    pub structs: Vec<StructDefinition>,
    pub enums: Vec<EnumDefinition>,
    pub type_aliases: Vec<TypeAlias>,
    pub constants: Vec<ConstantDef>,
    pub variables: Vec<LetStmt>,
    pub extern_functions: Vec<ExternFunction>,
    pub macros: Vec<MacroDef>,
    pub span: Span,
}

impl Module {
    pub fn new(functions: Vec<Function>, span: Span) -> Self {
        Self {
            imports: Vec::new(),
            functions,
            structs: Vec::new(),
            enums: Vec::new(),
            type_aliases: Vec::new(),
            constants: Vec::new(),
            variables: Vec::new(),
            extern_functions: Vec::new(),
            macros: Vec::new(),
            span
        }
    }
}

/**
 * 宏定义 (AST 节点)
 */
#[derive(Debug, Clone)]
pub struct MacroDef {
    /// 宏名称
    pub name: String,
    /// 宏参数
    pub params: Vec<String>,
    /// 宏体 Token 列表
    pub body: Vec<Token>,
    pub span: Span,
}

/**
 * 常量定义
 * 例如: 常量 最高分 = 100
 */
#[derive(Debug, Clone)]
pub struct ConstantDef {
    pub name: String,
    pub const_type: Type,
    pub value: Expr,
    pub span: Span,
}

/**
 * 外部函数声明 (FFI)
 * 例如: 外部 函数 malloc(大小: 整数) -> 指针 ["malloc"]
 */
#[derive(Debug, Clone)]
pub struct ExternFunction {
    pub name: String,
    pub params: Vec<FunctionParam>,
    pub return_type: Type,
    pub link_name: Option<String>,  // 可选的链接名，如 "malloc"
    pub span: Span,
}

/**
 * 模式匹配分支
 */
#[derive(Debug, Clone)]
pub struct MatchArm {
    /// 匹配模式
    pub pattern: MatchPattern,
    /// 分支执行体
    pub body: Box<Stmt>,
}

/**
 * 匹配模式
 */
#[derive(Debug, Clone)]
pub enum MatchPattern {
    /// 枚举变体模式: 情况 数字(n) => ...
    EnumVariant {
        enum_name: String,
        variant_name: String,
        /// 捕获的字段变量
        fields: Vec<MatchFieldBinding>,
    },
    /// 通配符模式（默认）: 默认 => ...
    Wildcard,
}

/**
 * 模式中的字段绑定
 */
#[derive(Debug, Clone)]
pub struct MatchFieldBinding {
    pub name: Option<String>,  // 命名字段或位置参数
    pub binding_name: String,  // 绑定到的变量名
}

/**
 * 模式匹配语句
 */
#[derive(Debug, Clone)]
pub struct MatchStmt {
    /// 要匹配的值
    pub subject: Expr,
    /// 匹配分支
    pub arms: Vec<MatchArm>,
    /// span 信息
    pub span: Span,
}

impl ASTNode for Module {
    fn span(&self) -> Span {
        self.span
    }
}

/**
 * 将 TokenType 转换为 BinaryOp
 */
pub fn token_to_binary_op(token: &TokenType) -> Option<BinaryOp> {
    match token {
        TokenType::加 => Some(BinaryOp::Add),
        TokenType::减 => Some(BinaryOp::Sub),
        TokenType::乘 => Some(BinaryOp::Mul),
        TokenType::除 => Some(BinaryOp::Div),
        TokenType::取余 => Some(BinaryOp::Rem),
        TokenType::等于 => Some(BinaryOp::Eq),
        TokenType::不等于 => Some(BinaryOp::Ne),
        TokenType::大于 => Some(BinaryOp::Gt),
        TokenType::小于 => Some(BinaryOp::Lt),
        TokenType::大于等于 => Some(BinaryOp::Ge),
        TokenType::小于等于 => Some(BinaryOp::Le),
        TokenType::与 => Some(BinaryOp::And),
        TokenType::或 => Some(BinaryOp::Or),
        _ => None,
    }
}

/**
 * 将 TokenType 转换为 UnaryOp
 */
pub fn token_to_unary_op(token: &TokenType) -> Option<UnaryOp> {
    match token {
        TokenType::减 => Some(UnaryOp::Neg),
        TokenType::非 => Some(UnaryOp::Not),
        TokenType::位非 => Some(UnaryOp::BitNot),
        _ => None,
    }
}

// ============================================================
// 异常处理相关定义
// ============================================================

/**
 * 异常类型
 */
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExceptionType {
    /// 通用异常
    Exception,
    /// 运行时异常
    RuntimeError,
    /// 类型错误
    TypeError,
    /// 空值异常
    NullPointer,
    /// 索引越界
    IndexOutOfBounds,
    /// 除零错误
    DivideByZero,
    /// 文件错误
    FileError,
    /// 网络错误
    NetworkError,
    /// 自定义异常类型
    Custom(String),
}

impl std::fmt::Display for ExceptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExceptionType::Exception => write!(f, "异常"),
            ExceptionType::RuntimeError => write!(f, "运行时错误"),
            ExceptionType::TypeError => write!(f, "类型错误"),
            ExceptionType::NullPointer => write!(f, "空指针异常"),
            ExceptionType::IndexOutOfBounds => write!(f, "索引越界"),
            ExceptionType::DivideByZero => write!(f, "除零错误"),
            ExceptionType::FileError => write!(f, "文件错误"),
            ExceptionType::NetworkError => write!(f, "网络错误"),
            ExceptionType::Custom(name) => write!(f, "{}", name),
        }
    }
}

/**
 * Catch 子句
 * 例如: 捕获 (e: 异常) { 处理代码 }
 */
#[derive(Debug, Clone)]
pub struct CatchClause {
    /// 捕获的异常变量名
    pub var_name: String,
    /// 异常类型 (可选，不指定则捕获所有异常)
    pub exception_type: Option<ExceptionType>,
    /// 异常处理代码块
    pub body: BlockStmt,
    /// span 信息
    pub span: Span,
}

impl CatchClause {
    pub fn new(var_name: String, exception_type: Option<ExceptionType>, body: BlockStmt, span: Span) -> Self {
        Self { var_name, exception_type, body, span }
    }

    /**
     * 检查是否捕获所有异常
     */
    pub fn catches_all(&self) -> bool {
        self.exception_type.is_none()
    }
}

/**
 * Try 语句 (异常处理)
 * 
 * 语法:
 * ```
 * 尝试 {
 *     // 可能抛出异常的代码
 * } 捕获 (e: 异常) {
 *     // 异常处理代码
 * } 最终 {
 *     // 无论是否发生异常都会执行的代码
 * }
 * ```
 * 
 * 示例:
 * ```
 * 尝试 {
 *     定义 结果 = 除法(10, 0)
 *     打印(结果)
 * } 捕获 (e: 除零错误) {
 *     打印("除零错误: " + e.信息)
 * } 最终 {
 *     打印("清理资源")
 * }
 * ```
 */
#[derive(Debug, Clone)]
pub struct TryStmt {
    /// try 代码块
    pub try_block: BlockStmt,
    /// catch 子句列表 (可以有多个，按顺序匹配)
    pub catch_clauses: Vec<CatchClause>,
    /// finally 代码块 (可选)
    pub finally_block: Option<BlockStmt>,
    /// span 信息
    pub span: Span,
}

impl TryStmt {
    pub fn new(
        try_block: BlockStmt,
        catch_clauses: Vec<CatchClause>,
        finally_block: Option<BlockStmt>,
        span: Span,
    ) -> Self {
        Self { try_block, catch_clauses, finally_block, span }
    }

    /**
     * 检查是否有 catch 子句
     */
    pub fn has_catch(&self) -> bool {
        !self.catch_clauses.is_empty()
    }

    /**
     * 检查是否有 finally 子句
     */
    pub fn has_finally(&self) -> bool {
        self.finally_block.is_some()
    }

    /**
     * 获取匹配指定异常类型的 catch 子句
     */
    pub fn get_matching_catch(&self, exception_type: &ExceptionType) -> Option<&CatchClause> {
        // 首先查找精确匹配
        for clause in &self.catch_clauses {
            if let Some(ref et) = clause.exception_type {
                if et == exception_type {
                    return Some(clause);
                }
            }
        }
        // 然后查找捕获所有的子句
        for clause in &self.catch_clauses {
            if clause.catches_all() {
                return Some(clause);
            }
        }
        None
    }
}

/**
 * Throw 语句 (抛出异常)
 * 
 * 语法:
 * ```
 * 抛出 异常类型("错误信息")
 * 抛出 变量名
 * ```
 * 
 * 示例:
 * ```
 * 若 年龄 < 0 {
 *     抛出 异常("年龄不能为负数")
 * }
 * 
 * 若 文件.不存在() {
 *     抛出 文件错误("文件不存在: " + 路径)
 * }
 * ```
 */
#[derive(Debug, Clone)]
pub struct ThrowStmt {
    /// 要抛出的异常表达式
    /// 可以是异常构造调用，也可以是异常变量
    pub exception: Expr,
    /// span 信息
    pub span: Span,
}

impl ThrowStmt {
    pub fn new(exception: Expr, span: Span) -> Self {
        Self { exception, span }
    }
}

/**
 * 异常信息结构体
 * 用于运行时存储异常详情
 */
#[derive(Debug, Clone)]
pub struct ExceptionInfo {
    /// 异常类型
    pub exception_type: ExceptionType,
    /// 错误信息
    pub message: String,
    /// 堆栈跟踪
    pub stack_trace: Vec<StackFrame>,
    /// 抛出位置
    pub source_location: Option<SourceLocation>,
}

/**
 * 堆栈帧
 */
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// 函数名
    pub function_name: String,
    /// 文件名
    pub file_name: Option<String>,
    /// 行号
    pub line: usize,
    /// 列号
    pub column: usize,
}

/**
 * 源码位置
 */
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

impl ExceptionInfo {
    pub fn new(exception_type: ExceptionType, message: String) -> Self {
        Self {
            exception_type,
            message,
            stack_trace: Vec::new(),
            source_location: None,
        }
    }

    /**
     * 添加堆栈帧
     */
    pub fn add_stack_frame(&mut self, frame: StackFrame) {
        self.stack_trace.push(frame);
    }

    /**
     * 格式化堆栈跟踪
     */
    pub fn format_stack_trace(&self) -> String {
        let mut result = String::new();
        result.push_str(&format!("{}: {}\n", self.exception_type, self.message));
        
        if let Some(ref loc) = self.source_location {
            result.push_str(&format!("  位于 {}:{}:{}\n", loc.file, loc.line, loc.column));
        }
        
        for frame in &self.stack_trace {
            result.push_str(&format!("  在 {}", frame.function_name));
            if let Some(ref file) = frame.file_name {
                result.push_str(&format!(" ({}:{}:{})", file, frame.line, frame.column));
            }
            result.push('\n');
        }
        
        result
    }
}
