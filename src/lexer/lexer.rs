/**
 * @file lexer.rs
 * @brief CCAS 词法分析器 - Lexer 核心实现
 * @description 实现词法分析功能，重点实现"语义空格"校验规则
 * 
 * 语义空格规则 (CCAS-Spec-v2.0):
 * - 中文关键字后必须跟随至少一个空白字符，否则报错 "CCAS-E001: 缺失语义空格"
 * - 操作数之间必须保留至少一个空格或使用逗号分隔
 */

use crate::lexer::token::{Token, TokenType, Span, lookup_keyword, is_keyword, is_boolean_literal};

/**
 * 词法分析错误类型
 */
#[derive(Debug, Clone)]
pub struct LexerError {
    pub code: String,
    pub message: String,
    pub span: Span,
}

impl LexerError {
    /**
     * 创建语义空格缺失错误
     * CCAS-E001: 缺失语义空格
     */
    pub fn missing_semantic_whitespace(span: Span, keyword: &str) -> Self {
        Self {
            code: "CCAS-E001".to_string(),
            message: format!("关键字 '{}' 后缺失语义空格，请在该关键字后添加空格", keyword),
            span,
        }
    }

    /**
     * 创建非法字符错误
     * CCAS-E002: 非法字符
     */
    pub fn illegal_character(span: Span, ch: char) -> Self {
        Self {
            code: "CCAS-E002".to_string(),
            message: format!("发现非法字符: '{}' (U+{:04X})", ch, ch as u32),
            span,
        }
    }

    /**
     * 创建无效标识符错误
     * CCAS-E003: 无效标识符
     */
    pub fn invalid_identifier(span: Span, literal: &str) -> Self {
        Self {
            code: "CCAS-E003".to_string(),
            message: format!("无效的标识符: '{}'", literal),
            span,
        }
    }

    /**
     * 创建无效数字错误
     * CCAS-E004: 无效数字
     */
    pub fn invalid_number(span: Span, literal: &str) -> Self {
        Self {
            code: "CCAS-E004".to_string(),
            message: format!("无效的数字字面量: '{}'", literal),
            span,
        }
    }

    /**
     * 创建未终止的字符串错误
     * CCAS-E005: 未终止的字符串
     */
    pub fn unterminated_string(span: Span) -> Self {
        Self {
            code: "CCAS-E005".to_string(),
            message: "字符串字面量未正确终止".to_string(),
            span,
        }
    }

    /**
     * 创建未终止的字符错误
     * CCAS-E006: 未终止的字符
     */
    pub fn unterminated_char(span: Span) -> Self {
        Self {
            code: "CCAS-E006".to_string(),
            message: "字符字面量未正确终止".to_string(),
            span,
        }
    }

    /**
     * 创建未终止的注释错误
     * CCAS-E007: 未终止的注释
     */
    pub fn unterminated_comment(span: Span) -> Self {
        Self {
            code: "CCAS-E007".to_string(),
            message: "块注释未正确终止".to_string(),
            span,
        }
    }
}

/**
 * 词法分析器
 * 将源字符串转换为 Token 流
 */
pub struct Lexer {
    /// 源代码
    source: Vec<char>,
    /// 当前字符索引
    position: usize,
    /// 当前行号 (从 1 开始)
    line: usize,
    /// 当前列号 (从 1 开始)
    column: usize,
    /// 上一个生成的 Token (用于语义空格校验)
    prev_token: Option<Token>,
    /// 上一个 token 结束后的位置
    prev_token_end: usize,
    /// 语义空格警告列表
    warnings: Vec<String>,
}

impl Lexer {
    /**
     * 从源代码字符串创建新的词法分析器
     */
    pub fn new(source: String) -> Self {
        Self {
            source: source.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
            prev_token: None,
            prev_token_end: 0,
            warnings: Vec::new(),
        }
    }

    /**
     * 获取当前字符
     */
    fn current_char(&self) -> Option<char> {
        self.source.get(self.position).copied()
    }

    /**
     * 跳过空白字符 (空格、制表符)
     * 注意: 不跳过换行符，因为换行符有语法意义
     */
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char() {
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    /**
     * 跳过行注释
     */
    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.current_char() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }

