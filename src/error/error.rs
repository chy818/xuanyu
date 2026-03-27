/**
 * @file error.rs
 * @brief CCAS 编译器错误处理
 * @description 定义编译错误类型和错误报告，支持国际化
 */

use crate::lexer::token::Span;
use crate::lexer::LexerError;

/**
 * 重新导出 Span 用于方便访问
 */
pub use crate::lexer::token::Span as SpanHelper;

/**
 * 错误语言选项
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorLanguage {
    /** 中文 (默认) */
    #[default]
    中文,
    /** 英文 */
    英文,
    /** 中英双语 */
    双语,
}

impl ErrorLanguage {
    /**
     * 从环境变量读取语言配置
     * 支持: XY_ERROR_LANG=zh|en|both
     */
    pub fn from_env() -> Self {
        std::env::var("XY_ERROR_LANG")
            .ok()
            .map(|s| match s.to_lowercase().as_str() {
                "en" | "english" => ErrorLanguage::英文,
                "both" | "bilingual" => ErrorLanguage::双语,
                _ => ErrorLanguage::中文,
            })
            .unwrap_or(ErrorLanguage::中文)
    }

    /**
     * 获取错误标签文本
     */
    pub fn error_label(&self) -> &'static str {
        match self {
            ErrorLanguage::中文 => "错误",
            ErrorLanguage::英文 => "Error",
            ErrorLanguage::双语 => "错误 / Error",
        }
    }

    /**
     * 获取警告标签文本
     */
    pub fn warning_label(&self) -> &'static str {
        match self {
            ErrorLanguage::中文 => "警告",
            ErrorLanguage::英文 => "Warning",
            ErrorLanguage::双语 => "警告 / Warning",
        }
    }

    /**
     * 获取行号前缀文本
     */
    pub fn line_prefix(&self) -> &'static str {
        match self {
            ErrorLanguage::中文 => "行",
            ErrorLanguage::英文 => "Line",
            ErrorLanguage::双语 => "行 / Line",
        }
    }

    /**
     * 获取列号前缀文本
     */
    pub fn column_prefix(&self) -> &'static str {
        match self {
            ErrorLanguage::中文 => "列",
            ErrorLanguage::英文 => "Column",
            ErrorLanguage::双语 => "列 / Column",
        }
    }
}

/**
 * 全局错误语言配置
 */
static ERROR_LANGUAGE: std::sync::LazyLock<std::sync::Mutex<ErrorLanguage>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(ErrorLanguage::from_env()));

/**
 * 获取当前全局错误语言配置
 */
pub fn get_error_language() -> ErrorLanguage {
    *ERROR_LANGUAGE.lock().unwrap()
}

/**
 * 设置全局错误语言 (运行时覆盖环境变量)
 */
