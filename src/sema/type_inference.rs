/**
 * @file type_inference.rs
 * @brief 类型推断引擎
 * @description 实现智能类型推导，包括变量类型推断、函数返回类型推断、泛型参数推断等
 * 
 * 功能特性:
 * - 变量类型推断: 从初始化表达式推断变量类型
 * - 函数返回类型推断: 从函数体推断返回类型
 * - 二元表达式类型推断: 处理运算符重载和类型提升
 * - 泛型参数推断: 从调用上下文推断泛型类型参数
 * - 类型统一: 解决类型约束
 */

use std::collections::HashMap;
use crate::ast::{Type, Expr, Stmt, BinaryOp, LiteralKind, Function};

/**
 * 类型推断引擎
 */
pub struct TypeInferenceEngine {
    /// 类型变量计数器 (用于生成唯一的类型变量名)
    type_var_counter: usize,
    /// 类型约束集合 (类型变量 -> 可能的类型集合)
    type_constraints: HashMap<String, Vec<Type>>,
    /// 已求解的类型变量
    solved_types: HashMap<String, Type>,
}

/**
 * 类型推断结果
 */
#[derive(Debug, Clone)]
pub struct InferenceResult {
    /// 推断出的类型
    pub inferred_type: Type,
    /// 置信度 (0.0 - 1.0)
    pub confidence: f64,
    /// 推断依据
    pub evidence: String,
}

/**
 * 类型统一结果
 */
#[derive(Debug, Clone)]
pub enum UnificationResult {
    /// 统一成功
    Success,
    /// 统一失败，类型不兼容
    Failure(String),
    /// 需要更多约束
    NeedsMoreConstraints,
}

impl TypeInferenceEngine {
    /**
     * 创建新的类型推断引擎
     */
    pub fn new() -> Self {
        Self {
            type_var_counter: 0,
            type_constraints: HashMap::new(),
            solved_types: HashMap::new(),
        }
    }

    /**
     * 生成新的类型变量
     */
    pub fn fresh_type_var(&mut self) -> Type {
        self.type_var_counter += 1;
        Type::TypeVar(format!("$T{}", self.type_var_counter))
    }

    /**
     * 重置引擎状态
     */
    pub fn reset(&mut self) {
        self.type_var_counter = 0;
        self.type_constraints.clear();
        self.solved_types.clear();
    }

    // ============================================================
    // 变量类型推断
    // ============================================================

    /**
     * 从初始化表达式推断变量类型
     * 
     * 示例:
     * - 定义 x = 10          -> 推断 x: 整数
     * - 定义 y = "你好"      -> 推断 y: 文本
     * - 定义 z = [1, 2, 3]   -> 推断 z: 列表<整数>
     * - 定义 f = 函数 => 42  -> 推断 f: 函数() => 整数
     */
    pub fn infer_variable_type(&mut self, initializer: &Expr) -> InferenceResult {
        let inferred_type = self.infer_expression_type(initializer);
        
        InferenceResult {
            confidence: 1.0,
            evidence: format!("从初始化表达式推断"),
            inferred_type,
        }
    }