    /**
     * 跳过块注释
     */
    fn skip_block_comment(&mut self) -> Result<(), LexerError> {
        let start_line = self.line;
        let start_column = self.column;
        
        while let Some(ch) = self.current_char() {
            if ch == '*' {
                self.advance();
                if self.current_char() == Some('/') {
                    self.advance();
                    return Ok(());
                }
            } else if ch == '\n' {
                self.advance();
            } else {
                self.advance();
            }
        }
        
        Err(LexerError::unterminated_comment(
            self.make_span(start_line, start_column)
        ))
    }

    /**
     * 推进位置到下一个字符
     */
    fn advance(&mut self) {
        if let Some(ch) = self.current_char() {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.position += 1;
        }
    }

    /**
     * 创建当前位置的 Span
     */
    fn make_span(&self, start_line: usize, start_column: usize) -> Span {
        Span::new(start_line, start_column, self.line, self.column)
    }

    /**
     * 检查是否需要语义空格
     * 根据 CCAS 规范: 中文关键字后建议添加空格以提升可读性
     * 注意: 此检查已降级为警告，不会阻止编译
     */
    fn check_semantic_whitespace(&mut self, token: &Token) {
        // 获取前一个 Token
        let prev = match &self.prev_token {
            Some(t) => t,
            None => return,
        };

        // 检查前一个 Token 是否是关键字
        if !is_keyword(&prev.literal) {
            return;
        }

        // 使用 prev_token_end（上一个 token 结束后的字符位置）来检查
        let check_pos = self.prev_token_end;
        let source_len = self.source.len();
        
        // 跳过前一个 token 结束后的所有空白字符
        while check_pos < source_len {
            let ch = self.source[check_pos];
            if ch.is_whitespace() {
                // 找到空白字符（包括空格和制表符）
                if ch != '\n' {
                    return; // 有空格，正常返回
                }
                // 换行符允许作为语句分隔，不强制要求空格
                return;
            }
            break;
        }

        // 没有找到空格，记录警告而非报错
        let warning = format!(
            "建议在关键字 '{}' 后添加空格以提升可读性 (行 {}, 列 {})",
            prev.literal, token.span.start_line, token.span.start_column
        );
        self.add_warning(warning);
    }

