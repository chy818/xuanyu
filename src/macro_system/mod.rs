/**
 * @file mod.rs
 * @brief 玄语宏系统
 * @description 实现声明宏和过程宏，支持元编程能力
 *
 * 功能特性:
 * - 声明宏 (宏 ... 展开 ...)
 * - 模式匹配和替换
 * - 卫生宏 (hygienic macros)
 * - 过程宏支持
 */

use std::collections::HashMap;

use crate::lexer::token::{Token, TokenType, Keyword, Span};

/**
 * 宏定义
 */
#[derive(Debug, Clone)]
pub struct MacroDefinition {
    /// 宏名称
    pub name: String,
    /// 宏参数列表
    pub params: Vec<MacroParam>,
    /// 宏体 (替换规则)
    pub body: Vec<MacroRule>,
    /// 卫生标记
    pub hygiene: MacroHygiene,
    /// span 信息
    pub span: Span,
}

/**
 * 宏参数
 */
#[derive(Debug, Clone)]
pub struct MacroParam {
    /// 参数模式
    pub pattern: MacroPattern,
    /// 参数名称
    pub name: String,
    /// 是否为可变参数
    pub is_varargs: bool,
}

/**
 * 宏模式
 */
#[derive(Debug, Clone, PartialEq)]
pub enum MacroPattern {
    /// 表达式
    Expr,
    /// 语句
    Stmt,
    /// 类型
    Type,
    /// 模式
    Pattern,
    /// 标识符
    Ident,
    /// 字符串
    String,
    /// 整数
    Integer,
    /// 浮点数
    Float,
    /// 块
    Block,
    /// 文件
    File,
}

/**
 * 宏卫生级别
 */
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MacroHygiene {
    /// 完全卫生 - 宏生成的代码不会污染外部作用域
    Full,
    /// 半卫生 - 只会污染宏定义的模块作用域
    Module,
    /// 不卫生 - 会污染全局作用域
    None,
}

/**
 * 宏替换规则
 */
#[derive(Debug, Clone)]
pub struct MacroRule {
    /// 匹配模式
    pub matcher: Vec<MatcherToken>,
    /// 替换模板
    pub template: Vec<Token>,
    /// 导出标记
    pub is_export: bool,
}

/**
 * 匹配器标记
 */
#[derive(Debug, Clone)]
pub enum MatcherToken {
    /// 匹配表达式
    MatchExpr(String),
    /// 匹配类型
    MatchType(String),
    /// 匹配语句
    MatchStmt(String),
    /// 匹配标识符
    MatchIdent(String),
    /// 匹配字面量
    MatchLiteral(String),
    /// 匹配重复
    MatchRepeat {
        name: String,
        pattern: Box<MatcherToken>,
        separator: Option<Token>,
        min: Option<usize>,
        max: Option<usize>,
    },
    /// 匹配零个或多个
    ZeroOrMore(Box<MatcherToken>),
    /// 匹配一个或多个
    OneOrMore(Box<MatcherToken>),
    /// 零个或一个 (可选)
    Optional(Box<MatcherToken>),
    /// 字面量匹配
    Literal(Token),
    /// 忽略分隔符
    Ignore,
}

/**
 * 宏调用
 */
#[derive(Debug, Clone)]
pub struct MacroCall {
    /// 宏名称
    pub name: String,
    /// 宏参数（每一参数为一组 token 序列）
    pub args: Vec<Vec<Token>>,
    /// span 信息
    pub span: Span,
    /// 卫生上下文
    pub hygiene_context: usize,
}

/**
 * 宏展开结果
 */
#[derive(Debug, Clone)]
pub enum MacroExpansion {
    /// 成功展开
    Success(Vec<Token>),
    /// 继续等待更多输入
    WaitingForMore,
    /// 展开失败
    Error(String),
}

/**
 * 宏展开器 - 负责将宏集成到编译流程
 */
pub struct MacroExpander {
    /// 宏系统
    macro_system: MacroSystem,
    /// 当前展开深度
    expansion_depth: usize,
    /// 最大展开深度
    max_depth: usize,
    /// 展开统计
    stats: MacroStats,
}

/**
 * 宏统计信息
 */
#[derive(Debug, Clone, Default)]
pub struct MacroStats {
    /// 展开次数
    pub expansions: usize,
    /// 定义数量
    pub definitions: usize,
    /// 错误数量
    pub errors: usize,
}

