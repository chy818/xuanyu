/**
 * @file mod.rs
 * @brief 编译器核心模块
 * @description 整合词法分析、宏展开、解析、语义分析和代码生成
 */

pub mod incremental;
pub mod module;

use std::collections::HashMap;
use std::path::Path;

use crate::lexer::{Lexer, Token};
use crate::parser::Parser;
use crate::ast::Module;
use crate::codegen::CodeGenerator;
use crate::macro_system::{MacroExpander, MacroDefinition, MacroRule, MacroPattern, MacroHygiene, MacroError};
use crate::lexer::token::{Span, TokenType};
use crate::error::CompilerError;

/**
 * 编译器配置
 */
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    /// 是否启用增量编译
    pub incremental: bool,
    /// 是否启用宏展开
    pub macros_enabled: bool,
    /// 最大宏展开深度
    pub max_macro_depth: usize,
    /// 是否显示调试信息
    pub debug: bool,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            incremental: true,
            macros_enabled: true,
            max_macro_depth: 64,
            debug: false,
        }
    }
}

/**
 * 编译结果
 */
#[derive(Debug)]
pub struct CompileResult {
    /// 编译是否成功
    pub success: bool,
    /// LLVM IR 代码
    pub ir: Option<String>,
    /// 错误列表
    pub errors: Vec<CompilerError>,
    /// 警告列表
    pub warnings: Vec<String>,
    /// 宏展开统计
    pub macro_stats: MacroCompileStats,
}

/**
 * 宏编译统计
 */
#[derive(Debug, Default)]
pub struct MacroCompileStats {
    /// 宏定义数量
    pub definitions: usize,
    /// 宏展开次数
    pub expansions: usize,
    /// 宏错误数量
    pub errors: usize,
}

/**
 * 玄语编译器
 * 整合词法分析、宏展开、解析、语义分析和代码生成
 */
pub struct Compiler {
    /// 编译器配置
    config: CompilerConfig,
    /// 宏展开器
    macro_expander: MacroExpander,
    /// 全局宏定义
    global_macros: HashMap<String, MacroDefinition>,
    /// 增量编译器
    incremental: incremental::IncrementalCompiler,
}

impl Compiler {
    /**
     * 创建新的编译器
     */
    pub fn new(config: CompilerConfig) -> Self {
        let mut compiler = Self {
            config,
            macro_expander: MacroExpander::new(),
            global_macros: HashMap::new(),
            incremental: incremental::IncrementalCompiler::new(Path::new(".cache/xuanyu").to_path_buf()),
        };

        compiler.register_builtin_macros();

        compiler
    }

    /**
     * 注册内建宏
     */
    fn register_builtin_macros(&mut self) {
        // 日志宏
        let log_macro = MacroDefinition {
            name: "日志".to_string(),
            params: vec![],
            body: vec![MacroRule {
                matcher: vec![],
                template: vec![],
                is_export: false,
            }],
            hygiene: MacroHygiene::Full,
            span: Span::dummy(),
        };
        self.global_macros.insert("日志".to_string(), log_macro);

        // 调试宏
        let debug_macro = MacroDefinition {
            name: "调试".to_string(),
            params: vec![],
            body: vec![MacroRule {
                matcher: vec![],
                template: vec![],
                is_export: false,
            }],
            hygiene: MacroHygiene::Full,
            span: Span::dummy(),
        };
        self.global_macros.insert("调试".to_string(), debug_macro);

        // 断言宏
        let assert_macro = MacroDefinition {
            name: "断言".to_string(),
            params: vec![],
            body: vec![MacroRule {
                matcher: vec![],
                template: vec![],
                is_export: false,
            }],
            hygiene: MacroHygiene::Full,
            span: Span::dummy(),
        };
        self.global_macros.insert("断言".to_string(), assert_macro);
    }

    /**
     * 编译源代码 - 返回 (成功标志, IR, 错误消息)
     */
    pub fn compile(&mut self, source: &str) -> CompileResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // 第一阶段：词法分析
        if self.config.debug {
            println!("[调试] 第一阶段：词法分析");
        }
        let tokens = self.lex(source, &mut errors);

        if !errors.is_empty() {
            return CompileResult {
                success: false,
                ir: None,
                errors,
                warnings,
                macro_stats: MacroCompileStats::default(),
            };
        }

        // 第二阶段：宏展开
        let tokens = if self.config.macros_enabled {
            if self.config.debug {
                println!("[调试] 第二阶段：宏展开");
            }
            self.expand_macros(tokens, &mut warnings)
        } else {
            tokens
        };

        // 第三阶段：解析
        if self.config.debug {
            println!("[调试] 第三阶段：语法解析");
        }
        let module = self.parse(tokens, &mut errors);

