/**
 * @file mod.rs
 * @brief CCAS 错误处理模块
 */

pub mod error;

pub use error::{
    CompilerError, ParserError, TypeError, CodegenError, report_error,
    ErrorLanguage, get_error_language, set_error_language,
    report_error_lang, report_error_with_context, report_error_with_context_lang,
    report_warning, report_warning_lang,
};
