/**
 * @file main.rs
 * @brief 玄语编译器 (xy) 主程序入口
 * @description 编译器命令行工具，用于编译 .xy 源文件
 */

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};
use std::time::SystemTime;

use xuanyu::compiler::MultiFileCompiler;
use xuanyu::ast::Expr;

#[cfg(target_os = "windows")]
fn setup_windows_console() {
    // 调用 Windows API 设置控制台代码页为 UTF-8
    unsafe {
        extern "system" {
            fn SetConsoleOutputCP(wCodePageID: u32) -> u32;
            fn SetConsoleCP(wCodePageID: u32) -> u32;
        }
        SetConsoleOutputCP(65001);
        SetConsoleCP(65001);
    }
}

#[cfg(not(target_os = "windows"))]
fn setup_windows_console() {
    // 非 Windows 系统不需要设置
}

fn main() {
    // 在任何输出之前设置控制台模式
    setup_windows_console();
    
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        exit(1);
    }

    // 解析参数
    let mut input_file = String::new();
    let mut run_mode = RunMode::IrOnly; // 默认只生成 IR
    let mut debug_mode = false;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage(&args[0]);
                exit(0);
            }
            "--run" => {
                run_mode = RunMode::Run;
                i += 1;
            }
            "--build" => {
                run_mode = RunMode::Build;
                i += 1;
            }
            "--ir" => {
                run_mode = RunMode::IrOnly;
                i += 1;
            }
            "--ir-pure" => {
                run_mode = RunMode::IrPure;
                i += 1;
            }
            "--ir-file" => {
                if i + 1 < args.len() {
                    run_mode = RunMode::IrFile(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("错误: --ir-file 需要指定输出文件路径");
                    print_usage(&args[0]);
                    exit(1);
                }
            }
            "--debug" => {
                debug_mode = true;
                i += 1;
            }
            "repl" | "--repl" | "-i" => {
                // 启动 REPL 模式
                xuanyu::start_repl(None);
                return;
            }
            _ => {
                if i > 0 && !arg.starts_with('-') && input_file.is_empty() {
                    input_file = arg.clone();
                }
                i += 1;
            }
        }
    }

    if input_file.is_empty() {
        eprintln!("错误: 请指定输入文件");
        print_usage(&args[0]);
        exit(1);
    }

    // 执行编译流程
    if let Err(e) = compile_file(&input_file, run_mode, debug_mode) {
        eprintln!("编译失败: {}", e);
        exit(1);
    }
}

#[derive(Debug, Clone, PartialEq)]
enum RunMode {
    IrOnly,  // 只生成 IR（带调试信息）
    IrPure,  // 只输出纯 IR（无调试信息）
    IrFile(String),  // 将 IR 写入指定文件
    Build,   // 生成可执行文件
    Run,     // 编译并运行
}

fn compile_file(filename: &str, mode: RunMode, debug: bool) -> Result<(), String> {
    // 默认使用多文件编译（支持引入解析）
    let is_multi_file = filename.ends_with(".xy") && Path::new(filename).exists();

    if is_multi_file {
        // 多文件编译（自动解析引入语句）
        compile_multi_file(filename, mode, debug)
    } else {
        // 单文件编译
        compile_single_file(filename, mode, debug)
    }
}