        if !errors.is_empty() {
            return CompileResult {
                success: false,
                ir: None,
                errors,
                warnings,
                macro_stats: self.get_macro_stats(),
            };
        }

        // 第四阶段：语义分析
        if self.config.debug {
            println!("[调试] 第四阶段：语义分析");
        }
        let validated = self.analyze(module, &mut errors);

        if !errors.is_empty() {
            return CompileResult {
                success: false,
                ir: None,
                errors,
                warnings,
                macro_stats: self.get_macro_stats(),
            };
        }

        // 第五阶段：代码生成
        if self.config.debug {
            println!("[调试] 第五阶段：代码生成");
        }
        let ir = self.generate_ir(validated, &mut errors);

        CompileResult {
            success: errors.is_empty(),
            ir: Some(ir),
            errors,
            warnings,
            macro_stats: self.get_macro_stats(),
        }
    }

    /**
     * 词法分析
     */
    fn lex(&self, source: &str, errors: &mut Vec<CompilerError>) -> Vec<Token> {
        let mut lexer = Lexer::new(source.to_string());
        let mut tokens = Vec::new();

        loop {
            match lexer.next_token() {
                Ok(token) => {
                    if matches!(token.token_type, TokenType::文件结束) {
                        tokens.push(token);
                        break;
                    }
                    tokens.push(token);
                }
                Err(e) => {
                    errors.push(CompilerError::Lexer(e));
                    break;
                }
            }
        }

        tokens
    }

    /**
     * 宏展开
     */
    fn expand_macros(&mut self, tokens: Vec<Token>, warnings: &mut Vec<String>) -> Vec<Token> {
        // 注册全局宏到展开器
        for (name, def) in &self.global_macros {
            if let Err(e) = self.macro_expander.define(def.clone()) {
                warnings.push(format!("宏定义错误 ({}): {}", name, e));
            }
        }

        // 执行宏展开
        match self.macro_expander.expand_tokens(tokens) {
            Ok(expanded) => expanded,
            Err(e) => {
                warnings.push(format!("宏展开错误: {}", e));
                Vec::new()
            }
        }
    }

    /**
     * 解析
     */
    fn parse(&self, tokens: Vec<Token>, errors: &mut Vec<CompilerError>) -> Module {
        let mut parser = Parser::new(tokens);

        match parser.parse_module() {
            Ok(module) => module,
            Err(e) => {
                errors.push(CompilerError::Parser(e));
                Module::new(vec![], Span::dummy())
            }
        }
    }

    /**
     * 语义分析
     */
    fn analyze(&self, module: Module, errors: &mut Vec<CompilerError>) -> Module {
        let mut analyzer = crate::sema::SemanticAnalyzer::new();

        match analyzer.analyze_module(&module) {
            Ok(_) => module,
            Err(type_errors) => {
                for e in type_errors {
                    errors.push(CompilerError::Type(e));
                }
                module
            }
        }
    }

    /**
     * 代码生成
     */
    fn generate_ir(&self, module: Module, errors: &mut Vec<CompilerError>) -> String {
        let mut codegen = CodeGenerator::new();

        match codegen.generate(&module) {
            Ok(ir) => ir,
            Err(e) => {
                errors.push(CompilerError::Codegen(e));
                String::new()
            }
        }
    }

    /**
     * 获取宏统计
     */
    fn get_macro_stats(&self) -> MacroCompileStats {
        let stats = self.macro_expander.get_stats();
        MacroCompileStats {
            definitions: stats.definitions,
            expansions: stats.expansions,
            errors: stats.errors,
        }
    }

    /**
     * 定义宏
     */
    pub fn define_macro(&mut self, name: String, params: Vec<String>, body: Vec<Token>) -> Result<(), MacroError> {
        let definition = MacroDefinition {
            name: name.clone(),
            params: params.into_iter().map(|p| crate::macro_system::MacroParam {
                pattern: MacroPattern::Expr,
                name: p,
                is_varargs: false,
            }).collect(),
            body: vec![MacroRule {
                matcher: vec![],
                template: body,
                is_export: false,
            }],
            hygiene: MacroHygiene::Full,
            span: Span::dummy(),
        };

        self.global_macros.insert(name, definition);
        Ok(())
    }

    /**
     * 获取已定义的宏列表
     */
    pub fn list_macros(&self) -> Vec<&str> {
        self.global_macros.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new(CompilerConfig::default())
    }
}

pub use incremental::{
    IncrementalCompiler, IncrementalResult, CompileTask,
    FileChange, BuildStats,
};

pub use module::{
    ModuleInfo, MultiFileCompiler,
};
