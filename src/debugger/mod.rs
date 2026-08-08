/**
 * @file debugger/mod.rs
 * @brief 调试器模块（v0.3.0 最小基础版）
 * @description 提供 --debug 标志、行号映射骨架与 REPL 调试命令占位。
 *              目前为骨架实现：行号映射表 + 调试命令解析器，
 *              真实断点/单步/变量查看将在 v0.3.0 正式接入
 *              （通过 llc -O0 + DWARF 行号映射，或 codegen 插桩）。
 */

use std::collections::HashMap;
use crate::ast::{Module, Stmt, BlockStmt, ASTNode};

/// 一条行映射记录：源码行号 -> 对应函数名
#[derive(Debug, Clone)]
pub struct LineMapping {
    /// 源码行号 -> (函数名, 语句描述)
    pub entries: Vec<(usize, String, String)>,
    /// 断点集合（源码行号 -> 是否启用的断点）
    pub breakpoints: HashMap<usize, bool>,
}

impl Default for LineMapping {
    fn default() -> Self {
        Self::new()
    }
}

impl LineMapping {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            breakpoints: HashMap::new(),
        }
    }

    /// 添加一行映射
    pub fn add(&mut self, line: usize, func: String, desc: String) {
        self.entries.push((line, func, desc));
    }

    /// 新增/切换断点
    pub fn toggle_breakpoint(&mut self, line: usize) {
        *self.breakpoints.entry(line).or_insert(false) = !self.breakpoints.get(&line).unwrap_or(&false);
    }

    /// 当前断点列表（已启用行号）
    pub fn enabled_breakpoints(&self) -> Vec<usize> {
        self.breakpoints.iter()
            .filter(|(_, enabled)| **enabled)
            .map(|(line, _)| *line)
            .collect()
    }

    /// 查询某行属于哪个函数
    pub fn function_of_line(&self, line: usize) -> Option<&str> {
        self.entries.iter()
            .find(|(l, _, _)| *l == line)
            .map(|(_, func, _)| func.as_str())
    }
}

/// 从 AST 构建行号映射骨架
/// 说明：真实调试依赖 IR 行号元数据（LLVM debug info），
/// 此处先以「函数 + 顶层语句行号」建立映射骨架，供后续 DWARF 映射使用。
pub fn build_line_mapping(module: &Module) -> LineMapping {
    let mut mapping = LineMapping::new();
    for func in &module.functions {
        collect_stmt_lines(&func.body, &func.name, &mut mapping);
    }
    mapping
}

/// 递归收集块中顶层语句的起始行号
fn collect_stmt_lines(block: &BlockStmt, func: &str, mapping: &mut LineMapping) {
    for stmt in &block.statements {
        mapping.add(stmt.span().start_line, func.to_string(), describe_stmt(stmt));
        // 嵌套块递归收集
        if let Stmt::Block(inner) = stmt {
            collect_stmt_lines(inner, func, mapping);
        }
    }
}

/// 简单的语句描述（用于行映射的可读展示）
fn describe_stmt(stmt: &crate::ast::Stmt) -> String {
    let desc = match stmt {
        Stmt::Let(_) => "变量声明",
        Stmt::Assignment(_) => "赋值",
        Stmt::Return(_) => "返回",
        Stmt::If(_) => "条件分支",
        Stmt::Loop(_) => "循环",
        Stmt::Match(_) => "匹配",
        Stmt::Block(_) => "代码块",
        Stmt::Expr(_) => "表达式",
        Stmt::Break(_) => "中断",
        Stmt::Continue(_) => "继续",
        _ => "其他",
    };
    desc.to_string()
}

/// 调试会话的响应
#[derive(Debug, Clone, PartialEq)]
pub enum DebugCommand {
    /// 设置/移除断点（行号）
    Breakpoint(usize),
    /// 显示当前行映射表
    ListMappings,
    /// 显示断点列表
    ListBreakpoints,
    /// 继续执行（占位：结束当前会话）
    Continue,
    /// 单步（占位）
    Step,
    /// 查看变量（占位）
    ViewVars,
    /// 帮助
    Help,
    /// 退出调试器
    Quit,
    /// 未知命令
    Unknown(String),
}

/// 解析调试命令（最小基础版：支持中英文别名）
pub fn parse_debug_command(input: &str) -> DebugCommand {
    let parts: Vec<&str> = input.trim().splitn(2, ' ').collect();
    match parts[0] {
        "b" | "断点" | "break" => {
            if let Some(num) = parts.get(1).and_then(|s| s.parse::<usize>().ok()) {
                DebugCommand::Breakpoint(num)
            } else {
                DebugCommand::Unknown("断点命令需要行号参数，如: 断点 12".to_string())
            }
        }
        "map" | "行映射" | "行号" => DebugCommand::ListMappings,
        "lb" | "断点列表" => DebugCommand::ListBreakpoints,
        "c" | "继续" | "continue" => DebugCommand::Continue,
        "s" | "单步" | "step" => DebugCommand::Step,
        "v" | "变量" | "vars" => DebugCommand::ViewVars,
        "h" | "帮助" | "help" | "?" => DebugCommand::Help,
        "q" | "退出" | "quit" => DebugCommand::Quit,
        _ => DebugCommand::Unknown(input.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_mapping_default() {
        let m = LineMapping::new();
        assert!(m.entries.is_empty());
        assert!(m.enabled_breakpoints().is_empty());
    }

    #[test]
    fn test_breakpoint_toggle_and_enabled() {
        let mut m = LineMapping::new();
        m.toggle_breakpoint(12);
        assert_eq!(m.enabled_breakpoints(), vec![12]);
        m.toggle_breakpoint(12);
        assert!(m.enabled_breakpoints().is_empty());
    }

    #[test]
    fn test_parse_debug_command_en() {
        assert_eq!(parse_debug_command("break 12"), DebugCommand::Breakpoint(12));
        assert_eq!(parse_debug_command("step"), DebugCommand::Step);
        assert_eq!(parse_debug_command("continue"), DebugCommand::Continue);
        assert_eq!(parse_debug_command("quit"), DebugCommand::Quit);
    }

    #[test]
    fn test_parse_debug_command_zh() {
        assert_eq!(parse_debug_command("断点 12"), DebugCommand::Breakpoint(12));
        assert_eq!(parse_debug_command("单步"), DebugCommand::Step);
        assert_eq!(parse_debug_command("继续"), DebugCommand::Continue);
        assert_eq!(parse_debug_command("退出"), DebugCommand::Quit);
        assert!(matches!(parse_debug_command("未知 xx"), DebugCommand::Unknown(_)));
    }
}