pub fn set_error_language(lang: ErrorLanguage) {
    *ERROR_LANGUAGE.lock().unwrap() = lang;
}

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

    /**
     * 获取双语错误消息
     */
    pub fn unexpected_token_bilingual(expected: &str, found: &str, span: Span) -> Self {
        Self {
            code: "CCAS-P001".to_string(),
            message: format!(
                "期望 / Expected {}, 但遇到 / found {}",
                expected, found
            ),
            span,
        }
    }

    /**
     * 获取英文错误消息
     */
    pub fn unexpected_token_english(expected: &str, found: &str, span: Span) -> Self {
        Self {
            code: "CCAS-P001".to_string(),
            message: format!("Expected {}, found {}", expected, found),
            span,
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

    /**
     * 获取双语错误消息
     */
    pub fn type_mismatch_bilingual(expected: &str, found: &str, span: Span) -> Self {
        Self {
            code: "CCAS-T001".to_string(),
            message: format!(
                "类型不匹配 / Type mismatch: 期望 / Expected {}, 但找到 / found {}",
                expected, found
            ),
            span,
        }
    }

    /**
     * 获取英文错误消息
     */
    pub fn type_mismatch_english(expected: &str, found: &str, span: Span) -> Self {
        Self {
            code: "CCAS-T001".to_string(),
            message: format!("Type mismatch: Expected {}, found {}", expected, found),
            span,
        }
    }

    /**
     * 获取英文未知类型错误
     */
    pub fn unknown_type_english(type_name: &str, span: Span) -> Self {
        Self {
            code: "CCAS-T002".to_string(),
            message: format!("Unknown type: {}", type_name),
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

    /**
     * 获取双语错误消息
     */
    pub fn unsupported_feature_bilingual(feature: &str) -> Self {
        Self {
            code: "CCAS-C001".to_string(),
            message: format!("不支持的功能 / Unsupported feature: {}", feature),
        }
    }

    /**
     * 获取英文错误消息
     */
    pub fn unsupported_feature_english(feature: &str) -> Self {
        Self {
            code: "CCAS-C001".to_string(),
            message: format!("Unsupported feature: {}", feature),
        }
    }

    /**
     * 获取英文错误消息
     */
    pub fn new_english(message: &str) -> Self {
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
 * 带源码上下文的错误报告 (使用全局语言配置)
 */
pub fn report_error_with_context(error: &CompilerError, source_lines: &[String]) {
    let lang = get_error_language();
    report_error_with_context_lang(error, source_lines, lang);
}

/**
 * 带源码上下文的错误报告 (指定语言)
 */
pub fn report_error_with_context_lang(
    error: &CompilerError,
    source_lines: &[String],
    lang: ErrorLanguage,
) {
    let label = lang.error_label();
    match error {
        CompilerError::Lexer(e) => {
            eprintln!("\n\x1b[31m{}\x1b[0m [{}]: {}", label, e.code, e.message);
            print_source_context(source_lines, e.span.start_line, e.span.start_column, e.span.end_column);
        }
        CompilerError::Parser(e) => {
            eprintln!("\n\x1b[31m{}\x1b[0m [{}]: {}", label, e.code, e.message);
            print_source_context(source_lines, e.span.start_line, e.span.start_column, e.span.end_column);
        }
        CompilerError::Type(e) => {
            eprintln!("\n\x1b[31m{}\x1b[0m [{}]: {}", label, e.code, e.message);
            print_source_context(source_lines, e.span.start_line, e.span.start_column, e.span.end_column);
        }
        CompilerError::Codegen(e) => {
            eprintln!("\n\x1b[31m{}\x1b[0m [{}]: {}", label, e.code, e.message);
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
 * 简单的错误报告 (使用全局语言配置)
 */
pub fn report_error(error: &CompilerError) {
    let lang = get_error_language();
    report_error_lang(error, lang);
}

/**
 * 简单的错误报告 (指定语言)
 */
pub fn report_error_lang(error: &CompilerError, lang: ErrorLanguage) {
    let label = lang.error_label();
    let line_prefix = lang.line_prefix();
    let col_prefix = lang.column_prefix();

    match error {
        CompilerError::Lexer(e) => {
            eprintln!(
                "\n\x1b[31m{}\x1b[0m [{}]: {} ({} {}, {} {})",
                label,
                e.code,
                e.message,
                line_prefix,
                e.span.start_line,
                col_prefix,
                e.span.start_column
            );
        }
        CompilerError::Parser(e) => {
            eprintln!(
                "\n\x1b[31m{}\x1b[0m [{}]: {} ({} {}, {} {})",
                label,
                e.code,
                e.message,
                line_prefix,
                e.span.start_line,
                col_prefix,
                e.span.start_column
            );
        }
        CompilerError::Type(e) => {
            eprintln!(
                "\n\x1b[31m{}\x1b[0m [{}]: {} ({} {}, {} {})",
                label,
                e.code,
                e.message,
                line_prefix,
                e.span.start_line,
                col_prefix,
                e.span.start_column
            );
        }
        CompilerError::Codegen(e) => {
            eprintln!("\n\x1b[31m{}\x1b[0m [{}]: {}", label, e.code, e.message);
        }
    }
}

/**
 * 警告报告 (使用全局语言配置)
 */
pub fn report_warning(message: &str, line: usize, col: usize) {
    let lang = get_error_language();
    report_warning_lang(message, line, col, lang);
}

/**
 * 警告报告 (指定语言)
 */
pub fn report_warning_lang(message: &str, line: usize, col: usize, lang: ErrorLanguage) {
    let label = lang.warning_label();
    let line_prefix = lang.line_prefix();
    let col_prefix = lang.column_prefix();
    eprintln!(
        "\n\x1b[33m{}\x1b[0m: {} ({} {}, {} {})",
        label, message, line_prefix, line, col_prefix, col
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_language_labels() {
        assert_eq!(ErrorLanguage::中文.error_label(), "错误");
        assert_eq!(ErrorLanguage::英文.error_label(), "Error");
        assert_eq!(ErrorLanguage::双语.error_label(), "错误 / Error");

        assert_eq!(ErrorLanguage::中文.warning_label(), "警告");
        assert_eq!(ErrorLanguage::英文.warning_label(), "Warning");
        assert_eq!(ErrorLanguage::双语.warning_label(), "警告 / Warning");
    }

    #[test]
    fn test_error_language_line_prefix() {
        assert_eq!(ErrorLanguage::中文.line_prefix(), "行");
        assert_eq!(ErrorLanguage::英文.line_prefix(), "Line");
        assert_eq!(ErrorLanguage::双语.line_prefix(), "行 / Line");

        assert_eq!(ErrorLanguage::中文.column_prefix(), "列");
        assert_eq!(ErrorLanguage::英文.column_prefix(), "Column");
        assert_eq!(ErrorLanguage::双语.column_prefix(), "列 / Column");
    }

    #[test]
    fn test_parser_error_bilingual() {
        let span = Span::new(1, 1, 1, 5);
        let err = ParserError::unexpected_token_bilingual("整数", "文本", span);
        assert!(err.message.contains("期望 / Expected"));
        assert!(err.message.contains("整数"));
        assert!(err.message.contains("但遇到 / found"));
        assert!(err.message.contains("文本"));
    }

    #[test]
    fn test_parser_error_english() {
        let span = Span::new(1, 1, 1, 5);
        let err = ParserError::unexpected_token_english("int", "string", span);
        assert_eq!(err.message, "Expected int, found string");
    }

    #[test]
    fn test_type_error_bilingual() {
        let span = Span::new(1, 1, 1, 5);
        let err = TypeError::type_mismatch_bilingual("整数", "文本", span);
        assert!(err.message.contains("类型不匹配 / Type mismatch"));
    }

    #[test]
    fn test_type_error_english() {
        let span = Span::new(1, 1, 1, 5);
        let err = TypeError::type_mismatch_english("int", "string", span);
        assert_eq!(err.message, "Type mismatch: Expected int, found string");
    }

    #[test]
    fn test_codegen_error_bilingual() {
        let err = CodegenError::unsupported_feature_bilingual("尾递归优化");
        assert!(err.message.contains("不支持的功能 / Unsupported feature"));
    }

    #[test]
    fn test_codegen_error_english() {
        let err = CodegenError::unsupported_feature_english("tail recursion optimization");
        assert_eq!(err.message, "Unsupported feature: tail recursion optimization");
    }

    #[test]
    fn test_set_and_get_error_language() {
        set_error_language(ErrorLanguage::英文);
        assert_eq!(get_error_language(), ErrorLanguage::英文);

        set_error_language(ErrorLanguage::双语);
        assert_eq!(get_error_language(), ErrorLanguage::双语);

        set_error_language(ErrorLanguage::中文);
        assert_eq!(get_error_language(), ErrorLanguage::中文);
    }
}
