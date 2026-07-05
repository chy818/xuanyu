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
pub mod repl;
pub mod package;
pub mod macro_system;
pub mod compiler;

pub use lexer::{Lexer, LexerError, Token, TokenType, Keyword, Span};
pub use parser::{Parser, parse};
pub use ast::{Module, Function, Stmt, Expr};
pub use sema::{SemanticAnalyzer, analyze};
pub use codegen::{CodeGenerator, generate_ir, generate_ir_with_module_name};
pub use error::{
    CompilerError, ParserError, TypeError, CodegenError,
    ErrorLanguage, get_error_language, set_error_language,
    report_error, report_error_lang,
};
pub use repl::{Repl, ReplConfig, ReplContext, start_repl};
pub use package::{PackageConfig, PackageManager, run_package_command};
pub use macro_system::{MacroSystem, MacroExpander, MacroDefinition, MacroCall, MacroExpansion, MacroError, MacroStats, parse_macro_definition};
pub use compiler::{Compiler, CompilerConfig, CompileResult, IncrementalCompiler, IncrementalResult, FileChange, ModuleInfo, BuildStats};