fn compile_single_file(filename: &str, mode: RunMode, debug: bool) -> Result<(), String> {
    // 读取源文件
    let source = fs::read_to_string(filename)
        .map_err(|e| format!("无法读取文件 '{}': {}", filename, e))?;

    // 如果是纯 IR 模式，不输出调试信息
    if mode != RunMode::IrPure {
        println!("正在编译: {}", filename);
        println!("源文件大小: {} 字节", source.len());
    }

    // ========== 增量编译检查 ==========
    let cache_valid = check_cache(filename, &source)?;

    // 如果缓存有效且不是强制重新编译，可以跳过大部分工作
    if cache_valid && mode == RunMode::IrOnly {
        println!("[缓存] 源文件未修改，跳过编译");
        return Ok(());
    }

    // ========== 词法分析 ==========
    if mode != RunMode::IrPure {
        println!("\n=== 词法分析 ===");
    }
    let mut lexer = xuanyu::Lexer::new(source.clone());
    
    let tokens = lexer.tokenize()
        .map_err(|e| format!("词法错误 [{}]: {} (行 {}, 列 {})", 
            e.code, e.message, e.span.start_line, e.span.start_column))?;
    
    if mode != RunMode::IrPure {
        println!("词法分析完成，共 {} 个 Token", tokens.len());

        // 打印前 10 个 Token (调试用)
        for (i, token) in tokens.iter().take(10).enumerate() {
            if token.token_type == xuanyu::TokenType::文件结束 {
                break;
            }
            println!("  {:4}: {:?}", i + 1, token);
        }
    }

    // ========== 语法分析 ==========
    if mode != RunMode::IrPure {
        println!("\n=== 语法分析 ===");
    }
    let ast = xuanyu::parse(tokens)
        .map_err(|e| format!("语法错误 [{}]: {} (行 {}, 列 {})", 
            e.code, e.message, e.span.start_line, e.span.start_column))?;

    if mode != RunMode::IrPure {
        println!("语法分析完成");
        println!("  函数数量: {}", ast.functions.len());
        
        for func in &ast.functions {
            println!("    - {} (参数: {}, 返回类型: {:?})", 
                func.name, 
                func.params.len(),
                func.return_type
            );
        }
    }

    // ========== 语义分析 ==========
    if mode != RunMode::IrPure {
        println!("\n=== 语义分析 ===");
    }
    xuanyu::analyze(&ast)
        .map_err(|errors| {
            let msg: Vec<String> = errors.iter()
                .map(|e| format!("[{}]: {} (行 {}, 列 {})", 
                    e.code, e.message, e.span.start_line, e.span.start_column))
                .collect();
            format!("语义错误 ({} 个): {}", errors.len(), msg.join(", "))
        })?;

    if mode != RunMode::IrPure {
        println!("语义分析完成，无错误");
    }

    // ========== 调试模式：输出行号映射骨架 ==========
    if debug {
        println!("\n=== 调试模式 (--debug) ===");
        println!("[最小版] 输出源码行号 -> 函数 映射骨架，真实断点/单步待 v0.3.0 接入");
        let mapping = xuanyu::build_line_mapping(&ast);
        for (line, func_name, desc) in &mapping.entries {
            println!("  第 {:4} 行: {} ({})", line, func_name, desc);
        }
        println!("映射记录数: {}", mapping.entries.len());
    }

    // ========== 代码生成 ==========
    if mode != RunMode::IrPure {
        println!("\n=== 代码生成 ===");
    }
    
    // 提取文件名作为模块名，用于生成唯一的函数名（避免多模块链接时符号冲突）
    let module_name = Path::new(filename)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("");
    
    let ir = if debug {
        xuanyu::generate_ir_debug(&ast, module_name, &[])
            .map_err(|e| format!("代码生成错误 [{}]: {}", e.code, e.message))?
    } else {
        xuanyu::generate_ir_with_module_name(&ast, module_name)
            .map_err(|e| format!("代码生成错误 [{}]: {}", e.code, e.message))?
    };

    if mode != RunMode::IrPure {
        println!("代码生成完成");
    }

    // 根据模式执行不同操作
    match mode {
        RunMode::IrOnly => {
            println!("\n--- LLVM IR ---");
            println!("{}", ir);
            println!("\n编译成功!");
        }
        RunMode::IrPure => {
            println!("{}", ir);
        }
        RunMode::IrFile(filepath) => {
            fs::write(&filepath, &ir)
                .map_err(|e| format!("无法写入 IR 文件: {}", e))?;
            println!("IR 已写入: {}", filepath);
        }
        RunMode::Build | RunMode::Run => {
            // 保存 IR 到临时文件 - 使用唯一名称
            let temp_ir = format!("xuanyu_ir_{}.ll", std::process::id());
            fs::write(&temp_ir, &ir)
                .map_err(|e| format!("无法写入临时 IR 文件: {}", e))?;

            println!("\n--- LLVM IR ---");
            println!("{}", ir);

            // 生成对象文件
            println!("\n=== 生成对象文件 ===");
            let temp_obj = "temp_output.o";
            
            // 执行 llc 命令（启用 O2 优化，提升生成代码性能）
            let llc_result = Command::new("llc")
                .arg(&temp_ir)
                .arg("-filetype=obj")
                .arg("-O2")
                .arg("-o")
                .arg(temp_obj)
                .status();

            match llc_result {
                Ok(status) => {
                    if !status.success() {
                        eprintln!("IR 文件保存在: {}", temp_ir);
                        return Err(format!("llc 执行失败，退出码: {}", status.code().unwrap_or(-1)));
                    }
                }
                Err(e) => {
                    eprintln!("IR 文件保存在: {}", temp_ir);
                    return Err(format!("无法执行 llc: {}\n请确保已安装 LLVM 并配置环境变量。", e));
                }
            }

            let _guard = TempFileGuard {
                ir_file: temp_ir.clone(),
                obj_file: temp_obj.to_string(),
            };

            println!("对象文件生成成功: {}", temp_obj);

            // 查找 runtime.c
            let exe_dir = env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| env::current_dir().unwrap_or_default());
            
            // 尝试多个可能的 runtime 路径
            let runtime_paths = vec![
                exe_dir.join("runtime").join("runtime.c"),
                Path::new("runtime").join("runtime.c"),
                Path::new("../runtime/runtime.c").to_path_buf(),
            ];

            let runtime_path = runtime_paths.iter()
                .find(|p| p.exists())
                .cloned()
                .ok_or_else(|| {
                    let paths = runtime_paths.iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("找不到 runtime.c，请确保文件存在于以下位置之一: {}", paths)
                })?;

            println!("找到运行时库: {}", runtime_path.display());

            // 生成可执行文件
            println!("\n=== 链接 ===");
            let output_exe = if cfg!(target_os = "windows") {
                "output.exe"
            } else {
                "output"
            };

            // 编译 runtime.c 为目标文件
            let runtime_obj = "runtime.obj";
            let compile_runtime_result = Command::new("clang")
                .arg("-c")
                .arg(runtime_path)
                .arg("-o")
                .arg(runtime_obj)
                .status();

            match compile_runtime_result {
                Ok(status) => {
                    if !status.success() {
                        return Err(format!("编译 runtime.c 失败，退出码: {}", status.code().unwrap_or(-1)));
                    }
                }
                Err(e) => {
                    return Err(format!("无法执行 clang: {}\n请确保已安装 Clang/LLVM 并配置环境变量.", e));
                }
            }

            let linker_result = Command::new("clang")
                .arg(runtime_obj)
                .arg(temp_obj)
                .arg("-o")
                .arg(output_exe)
                .arg("-Wl,/SUBSYSTEM:console")
                .status();

            match linker_result {
                Ok(status) => {
                    if !status.success() {
                        return Err(format!("链接失败，退出码: {}", status.code().unwrap_or(-1)));
                    }
                }
                Err(e) => {
                    return Err(format!("无法执行 clang: {}\n请确保已安装 Clang/LLVM 并配置环境变量.", e));
                }
            }

            println!("链接成功: {}", output_exe);

            // 更新缓存
            let _ = update_cache(filename, &source.clone());

            println!("\n编译成功!");

            // 如果是运行模式，执行程序
            if mode == RunMode::Run {
                println!("\n--- 运行结果 ---");
                
                let cwd = std::env::current_dir().unwrap_or_default();
                let exe_path = cwd.join(output_exe);
                
                let run_result = Command::new(&exe_path)
                    .current_dir(&cwd)
                    .output();

                match run_result {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        
                        if !stdout.is_empty() {
                            print!("{}", stdout);
                        }
                        if !stderr.is_empty() {
                            eprint!("{}", stderr);
                        }
                        
                        if !output.status.success() {
                            return Err(format!("程序退出码: {}", output.status.code().unwrap_or(-1)));
                        }
                    }
                    Err(e) => {
                        return Err(format!("运行失败: {}", e));
                    }
                }
                println!("----------------");
            }
        }
    }

    Ok(())
}

