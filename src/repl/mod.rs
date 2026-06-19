/**
 * @file repl.rs
 * @brief 玄语 REPL (Read-Eval-Print-Loop) 交互式环境
 * @description 提供交互式的代码执行环境，支持表达式求值、语句执行和变量持久化
 * 
 * 功能特性:
 * - 交互式代码输入和执行
 * - 表达式即时求值
 * - 变量和函数持久化
 * - 多行代码块支持
 * - 内置命令系统
 * - 历史记录支持
 */

use std::collections::HashMap;
use std::io::{self, Write};
use std::fs;
use std::process::Command;

use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::sema::SemanticAnalyzer;
use crate::ast::Function;

/**
 * REPL 配置选项
 */
#[derive(Debug, Clone)]
pub struct ReplConfig {
    /// 是否显示详细的编译信息
    pub verbose: bool,
    /// 是否启用类型检查
    pub type_check: bool,
    /// 提示符样式
    pub prompt_style: PromptStyle,
    /// 是否启用历史记录
    pub enable_history: bool,
    /// 历史记录文件路径
    pub history_file: Option<String>,
}

impl Default for ReplConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            type_check: true,
            prompt_style: PromptStyle::Unicode,
            enable_history: true,
            history_file: None,
        }
    }
}

/**
 * 提示符样式
 */
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PromptStyle {
    /// Unicode 字符 (玄>)
    Unicode,
    /// ASCII 字符 (xy>)
    Ascii,
    /// 简洁样式 (>)
    Minimal,
}

impl PromptStyle {
    /// 获取主提示符
    pub fn primary(&self) -> &'static str {
        match self {
            PromptStyle::Unicode => "玄> ",
            PromptStyle::Ascii => "xy> ",
            PromptStyle::Minimal => "> ",
        }
    }

    /// 获取续行提示符
    pub fn continuation(&self) -> &'static str {
        match self {
            PromptStyle::Unicode => "  │ ",
            PromptStyle::Ascii => "  | ",
            PromptStyle::Minimal => "  ",
        }
    }
}

/**
 * REPL 执行上下文
 * 保存 REPL 会话中的变量、函数等状态
 */
#[derive(Debug, Clone)]
pub struct ReplContext {
    /// 已定义的变量 (变量名 -> 值字符串表示)
    pub variables: HashMap<String, String>,
    /// 已定义的函数
    pub functions: Vec<Function>,
    /// 已定义的结构体
    pub structs: Vec<crate::ast::StructDefinition>,
    /// 已定义的枚举
    pub enums: Vec<crate::ast::EnumDefinition>,
    /// 输入计数器
    pub input_count: usize,
    /// 最后一次求值结果
    pub last_result: Option<String>,
}

impl Default for ReplContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplContext {
    /// 创建新的 REPL 上下文
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            functions: Vec::new(),
            structs: Vec::new(),
            enums: Vec::new(),
            input_count: 0,
            last_result: None,
        }
    }

    /// 生成唯一的临时函数名
    pub fn generate_temp_function_name(&mut self) -> String {
        self.input_count += 1;
        format!("__repl_expr_{}", self.input_count)
    }

    /// 检查变量是否已定义
    pub fn has_variable(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }

    /// 获取变量值
    pub fn get_variable(&self, name: &str) -> Option<&String> {
        self.variables.get(name)
    }

    /// 设置变量
    pub fn set_variable(&mut self, name: String, value: String) {
        self.variables.insert(name, value);
    }

    /// 添加函数定义
    pub fn add_function(&mut self, func: Function) {
        // 如果函数已存在，替换它
        if let Some(pos) = self.functions.iter().position(|f| f.name == func.name) {
            self.functions[pos] = func;
        } else {
            self.functions.push(func);
        }
    }
}

/**
 * REPL 命令
 */