    /**
     * 推断表达式类型
     */
    pub fn infer_expression_type(&mut self, expr: &Expr) -> Type {
        match expr {
            // 字面量类型推断
            Expr::Literal(lit) => self.infer_literal_type(&lit.kind),
            
            // 标识符类型推断
            Expr::Identifier(ident) => {
                // 如果是类型变量，返回它
                Type::TypeVar(ident.name.clone())
            },
            
            // 二元表达式类型推断
            Expr::Binary(binary) => self.infer_binary_type(&binary.op, &binary.left, &binary.right),
            
            // 一元表达式类型推断
            Expr::Unary(unary) => self.infer_unary_type(&unary.op, &unary.operand),
            
            // 函数调用类型推断
            Expr::Call(call) => {
                // 推断函数类型
                let func_type = self.infer_expression_type(&call.function);
                
                // 如果是函数类型，返回返回类型
                if let Type::Function(_, return_type) = func_type {
                    *return_type
                } else {
                    Type::Unknown
                }
            },
            
            // 成员访问类型推断
            Expr::MemberAccess(member) => {
                // 简化：返回未知类型
                // 完整实现需要查找结构体定义
                let _obj_type = self.infer_expression_type(&member.object);
                Type::Unknown
            },
            
            // 列表字面量类型推断
            Expr::ListLiteral(list) => {
                if list.elements.is_empty() {
                    // 空列表，元素类型未知
                    Type::List(Box::new(Type::Unknown))
                } else {
                    // 推断第一个元素的类型
                    let elem_type = self.infer_expression_type(&list.elements[0]);
                    
                    // 检查所有元素类型是否一致
                    let mut all_same = true;
                    for elem in &list.elements[1..] {
                        let t = self.infer_expression_type(elem);
                        if !self.types_equal(&elem_type, &t) {
                            all_same = false;
                            break;
                        }
                    }
                    
                    if all_same {
                        Type::List(Box::new(elem_type))
                    } else {
                        // 混合类型列表
                        Type::List(Box::new(Type::Unknown))
                    }
                }
            },
            
            // 索引访问类型推断
            Expr::IndexAccess(index) => {
                let obj_type = self.infer_expression_type(&index.object);
                
                match obj_type {
                    Type::List(elem_type) => *elem_type,
                    Type::Array(elem_type) => *elem_type,
                    _ => Type::Unknown,
                }
            },
            
            // 列表推导式类型推断
            Expr::ListComprehension(comp) => {
                let output_type = self.infer_expression_type(&comp.output);
                Type::List(Box::new(output_type))
            },
            
            // Lambda 表达式类型推断
            Expr::Lambda(lambda) => {
                // 推断参数类型 (如果未指定)
                let param_types: Vec<Type> = lambda.params.iter()
                    .map(|p| p.param_type.clone())
                    .collect();
                
                // 推断返回类型
                let return_type = self.infer_expression_type(&lambda.body);
                
                Type::Function(param_types, Box::new(return_type))
            },
            
            // 括号表达式
            Expr::Grouped(expr) => self.infer_expression_type(expr),
            
            // Await 表达式
            // 等待 Future<T> 返回 T
            Expr::Await(await_expr) => {
                let inner_type = self.infer_expression_type(&await_expr.expr);
                // 如果内部类型是 Future<T>，返回 T
                match inner_type {
                    Type::Future(t) => *t,
                    _ => inner_type, // 其他情况返回原类型
                }
            },
        }
    }

    /**
     * 推断字面量类型
     */
    fn infer_literal_type(&self, kind: &LiteralKind) -> Type {
        match kind {
            LiteralKind::Integer(_) => Type::Int,
            LiteralKind::Float(_) => Type::Float,
            LiteralKind::String(_) => Type::String,
            LiteralKind::Char(_) => Type::Char,
            LiteralKind::Boolean(_) => Type::Bool,
        }
    }

    /**
     * 推断二元表达式类型
     */
    fn infer_binary_type(&mut self, op: &BinaryOp, left: &Expr, right: &Expr) -> Type {
        let left_type = self.infer_expression_type(left);
        let right_type = self.infer_expression_type(right);
        
        match op {
            // 算术运算符: 返回操作数类型 (或提升后的类型)
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {
                self.infer_arithmetic_result_type(&left_type, &right_type)
            }
            
            // 比较运算符: 返回布尔类型
            BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Gt | BinaryOp::Lt | BinaryOp::Ge | BinaryOp::Le => {
                Type::Bool
            }
            
            // 逻辑运算符: 返回布尔类型
            BinaryOp::And | BinaryOp::Or => {
                Type::Bool
            }
            
            // 位运算符: 返回整数类型
            BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor | BinaryOp::Shl | BinaryOp::Shr => {
                Type::Int
            }
            
            // 赋值运算符: 返回 void
            BinaryOp::Assign => {
                Type::Void
            }
            
            // 哈希运算符
            BinaryOp::Hash => {
                Type::Int
            }
        }
    }

    /**
     * 推断算术运算结果类型
     * 处理类型提升: 整数 + 浮点 -> 浮点
     */
    fn infer_arithmetic_result_type(&self, left: &Type, right: &Type) -> Type {
        match (left, right) {
            // 整数 + 整数 = 整数
            (Type::Int, Type::Int) => Type::Int,
            (Type::Long, Type::Long) => Type::Long,
            
            // 浮点 + 浮点 = 浮点
            (Type::Float, Type::Float) => Type::Float,
            (Type::Double, Type::Double) => Type::Double,
            
            // 类型提升: 整数 + 浮点 = 浮点
            (Type::Int, Type::Float) | (Type::Float, Type::Int) => Type::Float,
            (Type::Int, Type::Double) | (Type::Double, Type::Int) => Type::Double,
            (Type::Long, Type::Float) | (Type::Float, Type::Long) => Type::Float,
            (Type::Long, Type::Double) | (Type::Double, Type::Long) => Type::Double,
            
            // 文本 + 任意 = 文本 (字符串拼接)
            (Type::String, _) | (_, Type::String) => Type::String,
            
            // 未知类型
            _ => Type::Unknown,
        }
    }