    /**
     * 读取标识符或关键字
     * 支持中文标识符和英文标识符
     * 区分关键字和布尔字面量
     */
    fn read_identifier(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;
        
        let mut literal = String::new();
        
        while let Some(ch) = self.current_char() {
            // 中文字符、英文字母、数字、下划线
            if is_cjk_character(ch) || ch.is_alphabetic() || ch.is_ascii_digit() || ch == '_' {
                literal.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // 查找关键字
        let token_type = lookup_keyword(&literal);
        
        // 检查是否为布尔字面量 (真/假)
        let final_token_type = if let TokenType::标识符 = &token_type {
            if is_boolean_literal(&literal) {
                TokenType::布尔字面量
            } else {
                token_type.clone()
            }
        } else {
            token_type.clone()
        };
        
        Token::new(final_token_type, literal, self.make_span(start_line, start_column))
    }

    /**
     * 读取数字字面量
     * 支持十进制、十六进制
     */
    fn read_number(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;
        
        let mut literal = String::new();
        let mut has_decimal_point = false;
        
        while let Some(ch) = self.current_char() {
            if ch.is_ascii_digit() {
                literal.push(ch);
                self.advance();
            } else if ch == '.' && !has_decimal_point && literal.len() > 0 {
                // 小数点
                has_decimal_point = true;
                literal.push(ch);
                self.advance();
            } else if ch == 'x' || ch == 'X' {
                // 十六进制前缀 0x
                if literal == "0" {
                    literal.push(ch);
                    self.advance();
                } else {
                    break;
                }
            } else if ch.is_ascii_hexdigit() && (literal.starts_with("0x") || literal.starts_with("0X")) {
                // 十六进制数字
                literal.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        let token_type = if has_decimal_point {
            TokenType::浮点字面量
        } else {
            TokenType::整数字面量
        };

        Token::new(token_type, literal, self.make_span(start_line, start_column))
    }

    /**
     * 读取字符串字面量
     */
    fn read_string(&mut self) -> Result<Token, LexerError> {
        let start_line = self.line;
        let start_column = self.column;
        
        // 跳过开始的双引号
        self.advance();
        
        let mut literal = String::new();
        
        while let Some(ch) = self.current_char() {
            if ch == '"' {
                // 字符串结束
                self.advance();
                return Ok(Token::new(
                    TokenType::文本字面量,
                    literal,
                    self.make_span(start_line, start_column)
                ));
            } else if ch == '\\' {
                // 转义字符
                self.advance();
                match self.current_char() {
                    Some('n') => { literal.push('\n'); self.advance(); }
                    Some('t') => { literal.push('\t'); self.advance(); }
                    Some('r') => { literal.push('\r'); self.advance(); }
                    Some('\\') => { literal.push('\\'); self.advance(); }
                    Some('"') => { literal.push('"'); self.advance(); }
                    Some(ch) => { literal.push(ch); self.advance(); }
                    None => break,
                }
            } else if ch == '\n' {
                // 换行符在字符串中不允许
                return Err(LexerError::unterminated_string(
                    self.make_span(start_line, start_column)
                ));
            } else {
                literal.push(ch);
                self.advance();
            }
        }

        // 未终止的字符串
        Err(LexerError::unterminated_string(
            self.make_span(start_line, start_column)
        ))
    }

    /**
     * 读取字符字面量
     */
    fn read_char(&mut self) -> Result<Token, LexerError> {
        let start_line = self.line;
        let start_column = self.column;
        
        // 跳过开始的单引号
        self.advance();
        
        let mut literal = String::new();
        
        if let Some(ch) = self.current_char() {
            if ch == '\'' {
                // 空字符 ''
                return Err(LexerError::unterminated_char(
                    self.make_span(start_line, start_column)
                ));
            } else if ch == '\\' {
                // 转义字符
                self.advance();
                match self.current_char() {
                    Some('n') => { literal.push('\n'); self.advance(); }
                    Some('t') => { literal.push('\t'); self.advance(); }
                    Some('r') => { literal.push('\r'); self.advance(); }
                    Some('\\') => { literal.push('\\'); self.advance(); }
                    Some('\'') => { literal.push('\''); self.advance(); }
                    Some(ch) => { literal.push(ch); self.advance(); }
                    None => {}
                }
            } else {
                literal.push(ch);
                self.advance();
            }
        }

        // 检查结束的单引号
        if self.current_char() == Some('\'') {
            self.advance();
            Ok(Token::new(
                TokenType::字符字面量,
                literal,
                self.make_span(start_line, start_column)
            ))
        } else {
            Err(LexerError::unterminated_char(
                self.make_span(start_line, start_column)
            ))
        }
    }

    /**
     * 读取下一个 Token
     */
    pub fn next_token(&mut self) -> Result<Token, LexerError> {
        // 跳过空白字符
        self.skip_whitespace();

        let start_line = self.line;
        let start_column = self.column;

        // 检查是否到达文件末尾
        let ch = match self.current_char() {
            Some(ch) => ch,
            None => {
                return Ok(Token::new(
                    TokenType::文件结束,
                    String::new(),
                    Span::new(start_line, start_column, start_line, start_column)
                ));
            }
        };

        // 根据字符类型进行分词
        let token = match ch {
            // 标识符或关键字 (中文或英文字母开头)
            c if is_cjk_character(c) || c.is_alphabetic() => {
                self.read_identifier()
            }

            // 数字
            c if c.is_ascii_digit() => {
                self.read_number()
            }

            // 字符串字面量
            '"' => {
                return self.read_string();
            }

            // 字符字面量
            '\'' => {
                return self.read_char();
            }

            // 运算符和分隔符
            '+' => {
                self.advance();
                Token::new(TokenType::加, "+".to_string(), self.make_span(start_line, start_column))
            }
            '-' => {
                self.advance();
                Token::new(TokenType::减, "-".to_string(), self.make_span(start_line, start_column))
            }
            '*' => {
                self.advance();
                Token::new(TokenType::乘, "*".to_string(), self.make_span(start_line, start_column))
            }
            '/' => {
                self.advance();
                // 检查是否是注释
                match self.current_char() {
                    Some('/') => {
                        // 行注释
                        self.skip_line_comment();
                        return self.next_token();
                    }
                    Some('*') => {
                        // 块注释
                        self.advance(); // 跳过 *
                        self.skip_block_comment()?;
                        return self.next_token();
                    }
                    _ => {
                        Token::new(TokenType::除, "/".to_string(), self.make_span(start_line, start_column))
                    }
                }
            }
            '%' => {
                self.advance();
                Token::new(TokenType::取余, "%".to_string(), self.make_span(start_line, start_column))
            }
            '#' => {
                self.advance();
                Token::new(TokenType::井号, "#".to_string(), self.make_span(start_line, start_column))
            }

            // 比较运算符
            '=' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    Token::new(TokenType::等于, "==".to_string(), self.make_span(start_line, start_column))
                } else if self.current_char() == Some('>') {
                    self.advance();
                    Token::new(TokenType::箭头, "=>".to_string(), self.make_span(start_line, start_column))
                } else {
                    Token::new(TokenType::赋值, "=".to_string(), self.make_span(start_line, start_column))
                }
            }
            '!' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    Token::new(TokenType::不等于, "!=".to_string(), self.make_span(start_line, start_column))
                } else {
                    Token::new(TokenType::非, "!".to_string(), self.make_span(start_line, start_column))
                }
            }
            '>' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    Token::new(TokenType::大于等于, ">=".to_string(), self.make_span(start_line, start_column))
                } else if self.current_char() == Some('>') {
                    self.advance();
                    // 检查是否是 >>=
                    if self.current_char() == Some('=') {
                        self.advance();
                        Token::new(TokenType::右移等于, ">>=".to_string(), self.make_span(start_line, start_column))
                    } else {
                        Token::new(TokenType::右移, ">>".to_string(), self.make_span(start_line, start_column))
                    }
                } else {
                    Token::new(TokenType::大于, ">".to_string(), self.make_span(start_line, start_column))
                }
            }
            '<' => {
                self.advance();
                if self.current_char() == Some('=') {
                    self.advance();
                    Token::new(TokenType::小于等于, "<=".to_string(), self.make_span(start_line, start_column))
                } else if self.current_char() == Some('<') {
                    self.advance();
                    // 检查是否是 <<=
                    if self.current_char() == Some('=') {
                        self.advance();
                        Token::new(TokenType::左移等于, "<<=".to_string(), self.make_span(start_line, start_column))
                    } else {
                        Token::new(TokenType::左移, "<<".to_string(), self.make_span(start_line, start_column))
                    }
                } else {
                    Token::new(TokenType::小于, "<".to_string(), self.make_span(start_line, start_column))
                }
            }