#[derive(Debug, Clone)]
pub enum ReplCommand {
    /// 退出 REPL
    Quit,
    /// 显示帮助
    Help,
    /// 加载文件
    Load(String),
    /// 保存当前会话
    Save(String),
    /// 清除所有变量
    Clear,
    /// 显示所有变量
    Vars,
    /// 显示所有函数
    Funcs,
    /// 切换详细模式
    Verbose,
    /// 切换类型检查
    TypeCheck,
    /// 显示版本
    Version,
    /// 执行系统命令
    Shell(String),
    /// 重置 REPL
    Reset,
    /// 未知命令
    Unknown(String),
}

/**
 * REPL 核心结构
 */
pub struct Repl {
    /// 配置
    config: ReplConfig,
    /// 执行上下文
    context: ReplContext,
    /// 是否正在运行
    running: bool,
    /// 多行缓冲区
    multiline_buffer: String,
    /// 是否处于多行模式
    in_multiline: bool,
    /// 括号计数器 (用于检测多行输入)
    bracket_count: i32,
    /// 大括号计数器
    brace_count: i32,
}

impl Repl {
    /// 创建新的 REPL 实例
    pub fn new(config: ReplConfig) -> Self {
        Self {
            config,
            context: ReplContext::new(),
            running: true,
            multiline_buffer: String::new(),
            in_multiline: false,
            bracket_count: 0,
            brace_count: 0,
        }
    }

    /// 启动 REPL 主循环
    pub fn run(&mut self) {
        self.print_welcome();

        while self.running {
            // 获取输入
            let input = match self.read_input() {
                Some(i) => i,
                None => continue,
            };

            // 处理空输入
            if input.trim().is_empty() {
                continue;
            }

            // 检查是否是命令
            if input.starts_with(':') || input.starts_with("：") {
                self.handle_command(&input);
                continue;
            }

            // 检查多行模式
            if self.in_multiline {
                self.handle_multiline(&input);
                continue;
            }

            // 检查是否需要进入多行模式
            if self.needs_multiline(&input) {
                self.start_multiline(&input);
                continue;
            }

            // 执行单行代码
            self.execute(&input);
        }
    }

    /// 打印欢迎信息
    fn print_welcome(&self) {
        println!();
        println!("╔═══════════════════════════════════════════════════════════╗");
        println!("║                    玄语 REPL v0.1.0                        ║");
        println!("║              交互式编程环境 - 以中文，写世界                 ║");
        println!("╚═══════════════════════════════════════════════════════════╝");
        println!();
        println!("输入 :帮助 或 :help 查看可用命令");
        println!("输入 :退出 或 :quit 退出 REPL");
        println!();
    }