    /**
     * 推断一元表达式类型
     */
    fn infer_unary_type(&mut self, _op: &crate::ast::UnaryOp, operand: &Expr) -> Type {
        let operand_type = self.infer_expression_type(operand);
        
        // 一元运算符通常返回操作数类型
        operand_type
    }

    // ============================================================
    // 函数返回类型推断
    // ============================================================

    /**
     * 从函数体推断返回类型
     * 
     * 示例:
     * 函数 加一 => x + 1           -> 推断返回: 整数
     * 函数 问候 => "你好, " + 名字  -> 推断返回: 文本
     */
    pub fn infer_function_return_type(&mut self, func: &Function) -> InferenceResult {
        // 分析函数体中的所有返回语句
        let return_types = self.collect_return_types(&func.body);
        
        if return_types.is_empty() {
            // 没有返回语句，返回 void
            InferenceResult {
                inferred_type: Type::Void,
                confidence: 1.0,
                evidence: "函数体没有返回语句".to_string(),
            }
        } else if return_types.len() == 1 {
            // 只有一个返回类型
            InferenceResult {
                inferred_type: return_types[0].clone(),
                confidence: 1.0,
                evidence: "从唯一返回语句推断".to_string(),
            }
        } else {
            // 多个返回类型，尝试统一
            let unified = self.unify_types(&return_types);
            InferenceResult {
                inferred_type: unified,
                confidence: 0.8,
                evidence: format!("从 {} 个返回语句推断", return_types.len()),
            }
        }
    }

    /**
     * 收集函数体中的所有返回类型
     */
    fn collect_return_types(&mut self, body: &crate::ast::BlockStmt) -> Vec<Type> {
        let mut types = Vec::new();
        self.collect_return_types_from_statements(&body.statements, &mut types);
        types
    }

    /**
     * 从语句列表中收集返回类型
     */
    fn collect_return_types_from_statements(&mut self, statements: &[Stmt], types: &mut Vec<Type>) {
        for stmt in statements {
            self.collect_return_types_from_statement(stmt, types);
        }
    }

    /**
     * 从单个语句中收集返回类型
     */
    fn collect_return_types_from_statement(&mut self, stmt: &Stmt, types: &mut Vec<Type>) {
        match stmt {
            Stmt::Return(return_stmt) => {
                if let Some(ref value) = return_stmt.value {
                    types.push(self.infer_expression_type(value));
                } else {
                    types.push(Type::Void);
                }
            }
            Stmt::If(if_stmt) => {
                for branch in &if_stmt.branches {
                    self.collect_return_types_from_statement(&branch.body, types);
                }
                if let Some(ref else_body) = if_stmt.else_branch {
                    self.collect_return_types_from_statement(else_body, types);
                }
            }
            Stmt::Block(block) => {
                self.collect_return_types_from_statements(&block.statements, types);
            }
            Stmt::Loop(loop_stmt) => {
                self.collect_return_types_from_statement(&loop_stmt.body, types);
            }
            Stmt::Match(match_stmt) => {
                for arm in &match_stmt.arms {
                    self.collect_return_types_from_statement(&arm.body, types);
                }
            }
            Stmt::Try(try_stmt) => {
                self.collect_return_types_from_statements(&try_stmt.try_block.statements, types);
                for catch in &try_stmt.catch_clauses {
                    self.collect_return_types_from_statements(&catch.body.statements, types);
                }
                if let Some(ref finally) = try_stmt.finally_block {
                    self.collect_return_types_from_statements(&finally.statements, types);
                }
            }
            _ => {}
        }
    }

    // ============================================================
    // 泛型参数推断
    // ============================================================