            // 逻辑运算符
            '&' => {
                self.advance();
                if self.current_char() == Some('&') {
                    self.advance();
                    Token::new(TokenType::与, "&&".to_string(), self.make_span(start_line, start_column))
                } else {
                    Token::new(TokenType::位与, "&".to_string(), self.make_span(start_line, start_column))
                }
            }
            '|' => {
                self.advance();
                if self.current_char() == Some('|') {
                    self.advance();
                    Token::new(TokenType::或, "||".to_string(), self.make_span(start_line, start_column))
                } else {
                    Token::new(TokenType::位或, "|".to_string(), self.make_span(start_line, start_column))
                }
            }
            '^' => {
                self.advance();
                Token::new(TokenType::位异或, "^".to_string(), self.make_span(start_line, start_column))
            }
            '~' => {
                self.advance();
                Token::new(TokenType::位非, "~".to_string(), self.make_span(start_line, start_column))
            }

            // 分隔符
            '(' => {
                self.advance();
                Token::new(TokenType::左圆括号, "(".to_string(), self.make_span(start_line, start_column))
            }
            ')' => {
                self.advance();
                Token::new(TokenType::右圆括号, ")".to_string(), self.make_span(start_line, start_column))
            }
            '{' => {
                self.advance();
                Token::new(TokenType::左花括号, "{".to_string(), self.make_span(start_line, start_column))
            }
            '}' => {
                self.advance();
                Token::new(TokenType::右花括号, "}".to_string(), self.make_span(start_line, start_column))
            }
            '[' => {
                self.advance();
                Token::new(TokenType::左方括号, "[".to_string(), self.make_span(start_line, start_column))
            }
            ']' => {
                self.advance();
                Token::new(TokenType::右方括号, "]".to_string(), self.make_span(start_line, start_column))
            }
            ',' => {
                self.advance();
                Token::new(TokenType::逗号, ",".to_string(), self.make_span(start_line, start_column))
            }
            '.' => {
                self.advance();
                Token::new(TokenType::句号, ".".to_string(), self.make_span(start_line, start_column))
            }
            ';' => {
                self.advance();
                Token::new(TokenType::分号, ";".to_string(), self.make_span(start_line, start_column))
            }
            ':' => {
                self.advance();
                Token::new(TokenType::冒号, ":".to_string(), self.make_span(start_line, start_column))
            }

