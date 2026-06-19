/**
 * @file module.rs
 * @brief 模块解析和依赖分析
 * @description 实现L2模块系统的模块解析、依赖分析和多文件编译
 */

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::PathBuf;

use crate::ast::Module;
use crate::error::CompilerError;
use crate::LexerError;
use crate::ParserError;
use crate::lexer::Lexer;
use crate::parser::parse;

/**
 * 模块信息
 */
#[derive(Clone)]
pub struct ModuleInfo {
    /// 模块路径
    pub path: PathBuf,
    /// 模块AST
    pub module: Module,
    /// 依赖的模块
    pub dependencies: Vec<String>,
}

/**
 * 模块解析器
 */
pub struct ModuleResolver {
    /// 模块缓存
    modules: HashMap<String, ModuleInfo>,
    /// 搜索路径
    search_paths: Vec<PathBuf>,
}

impl ModuleResolver {
    /**
     * 创建新的模块解析器
     */
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            search_paths: vec![PathBuf::from(".")],
        }
    }

    /**
     * 添加搜索路径
     */
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }

    /**
     * 解析模块
     */
    pub fn resolve_module(&mut self, module_path: &str) -> Result<ModuleInfo, CompilerError> {
        // 检查缓存
        if let Some(info) = self.modules.get(module_path) {
            return Ok(info.clone());
        }

        // 查找模块文件
        let file_path = self.find_module_file(module_path)?;
        
        // 读取文件内容
        let source = fs::read_to_string(&file_path)
            .map_err(|e| CompilerError::Lexer(LexerError {
                code: "MODULE-E001".to_string(),
                message: format!("无法读取文件: {}", e),
                span: crate::lexer::token::Span::dummy()
            }))?;

        // 词法分析
        let mut lexer = Lexer::new(source.clone());
        let tokens = lexer.tokenize()
            .map_err(|e| CompilerError::Lexer(e))?;

        // 语法分析
        let module = parse(tokens)
            .map_err(|e| CompilerError::Parser(e))?;

        // 分析依赖
        let dependencies = self.analyze_dependencies(&module);

        // 递归解析依赖
        for dep in &dependencies {
            self.resolve_module(dep)?;
        }

        let module_info = ModuleInfo {
            path: file_path,
            module,
            dependencies,
        };

        // 缓存模块
        self.modules.insert(module_path.to_string(), module_info.clone());

        Ok(module_info)
    }

    /**
     * 查找模块文件
     */
    fn find_module_file(&self, module_path: &str) -> Result<PathBuf, CompilerError> {
        // 将模块路径转换为文件路径
        let file_name = format!("{}.xy", module_path.replace("::", "/"));
        
        // 搜索所有路径
        for search_path in &self.search_paths {
            let candidate = search_path.join(&file_name);
            if candidate.exists() {
                return Ok(candidate);
            }

            // 尝试目录模块
            let dir_candidate = search_path.join(module_path.replace("::", "/"));
            let mod_file = dir_candidate.join("mod.xy");
            if mod_file.exists() {
                return Ok(mod_file);
            }
        }

        Err(CompilerError::Lexer(LexerError {
            code: "MODULE-E002".to_string(),
            message: format!("找不到模块: {}", module_path),
            span: crate::lexer::token::Span::dummy()
        }))
    }

    /**
     * 分析模块依赖
     */
    fn analyze_dependencies(&self, module: &Module) -> Vec<String> {
        let mut dependencies = Vec::new();

        for import in &module.imports {
            dependencies.push(import.module_path.clone());
        }

        dependencies
    }

    /**
     * 构建依赖图
     */
    pub fn build_dependency_graph(&self) -> HashMap<String, Vec<String>> {
        let mut graph = HashMap::new();

        for (module_path, info) in &self.modules {
            graph.insert(module_path.clone(), info.dependencies.clone());
        }

        graph
    }

    /**
     * 拓扑排序
     */
    pub fn topological_sort(&self) -> Result<Vec<String>, CompilerError> {
        let graph = self.build_dependency_graph();
        let mut in_degree = HashMap::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // 初始化入度
        for (node, dependencies) in &graph {
            if !in_degree.contains_key(node) {
                in_degree.insert(node.clone(), 0);
            }
            for dep in dependencies {
                *in_degree.entry(dep.clone()).or_insert(0) += 1;
            }
        }

        // 入度为0的节点入队
        for (node, degree) in &in_degree {
            if *degree == 0 {
                queue.push_back(node.clone());
            }
        }

        // 拓扑排序
        while let Some(node) = queue.pop_front() {
            result.push(node.clone());

            if let Some(dependencies) = graph.get(&node) {
                for dep in dependencies {
                    if let Some(degree) = in_degree.get_mut(dep) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dep.clone());
                        }
                    }
                }
            }
        }

        // 检查循环依赖
        if result.len() != in_degree.len() {
            return Err(CompilerError::Lexer(LexerError {
                code: "MODULE-E003".to_string(),
                message: "模块间存在循环依赖".to_string(),
                span: crate::lexer::token::Span::dummy()
            }));
        }

        Ok(result)
    }

    /**
     * 获取所有模块
     */
    pub fn get_modules(&self) -> &HashMap<String, ModuleInfo> {
        &self.modules
    }
}

/**
 * 多文件编译器
 */
pub struct MultiFileCompiler {
    module_resolver: ModuleResolver,
}

impl MultiFileCompiler {
    /**
     * 创建新的多文件编译器
     */
    pub fn new() -> Self {
        Self {
            module_resolver: ModuleResolver::new(),
        }
    }

    /**
     * 添加搜索路径
     */
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.module_resolver.add_search_path(path);
    }

    /**
     * 编译主模块
     */
    pub fn compile(&mut self, main_module: &str) -> Result<Vec<ModuleInfo>, CompilerError> {
        // 解析主模块
        self.module_resolver.resolve_module(main_module)?;

        // 拓扑排序
        let sorted_modules = self.module_resolver.topological_sort()?;

        // 收集模块信息
        let mut modules = Vec::new();
        for module_path in sorted_modules {
            if let Some(info) = self.module_resolver.get_modules().get(&module_path) {
                modules.push((*info).clone());
            }
        }

        Ok(modules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_resolver() {
        let mut resolver = ModuleResolver::new();
        
        // 测试模块解析
        // 这里需要实际的测试文件，暂时跳过
    }

    #[test]
    fn test_topological_sort() {
        let mut resolver = ModuleResolver::new();
        
        // 测试拓扑排序
        // 这里需要实际的模块依赖，暂时跳过
    }
}