/**
 * 宏系统
 */
pub struct MacroSystem {
    /// 已定义的宏
    macros: HashMap<String, MacroDefinition>,
    /// 宏调用栈 (用于检测递归)
    call_stack: Vec<String>,
    /// 最大递归深度
    max_depth: usize,
    /// 当前卫生上下文ID
    current_hygiene: usize,
    /// 卫生上下文映射
    hygiene_contexts: HashMap<usize, HygieneContext>,
}

/**
 * 卫生上下文
 */
#[derive(Debug, Clone)]
pub struct HygieneContext {
    /// 上下文ID
    pub id: usize,
    /// 捕获的变量
    pub captured_vars: Vec<String>,
    /// 生成的新变量
    pub generated_vars: Vec<String>,
    /// 生成的新标签
    pub generated_labels: Vec<String>,
}

impl MacroExpander {
    /**
     * 创建新的宏展开器
     */
    pub fn new() -> Self {
        Self {
            macro_system: MacroSystem::new(),
            expansion_depth: 0,
            max_depth: 64,
            stats: MacroStats::default(),
        }
    }

    /**
     * 定义宏
     */
    pub fn define(&mut self, definition: MacroDefinition) -> Result<(), MacroError> {
        self.stats.definitions += 1;
        self.macro_system.define(definition)
    }

    /**
     * 检查是否为宏调用
     */
    pub fn is_macro_call(&self, token: &Token) -> bool {
        if let TokenType::标识符 = &token.token_type {
            self.macro_system.is_defined(&token.literal)
        } else {
            false
        }
    }

    /**
     * 展开宏调用
     */
    pub fn expand(&mut self, call: &MacroCall) -> Result<MacroExpansion, MacroError> {
        if self.expansion_depth >= self.max_depth {
            return Err(MacroError::TooManyRecursions(self.max_depth));
        }

        self.expansion_depth += 1;
        let result = self.macro_system.expand(call);
        self.expansion_depth -= 1;

        match &result {
            Ok(MacroExpansion::Success(_)) => self.stats.expansions += 1,
            Err(_) => self.stats.errors += 1,
            _ => {}
        }

        result
    }

    /**
     * 展开Token流中的所有宏调用
     * 遍历过程中会提取并注册宏定义（宏 ... 展开 { ... }），
     * 并将宏调用展开为其模板内容（实参按位置替换形参标识符）。
     */
    pub fn expand_tokens(&mut self, tokens: Vec<Token>) -> Result<Vec<Token>, MacroError> {
        let mut result = Vec::new();
        let mut i = 0;

        while i < tokens.len() {
            // 遇到宏定义关键字，先注册该宏定义并跳过定义体
            if matches!(tokens[i].token_type, TokenType::Keyword(Keyword::宏)) {
                let (definition, next) = parse_macro_definition(&tokens, i)?;
                self.define(definition)?;
                i = next;
                continue;
            }

            let token = &tokens[i];

            // 检查是否为宏调用（已定义的宏名）
            if self.is_macro_call(token) {
                // 收集括号内的实参（支持嵌套括号配对，顶层逗号分隔参数）
                let mut args = Vec::new();
                i += 1;

                if i < tokens.len() && matches!(tokens[i].token_type, TokenType::左圆括号) {
                    i += 1; // 跳过左括号
                    let mut depth = 1;
                    let mut current = Vec::new();
                    while i < tokens.len() && depth > 0 {
                        match &tokens[i].token_type {
                            TokenType::左圆括号 => depth += 1,
                            TokenType::右圆括号 => depth -= 1,
                            _ => {}
                        }
                        if depth == 0 {
                            break;
                        }
                        if depth == 1 && matches!(tokens[i].token_type, TokenType::逗号) {
                            args.push(current.clone());
                            current.clear();
                        } else {
                            current.push(tokens[i].clone());
                        }
                        i += 1;
                    }
                    // 最后一个实参（无后继逗号）
                    if !current.is_empty() {
                        args.push(current);
                    }
                    i += 1; // 跳过右括号
                }

                // 创建宏调用并展开
                let call = MacroCall {
                    name: token.literal.clone(),
                    args,
                    span: token.span,
                    hygiene_context: self.macro_system.current_hygiene,
                };

                match self.expand(&call)? {
                    MacroExpansion::Success(expanded) => {
                        // 将展开结果（可能仍含宏调用）递归展开
                        let nested = self.expand_tokens(expanded)?;
                        result.extend(nested);
                    }
                    MacroExpansion::WaitingForMore => {
                        // 需要更多输入，暂不处理
                        result.push(token.clone());
                    }
                    MacroExpansion::Error(msg) => {
                        return Err(MacroError::ExpansionError(msg));
                    }
                }
            } else {
                result.push(token.clone());
                i += 1;
            }
        }

        Ok(result)
    }

