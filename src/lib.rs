/**
 * @file lib.rs
 * @brief 玄语编译器 主库
 * @description 编译器核心模块，包含词法分析、语法分析、代码生成
 */

pub mod lexer;
pub mod parser;
pub mod ast;
pub mod types;
pub mod error;
pub mod codegen;
pub mod sema;

pub use lexer::{Lexer, LexerError, Token, TokenType, Keyword, Span};
pub use parser::{Parser, parse};
pub use ast::{Module, Function, Stmt, Expr};
pub use sema::{SemanticAnalyzer, analyze};
pub use codegen::{CodeGenerator, generate_ir};
pub use error::{
    CompilerError, ParserError, TypeError, CodegenError,
    ErrorLanguage, get_error_language, set_error_language,
    report_error, report_error_lang,
};