fn update_expr_function_names(expr: &mut Expr, module_name: &str, func_names: &[String]) {
    match expr {
        Expr::Call(call) => {
            if let Expr::Identifier(ident) = &mut *call.function {
                let original_name = ident.name.clone();
                if func_names.contains(&original_name) && original_name != "主" && original_name != "主函数" && original_name != "main" {
                    ident.name = format!("{}::{}", module_name, original_name);
                }
            }
            update_expr_function_names(&mut *call.function, module_name, func_names);
            for arg in &mut call.arguments {
                update_expr_function_names(arg, module_name, func_names);
            }
        }
        Expr::Binary(binary) => {
            update_expr_function_names(&mut binary.left, module_name, func_names);
            update_expr_function_names(&mut binary.right, module_name, func_names);
        }
        Expr::Unary(unary) => {
            update_expr_function_names(&mut unary.operand, module_name, func_names);
        }
        Expr::MemberAccess(member) => {
            update_expr_function_names(&mut member.object, module_name, func_names);
        }
        Expr::IndexAccess(index) => {
            update_expr_function_names(&mut index.object, module_name, func_names);
            update_expr_function_names(&mut index.index, module_name, func_names);
        }
        Expr::ListLiteral(list) => {
            for item in &mut list.elements {
                update_expr_function_names(item, module_name, func_names);
            }
        }
        Expr::ListComprehension(list_comp) => {
            update_expr_function_names(&mut list_comp.output, module_name, func_names);
            update_expr_function_names(&mut list_comp.iterable, module_name, func_names);
            if let Some(cond) = &mut list_comp.condition {
                update_expr_function_names(cond, module_name, func_names);
            }
        }
        Expr::Await(await_expr) => {
            update_expr_function_names(&mut await_expr.expr, module_name, func_names);
        }
        Expr::Spawn(spawn_expr) => {
            update_expr_function_names(&mut spawn_expr.expr, module_name, func_names);
        }
        _ => {}
    }
}

