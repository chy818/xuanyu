/**
 * @file error.rs
 * @brief CCAS 编译器错误处理
 * @description 定义编译错误类型和错误报告
 */

use crate::lexer::token::Span;
use crate::lexer::LexerError;

/**
 * 重新导出 Span 用于方便访问
 */
pub use crate::lexer::token::Span as SpanHelper;

/**
 * 编译器错误类型
 */
#[derive(Debug, Clone)]
pub enum CompilerError {
    /**
     * 词法分析错误
     */
    Lexer(LexerError),

    /**
     * 语法分析错误
     */
    Parser(ParserError),

    /**
     * 类型错误
     */
    Type(TypeError),

    /**
     * 代码生成错误
     */
    Codegen(CodegenError),
}

/**
 * 语法分析错误
 */
#[derive(Debug, Clone)]
pub struct ParserError {
    pub code: String,
    pub message: String,
    pub span: Span,
}

impl ParserError {
    pub fn unexpected_token(expected: &str, found: &str, span: Span) -> Self {
        Self {
            code: "CCAS-P001".to_string(),
            message: format!("期望 {}, 但遇到 {}", expected, found),
            span,
        }
    }

    pub fn unexpected_token_at(line: usize, col: usize, message: &str) -> Self {
        Self {
            code: "CCAS-P001".to_string(),
            message: message.to_string(),
            span: Span::new(line, col, line, col),
        }
    }
}

/**
 * 类型错误
 */
#[derive(Debug, Clone)]
pub struct TypeError {
    pub code: String,
    pub message: String,
    pub span: Span,
}

impl TypeError {
    pub fn type_mismatch(expected: &str, found: &str, span: Span) -> Self {
        Self {
            code: "CCAS-T001".to_string(),
            message: format!("类型不匹配: 期望 {}, 但找到 {}", expected, found),
            span,
        }
    }

    pub fn unknown_type(type_name: &str, span: Span) -> Self {
        Self {
            code: "CCAS-T002".to_string(),
            message: format!("未知的类型: {}", type_name),
            span,
        }
    }
}

/**
 * 代码生成错误
 */
#[derive(Debug, Clone)]
pub struct CodegenError {
    pub code: String,
    pub message: String,
}

impl CodegenError {
    pub fn unsupported_feature(feature: &str) -> Self {
        Self {
            code: "CCAS-C001".to_string(),
            message: format!("不支持的功能: {}", feature),
        }
    }
    
    pub fn new(message: &str) -> Self {
        Self {
            code: "CCAS-C002".to_string(),
            message: message.to_string(),
        }
    }
}

/**
 * 错误报告
 */

/**
 * 带源码上下文的错误报告
 */
pub fn report_error_with_context(error: &CompilerError, source_lines: &[String]) {
    match error {
        CompilerError::Lexer(e) => {
            eprintln!("\n\x1b[31m错误\x1b[0m [{}]: {}", e.code, e.message);
            print_source_context(source_lines, e.span.start_line, e.span.start_column, e.span.end_column);
        }
        CompilerError::Parser(e) => {
            eprintln!("\n\x1b[31m错误\x1b[0m [{}]: {}", e.code, e.message);
            print_source_context(source_lines, e.span.start_line, e.span.start_column, e.span.end_column);
        }
        CompilerError::Type(e) => {
            eprintln!("\n\x1b[31m错误\x1b[0m [{}]: {}", e.code, e.message);
            print_source_context(source_lines, e.span.start_line, e.span.start_column, e.span.end_column);
        }
        CompilerError::Codegen(e) => {
            eprintln!("\n\x1b[31m错误\x1b[0m [{}]: {}", e.code, e.message);
        }
    }
}

/**
 * 打印源码上下文
 */
fn print_source_context(source_lines: &[String], start_line: usize, start_col: usize, end_col: usize) {
    let line_idx = start_line.saturating_sub(1);
    if line_idx < source_lines.len() {
        let line = &source_lines[line_idx];
        eprintln!("\n  \x1b[34m{}\x1b[0m | {}", start_line, line);

        // 打印空格和对齐的插入符号
        let spaces = format!("  {} | ", start_line).len() + start_col;
        let carets = if end_col > start_col {
            (end_col - start_col).max(1)
        } else {
            1
        };
        eprintln!("{} \x1b[31m{}\x1b[0m", " ".repeat(spaces), "^".repeat(carets));
    }
}

/**
 * 简单的错误报告（向后兼容）
 */
pub fn report_error(error: &CompilerError) {
    match error {
        CompilerError::Lexer(e) => {
            eprintln!("\n\x1b[31m错误\x1b[0m [{}]: {} (行 {}, 列 {})",
                e.code, e.message, e.span.start_line, e.span.start_column);
        }
        CompilerError::Parser(e) => {
            eprintln!("\n\x1b[31m错误\x1b[0m [{}]: {} (行 {}, 列 {})",
                e.code, e.message, e.span.start_line, e.span.start_column);
        }
        CompilerError::Type(e) => {
            eprintln!("\n\x1b[31m错误\x1b[0m [{}]: {} (行 {}, 列 {})",
                e.code, e.message, e.span.start_line, e.span.start_column);
        }
        CompilerError::Codegen(e) => {
            eprintln!("\n\x1b[31m错误\x1b[0m [{}]: {}", e.code, e.message);
        }
    }
}

/**
 * 警告报告
 */
pub fn report_warning(message: &str, line: usize, col: usize) {
    eprintln!("\n\x1b[33m警告\x1b[0m: {} (行 {}, 列 {})", message, line, col);
}