    /**
     * 从函数调用推断泛型类型参数
     * 
     * 示例:
     * 函数 恒等 => x
     * 恒等(42)              -> 推断 T = 整数
     * 恒等("你好")          -> 推断 T = 文本
     * 
     * 函数 第一个(列表: 列表): T
     * 第一个([1, 2, 3])     -> 推断 T = 整数
     */
    pub fn infer_generic_type_args(
        &mut self,
        type_params: &[crate::ast::TypeParam],
        param_types: &[Type],
        arg_types: &[Type],
    ) -> HashMap<String, Type> {
        let mut inferred = HashMap::new();
        
        // 遍历参数和实参
        for (param_type, arg_type) in param_types.iter().zip(arg_types.iter()) {
            self.infer_type_from_argument(param_type, arg_type, &mut inferred);
        }
        
        // 对于未推断的类型变量，使用默认类型
        for type_param in type_params {
            if !inferred.contains_key(&type_param.name) {
                inferred.insert(type_param.name.clone(), Type::Unknown);
            }
        }
        
        inferred
    }

    /**
     * 从单个参数推断类型变量
     */
    fn infer_type_from_argument(
        &self,
        param_type: &Type,
        arg_type: &Type,
        inferred: &mut HashMap<String, Type>,
    ) {
        match (param_type, arg_type) {
            // 类型变量: 直接绑定
            (Type::TypeVar(name), concrete) => {
                if !inferred.contains_key(name) {
                    inferred.insert(name.clone(), concrete.clone());
                } else {
                    // 已有绑定，检查一致性
                    // TODO: 类型统一
                }
            }
            
            // 列表类型: 递归推断元素类型
            (Type::List(param_elem), Type::List(arg_elem)) => {
                self.infer_type_from_argument(param_elem, arg_elem, inferred);
            }
            
            // 可选类型: 递归推断
            (Type::Optional(param_inner), Type::Optional(arg_inner)) => {
                self.infer_type_from_argument(param_inner, arg_inner, inferred);
            }
            
            // 函数类型: 递归推断
            (Type::Function(param_params, param_ret), Type::Function(arg_params, arg_ret)) => {
                for (p, a) in param_params.iter().zip(arg_params.iter()) {
                    self.infer_type_from_argument(p, a, inferred);
                }
                self.infer_type_from_argument(param_ret, arg_ret, inferred);
            }
            
            _ => {}
        }
    }

    // ============================================================
    // 类型统一
    // ============================================================

    /**
     * 统一多个类型
     * 找出所有类型的共同类型
     */
    pub fn unify_types(&self, types: &[Type]) -> Type {
        if types.is_empty() {
            return Type::Unknown;
        }
        
        if types.len() == 1 {
            return types[0].clone();
        }
        
        // 检查所有类型是否相同
        let first = &types[0];
        if types.iter().all(|t| self.types_equal(t, first)) {
            return first.clone();
        }
        
        // 尝试找到最具体的公共类型
        // 例如: 整数 和 浮点 -> 浮点
        let has_float = types.iter().any(|t| matches!(t, Type::Float | Type::Double));
        let has_int = types.iter().any(|t| matches!(t, Type::Int | Type::Long));
        
        if has_float && has_int {
            return Type::Float;
        }
        
        // 无法统一，返回未知
        Type::Unknown
    }

