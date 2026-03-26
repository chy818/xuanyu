/**
 * @file parser.rs
 * @brief CCAS 语法分析器 (Parser)
 * @description 将 Token 流转换为抽象语法树 (AST)
 * 
 * 支持的语法:
 * - 变量声明: 定义 x: 整数 = 10
 * - 函数定义: 函数 主 函数() { ... }
 * - 表达式: 算术、比较、逻辑运算
 * - 控制流: 若/当/循环/匹配
 */

use crate::lexer::{Token, TokenType, Keyword, Span};
use crate::ast::*;
use crate::error::ParserError;

/**
 * 语法分析器
 * 使用递归下降解析算法
 */
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    /**
     * 从 Token 列表创建解析器
     */
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    /**
     * 获取当前位置的 Token
     */
    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    /**
     * 向前看 n 个 Token
     */
    fn peek(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.position + offset)
    }

    /**
     * 向前移动一个位置，返回前一个 Token
     */
    fn advance(&mut self) -> Option<&Token> {
        let result = self.tokens.get(self.position);
        self.position += 1;
        result
    }

    /**
     * 返回前一个 Token
     */
    fn previous(&self) -> Option<&Token> {
        if self.position > 0 {
            self.tokens.get(self.position - 1)
        } else {
            None
        }
    }

    /**
     * 检查当前 Token 是否为指定类型
     */
    fn check(&self, token_type: &TokenType) -> bool {
        self.current()
            .map(|t| &t.token_type == token_type)
            .unwrap_or(false)
    }

    /**
     * 检查当前 Token 是否为指定关键字
     */
    fn check_keyword(&self, keyword: &Keyword) -> bool {
        if let Some(Token { token_type: TokenType::Keyword(k), .. }) = self.current() {
            k == keyword
        } else {
            false
        }
    }

    /**
     * 消耗当前 Token，如果匹配则推进位置
     */
    fn match_token(&mut self, token_type: &TokenType) -> bool {
        if self.check(token_type) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    /**
     * 消耗当前 Token，必须匹配否则报错
     */
    fn expect(&mut self, token_type: &TokenType) -> Result<Token, ParserError> {
        let current_token = self.current().cloned();
        
        if let Some(token) = current_token {
            if &token.token_type == token_type {
                self.position += 1;
                return Ok(token);
            }
        }
        
        let expected = format!("{:?}", token_type);
        let found = self.current()
            .map(|t| format!("{:?}", t.token_type))
            .unwrap_or_else(|| "文件结束".to_string());
        
        let span = self.current()
            .map(|t| t.span)
            .unwrap_or(Span::dummy());
        
        Err(ParserError::unexpected_token(&expected, &found, span))
    }

    /**
     * 消耗当前 Token，如果匹配指定关键字则推进位置
     */
    fn match_keyword(&mut self, keyword: &Keyword) -> bool {
        if self.check_keyword(keyword) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    /**
     * 解析整个模块
     * 模块 -> 导入列表? (函数|结构体|枚举|类型别名|常量|外部)*
     */
    pub fn parse_module(&mut self) -> Result<Module, ParserError> {
        let mut imports = Vec::new();
        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut type_aliases = Vec::new();
        let mut constants = Vec::new();
        let mut extern_functions = Vec::new();
        let start_span = self.current()
            .map(|t| t.span)
            .unwrap_or(Span::dummy());

        while !self.check(&TokenType::文件结束) {
            // 根据关键字类型分发解析
            if self.check(&TokenType::Keyword(Keyword::引入)) {
                match self.parse_import() {
                    Ok(imp) => imports.push(imp),
                    Err(e) => return Err(e),
                }
            } else if self.check(&TokenType::Keyword(Keyword::函数)) {
                match self.parse_function() {
                    Ok(func) => functions.push(func),
                    Err(e) => return Err(e),
                }
            } else if self.check(&TokenType::Keyword(Keyword::结构体)) {
                match self.parse_struct() {
                    Ok(s) => structs.push(s),
                    Err(e) => return Err(e),
                }
            } else if self.check(&TokenType::Keyword(Keyword::枚举)) {
                match self.parse_enum() {
                    Ok(e) => enums.push(e),
                    Err(e) => return Err(e),
                }
            } else if self.check(&TokenType::Keyword(Keyword::类型别名)) {
                match self.parse_type_alias() {
                    Ok(t) => type_aliases.push(t),
                    Err(e) => return Err(e),
                }
            } else if self.check(&TokenType::Keyword(Keyword::常量)) {
                match self.parse_constant() {
                    Ok(c) => constants.push(c),
                    Err(e) => return Err(e),
                }
            } else if self.check(&TokenType::Keyword(Keyword::外部)) {
                match self.parse_extern_function() {
                    Ok(e) => extern_functions.push(e),
                    Err(e) => return Err(e),
                }
            } else {
                // 跳过无法识别的声明，继续解析
                self.advance();
            }
        }

        let end_span = self.tokens.last()
            .map(|t| t.span)
            .unwrap_or(start_span);

        let mut module = Module::new(functions, start_span.merge(end_span));
        module.imports = imports;
        module.structs = structs;
        module.enums = enums;
        module.type_aliases = type_aliases;
        module.constants = constants;
        module.extern_functions = extern_functions;
        Ok(module)
    }

    /**
     * 解析导入语句
     * 引入 "模块路径"
     * 引入 "模块路径" { 项1, 项2 }
     */
    fn parse_import(&mut self) -> Result<ImportStmt, ParserError> {
        // 消耗 '引入' 关键字
        self.expect(&TokenType::Keyword(Keyword::引入))?;
        
        let start_span = self.previous().unwrap().span.clone();
        
        // 解析模块路径 (字符串字面量)
        let module_path = match self.current() {
            Some(Token { token_type: TokenType::文本字面量, literal, .. }) => {
                literal.clone()
            }
            _ => {
                return Err(ParserError::unexpected_token(
                    "模块路径",
                    &self.current().map(|t| t.literal.clone()).unwrap_or_default(),
                    self.current().map(|t| t.span).unwrap_or(Span::dummy())
                ));
            }
        };
        self.advance();
        
        // 解析可选的导入项列表 { 项1, 项2 }
        let mut imported_items = Vec::new();
        if self.check(&TokenType::左花括号) {
            self.advance();
            while !self.check(&TokenType::右花括号) {
                match self.current() {
                    Some(Token { token_type: TokenType::标识符, literal, .. }) => {
                        imported_items.push(literal.clone());
                        self.advance();
                        // 跳过逗号
                        if self.check(&TokenType::逗号) {
                            self.advance();
                        }
                    }
                    _ => {
                        return Err(ParserError::unexpected_token(
                            "导入项",
                            &self.current().map(|t| t.literal.clone()).unwrap_or_default(),
                            self.current().map(|t| t.span).unwrap_or(Span::dummy())
                        ));
                    }
                }
            }
            self.expect(&TokenType::右花括号)?;
        }
        
        let end_span = self.previous().unwrap().span.clone();
        Ok(ImportStmt {
            module_path,
            imported_items,
            span: start_span.merge(end_span),
        })
    }

    /**
     * 解析结构体定义
     * 结构体 用户 { 姓名: 文本, 年龄: 整数 }
     */
    fn parse_struct(&mut self) -> Result<StructDefinition, ParserError> {
        // 消耗 '结构体' 关键字
        self.expect(&TokenType::Keyword(Keyword::结构体))?;
        
        let start_span = self.previous().unwrap().span.clone();
        
        // 结构体名
        let name = match self.current() {
            Some(Token { token_type: TokenType::标识符, literal, .. }) => {
                literal.clone()
            }
            _ => {
                return Err(ParserError::unexpected_token(
                    "结构体名",
                    &self.current().map(|t| t.literal.clone()).unwrap_or_default(),
                    self.current().map(|t| t.span).unwrap_or(Span::dummy())
                ));
            }
        };
        self.advance();
        
        // 期望 '{'
        self.expect(&TokenType::左花括号)?;
        
        // 解析字段列表
        let mut fields = Vec::new();
        while !self.check(&TokenType::右花括号) {
            // 解析字段名（支持关键字作为字段名）
            let field_name = match self.current() {
                Some(Token { token_type: TokenType::标识符, literal, .. }) => {
                    literal.clone()
                }
                Some(Token { token_type: TokenType::Keyword(_), literal, .. }) => {
                    // 关键字作为字段名
                    literal.clone()
                }
                _ => {
                    return Err(ParserError::unexpected_token(
                        "字段名",
                        &self.current().map(|t| t.literal.clone()).unwrap_or_default(),
                        self.current().map(|t| t.span).unwrap_or(Span::dummy())
                    ));
                }
            };
            self.advance();
            
            // 检查是否有类型标注: '字段' ':' '类型'
            let field_type = if self.check(&TokenType::冒号) {
                self.advance(); // 消耗 ':'
                // 解析类型
                self.parse_type()?
            } else {
                // 没有类型标注，默认使用空类型（用于自展编译器）
                Type::Void
            };
            
            fields.push(StructField {
                name: field_name,
                field_type,
            });
            
            // 检查 ',' 或 '}'
            if self.check(&TokenType::逗号) {
                self.advance();
            } else if !self.check(&TokenType::右花括号) {
                return Err(ParserError::unexpected_token(
                    "',' 或 '}'",
                    &self.current().map(|t| t.literal.clone()).unwrap_or_default(),
                    self.current().map(|t| t.span).unwrap_or(Span::dummy())
                ));
            }
        }
        
        // 期望 '}'
        self.expect(&TokenType::右花括号)?;
        
        let end_span = self.previous().unwrap().span.clone();
        Ok(StructDefinition {
            name,
            fields,
            span: start_span.merge(end_span),
        })
    }

    /**
     * 解析枚举定义
     * 枚举 颜色 { 红, 绿, 蓝 }
     * 枚举 表达式 { 数字(整数), 加法(左: 节点, 右: 节点) }
     */
    fn parse_enum(&mut self) -> Result<EnumDefinition, ParserError> {
        // 消耗 '枚举' 关键字
        self.expect(&TokenType::Keyword(Keyword::枚举))?;
        
        let start_span = self.previous().unwrap().span.clone();
        
        // 枚举名
        let name = match self.current() {
            Some(Token { token_type: TokenType::标识符, literal, .. }) => {
                literal.clone()
            }
            _ => {
                return Err(ParserError::unexpected_token(
                    "枚举名",
                    &self.current().map(|t| t.literal.clone()).unwrap_or_default(),
                    self.current().map(|t| t.span).unwrap_or(Span::dummy())
                ));
            }
        };
        self.advance();
        
        // 期望 '{'
        self.expect(&TokenType::左花括号)?;
        
        // 解析变体列表
        let mut variants = Vec::new();
        while !self.check(&TokenType::右花括号) {
            // 支持关键字作为枚举变体名（如：字符, 整数, 文本等）
            let variant_name = match self.current() {
                Some(Token { token_type: TokenType::标识符, literal, .. }) => {
                    literal.clone()
                }
                Some(Token { token_type: TokenType::Keyword(kw), literal, .. }) => {
                    // 关键字作为变体名，使用其字面值或Debug格式
                    if literal.is_empty() {
                        format!("{:?}", kw)
                    } else {
                        literal.clone()
                    }
                }
                _ => {
                    return Err(ParserError::unexpected_token(
                        "枚举变体名",
                        &self.current().map(|t| t.literal.clone()).unwrap_or_default(),
                        self.current().map(|t| t.span).unwrap_or(Span::dummy())
                    ));
                }
            };
            self.advance();

            // 跳过枚举值赋值: = 数值 (如: 标识符 = 0)
            if self.check(&TokenType::赋值) {
                self.advance(); // 消耗 '='
                // 跳过赋值的数值
                while !self.check(&TokenType::逗号) && !self.check(&TokenType::右花括号) {
                    self.advance();
                }
            }

            // 检查是否有字段列表: (类型1, 类型2) 或 (字段1: 类型1, 字段2: 类型2)
            let mut fields = Vec::new();
            if self.check(&TokenType::左圆括号) {
                self.advance(); // 消耗 '('
                
                // 解析字段列表
                while !self.check(&TokenType::右圆括号) {
                    // 检查命名字段: 标识符 ':' 类型
                    let field_name = if self.peek(1).map(|t| &t.token_type).unwrap_or(&TokenType::未知) == &TokenType::冒号 {
                        // 命名字段: 字段名 : 类型
                        let name = match self.current() {
                            Some(Token { token_type: TokenType::标识符, literal, .. }) => {
                                Some(literal.clone())
                            }
                            _ => {
                                return Err(ParserError::unexpected_token(
                                    "字段名",
                                    &self.current().map(|t| t.literal.clone()).unwrap_or_default(),
                                    self.current().map(|t| t.span).unwrap_or(Span::dummy())
                                ));
                            }
                        };
                        self.advance();
                        self.expect(&TokenType::冒号)?; // 消耗 ':'
                        name
                    } else {
                        None // 位置参数
                    };
                    
                    // 解析字段类型
                    let field_type = self.parse_type()?;
                    
                    fields.push(EnumVariantField {
                        name: field_name,
                        field_type,
                    });
                    
                    // 跳过逗号
                    if self.check(&TokenType::逗号) {
                        self.advance();
                    }
                }
                
                self.expect(&TokenType::右圆括号)?; // 消耗 ')'
            }
            
            variants.push(EnumVariant {
                name: variant_name,
                fields,
            });
            
            // 检查 ',' 或 '}'
            // 如果没有逗号但也不是 }，可能是换行分隔，允许继续
            if self.check(&TokenType::逗号) {
                self.advance();
            }
            // 继续循环，让 while 条件检查是否遇到 }
        }
        
        // 期望 '}'
        self.expect(&TokenType::右花括号)?;
        
        let end_span = self.previous().unwrap().span.clone();
        Ok(EnumDefinition {
            name,
            variants,
            span: start_span.merge(end_span),
        })
    }

    /**
     * 解析常量定义
     * 常量 最高分 = 100
     * 常量 消息 = "你好"
     */
    fn parse_constant(&mut self) -> Result<ConstantDef, ParserError> {
        // 消耗 '常量' 关键字
        self.expect(&TokenType::Keyword(Keyword::常量))?;

        let start_span = self.previous().unwrap().span.clone();
        
        // 常量名
        let name = match self.current() {
            Some(Token { token_type: TokenType::标识符, literal, .. }) => {
                literal.clone()
            }
            _ => {
                return Err(ParserError::unexpected_token(
                    "常量名",
                    &self.current().map(|t| t.literal.clone()).unwrap_or_default(),
                    self.current().map(|t| t.span).unwrap_or(Span::dummy())
                ));
            }
        };
        self.advance();
        
        // 可选类型标注
        let const_type = if self.match_token(&TokenType::冒号) {
            self.parse_type()?
        } else {
            Type::Int // 默认类型
        };
        
        // 期望 '='
        self.expect(&TokenType::赋值)?;
        
        // 解析常量值
        let value = self.parse_expression()?;
        
        let end_span = self.previous().unwrap().span.clone();
        Ok(ConstantDef {
            name,
            const_type,
            value,
            span: start_span.merge(end_span),
        })
    }

    /**
     * 解析外部函数声明 (FFI)
     * 外部 函数 malloc(大小: 整数) -> 指针 ["malloc"]
     */
    fn parse_extern_function(&mut self) -> Result<ExternFunction, ParserError> {
        // 消耗 '外部' 关键字
        self.expect(&TokenType::Keyword(Keyword::外部))?;
        
        let start_span = self.previous().unwrap().span.clone();
        
        // 期望 '函数' 关键字
        self.expect(&TokenType::Keyword(Keyword::函数))?;
        
        // 函数名
        let name = match self.current() {
            Some(Token { token_type: TokenType::标识符, literal, .. }) => {
                literal.clone()
            }
            _ => {
                return Err(ParserError::unexpected_token(
                    "函数名",
                    &self.current().map(|t| t.literal.clone()).unwrap_or_default(),
                    self.current().map(|t| t.span).unwrap_or(Span::dummy())
                ));
            }
        };
        self.advance();
        
        // 参数列表
        let params = self.parse_parameter_list()?;
        
        // 可选返回类型
        let return_type = if self.match_token(&TokenType::冒号) {
            self.parse_type()?
        } else {
            Type::Void
        };
        
        // 可选链接名: ["malloc"]
        let link_name = if self.check(&TokenType::左方括号) {
            self.advance();
            let link = match self.current() {
                Some(Token { token_type: TokenType::文本字面量, literal, .. }) => {
                    literal.clone()
                }
                _ => {
                    return Err(ParserError::unexpected_token(
                        "链接名",
                        &self.current().map(|t| t.literal.clone()).unwrap_or_default(),
                        self.current().map(|t| t.span).unwrap_or(Span::dummy())
                    ));
                }
            };
            self.advance();
            self.expect(&TokenType::右方括号)?;
            Some(link)
        } else {
            None
        };
        
        let end_span = self.previous().unwrap().span.clone();
        Ok(ExternFunction {
            name,
            params,
            return_type,
            link_name,
            span: start_span.merge(end_span),
        })
    }

    /**
     * 解析类型别名
     * 类型 整数别名 = 整数
     */
    fn parse_type_alias(&mut self) -> Result<TypeAlias, ParserError> {
        // 消耗 '类型' 关键字
        self.expect(&TokenType::Keyword(Keyword::类型别名))?;
        
        let start_span = self.previous().unwrap().span.clone();
        
        // 别名名
        let name = match self.current() {
            Some(Token { token_type: TokenType::标识符, literal, .. }) => {
                literal.clone()
            }
            _ => {
                return Err(ParserError::unexpected_token(
                    "类型别名",
                    &self.current().map(|t| t.literal.clone()).unwrap_or_default(),
                    self.current().map(|t| t.span).unwrap_or(Span::dummy())
                ));
            }
        };
        self.advance();
        
        // 期望 '='
        self.expect(&TokenType::赋值)?;
        
        // 解析类型
        let aliased_type = self.parse_type()?;
        
        let end_span = self.previous().unwrap().span.clone();
        Ok(TypeAlias {
            name,
            aliased_type,
            span: start_span.merge(end_span),
        })
    }

    /**
     * 解析函数定义
     * 函数 -> '函数' 标识符 '函数' 参数列表 返回类型? '{' 语句列表 '}'
     */
    fn parse_function(&mut self) -> Result<Function, ParserError> {
        // 消耗 '函数' 关键字
        self.expect(&TokenType::Keyword(Keyword::函数))?;

        // 函数名（支持关键字作为函数名，如：列表、文本等）
        let name = match self.current() {
            Some(Token { token_type: TokenType::标识符, literal, .. }) => {
                literal.clone()
            }
            Some(Token { token_type: TokenType::Keyword(_), literal, .. }) => {
                literal.clone()
            }
            _ => {
                return Err(ParserError::unexpected_token(
                    "函数名",
                    &self.current().map(|t| t.literal.clone()).unwrap_or_default(),
                    self.current().map(|t| t.span).unwrap_or(Span::dummy())
                ));
            }
        };
        self.position += 1;

        // 可选的 '函数' 关键字 (函数名后的 '函数')
        // 支持两种语法: 函数 主() 和 函数 主 函数()
        let _ = self.match_token(&TokenType::Keyword(Keyword::函数));

        // 参数列表
        let params = self.parse_parameter_list()?;

        // 可选返回类型
        let return_type = self.parse_return_type()?;

        // 函数体
        let body_span = self.current()
            .map(|t| t.span)
            .unwrap_or(Span::dummy());
        
        self.expect(&TokenType::左花括号)?;
        let statements = self.parse_statement_list()?;
        self.expect(&TokenType::右花括号)?;

        let span = body_span; // TODO: 合并完整 span

        Ok(Function::new(name, params, return_type, BlockStmt::new(statements, span), span))
    }

    /**
     * 解析参数列表
     * 参数列表 -> '(' (参数 (',' 参数)*)? ')'
     */
    fn parse_parameter_list(&mut self) -> Result<Vec<FunctionParam>, ParserError> {
        self.expect(&TokenType::左圆括号)?;
        
        let mut params = Vec::new();
        
        // 如果下一个是 ')'，参数列表为空
        if self.check(&TokenType::右圆括号) {
            self.position += 1;
            return Ok(params);
        }

        // 解析第一个参数
        params.push(self.parse_parameter()?);

        // 解析剩余参数
        while self.match_token(&TokenType::逗号) {
            params.push(self.parse_parameter()?);
        }

        self.expect(&TokenType::右圆括号)?;

        Ok(params)
    }

    /**
     * 解析单个参数
     * 参数 -> 标识符 ':' 类型
     */
    fn parse_parameter(&mut self) -> Result<FunctionParam, ParserError> {
        // 参数名（支持关键字作为参数名）
        let name = match self.current() {
            Some(Token { token_type: TokenType::标识符, literal, .. }) => {
                literal.clone()
            }
            Some(Token { token_type: TokenType::Keyword(_), literal, .. }) => {
                literal.clone()
            }
            _ => {
                return Err(ParserError::unexpected_token(
                    "参数名",
                    &self.current().map(|t| t.literal.clone()).unwrap_or_default(),
                    self.current().map(|t| t.span).unwrap_or(Span::dummy())
                ));
            }
        };
        self.position += 1;

        // 可选的冒号和类型 (参数: 类型)
        // 如果没有冒号，默认类型为 Int
        let param_type = if self.check(&TokenType::冒号) {
            self.position += 1;  // 消耗冒号
            self.parse_type()?
        } else {
            Type::Int  // 默认类型为整数
        };

        Ok(FunctionParam { name, param_type })
    }

    /**
     * 解析返回类型
     * 返回类型 -> ':' 类型
     */
    fn parse_return_type(&mut self) -> Result<Type, ParserError> {
        if self.match_token(&TokenType::冒号) {
            self.parse_type()
        } else {
            Ok(Type::Void) // 默认无返回
        }
    }

    /**
     * 解析类型
     * 类型 -> 基础类型 ('<' 类型 ')' )?
     */
    fn parse_type(&mut self) -> Result<Type, ParserError> {
        let base_type = match self.current() {
            Some(Token { token_type: TokenType::Keyword(Keyword::整数), .. }) => {
                self.position += 1;
                Type::Int
            }
            Some(Token { token_type: TokenType::Keyword(Keyword::长整数), .. }) => {
                self.position += 1;
                Type::Long
            }
            Some(Token { token_type: TokenType::Keyword(Keyword::浮点数), .. }) => {
                self.position += 1;
                Type::Float
            }
            Some(Token { token_type: TokenType::Keyword(Keyword::双精度), .. }) => {
                self.position += 1;
                Type::Double
            }
            Some(Token { token_type: TokenType::Keyword(Keyword::布尔), .. }) => {
                self.position += 1;
                Type::Bool
            }
            Some(Token { token_type: TokenType::Keyword(Keyword::文本), .. }) => {
                self.position += 1;
                Type::String
            }
            Some(Token { token_type: TokenType::Keyword(Keyword::字符), .. }) => {
                self.position += 1;
                Type::Char
            }
            Some(Token { token_type: TokenType::Keyword(Keyword::无返回), .. }) => {
                self.position += 1;
                Type::Void
            }
            Some(Token { token_type: TokenType::Keyword(Keyword::指针), .. }) => {
                self.position += 1;
                Type::Pointer
            }
            Some(Token { token_type: TokenType::Keyword(Keyword::列表), .. }) => {
                self.position += 1;
                // 列表类型：支持泛型参数 列表<类型>
                // 注意：< 在词法分析中被解析为 TokenType::小于
                if self.check(&TokenType::小于) {
                    self.position += 1;
                    let elem_type = Box::new(self.parse_type()?);
                    // 期望 > (在词法分析中被解析为 TokenType::大于)
                    self.expect(&TokenType::大于)?;
                    return Ok(Type::List(elem_type));
                } else {
                    // 无泛型参数，默认为整数列表
                    Type::List(Box::new(Type::Int))
                }
            }
            Some(Token { token_type: TokenType::Keyword(Keyword::或许), .. }) => {
                self.position += 1;
                // 解析泛型参数
                // 注意：< 在词法分析中被解析为 TokenType::小于
                self.expect(&TokenType::小于)?;
                let inner_type = Box::new(self.parse_type()?);
                self.expect(&TokenType::大于)?;
                return Ok(Type::Optional(inner_type));
            }
            // 支持关键字作为自定义类型名（如 Parser、AST节点 等）
            // XY编译器自展时会遇到这种情况
            Some(Token { token_type: TokenType::Keyword(_), literal, .. }) => {
                let name = literal.clone();
                self.position += 1;
                Type::Custom(name)
            }
            Some(Token { token_type: TokenType::标识符, literal, .. }) => {
                let name = literal.clone();
                self.position += 1;
                Type::Custom(name)
            }
            _ => {
                return Err(ParserError::unexpected_token(
                    "类型",
                    &self.current().map(|t| t.literal.clone()).unwrap_or_default(),
                    self.current().map(|t| t.span).unwrap_or(Span::dummy())
                ));
            }
        };

        // 检查数组类型
        if self.check(&TokenType::左方括号) {
            self.position += 1;
            self.expect(&TokenType::右方括号)?;
            return Ok(Type::Array(Box::new(base_type)));
        }

        Ok(base_type)
    }

    /**
     * 解析语句列表
     * 语句列表 -> 语句*
     */
    fn parse_statement_list(&mut self) -> Result<Vec<Stmt>, ParserError> {
        let mut statements = Vec::new();

        while !self.check(&TokenType::右花括号) && !self.check(&TokenType::文件结束) {
            statements.push(self.parse_statement()?);
        }

        Ok(statements)
    }

    /**
     * 解析语句
     */
    fn parse_statement(&mut self) -> Result<Stmt, ParserError> {
        let token = self.current()
            .ok_or_else(|| ParserError::unexpected_token_at(1, 1, "期望语句"))?;

        match &token.token_type {
            // 变量声明: 定义 x: 整数 = 10
            TokenType::Keyword(Keyword::定义) => self.parse_let_statement(),
            
            // 返回语句: 返回 expr
            TokenType::Keyword(Keyword::返回) => self.parse_return_statement(),
            
            // 条件语句: 若 expr 则 { ... } 否则 { ... }
            // 也支持: 如果 expr 则 { ... } 否则 { ... }
            TokenType::Keyword(Keyword::若) | TokenType::Keyword(Keyword::如果) => self.parse_if_statement(),
            
            // 循环语句: 当 expr 则 { ... }
            TokenType::Keyword(Keyword::当) => self.parse_while_statement(),
            
            // 循环语句: 循环 { ... }
            TokenType::Keyword(Keyword::循环) => self.parse_loop_statement(),

            // 跳出循环: 退出 或 跳出
            TokenType::Keyword(Keyword::退出) | TokenType::Keyword(Keyword::跳出) => self.parse_break_statement(),

            // 跳过循环: 跳过
            TokenType::Keyword(Keyword::跳过) => self.parse_continue_statement(),
            
            // 模式匹配: 匹配 expr { ... }
            TokenType::Keyword(Keyword::匹配) => self.parse_match_statement(),
            
            // 块语句: { ... }
            TokenType::左花括号 => self.parse_block_statement(),
            
            // 表达式语句
            _ => {
                let expr = self.parse_expression()?;
                self.match_token(&TokenType::分号); // 可选分号
                let span = expr.span();
                Ok(Stmt::Expr(ExprStmt::new(expr, span)))
            }
        }
    }

    /**
     * 解析模式匹配语句
     * 匹配 值 {
     *     情况 数字(n) => n,
     *     情况 加法(a, b) => a + b,
     *     默认 => 0
     * }
     */
    fn parse_match_statement(&mut self) -> Result<Stmt, ParserError> {
        let start_span = self.current()
            .map(|t| t.span)
            .unwrap_or(Span::dummy());
        
        // 消耗 '匹配' 关键字
        self.expect(&TokenType::Keyword(Keyword::匹配))?;
        
        // 解析要匹配的值
        let subject = self.parse_expression()?;
        
        // 期望 '{'
        self.expect(&TokenType::左花括号)?;
        
        // 解析匹配分支
        let mut arms = Vec::new();
        while !self.check(&TokenType::右花括号) {
            // 检查 '情况' 或 '默认'
            if self.check_keyword(&Keyword::情况) {
                self.advance(); // 消耗 '情况'
                
                // 解析枚举变体模式: 变体名(字段1, 字段2)
                let variant_name = match self.current() {
                    Some(Token { token_type: TokenType::标识符, literal, .. }) => {
                        literal.clone()
                    }
                    _ => {
                        return Err(ParserError::unexpected_token(
                            "枚举变体名",
                            &self.current().map(|t| t.literal.clone()).unwrap_or_default(),
                            self.current().map(|t| t.span).unwrap_or(Span::dummy())
                        ));
                    }
                };
                self.advance();
                
                // 检查是否有字段绑定: (x, y) 或 (左: a, 右: b)
                let mut fields = Vec::new();
                if self.check(&TokenType::左圆括号) {
                    self.advance(); // 消耗 '('
                    
                    while !self.check(&TokenType::右圆括号) {
                        // 检查命名字段: 标识符 ':' 标识符
                        let field_binding = if self.peek(1).map(|t| &t.token_type).unwrap_or(&TokenType::未知) == &TokenType::冒号 {
                            // 命名字段: 字段名 : 变量名
                            let field_name = match self.current() {
                                Some(Token { token_type: TokenType::标识符, literal, .. }) => {
                                    Some(literal.clone())
                                }
                                _ => None
                            };
                            self.advance();
                            self.expect(&TokenType::冒号)?;
                            // 变量名
                            let binding_name = match self.current() {
                                Some(Token { token_type: TokenType::标识符, literal, .. }) => {
                                    literal.clone()
                                }
                                _ => {
                                    return Err(ParserError::unexpected_token(
                                        "变量名",
                                        &self.current().map(|t| t.literal.clone()).unwrap_or_default(),
                                        self.current().map(|t| t.span).unwrap_or(Span::dummy())
                                    ));
                                }
                            };
                            self.advance();
                            MatchFieldBinding {
                                name: field_name,
                                binding_name,
                            }
                        } else {
                            // 位置参数: 变量名
                            let binding_name = match self.current() {
                                Some(Token { token_type: TokenType::标识符, literal, .. }) => {
                                    literal.clone()
                                }
                                _ => {
                                    return Err(ParserError::unexpected_token(
                                        "变量名",
                                        &self.current().map(|t| t.literal.clone()).unwrap_or_default(),
                                        self.current().map(|t| t.span).unwrap_or(Span::dummy())
                                    ));
                                }
                            };
                            self.advance();
                            MatchFieldBinding {
                                name: None,
                                binding_name,
                            }
                        };
                        
                        // 跳过逗号
                        if self.check(&TokenType::逗号) {
                            self.advance();
                        }
                        
                        fields.push(field_binding);
                    }
                    
                    self.expect(&TokenType::右圆括号)?; // 消耗 ')'
                }
                
                // 期望 '=>'
                self.expect(&TokenType::大于)?;
                self.expect(&TokenType::赋值)?;
                
                // 解析分支体
                let body = Box::new(self.parse_statement()?);
                
                arms.push(MatchArm {
                    pattern: MatchPattern::EnumVariant {
                        enum_name: String::new(), // 简化：稍后填充
                        variant_name,
                        fields,
                    },
                    body,
                });
                
            } else if self.check_keyword(&Keyword::默认) {
                self.advance(); // 消耗 '默认'
                
                // 期望 '=>'
                self.expect(&TokenType::大于)?;
                self.expect(&TokenType::赋值)?;
                
                // 解析分支体
                let body = Box::new(self.parse_statement()?);
                
                arms.push(MatchArm {
                    pattern: MatchPattern::Wildcard,
                    body,
                });
            } else {
                return Err(ParserError::unexpected_token(
                    "'情况' 或 '默认'",
                    &self.current().map(|t| t.literal.clone()).unwrap_or_default(),
                    self.current().map(|t| t.span).unwrap_or(Span::dummy())
                ));
            }
        }
        
        // 期望 '}'
        self.expect(&TokenType::右花括号)?;
        
        let end_span = self.previous().unwrap().span.clone();
        Ok(Stmt::Match(MatchStmt {
            subject,
            arms,
            span: start_span.merge(end_span),
        }))
    }

    /**
     * 解析变量声明语句
     * 定义 x: 整数 = 10
     */
    fn parse_let_statement(&mut self) -> Result<Stmt, ParserError> {
        let start_span = self.current()
            .map(|t| t.span)
            .unwrap_or(Span::dummy());

        // 消耗 '定义' 关键字
        self.position += 1;

        // 可变修饰符
        let _is_mutable = self.match_keyword(&Keyword::可变);

        // 变量名
        let name = match self.current() {
            Some(Token { token_type: TokenType::标识符, literal, .. }) => {
                literal.clone()
            }
            _ => {
                return Err(ParserError::unexpected_token(
                    "变量名",
                    &self.current().map(|t| t.literal.clone()).unwrap_or_default(),
                    self.current().map(|t| t.span).unwrap_or(Span::dummy())
                ));
            }
        };
        self.position += 1;

        // 可选类型标注
        let type_annotation = if self.match_token(&TokenType::冒号) {
            Some(self.parse_type()?)
        } else {
            None
        };

        // 可选初始化值
        let initializer = if self.match_token(&TokenType::赋值) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.match_token(&TokenType::分号); // 可选分号

        let span = start_span; // TODO: 合并完整 span
        
        // 创建 LetStmt (注意: 当前 AST 没有 is_mutable 字段)
        Ok(Stmt::Let(LetStmt::new(name, type_annotation, initializer, span)))
    }

    /**
     * 解析返回语句
     */
    fn parse_return_statement(&mut self) -> Result<Stmt, ParserError> {
        let start_span = self.current()
            .map(|t| t.span)
            .unwrap_or(Span::dummy());

        self.position += 1; // 消耗 '返回'

        let value = if self.check(&TokenType::分号) || self.check(&TokenType::右花括号) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.match_token(&TokenType::分号);

        Ok(Stmt::Return(ReturnStmt::new(value, start_span)))
    }

    /**
     * 解析条件语句
     * 若 条件 则 { ... } 否则 { ... }
     */
    fn parse_if_statement(&mut self) -> Result<Stmt, ParserError> {
        let start_span = self.current()
            .map(|t| t.span)
            .unwrap_or(Span::dummy());

        // 支持 '若' 和 '如果' 两种语法
        if !self.check_keyword(&Keyword::若) && !self.check_keyword(&Keyword::如果) {
            return Err(ParserError::unexpected_token_at(
                start_span.start_line,
                start_span.start_column,
                "期望 '若' 或 '如果'",
            ));
        }
        self.position += 1; // 消耗 '若' 或 '如果'

        let condition = self.parse_expression()?;

        // 期望 '则'
        self.expect(&TokenType::Keyword(Keyword::则))?;

        let then_branch = Box::new(self.parse_statement()?);

        // 检查 '否则' 或 '否则若'
        let else_branch = if self.match_keyword(&Keyword::否则) {
            if self.check_keyword(&Keyword::若) {
                // 否则若 - 递归解析
                Some(Box::new(self.parse_if_statement()?))
            } else {
                Some(Box::new(self.parse_statement()?))
            }
        } else {
            None
        };

        Ok(Stmt::If(IfStmt::new(
            vec![Branch { condition, body: then_branch }],
            else_branch,
            start_span
        )))
    }

    /**
     * 解析 while 循环
     * 当 条件 则 { ... }
     */
    fn parse_while_statement(&mut self) -> Result<Stmt, ParserError> {
        let start_span = self.current()
            .map(|t| t.span)
            .unwrap_or(Span::dummy());

        self.position += 1; // 消耗 '当'

        let condition = self.parse_expression()?;

        // 期望 '则'
        self.expect(&TokenType::Keyword(Keyword::则))?;

        let body = Box::new(self.parse_statement()?);

        Ok(Stmt::Loop(LoopStmt::new(
            LoopKind::While,
            Some(condition),
            None,
            None,
            body,
            start_span
        )))
    }

    /**
     * 解析循环语句
     * 循环 { ... }
     * 循环 从 i 到 10 { ... }
     * 循环 从 i 取自 集合 { ... }
     */
    fn parse_loop_statement(&mut self) -> Result<Stmt, ParserError> {
        let start_span = self.current()
            .map(|t| t.span)
            .unwrap_or(Span::dummy());

        self.position += 1; // 消耗 '循环'

        // 检查循环类型
        if self.match_keyword(&Keyword::从) {
            // 计数循环或遍历循环
            return self.parse_counted_or_for_loop(start_span);
        }

        // 无限循环: 循环 { ... }
        let body = Box::new(self.parse_statement()?);

        Ok(Stmt::Loop(LoopStmt::new(
            LoopKind::Infinite,
            None,
            None,
            None,
            body,
            start_span
        )))
    }

    /**
     * 解析计数循环或遍历循环
     * 循环 从 i 到 10 { ... }
     * 循环 从 item 取自 集合 { ... }
     * 循环 从 i 到 5 则 { ... }
     */
    fn parse_counted_or_for_loop(&mut self, start_span: Span) -> Result<Stmt, ParserError> {
        // 解析循环变量 (标识符)
        let var_name = match self.current() {
            Some(token) if token.token_type == TokenType::标识符 => {
                let name = token.literal.clone();
                self.position += 1;
                name
            }
            _ => {
                return Err(ParserError::unexpected_token_at(
                    self.current().map(|t| t.span.start_line).unwrap_or(1),
                    self.current().map(|t| t.span.start_column).unwrap_or(1),
                    "期望循环变量"
                ));
            }
        };

        // 检查是否有 '到' (计数循环) 或 '取自' (遍历)
        if self.match_keyword(&Keyword::到) {
            // 计数循环: 循环 从 i 到 10 { ... }
            let end = self.parse_expression()?;
            
            // 创建计数器初始化 (变量名, 起始值0, 结束值, 无步长)
            let counter = CounterInit {
                variable: var_name,
                start: Expr::Literal(LiteralExpr::new(LiteralKind::Integer(0), Span::dummy())),
                end,
                step: None,
            };
            
            // 可选的 '则' 关键字
            self.match_keyword(&Keyword::则);
            
            let body = Box::new(self.parse_statement()?);

            return Ok(Stmt::Loop(LoopStmt::new(
                LoopKind::Counted,
                None,
                Some(counter),
                None,
                body,
                start_span
            )));
        } else if self.match_keyword(&Keyword::取自) {
            // 遍历循环: 循环 从 item 取自 集合 { ... }
            let iterator = self.parse_expression()?;
            
            // 可选的 '则' 关键字
            self.match_keyword(&Keyword::则);
            
            let body = Box::new(self.parse_statement()?);

            return Ok(Stmt::Loop(LoopStmt::new(
                LoopKind::For,
                None,
                None,
                Some(iterator),
                body,
                start_span
            )));
        }

        Err(ParserError::unexpected_token_at(
            self.current().map(|t| t.span.start_line).unwrap_or(1),
            self.current().map(|t| t.span.start_column).unwrap_or(1),
            "期望 '到' 或 '取自'"
        ))
    }

    /**
     * 解析 break 语句
     */
    fn parse_break_statement(&mut self) -> Result<Stmt, ParserError> {
        let span = self.current()
            .map(|t| t.span)
            .unwrap_or(Span::dummy());

        self.position += 1;
        self.match_token(&TokenType::分号);

        Ok(Stmt::Break(BreakStmt::new(None, span)))
    }

    /**
     * 解析 continue 语句
     */
    fn parse_continue_statement(&mut self) -> Result<Stmt, ParserError> {
        let span = self.current()
            .map(|t| t.span)
            .unwrap_or(Span::dummy());

        self.position += 1;
        self.match_token(&TokenType::分号);

        Ok(Stmt::Continue(ContinueStmt::new(None, span)))
    }

    /**
     * 解析块语句
     */
    fn parse_block_statement(&mut self) -> Result<Stmt, ParserError> {
        let start_span = self.current()
            .map(|t| t.span)
            .unwrap_or(Span::dummy());

        self.position += 1; // 消耗 '{'

        let statements = self.parse_statement_list()?;

        self.expect(&TokenType::右花括号)?;

        Ok(Stmt::Block(BlockStmt::new(statements, start_span)))
    }

    /**
     * 解析表达式
     * 使用运算符优先级解析
     */
    fn parse_expression(&mut self) -> Result<Expr, ParserError> {
        self.parse_assignment_expression()
    }

    /**
     * 解析赋值表达式
     */
    fn parse_assignment_expression(&mut self) -> Result<Expr, ParserError> {
        let left = self.parse_or_expression()?;

        if self.match_token(&TokenType::赋值) {
            let right = self.parse_assignment_expression()?;
            let span = left.span().merge(right.span());
            return Ok(Expr::Binary(BinaryExpr::new(
                BinaryOp::Assign,
                Box::new(left),
                Box::new(right),
                span
            )));
        }

        Ok(left)
    }

    /**
     * 解析逻辑或 (||)
     */
    fn parse_or_expression(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_and_expression()?;

        while self.match_keyword(&Keyword::或) {
            let right = self.parse_and_expression()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary(BinaryExpr::new(
                BinaryOp::Or,
                Box::new(left),
                Box::new(right),
                span
            ));
        }

        Ok(left)
    }

    /**
     * 解析逻辑与 (&&)
     * 支持: 与, 且
     */
    fn parse_and_expression(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_equality_expression()?;

        while self.match_token(&TokenType::与) || self.match_keyword(&Keyword::且) {
            let right = self.parse_equality_expression()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary(BinaryExpr::new(
                BinaryOp::And,
                Box::new(left),
                Box::new(right),
                span
            ));
        }

        Ok(left)
    }

    /**
     * 解析相等性比较 (==, !=)
     */
    fn parse_equality_expression(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_comparison_expression()?;

        while self.match_token(&TokenType::等于) {
            let right = self.parse_comparison_expression()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary(BinaryExpr::new(
                BinaryOp::Eq,
                Box::new(left),
                Box::new(right),
                span
            ));
        } while self.match_token(&TokenType::不等于) {
            let right = self.parse_comparison_expression()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary(BinaryExpr::new(
                BinaryOp::Ne,
                Box::new(left),
                Box::new(right),
                span
            ));
        }

        Ok(left)
    }

    /**
     * 解析比较运算 (>, <, >=, <=)
     */
    fn parse_comparison_expression(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_additive_expression()?;

        while self.match_token(&TokenType::大于) {
            let right = self.parse_additive_expression()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary(BinaryExpr::new(
                BinaryOp::Gt,
                Box::new(left),
                Box::new(right),
                span
            ));
        } while self.match_token(&TokenType::小于) {
            let right = self.parse_additive_expression()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary(BinaryExpr::new(
                BinaryOp::Lt,
                Box::new(left),
                Box::new(right),
                span
            ));
        } while self.match_token(&TokenType::大于等于) {
            let right = self.parse_additive_expression()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary(BinaryExpr::new(
                BinaryOp::Ge,
                Box::new(left),
                Box::new(right),
                span
            ));
        } while self.match_token(&TokenType::小于等于) {
            let right = self.parse_additive_expression()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary(BinaryExpr::new(
                BinaryOp::Le,
                Box::new(left),
                Box::new(right),
                span
            ));
        }

        Ok(left)
    }

    /**
     * 解析加法运算 (+, -, <<, >>)
     */
    fn parse_additive_expression(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_shift_expression()?;

        while self.match_token(&TokenType::加) {
            let right = self.parse_shift_expression()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary(BinaryExpr::new(
                BinaryOp::Add,
                Box::new(left),
                Box::new(right),
                span
            ));
        } while self.match_token(&TokenType::减) {
            let right = self.parse_shift_expression()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary(BinaryExpr::new(
                BinaryOp::Sub,
                Box::new(left),
                Box::new(right),
                span
            ));
        }

        Ok(left)
    }

    /**
     * 解析移位运算 (<<, >>)
     */
    fn parse_shift_expression(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_bitwise_expression()?;

        while self.match_token(&TokenType::左移) {
            let right = self.parse_bitwise_expression()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary(BinaryExpr::new(
                BinaryOp::Shl,
                Box::new(left),
                Box::new(right),
                span
            ));
        }

        while self.match_token(&TokenType::右移) {
            let right = self.parse_bitwise_expression()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary(BinaryExpr::new(
                BinaryOp::Shr,
                Box::new(left),
                Box::new(right),
                span
            ));
        }

        Ok(left)
    }

    /**
     * 解析位运算 (&, |, ^)
     */
    fn parse_bitwise_expression(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_multiplicative_expression()?;

        while self.match_token(&TokenType::位与) {
            let right = self.parse_multiplicative_expression()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary(BinaryExpr::new(
                BinaryOp::BitAnd,
                Box::new(left),
                Box::new(right),
                span
            ));
        }

        while self.match_token(&TokenType::位异或) {
            let right = self.parse_multiplicative_expression()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary(BinaryExpr::new(
                BinaryOp::BitXor,
                Box::new(left),
                Box::new(right),
                span
            ));
        }

        while self.match_token(&TokenType::位或) {
            let right = self.parse_multiplicative_expression()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary(BinaryExpr::new(
                BinaryOp::BitOr,
                Box::new(left),
                Box::new(right),
                span
            ));
        }

        while self.match_token(&TokenType::井号) {
            let right = self.parse_multiplicative_expression()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary(BinaryExpr::new(
                BinaryOp::Hash,
                Box::new(left),
                Box::new(right),
                span
            ));
        }

        Ok(left)
    }

    /**
     * 解析乘法运算 (*, /, %)
     */
    fn parse_multiplicative_expression(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_unary_expression()?;

        while self.match_token(&TokenType::乘) {
            let right = self.parse_unary_expression()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary(BinaryExpr::new(
                BinaryOp::Mul,
                Box::new(left),
                Box::new(right),
                span
            ));
        } while self.match_token(&TokenType::除) {
            let right = self.parse_unary_expression()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary(BinaryExpr::new(
                BinaryOp::Div,
                Box::new(left),
                Box::new(right),
                span
            ));
        } while self.match_token(&TokenType::取余) {
            let right = self.parse_unary_expression()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary(BinaryExpr::new(
                BinaryOp::Rem,
                Box::new(left),
                Box::new(right),
                span
            ));
        }

        Ok(left)
    }

    /**
     * 解析一元表达式 (!, -, ~)
     */
    fn parse_unary_expression(&mut self) -> Result<Expr, ParserError> {
        if self.match_token(&TokenType::非) {
            let operand = Box::new(self.parse_unary_expression()?);
            let span = operand.span();
            return Ok(Expr::Unary(UnaryExpr::new(UnaryOp::Not, operand, span)));
        }
        
        if self.match_token(&TokenType::减) {
            let operand = Box::new(self.parse_unary_expression()?);
            let span = operand.span();
            return Ok(Expr::Unary(UnaryExpr::new(UnaryOp::Neg, operand, span)));
        }

        if self.match_token(&TokenType::位非) {
            let operand = Box::new(self.parse_unary_expression()?);
            let span = operand.span();
            return Ok(Expr::Unary(UnaryExpr::new(UnaryOp::BitNot, operand, span)));
        }

        self.parse_postfix_expression()
    }

    /**
     * 解析后缀表达式 (函数调用, 成员访问, 数组访问)
     */
    fn parse_postfix_expression(&mut self) -> Result<Expr, ParserError> {
        let mut expr = self.parse_primary_expression()?;

        loop {
            // 函数调用: expr '(' args ')'
            if self.match_token(&TokenType::左圆括号) {
                let mut arguments = Vec::new();
                
                if !self.check(&TokenType::右圆括号) {
                    arguments.push(self.parse_expression()?);
                    while self.match_token(&TokenType::逗号) {
                        arguments.push(self.parse_expression()?);
                    }
                }

                self.expect(&TokenType::右圆括号)?;
                let span = expr.span();
                expr = Expr::Call(CallExpr::new(Box::new(expr), arguments, span));
            }
            // 成员访问: expr '.' 标识符
            else if self.match_token(&TokenType::句号) {
                let member = match self.current() {
                    Some(Token { token_type: TokenType::标识符, literal, .. }) => {
                        literal.clone()
                    }
                    Some(Token { token_type: TokenType::Keyword(_), literal, .. }) => {
                        literal.clone()
                    }
                    _ => {
                        return Err(ParserError::unexpected_token(
                            "成员名",
                            &self.current().map(|t| t.literal.clone()).unwrap_or_default(),
                            self.current().map(|t| t.span).unwrap_or(Span::dummy())
                        ));
                    }
                };
                self.position += 1;
                let span = expr.span();
                expr = Expr::MemberAccess(MemberAccessExpr::new(Box::new(expr), member, span));
            }
            // 列表/数组索引访问: expr '[' expr ']'
            else if self.match_token(&TokenType::左方括号) {
                let index = self.parse_expression()?;
                self.expect(&TokenType::右方括号)?;
                let span = expr.span();
                expr = Expr::IndexAccess(IndexAccessExpr::new(Box::new(expr), Box::new(index), span));
            }
            else {
                break;
            }
        }

        Ok(expr)
    }

    /**
     * 解析基本表达式 (字面量, 标识符, 分组)
     */
    fn parse_primary_expression(&mut self) -> Result<Expr, ParserError> {
        let token = self.current()
            .ok_or_else(|| ParserError::unexpected_token_at(1, 1, "期望表达式"))?;

        let result = match &token.token_type {
            // 标识符
            TokenType::标识符 => {
                let name = token.literal.clone();
                let span = token.span;
                self.position += 1;
                Ok(Expr::Identifier(IdentifierExpr::new(name, span)))
            }

            // Lambda 表达式: 函数(参数) => 表达式
            // 例如: 函数(x, y) => x + y
            TokenType::Keyword(Keyword::函数) => {
                let span = token.span.clone();
                self.expect(&TokenType::Keyword(Keyword::函数))?;

                // 解析参数列表
                self.expect(&TokenType::左圆括号)?;
                let params = self.parse_parameter_list()?;
                self.expect(&TokenType::右圆括号)?;

                // 期望箭头符号 =>
                self.expect(&TokenType::箭头)?;

                // 解析函数体表达式
                let body = self.parse_expression()?;

                Ok(Expr::Lambda(LambdaExpr::new(params, Box::new(body), span)))
            }

            // 整数字面量
            TokenType::整数字面量 => {
                let value: i64 = token.literal.parse()
                    .unwrap_or(0);
                let span = token.span;
                self.position += 1;
                Ok(Expr::Literal(LiteralExpr::new(
                    LiteralKind::Integer(value),
                    span
                )))
            }

            // 浮点数字面量
            TokenType::浮点字面量 => {
                let value: f64 = token.literal.parse()
                    .unwrap_or(0.0);
                let span = token.span;
                self.position += 1;
                Ok(Expr::Literal(LiteralExpr::new(
                    LiteralKind::Float(value),
                    span
                )))
            }

            // 文本字面量
            TokenType::文本字面量 => {
                let value = token.literal.clone();
                let span = token.span;
                self.position += 1;
                Ok(Expr::Literal(LiteralExpr::new(
                    LiteralKind::String(value),
                    span
                )))
            }

            // 字符字面量
            TokenType::字符字面量 => {
                let ch = token.literal.chars().next().unwrap_or('\0');
                let span = token.span;
                self.position += 1;
                Ok(Expr::Literal(LiteralExpr::new(
                    LiteralKind::Char(ch),
                    span
                )))
            }

            // 布尔字面量
            TokenType::布尔字面量 => {
                let value = token.literal == "真";
                let span = token.span;
                self.position += 1;
                Ok(Expr::Literal(LiteralExpr::new(
                    LiteralKind::Boolean(value),
                    span
                )))
            }

            // 分组表达式: '(' expr ')'
            TokenType::左圆括号 => {
                self.position += 1;
                let expr = self.parse_expression()?;
                self.expect(&TokenType::右圆括号)?;
                Ok(Expr::Grouped(Box::new(expr)))
            }

            // 列表字面量: '[' expr1, expr2, ... ']' 或列表推导式: '[' expr for x in list ']'
            TokenType::左方括号 => {
                let start_span = token.span;
                self.position += 1;
                
                // 空列表
                if self.check(&TokenType::右方括号) {
                    self.position += 1;
                    return Ok(Expr::ListLiteral(ListLiteralExpr::new(
                        Vec::new(),
                        start_span
                    )));
                }
                
                // 解析第一个表达式
                let first_elem = self.parse_expression()?;
                
                // 检查是否是列表推导式 (for x in list)
                if self.check_keyword(&Keyword::遍历) {
                    self.position += 1;  // 消费 '遍历'
                    
                    // 解析迭代变量
                    let var_token = self.current().cloned();
                    if let Some(ref token) = var_token {
                        if token.token_type != TokenType::标识符 {
                            return Err(ParserError::unexpected_token(
                                "标识符",
                                &format!("{:?}", token.token_type),
                                token.span
                            ));
                        }
                    } else {
                        return Err(ParserError::unexpected_token_at(0, 0, "列表推导式需要迭代变量名"));
                    }
                    let var_name = var_token.unwrap().literal.clone();
                    self.position += 1;
                    
                    // 期望 '在'
                    if !self.match_keyword(&Keyword::在) {
                        return Err(ParserError::unexpected_token("在", "其他", self.current().map(|t| t.span).unwrap_or(Span::dummy())));
                    }
                    
                    // 解析迭代列表
                    let iterable = Box::new(self.parse_expression()?);
                    
                    // 可选的条件过滤 (当 ...)
                    let condition = if self.check_keyword(&Keyword::当) {
                        self.position += 1;
                        Some(Box::new(self.parse_expression()?))
                    } else {
                        None
                    };
                    
                    self.expect(&TokenType::右方括号)?;
                    
                    let end_span = self.previous()
                        .map(|t| t.span)
                        .unwrap_or(start_span);
                    
                    return Ok(Expr::ListComprehension(ListComprehensionExpr::new(
                        Box::new(first_elem),
                        var_name,
                        iterable,
                        condition,
                        start_span.merge(end_span)
                    )));
                }
                
                // 普通列表字面量
                let mut elements = vec![first_elem];
                
                while self.match_token(&TokenType::逗号) {
                    let elem = self.parse_expression()?;
                    elements.push(elem);
                }
                
                self.expect(&TokenType::右方括号)?;
                
                let end_span = self.previous()
                    .map(|t| t.span)
                    .unwrap_or(start_span);
                
                Ok(Expr::ListLiteral(ListLiteralExpr::new(
                    elements,
                    start_span.merge(end_span)
                )))
            }

            // 类型关键字作为构造函数: 列表(), 整数(), 文本() 等
            // 也作为普通标识符使用（如函数参数名）
            TokenType::Keyword(_keyword) => {
                // 使用 token.literal 而不是 format!("{:?}", keyword)
                // 因为 "类型别名" 的 literal 是 "类型"，而 Debug 格式是 "类型别名"
                let name = token.literal.clone();
                let span = token.span;
                self.position += 1;
                
                // 检查是否是函数调用
                if self.check(&TokenType::左圆括号) {
                    self.position += 1; // 消耗 '('
                    
                    let mut arguments = Vec::new();
                    if !self.check(&TokenType::右圆括号) {
                        arguments.push(self.parse_expression()?);
                        
                        while self.match_token(&TokenType::逗号) {
                            arguments.push(self.parse_expression()?);
                        }
                    }
                    
                    self.expect(&TokenType::右圆括号)?;
                    
                    let end_span = self.previous()
                        .map(|t| t.span)
                        .unwrap_or(span);
                    
                    Ok(Expr::Call(CallExpr::new(
                        Box::new(Expr::Identifier(IdentifierExpr::new(name, span))),
                        arguments,
                        span.merge(end_span)
                    )))
                } else {
                    // 否则作为标识符处理
                    Ok(Expr::Identifier(IdentifierExpr::new(name, span)))
                }
            }

            // 未知 token
            _ => {
                Err(ParserError::unexpected_token(
                    "表达式",
                    &token.literal,
                    token.span
                ))
            }
        };
        
        result
    }
}

/**
 * 解析辅助函数
 */
pub fn parse(tokens: Vec<Token>) -> Result<Module, ParserError> {
    let mut parser = Parser::new(tokens);
    parser.parse_module()
}