    /**
     * 获取统计信息
     */
    pub fn get_stats(&self) -> &MacroStats {
        &self.stats
    }

    /**
     * 重置统计
     */
    pub fn reset_stats(&mut self) {
        self.stats = MacroStats::default();
    }
}

impl Default for MacroExpander {
    fn default() -> Self {
        Self::new()
    }
}

impl MacroSystem {
    /**
     * 创建新的宏系统
     */
    pub fn new() -> Self {
        Self {
            macros: HashMap::new(),
            call_stack: Vec::new(),
            max_depth: 64,
            current_hygiene: 0,
            hygiene_contexts: HashMap::new(),
        }
    }

    /**
     * 定义宏
     */
    pub fn define(&mut self, macro_def: MacroDefinition) -> Result<(), MacroError> {
        if self.macros.contains_key(&macro_def.name) {
            return Err(MacroError::AlreadyDefined(macro_def.name.clone()));
        }

        self.validate_macro(&macro_def)?;

        self.macros.insert(macro_def.name.clone(), macro_def);
        Ok(())
    }

    /**
     * 验证宏定义
     */
    fn validate_macro(&self, macro_def: &MacroDefinition) -> Result<(), MacroError> {
        // 允许零参数宏（如无参口号宏）
        for rule in &macro_def.body {
            for token in &rule.matcher {
                if let MatcherToken::MatchRepeat { ref name, .. } = token {
                    if name.is_empty() {
                        return Err(MacroError::InvalidDefinition(
                            "重复参数必须有名称".to_string()
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /**
     * 展开宏调用
     */
    pub fn expand(&mut self, call: &MacroCall) -> Result<MacroExpansion, MacroError> {
        let macro_def = match self.macros.get(&call.name).cloned() {
            Some(m) => m,
            None => return Err(MacroError::NotFound(call.name.clone())),
        };

        if self.call_stack.len() >= self.max_depth {
            return Err(MacroError::TooManyRecursions(self.max_depth));
        }

        if self.call_stack.contains(&call.name) {
            return Err(MacroError::RecursiveExpansion(call.name.clone()));
        }

        self.call_stack.push(call.name.clone());

        let result = self.match_and_expand(&macro_def, call);

        self.call_stack.pop();

        result
    }

    /**
     * 匹配并展开
     */
    fn match_and_expand(
        &self,
        macro_def: &MacroDefinition,
        call: &MacroCall,
    ) -> Result<MacroExpansion, MacroError> {
        for rule in &macro_def.body {
            if let Some(binding) = self.try_match_rule(rule, &call.args) {
                let expanded = self.expand_template(rule, &binding)?;
                return Ok(MacroExpansion::Success(expanded));
            }
        }

        Err(MacroError::NoMatchingRule(call.name.clone()))
    }

    /**
     * 尝试匹配规则
     */
    fn try_match_rule(&self, rule: &MacroRule, args: &[Vec<Token>]) -> Option<HashMap<String, Vec<Token>>> {
        let mut bindings = HashMap::new();

        if rule.matcher.is_empty() {
            return Some(bindings);
        }

        // 匹配规则中的占位符（形参）
        let mut arg_index = 0;
        for matcher in &rule.matcher {
            match matcher {
                MatcherToken::MatchExpr(name) | MatcherToken::MatchIdent(name) | MatcherToken::MatchType(name) => {
                    if arg_index >= args.len() {
                        return None;
                    }
                    bindings.insert(name.clone(), args[arg_index].clone());
                    arg_index += 1;
                }
                MatcherToken::MatchStmt(name) => {
                    if arg_index >= args.len() {
                        return None;
                    }
                    bindings.insert(name.clone(), args[arg_index].clone());
                    arg_index += 1;
                }
                MatcherToken::MatchLiteral(name) => {
                    if arg_index >= args.len() {
                        return None;
                    }
                    bindings.insert(name.clone(), args[arg_index].clone());
                    arg_index += 1;
                }
                MatcherToken::ZeroOrMore(inner) | MatcherToken::OneOrMore(inner) => {
                    // 零个/一个或多个：收集剩余所有实参
                    if let MatcherToken::MatchExpr(name) | MatcherToken::MatchIdent(name) = &**inner {
                        let rest: Vec<Token> = args[arg_index..].iter().flatten().cloned().collect();
                        if matches!(matcher, MatcherToken::OneOrMore(_)) && rest.is_empty() {
                            return None;
                        }
                        bindings.insert(name.clone(), rest);
                        arg_index = args.len();
                    }
                }
                MatcherToken::MatchRepeat { name, .. } => {
                    // 重复匹配：收集剩余所有参数
                    let rest: Vec<Token> = args[arg_index..].iter().flatten().cloned().collect();
                    bindings.insert(name.clone(), rest);
                    arg_index = args.len();
                }
                MatcherToken::Optional(inner) => {
                    if let MatcherToken::MatchExpr(name) | MatcherToken::MatchIdent(name) = &**inner {
                        if arg_index < args.len() {
                            bindings.insert(name.clone(), args[arg_index].clone());
                            arg_index += 1;
                        }
                    }
                }
                MatcherToken::Literal(_) | MatcherToken::Ignore => {}
            }
        }

        Some(bindings)
    }

    /**
     * 展开模板，将模板中的形参占位符替换为实参
     */
    fn expand_template(&self, rule: &MacroRule, binding: &HashMap<String, Vec<Token>>) -> Result<Vec<Token>, MacroError> {
        let mut result = Vec::new();

        for token in &rule.template {
            // 模板中的形参名（标识符）若出现在绑定表中，则替换为实参 token
            if let TokenType::标识符 = &token.token_type {
                if let Some(replacement) = binding.get(&token.literal) {
                    result.extend(replacement.iter().cloned());
                    continue;
                }
            }
            result.push(token.clone());
        }

        Ok(result)
    }

    /**
     * 检查是否已定义宏
     */
    pub fn is_defined(&self, name: &str) -> bool {
        self.macros.contains_key(name)
    }

    /**
     * 获取宏定义
     */
    pub fn get_macro(&self, name: &str) -> Option<&MacroDefinition> {
        self.macros.get(name)
    }

    /**
     * 列出所有宏
     */
    pub fn list_macros(&self) -> Vec<&str> {
        self.macros.keys().map(|s| s.as_str()).collect()
    }

    /**
     * 创建新的卫生上下文
     */
    pub fn new_hygiene_context(&mut self) -> usize {
        self.current_hygiene += 1;
        self.hygiene_contexts.insert(self.current_hygiene, HygieneContext {
            id: self.current_hygiene,
            captured_vars: Vec::new(),
            generated_vars: Vec::new(),
            generated_labels: Vec::new(),
        });
        self.current_hygiene
    }

    /**
     * 生成唯一的卫生变量名
     */
    pub fn generate_hygienic_var(&mut self, base_name: &str) -> String {
        let context = self.hygiene_contexts.get_mut(&self.current_hygiene);
        if let Some(ctx) = context {
            let unique_name = format!("{}_{}", base_name, ctx.generated_vars.len());
            ctx.generated_vars.push(unique_name.clone());
            unique_name
        } else {
            base_name.to_string()
        }
    }
}

/**
 * 解析宏定义
 * 语法: 宏 宏名称 (参数) 展开 { 模板 }
 */
pub fn parse_macro_definition(tokens: &[Token], start: usize) -> Result<(MacroDefinition, usize), MacroError> {
    let mut pos = start;

    // 跳过 '宏' 关键字
    if let TokenType::Keyword(Keyword::宏) = &tokens[pos].token_type {
        pos += 1;
    } else {
        return Err(MacroError::InvalidDefinition("期望 '宏' 关键字".to_string()));
    }

    // 获取宏名称
    let name = match &tokens[pos].token_type {
        TokenType::标识符 => tokens[pos].literal.clone(),
        _ => return Err(MacroError::InvalidDefinition("期望宏名称".to_string())),
    };
    pos += 1;

    // 解析参数列表
    let mut params = Vec::new();
    if let TokenType::左圆括号 = &tokens[pos].token_type {
        pos += 1;
        while pos < tokens.len() && !matches!(tokens[pos].token_type, TokenType::右圆括号) {
            if let TokenType::标识符 = &tokens[pos].token_type {
                params.push(MacroParam {
                    pattern: MacroPattern::Expr,
                    name: tokens[pos].literal.clone(),
                    is_varargs: false,
                });
            }
            pos += 1;
        }
        if pos < tokens.len() {
            pos += 1; // 跳过右括号
        }
    }

    // 跳过 '展开' 关键字
    if pos < tokens.len() {
        if let TokenType::Keyword(Keyword::展开) = &tokens[pos].token_type {
            pos += 1;
        }
    }

    // 解析模板 (收集到配对的右花括号)
    let mut template = Vec::new();
    let mut brace_count = 0;
    let mut started = false;

    while pos < tokens.len() {
        let token = &tokens[pos];
        if matches!(token.token_type, TokenType::左花括号) {
            brace_count += 1;
            if started {
                template.push(token.clone());
            }
            started = true;
        } else if matches!(token.token_type, TokenType::右花括号) {
            brace_count -= 1;
            if brace_count == 0 {
                pos += 1; // 越过右花括号
                break;
            }
            template.push(token.clone());
        } else if started {
            template.push(token.clone());
        }
        pos += 1;
    }

    // 根据参数列表生成匹配器，使模板中的形参名可被实参替换
    let matcher = params.iter().map(|p| MatcherToken::MatchExpr(p.name.clone())).collect();

    let definition = MacroDefinition {
        name,
        params,
        body: vec![MacroRule {
            matcher,
            template,
            is_export: false,
        }],
        hygiene: MacroHygiene::Full,
        span: Span::dummy(),
    };

    Ok((definition, pos))
}

/**
 * 宏错误
 */
#[derive(Debug, Clone)]
pub enum MacroError {
    /// 宏未找到
    NotFound(String),
    /// 宏已定义
    AlreadyDefined(String),
    /// 宏定义无效
    InvalidDefinition(String),
    /// 没有匹配的规则
    NoMatchingRule(String),
    /// 递归展开
    RecursiveExpansion(String),
    /// 展开深度超限
    TooManyRecursions(usize),
    /// 展开错误
    ExpansionError(String),
    /// 参数数量不匹配
    WrongArgCount { expected: usize, found: usize },
}

impl std::fmt::Display for MacroError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MacroError::NotFound(name) => write!(f, "未找到宏: {}", name),
            MacroError::AlreadyDefined(name) => write!(f, "宏已定义: {}", name),
            MacroError::InvalidDefinition(msg) => write!(f, "宏定义无效: {}", msg),
            MacroError::NoMatchingRule(name) => write!(f, "没有匹配的宏规则: {}", name),
            MacroError::RecursiveExpansion(name) => write!(f, "递归宏展开: {}", name),
            MacroError::TooManyRecursions(depth) => write!(f, "宏展开深度超限: {}", depth),
            MacroError::ExpansionError(msg) => write!(f, "宏展开错误: {}", msg),
            MacroError::WrongArgCount { expected, found } => {
                write!(f, "参数数量不匹配: 期望 {}, 实际 {}", expected, found)
            }
        }
    }
}

impl std::error::Error for MacroError {}

impl Default for MacroSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_definition() {
        let mut system = MacroSystem::new();

        let macro_def = MacroDefinition {
            name: "打印值".to_string(),
            params: vec![MacroParam {
                pattern: MacroPattern::Expr,
                name: "x".to_string(),
                is_varargs: false,
            }],
            body: vec![MacroRule {
                matcher: vec![MatcherToken::MatchExpr("x".to_string())],
                template: vec![],
                is_export: false,
            }],
            hygiene: MacroHygiene::Full,
            span: Span::dummy(),
        };

        assert!(system.define(macro_def).is_ok());
        assert!(system.is_defined("打印值"));
    }

    #[test]
    fn test_macro_expansion() {
        let mut system = MacroSystem::new();

        let macro_def = MacroDefinition {
            name: "打印值".to_string(),
            params: vec![MacroParam {
                pattern: MacroPattern::Expr,
                name: "x".to_string(),
                is_varargs: false,
            }],
            body: vec![MacroRule {
                matcher: vec![],
                template: vec![],
                is_export: false,
            }],
            hygiene: MacroHygiene::Full,
            span: Span::dummy(),
        };

        system.define(macro_def).unwrap();

        let call = MacroCall {
            name: "打印值".to_string(),
            args: vec![],
            span: Span::dummy(),
            hygiene_context: 0,
        };

        let result = system.expand(&call);
        assert!(result.is_ok());
    }

    #[test]
    fn test_macro_expander() {
        let mut expander = MacroExpander::new();

        let macro_def = MacroDefinition {
            name: "打印".to_string(),
            params: vec![crate::macro_system::MacroParam {
                pattern: MacroPattern::Expr,
                name: "值".to_string(),
                is_varargs: false,
            }],
            body: vec![MacroRule {
                matcher: vec![],
                template: vec![],
                is_export: false,
            }],
            hygiene: MacroHygiene::Full,
            span: Span::dummy(),
        };

        expander.define(macro_def).unwrap();

        let tokens = vec![
            Token {
                token_type: TokenType::标识符,
                literal: "打印".to_string(),
                span: Span::dummy(),
            },
            Token {
                token_type: TokenType::左圆括号,
                literal: "(".to_string(),
                span: Span::dummy(),
            },
            Token {
                token_type: TokenType::右圆括号,
                literal: ")".to_string(),
                span: Span::dummy(),
            },
        ];

        let result = expander.expand_tokens(tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_expand_tokens_with_arg_substitution() {
        let mut expander = MacroExpander::new();
        let span = Span::dummy();
        let id = |name: &str| Token {
            token_type: TokenType::标识符,
            literal: name.to_string(),
            span: span.clone(),
        };
        let int = |value: i64| Token {
            token_type: TokenType::整数字面量,
            literal: value.to_string(),
            span: span.clone(),
        };

        // 定义宏: 宏 双倍 (a) 展开 { a + a }
        let macro_def = MacroDefinition {
            name: "双倍".to_string(),
            params: vec![MacroParam {
                pattern: MacroPattern::Expr,
                name: "a".to_string(),
                is_varargs: false,
            }],
            body: vec![MacroRule {
                matcher: vec![MatcherToken::MatchExpr("a".to_string())],
                template: vec![id("a"), Token { token_type: TokenType::加, literal: "+".to_string(), span: span.clone() }, id("a")],
                is_export: false,
            }],
            hygiene: MacroHygiene::Full,
            span: span.clone(),
        };
        expander.define(macro_def).unwrap();

        // 调用: 双倍 ( 21 )
        let tokens = vec![
            id("双倍"),
            Token { token_type: TokenType::左圆括号, literal: "(".to_string(), span: span.clone() },
            int(21),
            Token { token_type: TokenType::右圆括号, literal: ")".to_string(), span: span.clone() },
        ];

        let result = expander.expand_tokens(tokens).unwrap();
        // 展开结果: 21 + 21
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].literal, "21");
        assert_eq!(result[1].literal, "+");
        assert_eq!(result[2].literal, "21");
    }

    #[test]
    fn test_macro_definition_inline_and_recursive() {
        let mut expander = MacroExpander::new();

        // 通过 expand_tokens 内联定义并调用: 宏 自增 (n) 展开 { n } 
        let tokens = vec![
            Token { token_type: TokenType::Keyword(Keyword::宏), literal: "宏".to_string(), span: Span::dummy() },
            Token { token_type: TokenType::标识符, literal: "同上".to_string(), span: Span::dummy() },
            Token { token_type: TokenType::左圆括号, literal: "(".to_string(), span: Span::dummy() },
            Token { token_type: TokenType::标识符, literal: "n".to_string(), span: Span::dummy() },
            Token { token_type: TokenType::右圆括号, literal: ")".to_string(), span: Span::dummy() },
            Token { token_type: TokenType::Keyword(Keyword::展开), literal: "展开".to_string(), span: Span::dummy() },
            Token { token_type: TokenType::左花括号, literal: "{".to_string(), span: Span::dummy() },
            Token { token_type: TokenType::标识符, literal: "n".to_string(), span: Span::dummy() },
            Token { token_type: TokenType::加, literal: "+".to_string(), span: Span::dummy() },
            Token { token_type: TokenType::整数字面量, literal: "1".to_string(), span: Span::dummy() },
            Token { token_type: TokenType::右花括号, literal: "}".to_string(), span: Span::dummy() },
        ];

        // 仅定义时，无宏调用，输出应过滤掉宏定义本身
        let result = expander.expand_tokens(tokens).unwrap();
        assert_eq!(result.len(), 0);
        assert!(expander.is_macro_call(&Token {
            token_type: TokenType::标识符,
            literal: "同上".to_string(),
            span: Span::dummy(),
        }));
    }
}