    /**
     * 检查两个类型是否相等
     */
    fn types_equal(&self, a: &Type, b: &Type) -> bool {
        match (a, b) {
            (Type::Int, Type::Int) => true,
            (Type::Long, Type::Long) => true,
            (Type::Float, Type::Float) => true,
            (Type::Double, Type::Double) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::String, Type::String) => true,
            (Type::Char, Type::Char) => true,
            (Type::Void, Type::Void) => true,
            (Type::Unknown, Type::Unknown) => true,
            // Any 与任何类型相等（支持异构列表）
            (Type::Any, _) | (_, Type::Any) => true,
            (Type::List(e1), Type::List(e2)) => self.types_equal(e1, e2),
            (Type::Optional(i1), Type::Optional(i2)) => self.types_equal(i1, i2),
            (Type::Array(e1), Type::Array(e2)) => self.types_equal(e1, e2),
            (Type::TypeVar(n1), Type::TypeVar(n2)) => n1 == n2,
            (Type::Custom(n1), Type::Custom(n2)) => n1 == n2,
            (Type::Function(p1, r1), Type::Function(p2, r2)) => {
                p1.len() == p2.len()
                    && p1.iter().zip(p2.iter()).all(|(a, b)| self.types_equal(a, b))
                    && self.types_equal(r1, r2)
            }
            _ => false,
        }
    }

    // ============================================================
    // 类型约束收集
    // ============================================================

    /**
     * 添加类型约束
     */
    pub fn add_constraint(&mut self, type_var: String, possible_type: Type) {
        self.type_constraints
            .entry(type_var)
            .or_default()
            .push(possible_type);
    }

    /**
     * 求解类型变量
     */
    pub fn solve_type_var(&mut self, type_var: &str) -> Option<Type> {
        if let Some(solved) = self.solved_types.get(type_var) {
            return Some(solved.clone());
        }
        
        if let Some(constraints) = self.type_constraints.get(type_var) {
            if !constraints.is_empty() {
                // 使用第一个约束作为解
                // TODO: 更智能的约束求解
                let solved = constraints[0].clone();
                self.solved_types.insert(type_var.to_string(), solved.clone());
                return Some(solved);
            }
        }
        
        None
    }

    /**
     * 检查类型是否可以隐式转换
     */
    pub fn can_implicit_convert(&self, from: &Type, to: &Type) -> bool {
        match (from, to) {
            // 相同类型
            _ if self.types_equal(from, to) => true,
            
            // 整数提升
            (Type::Int, Type::Long) => true,
            (Type::Int, Type::Float) => true,
            (Type::Int, Type::Double) => true,
            (Type::Long, Type::Float) => true,
            (Type::Long, Type::Double) => true,
            (Type::Float, Type::Double) => true,
            
            // 任意类型可以转换为未知类型
            (_, Type::Unknown) => true,
            
            _ => false,
        }
    }

    /**
     * 获取类型的默认值表达式
     */
    pub fn default_value_for_type(&self, ty: &Type) -> String {
        match ty {
            Type::Int | Type::Long => "0".to_string(),
            Type::Float | Type::Double => "0.0".to_string(),
            Type::Bool => "假".to_string(),
            Type::String => "\"\"".to_string(),
            Type::Char => "'\\0'".to_string(),
            Type::Void => "".to_string(),
            Type::List(_) => "[]".to_string(),
            Type::Optional(_) => "空".to_string(),
            Type::Unknown => "空".to_string(),
            Type::Custom(name) => format!("{}()", name),
            Type::TypeVar(_) => "空".to_string(),
            Type::Array(_) => "[]".to_string(),
            Type::Function(_, _) => "函数() => 空".to_string(),
            Type::Pointer => "空".to_string(),
            Type::Struct(name) => format!("{}()", name),
            Type::Future(inner) => format!("Future<{:?}>", inner),
            Type::Any => "0".to_string(),  // Any 类型默认值为 0（空指针）
        }
    }
}

impl Default for TypeInferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_inference() {
        let mut engine = TypeInferenceEngine::new();
        
        let expr = Expr::Literal(crate::ast::LiteralExpr {
            kind: LiteralKind::Integer(42),
            span: Span::dummy(),
        });
        
        let result = engine.infer_variable_type(&expr);
        assert_eq!(result.inferred_type, Type::Int);
    }

    #[test]
    fn test_list_inference() {
        let mut engine = TypeInferenceEngine::new();
        
        let expr = Expr::ListLiteral(crate::ast::ListLiteralExpr {
            elements: vec![
                Expr::Literal(crate::ast::LiteralExpr {
                    kind: LiteralKind::Integer(1),
                    span: Span::dummy(),
                }),
                Expr::Literal(crate::ast::LiteralExpr {
                    kind: LiteralKind::Integer(2),
                    span: Span::dummy(),
                }),
            ],
            span: Span::dummy(),
        });
        
        let result = engine.infer_variable_type(&expr);
        assert_eq!(result.inferred_type, Type::List(Box::new(Type::Int)));
    }

    #[test]
    fn test_type_promotion() {
        let engine = TypeInferenceEngine::new();
        
        let result = engine.infer_arithmetic_result_type(&Type::Int, &Type::Float);
        assert_eq!(result, Type::Float);
    }

    #[test]
    fn test_type_unification() {
        let engine = TypeInferenceEngine::new();
        
        let types = vec![Type::Int, Type::Int, Type::Int];
        let unified = engine.unify_types(&types);
        assert_eq!(unified, Type::Int);
        
        let types = vec![Type::Int, Type::Float];
        let unified = engine.unify_types(&types);
        assert_eq!(unified, Type::Float);
    }
}
