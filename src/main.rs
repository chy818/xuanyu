/**
 * @file main.rs
 * @brief 玄语编译器 (xy) 主程序入口
 * @description 编译器命令行工具，用于编译 .xy 源文件
 */

use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, exit};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        exit(1);
    }

    // 解析参数
    let mut input_file = String::new();
    let mut run_mode = RunMode::IrOnly; // 默认只生成 IR

    for (i, arg) in args.iter().enumerate() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage(&args[0]);
                exit(0);
            }
            "--run" => {
                run_mode = RunMode::Run;
            }
            "--build" => {
                run_mode = RunMode::Build;
            }
            "--ir" => {
                run_mode = RunMode::IrOnly;
            }
            _ => {
                if i > 0 && !arg.starts_with('-') && input_file.is_empty() {
                    input_file = arg.clone();
                }
            }
        }
    }

    if input_file.is_empty() {
        eprintln!("错误: 请指定输入文件");
        print_usage(&args[0]);
        exit(1);
    }

    // 执行编译流程
    if let Err(e) = compile_file(&input_file, run_mode) {
        eprintln!("编译失败: {}", e);
        exit(1);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum RunMode {
    IrOnly,  // 只生成 IR
    Build,   // 生成可执行文件
    Run,     // 编译并运行
}

fn compile_file(filename: &str, mode: RunMode) -> Result<(), String> {
    // 读取源文件
    let source = fs::read_to_string(filename)
        .map_err(|e| format!("无法读取文件 '{}': {}", filename, e))?;

    println!("正在编译: {}", filename);
    println!("源文件大小: {} 字节", source.len());

    // ========== 词法分析 ==========
    println!("\n=== 词法分析 ===");
    let mut lexer = xuanyu::Lexer::new(source);
    
    let tokens = lexer.tokenize()
        .map_err(|e| format!("词法错误 [{}]: {} (行 {}, 列 {})", 
            e.code, e.message, e.span.start_line, e.span.start_column))?;
    
    println!("词法分析完成，共 {} 个 Token", tokens.len());

    // 打印前 10 个 Token (调试用)
    for (i, token) in tokens.iter().take(10).enumerate() {
        if token.token_type == xuanyu::TokenType::文件结束 {
            break;
        }
        println!("  {:4}: {:?}", i + 1, token);
    }

    // ========== 语法分析 ==========
    println!("\n=== 语法分析 ===");
    let ast = xuanyu::parse(tokens)
        .map_err(|e| format!("语法错误 [{}]: {} (行 {}, 列 {})", 
            e.code, e.message, e.span.start_line, e.span.start_column))?;

    println!("语法分析完成");
    println!("  函数数量: {}", ast.functions.len());
    
    for func in &ast.functions {
        println!("    - {} (参数: {}, 返回类型: {:?})", 
            func.name, 
            func.params.len(),
            func.return_type
        );
    }

    // ========== 语义分析 ==========
    println!("\n=== 语义分析 ===");
    xuanyu::analyze(&ast)
        .map_err(|errors| {
            let msg: Vec<String> = errors.iter()
                .map(|e| format!("[{}]: {} (行 {}, 列 {})", 
                    e.code, e.message, e.span.start_line, e.span.start_column))
                .collect();
            format!("语义错误 ({} 个): {}", errors.len(), msg.join(", "))
        })?;

    println!("语义分析完成，无错误");

    // ========== 代码生成 ==========
    println!("\n=== 代码生成 ===");
    let ir = xuanyu::generate_ir(&ast)
        .map_err(|e| format!("代码生成错误 [{}]: {}", e.code, e.message))?;

    println!("代码生成完成");

    // 根据模式执行不同操作
    match mode {
        RunMode::IrOnly => {
            println!("\n--- LLVM IR ---");
            println!("{}", ir);
            println!("\n编译成功!");
        }
        RunMode::Build | RunMode::Run => {
            // 保存 IR 到临时文件
            let temp_ir = "temp_output.ll";
            fs::write(temp_ir, &ir)
                .map_err(|e| format!("无法写入临时 IR 文件: {}", e))?;

            println!("\n--- LLVM IR ---");
            println!("{}", ir);

            // 生成对象文件
            println!("\n=== 生成对象文件 ===");
            let temp_obj = "temp_output.o";
            
            let llc_result = Command::new("llc")
                .arg(temp_ir)
                .arg("-filetype=obj")
                .arg("-o")
                .arg(temp_obj)
                .output();

            match llc_result {
                Ok(output) => {
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        // 清理临时文件
                        let _ = fs::remove_file(temp_ir);
                        return Err(format!("llc 执行失败: {}", stderr));
                    }
                }
                Err(e) => {
                    let _ = fs::remove_file(temp_ir);
                    return Err(format!("无法执行 llc: {}\n请确保已安装 LLVM 并配置环境变量。", e));
                }
            }

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

            let linker_result = Command::new("clang")
                .arg(runtime_path)
                .arg(temp_obj)
                .arg("-o")
                .arg(output_exe)
                .output();

            match linker_result {
                Ok(output) => {
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        cleanup(temp_ir, temp_obj);
                        return Err(format!("链接失败: {}", stderr));
                    }
                }
                Err(e) => {
                    let _ = fs::remove_file(temp_ir);
                    let _ = fs::remove_file(temp_obj);
                    return Err(format!("无法执行 clang: {}\n请确保已安装 Clang/LLVM 并配置环境变量。", e));
                }
            }

            println!("链接成功: {}", output_exe);

            // 清理临时文件
            cleanup(temp_ir, temp_obj);

            println!("\n编译成功!");

            // 如果是运行模式，执行程序
            if mode == RunMode::Run {
                println!("\n--- 运行结果 ---");
                let run_result = Command::new(output_exe)
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

fn cleanup(ir_file: &str, obj_file: &str) {
    let _ = fs::remove_file(ir_file);
    let _ = fs::remove_file(obj_file);
}

fn print_usage(program: &str) {
    println!("CCAS 玄语编译器 (xuanyu) v0.1.0");
    println!();
    println!("用法: {} <源文件> [选项]", program);
    println!();
    println!("选项:");
    println!("  -h, --help    显示此帮助信息");
    println!("  --ir          只生成 LLVM IR (默认)");
    println!("  --build       生成可执行文件");
    println!("  --run         编译并运行程序");
    println!();
    println!("示例:");
    println!("  {} hello.xy          只生成 IR", program);
    println!("  {} hello.xy --build  生成可执行文件", program);
    println!("  {} hello.xy --run    编译并运行", program);
}