    /// 读取用户输入
    fn read_input(&mut self) -> Option<String> {
        // 打印提示符
        let prompt = if self.in_multiline {
            self.config.prompt_style.continuation()
        } else {
            self.config.prompt_style.primary()
        };

        print!("{}", prompt);
        io::stdout().flush().ok()?;

        // 读取输入
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => {
                // EOF (Ctrl+D)
                println!();
                self.running = false;
                None
            }
            Ok(_) => {
                // 移除末尾换行符
                let trimmed = input.trim_end_matches('\n').trim_end_matches('\r').to_string();
                Some(trimmed)
            }
            Err(_) => {
                None
            }
        }
    }

    /// 检查是否需要多行输入
    fn needs_multiline(&self, input: &str) -> bool {
        let mut brace = 0;
        let mut bracket = 0;
        let mut paren = 0;
        let mut in_string = false;
        let mut escape_next = false;

        for ch in input.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => brace += 1,
                '}' if !in_string => brace -= 1,
                '[' if !in_string => bracket += 1,
                ']' if !in_string => bracket -= 1,
                '(' if !in_string => paren += 1,
                ')' if !in_string => paren -= 1,
                _ => {}
            }
        }

        // 如果括号不匹配，需要多行输入
        brace > 0 || bracket > 0 || paren > 0
    }

    /// 开始多行模式
    fn start_multiline(&mut self, input: &str) {
        self.in_multiline = true;
        self.multiline_buffer = input.to_string();
        self.multiline_buffer.push('\n');

        // 更新括号计数
        self.update_bracket_counts(input);
    }

    /// 处理多行输入
    fn handle_multiline(&mut self, input: &str) {
        // 检查是否是结束标记
        if input.trim() == "end" || input.trim() == "结束" {
            self.execute_multiline();
            return;
        }

        // 添加到缓冲区
        if !self.multiline_buffer.is_empty() {
            self.multiline_buffer.push('\n');
        }
        self.multiline_buffer.push_str(input);

        // 更新括号计数
        self.update_bracket_counts(input);

        // 检查是否完成
        if self.bracket_count <= 0 && self.brace_count <= 0 {
            self.execute_multiline();
        }
    }

    /// 更新括号计数
    fn update_bracket_counts(&mut self, input: &str) {
        let mut in_string = false;
        let mut escape_next = false;

        for ch in input.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => self.brace_count += 1,
                '}' if !in_string => self.brace_count -= 1,
                '(' if !in_string => self.bracket_count += 1,
                ')' if !in_string => self.bracket_count -= 1,
                _ => {}
            }
        }
    }

    /// 执行多行代码
    fn execute_multiline(&mut self) {
        let code = std::mem::take(&mut self.multiline_buffer);
        self.in_multiline = false;
        self.bracket_count = 0;
        self.brace_count = 0;

        self.execute(&code);
    }

    /// 执行代码
    fn execute(&mut self, code: &str) {
        // 尝试解析为表达式或语句
        let wrapped_code = self.wrap_code(code);

        if self.config.verbose {
            println!("[调试] 包装后代码:\n{}", wrapped_code);
        }

        // 编译并执行
        match self.compile_and_run(&wrapped_code) {
            Ok(result) => {
                if !result.is_empty() {
                    println!("{}", result);
                    self.context.last_result = Some(result);
                }
            }
            Err(e) => {
                eprintln!("错误: {}", e);
            }
        }
    }

    /// 包装代码为完整程序
    fn wrap_code(&self, code: &str) -> String {
        // 构建已定义函数的声明
        let _func_decls: String = self.context.functions.iter()
            .map(|f| {
                let params: String = f.params.iter()
                    .map(|p| format!("{}: {:?}", p.name, p.param_type))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("函数 {}({}): {:?}", f.name, params, f.return_type)
            })
            .collect::<Vec<_>>()
            .join("\n");

        // 构建完整代码
        let mut full_code = String::new();

        // 添加已有函数定义
        for func in &self.context.functions {
            full_code.push_str(&self.function_to_string(func));
            full_code.push_str("\n\n");
        }

        // 添加主函数包装
        full_code.push_str("函数 主(): 整数 {\n");
        
        // 添加已有变量初始化
        for (name, value) in &self.context.variables {
            full_code.push_str(&format!("    定义 {}: 整数 = {}\n", name, value));
        }

        // 添加用户代码
        for line in code.lines() {
            full_code.push_str("    ");
            full_code.push_str(line);
            full_code.push('\n');
        }

        full_code.push_str("    返回 0\n");
        full_code.push_str("}\n");

        full_code
    }

    /// 将函数转换为字符串
    fn function_to_string(&self, func: &Function) -> String {
        let params: String = func.params.iter()
            .map(|p| format!("{}: {:?}", p.name, p.param_type))
            .collect::<Vec<_>>()
            .join(", ");

        let mut result = format!("函数 {}({}): {:?}", func.name, params, func.return_type);
        result.push_str(" {\n");

        for stmt in &func.body.statements {
            result.push_str(&format!("    {:?}\n", stmt));
        }

        result.push_str("}");
        result
    }

    /// 编译并运行代码
    fn compile_and_run(&mut self, code: &str) -> Result<String, String> {
        // 词法分析
        let mut lexer = Lexer::new(code.to_string());
        let tokens = lexer.tokenize()
            .map_err(|e| format!("词法错误: {}", e.message))?;

        if self.config.verbose {
            println!("[词法] 共 {} 个 Token", tokens.len());
        }

        // 语法分析
        let mut parser = Parser::new(tokens);
        let module = parser.parse_module()
            .map_err(|e| format!("语法错误: {}", e.message))?;

        if self.config.verbose {
            println!("[语法] {} 个函数定义", module.functions.len());
        }

        // 语义分析
        if self.config.type_check {
            let mut analyzer = SemanticAnalyzer::new();
            analyzer.analyze_module(&module)
                .map_err(|errors| {
                    let msgs: Vec<String> = errors.iter()
                        .map(|e| format!("{}", e.message))
                        .collect();
                    format!("语义错误: {}", msgs.join(", "))
                })?;
        }

        // 代码生成
        let ir = crate::codegen::generate_ir(&module)
            .map_err(|e| format!("代码生成错误: {}", e.message))?;

        if self.config.verbose {
            println!("[代码生成] LLVM IR 生成完成");
        }

        // 执行代码
        self.execute_ir(&ir)
    }

    /// 执行 LLVM IR
    fn execute_ir(&self, ir: &str) -> Result<String, String> {
        // 保存 IR 到临时文件
        let temp_ir = format!("repl_temp_{}.ll", std::process::id());
        fs::write(&temp_ir, ir)
            .map_err(|e| format!("无法写入临时文件: {}", e))?;

        // 编译为对象文件
        let temp_obj = format!("repl_temp_{}.o", std::process::id());
        
        let llc_result = Command::new("llc")
            .arg(&temp_ir)
            .arg("-filetype=obj")
            .arg("-o")
            .arg(&temp_obj)
            .output();

        match llc_result {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let _ = fs::remove_file(&temp_ir);
                    return Err(format!("llc 编译失败: {}", stderr));
                }
            }
            Err(e) => {
                let _ = fs::remove_file(&temp_ir);
                return Err(format!("无法执行 llc: {}。请确保已安装 LLVM。", e));
            }
        }

        // 查找 runtime.c
        let runtime_path = self.find_runtime()?;

        // 链接生成可执行文件
        let output_exe = if cfg!(target_os = "windows") {
            format!("repl_temp_{}.exe", std::process::id())
        } else {
            format!("repl_temp_{}", std::process::id())
        };

        let linker_result = Command::new("clang")
            .arg(&runtime_path)
            .arg(&temp_obj)
            .arg("-o")
            .arg(&output_exe)
            .output();

        match linker_result {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    self.cleanup_temp_files(&temp_ir, &temp_obj, &output_exe);
                    return Err(format!("链接失败: {}", stderr));
                }
            }
            Err(e) => {
                self.cleanup_temp_files(&temp_ir, &temp_obj, "");
                return Err(format!("无法执行 clang: {}", e));
            }
        }

        // 执行程序
        let run_result = Command::new(&format!("./{}", output_exe))
            .output();

        let result = match run_result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                
                if !stderr.is_empty() {
                    Err(format!("运行时错误: {}", stderr))
                } else {
                    Ok(stdout.trim().to_string())
                }
            }
            Err(e) => {
                Err(format!("执行失败: {}", e))
            }
        };

        // 清理临时文件
        self.cleanup_temp_files(&temp_ir, &temp_obj, &output_exe);

        result
    }

    /// 查找运行时库
    fn find_runtime(&self) -> Result<String, String> {
        let possible_paths = vec![
            "runtime/runtime.c",
            "../runtime/runtime.c",
            "./runtime.c",
        ];

        for path in possible_paths {
            if fs::metadata(path).is_ok() {
                return Ok(path.to_string());
            }
        }

        Err("找不到 runtime.c 运行时库".to_string())
    }

    /// 清理临时文件
    fn cleanup_temp_files(&self, ir: &str, obj: &str, exe: &str) {
        let _ = fs::remove_file(ir);
        let _ = fs::remove_file(obj);
        if !exe.is_empty() {
            let _ = fs::remove_file(exe);
        }
    }

    /// 处理 REPL 命令
    fn handle_command(&mut self, input: &str) {
        // 移除命令前缀
        let cmd_str = input.trim_start_matches(':').trim_start_matches("：").trim();
        
        let command = self.parse_command(cmd_str);
        
        match command {
            ReplCommand::Quit => {
                println!("再见！");
                self.running = false;
            }
            ReplCommand::Help => {
                self.print_help();
            }
            ReplCommand::Load(filename) => {
                self.load_file(&filename);
            }
            ReplCommand::Save(filename) => {
                self.save_session(&filename);
            }
            ReplCommand::Clear => {
                self.context = ReplContext::new();
                println!("已清除所有变量和函数定义");
            }
            ReplCommand::Vars => {
                self.print_variables();
            }
            ReplCommand::Funcs => {
                self.print_functions();
            }
            ReplCommand::Verbose => {
                self.config.verbose = !self.config.verbose;
                println!("详细模式: {}", if self.config.verbose { "开启" } else { "关闭" });
            }
            ReplCommand::TypeCheck => {
                self.config.type_check = !self.config.type_check;
                println!("类型检查: {}", if self.config.type_check { "开启" } else { "关闭" });
            }
            ReplCommand::Version => {
                println!("玄语 REPL v0.1.0");
                println!("编译器版本: v0.1.0");
                println!("后端: LLVM");
            }
            ReplCommand::Shell(cmd) => {
                self.execute_shell_command(&cmd);
            }
            ReplCommand::Reset => {
                self.context = ReplContext::new();
                self.multiline_buffer.clear();
                self.in_multiline = false;
                println!("REPL 已重置");
            }
            ReplCommand::Unknown(cmd) => {
                println!("未知命令: {}", cmd);
                println!("输入 :帮助 或 :help 查看可用命令");
            }
        }
    }

    /// 解析命令
    fn parse_command(&self, input: &str) -> ReplCommand {
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let cmd = parts[0];
        let arg = parts.get(1).map(|s| s.trim().to_string());

        // 支持中英文命令
        match cmd {
            "quit" | "退出" | "q" => ReplCommand::Quit,
            "help" | "帮助" | "h" | "?" => ReplCommand::Help,
            "load" | "加载" => {
                match arg {
                    Some(f) => ReplCommand::Load(f),
                    None => ReplCommand::Unknown("load 命令需要文件名参数".to_string()),
                }
            }
            "save" | "保存" => {
                match arg {
                    Some(f) => ReplCommand::Save(f),
                    None => ReplCommand::Unknown("save 命令需要文件名参数".to_string()),
                }
            }
            "clear" | "清除" => ReplCommand::Clear,
            "vars" | "变量" => ReplCommand::Vars,
            "funcs" | "函数" => ReplCommand::Funcs,
            "verbose" | "详细" | "v" => ReplCommand::Verbose,
            "typecheck" | "类型检查" | "tc" => ReplCommand::TypeCheck,
            "version" | "版本" => ReplCommand::Version,
            "shell" | "系统" | "!" => {
                match arg {
                    Some(c) => ReplCommand::Shell(c),
                    None => ReplCommand::Unknown("shell 命令需要命令参数".to_string()),
                }
            }
            "reset" | "重置" => ReplCommand::Reset,
            _ => ReplCommand::Unknown(input.to_string()),
        }
    }

    /// 打印帮助信息
    fn print_help(&self) {
        println!();
        println!("玄语 REPL 命令帮助");
        println!("═══════════════════════════════════════════════════════════");
        println!();
        println!("命令格式: :命令 [参数] 或 ：命令 [参数]");
        println!();
        println!("基本命令:");
        println!("  :帮助, :help, :h, :?     显示此帮助信息");
        println!("  :退出, :quit, :q         退出 REPL");
        println!("  :版本, :version          显示版本信息");
        println!();
        println!("代码管理:");
        println!("  :加载, :load <文件>      加载并执行 .xy 文件");
        println!("  :保存, :save <文件>      保存当前会话到文件");
        println!("  :清除, :clear            清除所有变量和函数");
        println!("  :重置, :reset            重置 REPL 状态");
        println!();
        println!("调试命令:");
        println!("  :变量, :vars             显示所有已定义变量");
        println!("  :函数, :funcs            显示所有已定义函数");
        println!("  :详细, :verbose, :v      切换详细输出模式");
        println!("  :类型检查, :typecheck    切换类型检查");
        println!();
        println!("系统命令:");
        println!("  :系统, :shell, :! <命令> 执行系统命令");
        println!();
        println!("多行输入:");
        println!("  输入未闭合的代码块会自动进入多行模式");
        println!("  输入 'end' 或 '结束' 可手动结束多行输入");
        println!();
        println!("示例:");
        println!("  玄> 定义 x = 10");
        println!("  玄> 打印(x + 5)");
        println!("  玄> 函数 加法(a, b) => a + b");
        println!("  玄> 打印(加法(3, 4))");
        println!();
    }

    /// 打印所有变量
    fn print_variables(&self) {
        if self.context.variables.is_empty() {
            println!("当前没有定义任何变量");
            return;
        }

        println!("已定义变量:");
        for (name, value) in &self.context.variables {
            println!("  {} = {}", name, value);
        }
    }

    /// 打印所有函数
    fn print_functions(&self) {
        if self.context.functions.is_empty() {
            println!("当前没有定义任何函数");
            return;
        }

        println!("已定义函数:");
        for func in &self.context.functions {
            let params: String = func.params.iter()
                .map(|p| p.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            println!("  {}({}) -> {:?}", func.name, params, func.return_type);
        }
    }

    /// 加载文件
    fn load_file(&mut self, filename: &str) {
        match fs::read_to_string(filename) {
            Ok(code) => {
                println!("已加载文件: {}", filename);
                self.execute(&code);
            }
            Err(e) => {
                eprintln!("无法加载文件 '{}': {}", filename, e);
            }
        }
    }

    /// 保存会话
    fn save_session(&self, filename: &str) {
        let mut content = String::new();
        
        // 添加注释头
        content.push_str("// 玄语 REPL 会话保存\n");
        content.push_str("// 保存时间: ...\n\n");

        // 添加变量定义
        for (name, value) in &self.context.variables {
            content.push_str(&format!("定义 {} = {};\n", name, value));
        }

        if !self.context.variables.is_empty() {
            content.push('\n');
        }

        // 添加函数定义
        for func in &self.context.functions {
            content.push_str(&self.function_to_string(func));
            content.push_str("\n\n");
        }

        match fs::write(filename, content) {
            Ok(_) => {
                println!("会话已保存到: {}", filename);
            }
            Err(e) => {
                eprintln!("无法保存会话: {}", e);
            }
        }
    }

    /// 执行系统命令
    fn execute_shell_command(&self, cmd: &str) {
        #[cfg(target_os = "windows")]
        let result = Command::new("cmd").args(["/C", cmd]).output();
        
        #[cfg(not(target_os = "windows"))]
        let result = Command::new("sh").args(["-c", cmd]).output();

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                
                if !stdout.is_empty() {
                    print!("{}", stdout);
                }
                if !stderr.is_empty() {
                    eprint!("{}", stderr);
                }
            }
            Err(e) => {
                eprintln!("执行系统命令失败: {}", e);
            }
        }
    }
}

/**
 * 启动 REPL
 */
pub fn start_repl(config: Option<ReplConfig>) {
    let config = config.unwrap_or_default();
    let mut repl = Repl::new(config);
    repl.run();
}