fn update_stmt_function_names(stmt: &mut xuanyu::ast::Stmt, module_name: &str, func_names: &[String]) {
    match stmt {
        xuanyu::ast::Stmt::Expr(expr_stmt) => {
            update_expr_function_names(&mut expr_stmt.expr, module_name, func_names);
        }
        xuanyu::ast::Stmt::Let(let_stmt) => {
            if let Some(init) = &mut let_stmt.initializer {
                update_expr_function_names(init, module_name, func_names);
            }
        }
        xuanyu::ast::Stmt::Return(ret_stmt) => {
            if let Some(value) = &mut ret_stmt.value {
                update_expr_function_names(value, module_name, func_names);
            }
        }
        xuanyu::ast::Stmt::If(if_stmt) => {
            for branch in &mut if_stmt.branches {
                update_expr_function_names(&mut branch.condition, module_name, func_names);
                update_stmt_function_names(&mut *branch.body, module_name, func_names);
            }
            if let Some(else_branch) = &mut if_stmt.else_branch {
                update_stmt_function_names(else_branch, module_name, func_names);
            }
        }
        xuanyu::ast::Stmt::Loop(loop_stmt) => {
            if let Some(condition) = &mut loop_stmt.condition {
                update_expr_function_names(condition, module_name, func_names);
            }
            update_stmt_function_names(&mut *loop_stmt.body, module_name, func_names);
        }
        xuanyu::ast::Stmt::Block(block_stmt) => {
            for stmt in &mut block_stmt.statements {
                update_stmt_function_names(stmt, module_name, func_names);
            }
        }
        xuanyu::ast::Stmt::Assignment(assign_stmt) => {
            update_expr_function_names(&mut assign_stmt.value, module_name, func_names);
        }
        _ => {}
    }
}