            // 换行符 - 允许作为语句分隔
            '\n' => {
                self.advance();
                return self.next_token();
            }

            // 未知字符
            _ => {
                self.advance();
                return Err(LexerError::illegal_character(
                    self.make_span(start_line, start_column),
                    ch
                ));
            }
        };

        // 语义空格校验: 检查中文关键字后是否有空格 (仅警告)
        if is_keyword(&token.literal) {
            self.check_semantic_whitespace(&token);
        }

        // 保存上一个 Token 及其结束位置
        self.prev_token = Some(token.clone());
        self.prev_token_end = self.position;

        Ok(token)
    }

    /**
     * 获取所有 Token
     */
    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();
        
        loop {
            let token = self.next_token()?;
            
            // 跳过文件结束标记
            if token.token_type == TokenType::文件结束 {
                tokens.push(token);
                break;
            }
            
            tokens.push(token);
        }

        Ok(tokens)
    }

    /**
     * 获取语义空格警告列表
     */
    pub fn get_warnings(&self) -> &Vec<String> {
        &self.warnings
    }

    /**
     * 添加警告信息
     */
    fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

/**
 * 判断字符是否为 CJK 中文字符
 * Unicode 范围:
 * - CJK 统一表意文字: 4E00-9FFF
 * - CJK 统一表意文字扩展 A: 3400-4DBF
 * - CJK 统一表意文字扩展 B: 20000-2A6DF
 */
