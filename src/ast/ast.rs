/**
 * @file ast.rs
 * @brief CCAS 抽象语法树 (AST) 节点定义
 * @description 定义支持中文标识符的基础 AST 节点结构
 */

use crate::lexer::token::{TokenType, Span};

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
    pub span: Span,
}

impl CallExpr {
    pub fn new(function: Box<Expr>, arguments: Vec<Expr>, span: Span) -> Self {
        Self { 
            function, 
            arguments, 
            return_type: None,
            span 
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
}

impl MemberAccessExpr {
    pub fn new(object: Box<Expr>, member: String, span: Span) -> Self {
        Self { object, member, span }
    }
}

impl ASTNode for MemberAccessExpr {
    fn span(&self) -> Span {
        self.span
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
 */
#[derive(Debug, Clone)]
pub struct LetStmt {
    pub name: String,
    pub type_annotation: Option<Type>,
    pub initializer: Option<Expr>,
    pub span: Span,
}

impl LetStmt {
    pub fn new(name: String, type_annotation: Option<Type>, initializer: Option<Expr>, span: Span) -> Self {
        Self { name, type_annotation, initializer, span }
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
    /// 列表类型
    List,
    /// 或许类型
    Optional(Box<Type>),
    /// 数组
    Array(Box<Type>),
    /// 自定义类型
    Custom(String),
    /// 结构体类型 (命名)
    Struct(String),
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
 * 函数定义
 */
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<FunctionParam>,
    pub return_type: Type,
    pub body: BlockStmt,
    pub span: Span,
}

impl Function {
    pub fn new(name: String, params: Vec<FunctionParam>, return_type: Type, 
               body: BlockStmt, span: Span) -> Self {
        Self { name, params, return_type, body, span }
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
    pub extern_functions: Vec<ExternFunction>,
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
            extern_functions: Vec::new(),
            span 
        }
    }
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