fn compile_multi_file(filename: &str, mode: RunMode, debug: bool) -> Result<(), String> {
    // 如果是纯 IR 模式，不输出调试信息
    if mode != RunMode::IrPure {
        println!("正在编译多文件项目: {}", filename);
    }

    // 创建多文件编译器
    let mut compiler = MultiFileCompiler::new();

    // 添加文件所在目录作为搜索路径
    let file_path = Path::new(filename);
    if let Some(dir) = file_path.parent() {
        compiler.add_search_path(dir.to_path_buf());
    }
    // 添加当前目录作为搜索路径
    compiler.add_search_path(PathBuf::from("."));

    // 解析模块名称
    let module_name = file_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| format!("无法解析模块名称: {}", filename))?;

    // 编译模块
    let modules = compiler.compile(module_name)
        .map_err(|e| format!("编译失败: {:?}", e))?;

    if mode != RunMode::IrPure {
        println!("\n=== 模块依赖 ===");
        println!("共编译 {} 个模块:", modules.len());
        for (i, module) in modules.iter().enumerate() {
            println!("  {}. {}", i + 1, module.path.display());
        }
    }

    // 将所有模块合并为一个模块，然后一次性生成 IR
    // 这样可以避免重复的运行时声明和外部函数声明
    
    // 首先，为每个模块的所有函数调用添加模块名前缀
    let mut updated_modules: Vec<(String, xuanyu::ast::Module)> = Vec::new();
    
    for module in &modules {
        let module_name = module.path.file_stem().unwrap().to_string_lossy().to_string();
        let mut updated_module = module.module.clone();
        
        // 收集当前模块中的函数名
        let current_module_func_names: Vec<String> = updated_module.functions
            .iter()
            .filter(|f| f.name != "主" && f.name != "主函数" && f.name != "main")
            .map(|f| f.name.clone())
            .collect();
        
        // 更新当前模块中的所有函数调用：
        // 1. 更新对当前模块内部函数的调用（添加当前模块前缀）
        // 2. 更新对其他模块函数的调用（添加其他模块前缀）
        for func in updated_module.functions.iter_mut() {
            // 更新对当前模块内部函数的调用
            for stmt in &mut func.body.statements {
                update_stmt_function_names(stmt, &module_name, &current_module_func_names);
            }
            // 更新对其他模块函数的调用
            for other_module in &modules {
                if other_module.path != module.path {
                    let other_module_name = other_module.path.file_stem().unwrap().to_string_lossy().to_string();
                    let other_func_names: Vec<String> = other_module.module.functions
                        .iter()
                        .filter(|f| f.name != "主" && f.name != "主函数" && f.name != "main")
                        .map(|f| f.name.clone())
                        .collect();
                    for stmt in &mut func.body.statements {
                        update_stmt_function_names(stmt, &other_module_name, &other_func_names);
                    }
                }
            }
        }
        
        // 为当前模块的函数添加模块名前缀（主函数和英文 main 别名除外）
        for func in updated_module.functions.iter_mut() {
            if func.name != "主" && func.name != "主函数" && func.name != "main" {
                func.name = format!("{}::{}", module_name, func.name);
            }
        }
        
        updated_modules.push((module_name, updated_module));
    }
    
    // 合并所有更新后的模块
    let mut merged_module = updated_modules[0].1.clone();
    
    // 合并其他模块的内容
    for (_, module) in &updated_modules[1..] {
        // 合并函数
        for func in &module.functions {
            merged_module.functions.push(func.clone());
        }
        // 合并结构体（去重）
        for s in &module.structs {
            if !merged_module.structs.iter().any(|f| f.name == s.name) {
                merged_module.structs.push(s.clone());
            }
        }
        // 合并枚举（去重）
        for e in &module.enums {
            if !merged_module.enums.iter().any(|f| f.name == e.name) {
                merged_module.enums.push(e.clone());
            }
        }
        // 合并外部函数声明（去重：按函数名去重）
        for ext in &module.extern_functions {
            if !merged_module.extern_functions.iter().any(|e| e.name == ext.name) {
                merged_module.extern_functions.push(ext.clone());
            }
        }
        // 合并常量（去重）
        for c in &module.constants {
            if !merged_module.constants.iter().any(|f| f.name == c.name) {
                merged_module.constants.push(c.clone());
            }
        }
        // 合并导入（去重）
        for imp in &module.imports {
            if !merged_module.imports.iter().any(|i| i.module_path == imp.module_path) {
                merged_module.imports.push(imp.clone());
            }
        }
    }

    // 清空导入列表，因为所有导入的模块已经被合并
    // 语义分析器会尝试重新加载导入的模块，导致重复处理
    merged_module.imports.clear();

    // 语义分析
    if mode != RunMode::IrPure {
        println!("\n=== 语义分析 ===");
    }
    xuanyu::analyze(&merged_module)
        .map_err(|errors| {
            let msg: Vec<String> = errors.iter()
                .map(|e| format!("[{}]: {} (行 {}, 列 {})", 
                    e.code, e.message, e.span.start_line, e.span.start_column))
                .collect();
            format!("语义错误 ({} 个): {}", errors.len(), msg.join(", "))
        })?;

    if mode != RunMode::IrPure {
        println!("语义分析完成，无错误");
    }

    // ========== 调试模式：输出行号映射骨架 ==========
    if debug {
        println!("\n=== 调试模式 (--debug) ===");
        println!("[最小版] 输出源码行号 -> 函数 映射骨架，真实断点/单步待 v0.3.0 接入");
        let mapping = xuanyu::build_line_mapping(&merged_module);
        for (line, func_name, desc) in &mapping.entries {
            println!("  第 {:6} 行: {} ({})", line, func_name, desc);
        }
        println!("映射记录数: {}", mapping.entries.len());
    }

    // 一次性生成合并后的 IR
    // 使用主模块名作为模块名前缀，确保生成的函数名唯一
    let combined_ir = if debug {
        xuanyu::generate_ir_debug(&merged_module, module_name, &[])
            .map_err(|e| format!("代码生成错误: {}", e.message))?
    } else {
        xuanyu::generate_ir_with_module_name(&merged_module, module_name)
            .map_err(|e| format!("代码生成错误: {}", e.message))?
    };

    if mode != RunMode::IrPure {
        println!("\n=== 代码生成 ===");
        println!("代码生成完成");
    }

    // 根据模式执行不同操作
    match mode {
        RunMode::IrOnly => {
            println!("\n--- LLVM IR ---");
            println!("{}", combined_ir);
            println!("\n编译成功!");
        }
        RunMode::IrPure => {
            println!("{}", combined_ir);
        }
        RunMode::IrFile(filepath) => {
            fs::write(&filepath, &combined_ir)
                .map_err(|e| format!("无法写入 IR 文件: {}", e))?;
            println!("IR 已写入: {}", filepath);
        }
        RunMode::Build | RunMode::Run => {
            // 保存 IR 到临时文件 - 使用唯一名称
            let temp_ir = format!("xuanyu_ir_{}.ll", std::process::id());
            fs::write(&temp_ir, &combined_ir)
                .map_err(|e| format!("无法写入临时 IR 文件: {}", e))?;

            println!("\n--- LLVM IR ---");
            println!("{}", combined_ir);

            // 生成对象文件
            println!("\n=== 生成对象文件 ===");
            let temp_obj = "temp_output.o";
            
            // 执行 llc 命令（启用 O2 优化，提升生成代码性能）
            let llc_result = Command::new("llc")
                .arg(&temp_ir)
                .arg("-filetype=obj")
                .arg("-O2")
                .arg("-o")
                .arg(temp_obj)
                .status();

            match llc_result {
                Ok(status) => {
                    if !status.success() {
                        eprintln!("IR 文件保存在: {}", temp_ir);
                        return Err(format!("llc 执行失败，退出码: {}", status.code().unwrap_or(-1)));
                    }
                }
                Err(e) => {
                    eprintln!("IR 文件保存在: {}", temp_ir);
                    return Err(format!("无法执行 llc: {}\n请确保已安装 LLVM 并配置环境变量。", e));
                }
            }

            let _guard = TempFileGuard {
                ir_file: temp_ir.clone(),
                obj_file: temp_obj.to_string(),
            };

            println!("对象文件生成成功: {}", temp_obj);

            // 查找 runtime.c
            let exe_dir = env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| env::current_dir().unwrap_or_default());
            
            // 尝试多个可能的 runtime 路径
            let runtime_paths = vec![
                exe_dir.join("runtime").join("runtime.c"),
                Path::new("runtime").join("runtime.c"),
                Path::new("../runtime/runtime.c").to_path_buf(),
            ];

            let runtime_path = runtime_paths.iter()
                .find(|p| p.exists())
                .cloned()
                .ok_or_else(|| {
                    let paths = runtime_paths.iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("找不到 runtime.c，请确保文件存在于以下位置之一: {}", paths)
                })?;

            println!("找到运行时库: {}", runtime_path.display());

            // 生成可执行文件
            println!("\n=== 链接 ===");
            let output_exe = if cfg!(target_os = "windows") {
                "output.exe"
            } else {
                "output"
            };

            // 编译 runtime.c 为目标文件
            let runtime_obj = "runtime.obj";
            let compile_runtime_result = Command::new("clang")
                .arg("-c")
                .arg(runtime_path)
                .arg("-o")
                .arg(runtime_obj)
                .status();

            match compile_runtime_result {
                Ok(status) => {
                    if !status.success() {
                        return Err(format!("编译 runtime.c 失败，退出码: {}", status.code().unwrap_or(-1)));
                    }
                }
                Err(e) => {
                    return Err(format!("无法执行 clang: {}\n请确保已安装 Clang/LLVM 并配置环境变量.", e));
                }
            }

            let linker_result = Command::new("clang")
                .arg(runtime_obj)
                .arg(temp_obj)
                .arg("-o")
                .arg(output_exe)
                .arg("-Wl,/SUBSYSTEM:console")
                .status();

            match linker_result {
                Ok(status) => {
                    if !status.success() {
                        return Err(format!("链接失败，退出码: {}", status.code().unwrap_or(-1)));
                    }
                }
                Err(e) => {
                    return Err(format!("无法执行 clang: {}\n请确保已安装 Clang/LLVM 并配置环境变量.", e));
                }
            }

            println!("链接成功: {}", output_exe);

            println!("\n编译成功!");

            // 如果是运行模式，执行程序
            if mode == RunMode::Run {
                println!("\n--- 运行结果 ---");
                
                let cwd = std::env::current_dir().unwrap_or_default();
                let exe_path = cwd.join(output_exe);
                
                let run_result = Command::new(&exe_path)
                    .current_dir(&cwd)
                    .output();

                match run_result {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        
                        if !stdout.is_empty() {
                            print!("{}", stdout);
                        }
                        if !stderr.is_empty() {
                            eprint!("{}", stderr);
                        }
                        
                        if !output.status.success() {
                            return Err(format!("程序退出码: {}", output.status.code().unwrap_or(-1)));
                        }
                    }
                    Err(e) => {
                        return Err(format!("运行失败: {}", e));
                    }
                }
                println!("----------------");
            }
        }
    }

    Ok(())
}