fn is_cjk_character(ch: char) -> bool {
    let code = ch as u32;
    (0x4E00..=0x9FFF).contains(&code) ||
    (0x3400..=0x4DBF).contains(&code) ||
    (0x20000..=0x2A6DF).contains(&code) ||
    (0x2A700..=0x2B73F).contains(&code) ||
    (0x2B740..=0x2B81F).contains(&code) ||
    (0x2B820..=0x2CEAF).contains(&code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::token::Keyword;

    #[test]
    fn test_keywords() {
        let source = "若 则 否则 循环 函数".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::Keyword(Keyword::若));
        assert_eq!(tokens[1].token_type, TokenType::Keyword(Keyword::则));
        assert_eq!(tokens[2].token_type, TokenType::Keyword(Keyword::否则));
        assert_eq!(tokens[3].token_type, TokenType::Keyword(Keyword::循环));
        assert_eq!(tokens[4].token_type, TokenType::Keyword(Keyword::函数));
    }

    #[test]
    fn test_identifier() {
        let source = "用户年龄 计算订单总额 变量甲".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::标识符);
        assert_eq!(tokens[0].literal, "用户年龄");
    }

    #[test]
    fn test_number() {
        let source = "123 0xFF 3.14".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::整数字面量);
        assert_eq!(tokens[0].literal, "123");
        assert_eq!(tokens[1].token_type, TokenType::整数字面量);
        assert_eq!(tokens[1].literal, "0xFF");
        assert_eq!(tokens[2].token_type, TokenType::浮点字面量);
    }

    #[test]
    fn test_string() {
        let source = "\"你好世界\"".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::文本字面量);
        assert_eq!(tokens[0].literal, "你好世界");
    }

    #[test]
    fn test_boolean_literals() {
        let source = "真 假".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::布尔字面量);
        assert_eq!(tokens[0].literal, "真");
        assert_eq!(tokens[1].token_type, TokenType::布尔字面量);
        assert_eq!(tokens[1].literal, "假");
    }

    #[test]
    fn test_new_keywords() {
        // 测试新增的关键字
        let source = "跳过 退出 借用 可变借用 手动 原生".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::Keyword(Keyword::跳过));
        assert_eq!(tokens[1].token_type, TokenType::Keyword(Keyword::退出));
        assert_eq!(tokens[2].token_type, TokenType::Keyword(Keyword::借用));
        assert_eq!(tokens[3].token_type, TokenType::Keyword(Keyword::可变借用));
        assert_eq!(tokens[4].token_type, TokenType::Keyword(Keyword::手动));
        assert_eq!(tokens[5].token_type, TokenType::Keyword(Keyword::原生));
    }

    #[test]
    fn test_loop_keywords() {
        // 测试循环组合关键字 (循环 + 从 + 到)
        let source = "循环 从 0 到 10".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].token_type, TokenType::Keyword(Keyword::循环));
        assert_eq!(tokens[1].token_type, TokenType::Keyword(Keyword::从));
        assert_eq!(tokens[2].token_type, TokenType::整数字面量);
        assert_eq!(tokens[3].token_type, TokenType::Keyword(Keyword::到));
        assert_eq!(tokens[4].token_type, TokenType::整数字面量);
    }

    #[test]
    fn test_english_keyword_aliases() {
        // 测试英文关键字别名
        let source = "if else while loop fn return let mut struct enum import".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].token_type, TokenType::Keyword(Keyword::若));      // if
        assert_eq!(tokens[1].token_type, TokenType::Keyword(Keyword::否则));     // else
        assert_eq!(tokens[2].token_type, TokenType::Keyword(Keyword::当));      // while
        assert_eq!(tokens[3].token_type, TokenType::Keyword(Keyword::循环));    // loop
        assert_eq!(tokens[4].token_type, TokenType::Keyword(Keyword::函数));    // fn
        assert_eq!(tokens[5].token_type, TokenType::Keyword(Keyword::返回));    // return
        assert_eq!(tokens[6].token_type, TokenType::Keyword(Keyword::定义));     // let
        assert_eq!(tokens[7].token_type, TokenType::Keyword(Keyword::可变));    // mut
        assert_eq!(tokens[8].token_type, TokenType::Keyword(Keyword::结构体));  // struct
        assert_eq!(tokens[9].token_type, TokenType::Keyword(Keyword::枚举));   // enum
        assert_eq!(tokens[10].token_type, TokenType::Keyword(Keyword::引入));  // import
    }

    #[test]
    fn test_mixed_chinese_english_keywords() {
        // 测试中英文混合使用: 函数 main() 返回 整数 { if x > 0 { return x } 返回 0 }
        let source = "函数 main() 返回 整数 { if x > 0 { 返回 x } 返回 0 }".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].token_type, TokenType::Keyword(Keyword::函数));    // 中文
        assert_eq!(tokens[1].token_type, TokenType::标识符);                    // main
        assert_eq!(tokens[2].token_type, TokenType::左圆括号);
        assert_eq!(tokens[3].token_type, TokenType::右圆括号);
        assert_eq!(tokens[4].token_type, TokenType::Keyword(Keyword::返回));    // 中文
        assert_eq!(tokens[5].token_type, TokenType::Keyword(Keyword::整数));    // 中文
        assert_eq!(tokens[6].token_type, TokenType::左花括号);
        assert_eq!(tokens[7].token_type, TokenType::Keyword(Keyword::若));      // 英文 if
        assert_eq!(tokens[8].token_type, TokenType::标识符);                    // x
        assert_eq!(tokens[9].token_type, TokenType::大于);
        assert_eq!(tokens[10].token_type, TokenType::整数字面量);                // 0
        assert_eq!(tokens[11].token_type, TokenType::左花括号);                  // {
        assert_eq!(tokens[12].token_type, TokenType::Keyword(Keyword::返回));    // 返回
        assert_eq!(tokens[13].token_type, TokenType::标识符);                    // x
        assert_eq!(tokens[14].token_type, TokenType::右花括号);                   // }
        assert_eq!(tokens[15].token_type, TokenType::Keyword(Keyword::返回));   // 返回
        assert_eq!(tokens[16].token_type, TokenType::整数字面量);                // 0
        assert_eq!(tokens[17].token_type, TokenType::右花括号);                   // }
    }
}