struct TempFileGuard {
    ir_file: String,
    obj_file: String,
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.ir_file);
        let _ = fs::remove_file(&self.obj_file);
        let _ = fs::remove_file("runtime.obj");
    }
}



fn get_source_hash(source: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

fn get_file_mod_time(filename: &str) -> Option<SystemTime> {
    fs::metadata(filename).ok().and_then(|m| m.modified().ok())
}

fn check_cache(filename: &str, source: &str) -> Result<bool, String> {
    let cache_file = format!("{}.cache", filename);
    let source_hash = get_source_hash(source);
    let source_mod_time = get_file_mod_time(filename);

    if let Ok(cache_content) = fs::read_to_string(&cache_file) {
        let parts: Vec<&str> = cache_content.split(',').collect();
        if parts.len() >= 2 {
            if let (Ok(cache_hash), Ok(cache_time)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
                if cache_hash == source_hash {
                    if let Some(mod_time) = source_mod_time {
                        if let Ok(duration) = mod_time.duration_since(SystemTime::UNIX_EPOCH) {
                            if duration.as_secs() == cache_time {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(false)
}

fn update_cache(filename: &str, source: &str) -> Result<(), String> {
    let cache_file = format!("{}.cache", filename);
    let source_hash = get_source_hash(source);
    let source_mod_time = get_file_mod_time(filename)
        .ok_or_else(|| "无法获取文件修改时间".to_string())?;

    let duration = source_mod_time.duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| "无法计算时间戳")?;

    let cache_content = format!("{},{}", source_hash, duration.as_secs());
    fs::write(&cache_file, cache_content)
        .map_err(|e| format!("无法写入缓存文件: {}", e))?;

    println!("[缓存] 已更新缓存");
    Ok(())
}

fn print_usage(program: &str) {
    println!("CCAS 玄语编译器 (xuanyu) {}", xuanyu::version!());
    println!();
    println!("用法: {} <源文件> [选项]", program);
    println!("      {} repl [选项]", program);
    println!();
    println!("命令:");
    println!("  repl, --repl, -i    启动交互式 REPL 环境");
    println!();
    println!("选项:");
    println!("  -h, --help    显示此帮助信息");
    println!("  --ir          只生成 LLVM IR (带调试信息, 默认)");
    println!("  --ir-pure     只输出纯 LLVM IR (无调试信息)");
    println!("  --build       生成可执行文件");
    println!("  --run         编译并运行程序");
    println!("  --debug       调试模式：输出源码行号映射骨架 (v0.3.0 最小版)");
    println!();
    println!("示例:");
    println!("  {} hello.xy          只生成 IR", program);
    println!("  {} hello.xy --build  生成可执行文件", program);
    println!("  {} hello.xy --run    编译并运行", program);
    println!("  {} hello.xy --debug  输出调试行号映射", program);
    println!("  {} repl              启动交互式环境", program);
}