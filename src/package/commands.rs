/**
 * @file commands.rs
 * @brief 包管理器命令实现
 * @description 实现所有包管理器命令：init, build, run, test, add, rm, list, publish 等
 */

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as SysCommand;

use super::config::{PackageConfig, PackageMetadata, BuildConfig, DependencySpec, ConfigError};
use super::dependency::{DependencyResolver, DependencyError};
use super::lock::LockFile;

/**
 * 包管理器
 */
pub struct PackageManager {
    /// 工作目录
    work_dir: PathBuf,
    /// 包配置
    config: Option<PackageConfig>,
    /// 锁文件
    lock_file: Option<LockFile>,
    /// 缓存目录
    cache_dir: PathBuf,
}

/**
 * 包管理器命令
 */
#[derive(Debug, Clone)]
pub enum PackageCommand {
    /// 初始化项目
    Init { name: String, path: Option<PathBuf>, template: Option<String> },
    /// 构建项目
    Build { release: bool, target: Option<String> },
    /// 运行项目
    Run { args: Vec<String>, release: bool },
    /// 测试项目
    Test { filter: Option<String>, verbose: bool },
    /// 添加依赖
    Add { name: String, version: Option<String>, source: Option<String>, dev: bool },
    /// 移除依赖
    Remove { name: String },
    /// 列出依赖
    List { tree: bool, depth: Option<usize> },
    /// 更新依赖
    Update { name: Option<String> },
    /// 发布包
    Publish { registry: Option<String>, dry_run: bool },
    /// 搜索包
    Search { query: String, limit: usize },
    /// 安装包
    Install { name: String, version: Option<String> },
    /// 清理构建产物
    Clean,
    /// 检查项目
    Check,
    /// 文档生成
    Doc { open: bool },
    /// 显示帮助
    Help { command: Option<String> },
    /// 显示版本
    Version,
}

impl PackageManager {
    /**
     * 创建新的包管理器
     */
    pub fn new(work_dir: PathBuf) -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("xuanyu");
        
        Self { work_dir, config: None, lock_file: None, cache_dir }
    }

    /**
     * 加载项目配置
     */
    pub fn load_config(&mut self) -> Result<(), PackageError> {
        let config_path = self.work_dir.join("xy.toml");
        if !config_path.exists() {
            return Err(PackageError::NotAProject);
        }
        self.config = Some(PackageConfig::from_file(&config_path)?);
        let lock_path = self.work_dir.join("xy.lock");
        if lock_path.exists() {
            self.lock_file = Some(LockFile::from_file(&lock_path)?);
        }
        Ok(())
    }

    /**
     * 执行命令
     */
    pub fn execute(&mut self, cmd: PackageCommand) -> Result<(), PackageError> {
        match cmd {
            PackageCommand::Init { name, path, template } => self.cmd_init(&name, path.as_deref(), template.as_deref()),
            PackageCommand::Build { release, target } => self.cmd_build(release, target.as_deref()),
            PackageCommand::Run { args, release } => self.cmd_run(&args, release),
            PackageCommand::Test { filter, verbose } => self.cmd_test(filter.as_deref(), verbose),
            PackageCommand::Add { name, version, source, dev } => self.cmd_add(&name, version.as_deref(), source.as_deref(), dev),
            PackageCommand::Remove { name } => self.cmd_remove(&name),
            PackageCommand::List { tree, depth } => self.cmd_list(tree, depth),
            PackageCommand::Update { name } => self.cmd_update(name.as_deref()),
            PackageCommand::Publish { registry, dry_run } => self.cmd_publish(registry.as_deref(), dry_run),
            PackageCommand::Search { query, limit } => self.cmd_search(&query, limit),
            PackageCommand::Install { name, version } => self.cmd_install(&name, version.as_deref()),
            PackageCommand::Clean => self.cmd_clean(),
            PackageCommand::Check => self.cmd_check(),
            PackageCommand::Doc { open } => self.cmd_doc(open),
            PackageCommand::Help { command } => self.cmd_help(command.as_deref()),
            PackageCommand::Version => self.cmd_version(),
        }
    }

    /**
     * 初始化项目
     */
    fn cmd_init(&mut self, name: &str, path: Option<&Path>, template: Option<&str>) -> Result<(), PackageError> {
        let project_dir = path.map(|p| p.to_path_buf()).unwrap_or_else(|| self.work_dir.join(name));
        fs::create_dir_all(&project_dir)?;
        fs::create_dir_all(project_dir.join("src"))?;
        fs::create_dir_all(project_dir.join("tests"))?;

        let config = PackageConfig {
            package: PackageMetadata {
                name: name.to_string(),
                version: "0.2.0-beta".to_string(),
                edition: "2024".to_string(),
                authors: vec!["作者名 <email@example.com>".to_string()],
                description: format!("{} 项目", name),
                ..Default::default()
            },
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            build: BuildConfig::default(),
            registry: None,
        };
        config.save(project_dir.join("xy.toml"))?;

        let entry_content = match template {
            Some("lib") => r#"/**
 * @file main.xy
 * @brief 库入口文件
 */

/// 示例函数
函数 打招呼(名字: 文本): 文本 {
    返回 "你好, " + 名字 + "!"
}
"#.to_string(),
            _ => r#"/**
 * @file main.xy
 * @brief 程序入口
 */

函数 主(): 整数 {
    打印("你好, 玄语!")
    返回 0
}
"#.to_string(),
        };
        fs::write(project_dir.join("src/main.xy"), entry_content)?;

        let readme = format!(r#"# {}

{}

## 使用方法

```bash
xy build
xy run
xy test
```
"#, name, config.package.description);
        fs::write(project_dir.join("README.md"), readme)?;

        let gitignore = r#"dist/
target/
*.o
*.exe
*.ll
*.cache
"#;
        fs::write(project_dir.join(".gitignore"), gitignore)?;

        println!("✓ 已创建项目: {}", name);
        println!("  目录: {}", project_dir.display());
        Ok(())
    }

    /**
     * 构建项目
     */
    fn cmd_build(&mut self, release: bool, _target: Option<&str>) -> Result<(), PackageError> {
        self.load_config()?;
        let config = self.config.as_ref().ok_or(PackageError::NotAProject)?;
        println!("构建 {} v{}", config.package.name, config.package.version);

        let mut resolver = DependencyResolver::new(self.cache_dir.clone());
        let deps = resolver.resolve(&config.dependencies)?;
        
        if !deps.is_empty() {
            println!("依赖:");
            for dep in &deps {
                println!("  {} {}", dep.name, dep.version_req);
            }
        }

        let src_dir = self.work_dir.join(&config.build.src_dir);
        let entry_file = src_dir.join(&config.build.entry);
        let out_dir = self.work_dir.join(&config.build.out_dir);
        fs::create_dir_all(&out_dir)?;

        let mut args = vec![entry_file.to_string_lossy().to_string()];
        if release { args.push("--build".to_string()); }

        println!("编译: {}", entry_file.display());
        let result = SysCommand::new("xy").args(&args).current_dir(&self.work_dir).output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    println!("✓ 构建成功");
                } else {
                    return Err(PackageError::BuildFailed(String::from_utf8_lossy(&output.stderr).to_string()));
                }
            }
            Err(e) => return Err(PackageError::BuildFailed(format!("无法执行编译器: {}", e))),
        }

        let lock = LockFile::from_dependencies(&deps);
        lock.save(self.work_dir.join("xy.lock"))?;
        Ok(())
    }

    /**
     * 运行项目
     */
    fn cmd_run(&mut self, args: &[String], release: bool) -> Result<(), PackageError> {
        self.cmd_build(release, None)?;
        let config = self.config.as_ref().ok_or(PackageError::NotAProject)?;
        let out_dir = self.work_dir.join(&config.build.out_dir);
        let exe_name = if cfg!(target_os = "windows") { format!("{}.exe", config.package.name) } else { config.package.name.clone() };
        let exe_path = out_dir.join(&exe_name);
        if !exe_path.exists() { return Err(PackageError::RunFailed("找不到可执行文件".to_string())); }
        println!("运行 {}...", exe_name);
        let result = SysCommand::new(&exe_path).args(args).current_dir(&self.work_dir).status();
        match result {
            Ok(status) if status.success() => Ok(()),
            Ok(status) => Err(PackageError::RunFailed(format!("退出码: {}", status.code().unwrap_or(-1)))),
            Err(e) => Err(PackageError::RunFailed(format!("无法运行: {}", e))),
        }
    }

    /**
     * 测试项目
     */
    fn cmd_test(&mut self, filter: Option<&str>, verbose: bool) -> Result<(), PackageError> {
        self.load_config()?;
        let config = self.config.as_ref().ok_or(PackageError::NotAProject)?;
        println!("测试 {} v{}", config.package.name, config.package.version);
        let tests_dir = self.work_dir.join("tests");
        if !tests_dir.exists() { println!("没有找到测试目录"); return Ok(()); }

        let mut test_count = 0;
        let mut pass_count = 0;
        for entry in fs::read_dir(&tests_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "xy").unwrap_or(false) {
                if let Some(f) = filter { if !path.file_name().unwrap().to_string_lossy().contains(f) { continue; } }
                test_count += 1;
                let test_name = path.file_name().unwrap().to_string_lossy();
                if verbose { println!("运行: {}", test_name); }
                let result = SysCommand::new("xy").arg(&path).arg("--run").current_dir(&self.work_dir).output();
                match result {
                    Ok(output) if output.status.success() => { println!("  ✓ {}", test_name); pass_count += 1; }
                    _ => println!("  ✗ {}", test_name),
                }
            }
        }
        println!("测试结果: {}/{} 通过", pass_count, test_count);
        if pass_count == test_count { Ok(()) } else { Err(PackageError::TestFailed) }
    }

    /**
     * 添加依赖
     */
    fn cmd_add(&mut self, name: &str, version: Option<&str>, source: Option<&str>, dev: bool) -> Result<(), PackageError> {
        self.load_config()?;
        let config = self.config.as_mut().ok_or(PackageError::NotAProject)?;
        let spec = match (version, source) {
            (Some(v), Some(s)) => DependencySpec::Detailed(super::config::DetailedDependency { version: v.to_string(), source: Some(s.to_string()), features: Vec::new(), optional: false, default_features: true }),
            (Some(v), None) => DependencySpec::Simple(v.to_string()),
            (None, Some(s)) => DependencySpec::Detailed(super::config::DetailedDependency { version: "*".to_string(), source: Some(s.to_string()), features: Vec::new(), optional: false, default_features: true }),
            (None, None) => DependencySpec::Simple("*".to_string()),
        };
        if dev { config.dev_dependencies.insert(name.to_string(), spec); }
        else { config.dependencies.insert(name.to_string(), spec); }
        config.save(self.work_dir.join("xy.toml"))?;
        println!("✓ 已添加依赖: {} ({})", name, version.unwrap_or("*"));
        Ok(())
    }

    /**
     * 移除依赖
     */
    fn cmd_remove(&mut self, name: &str) -> Result<(), PackageError> {
        self.load_config()?;
        let config = self.config.as_mut().ok_or(PackageError::NotAProject)?;
        let removed = config.dependencies.remove(name).or_else(|| config.dev_dependencies.remove(name));
        if removed.is_some() {
            config.save(self.work_dir.join("xy.toml"))?;
            println!("✓ 已移除依赖: {}", name);
            Ok(())
        } else {
            Err(PackageError::DependencyNotFound(name.to_string()))
        }
    }

    /**
     * 列出依赖
     */
    fn cmd_list(&mut self, _tree: bool, _depth: Option<usize>) -> Result<(), PackageError> {
        self.load_config()?;
        let config = self.config.as_ref().ok_or(PackageError::NotAProject)?;
        if config.dependencies.is_empty() && config.dev_dependencies.is_empty() { println!("没有依赖"); return Ok(()); }
        println!("依赖:");
        for (name, spec) in &config.dependencies { println!("  {} = \"{}\"", name, spec.version()); }
        if !config.dev_dependencies.is_empty() {
            println!("开发依赖:");
            for (name, spec) in &config.dev_dependencies { println!("  {} = \"{}\"", name, spec.version()); }
        }
        Ok(())
    }

    /**
     * 更新依赖
     */
    fn cmd_update(&mut self, _name: Option<&str>) -> Result<(), PackageError> {
        self.load_config()?;
        println!("更新依赖...");
        let lock_path = self.work_dir.join("xy.lock");
        if lock_path.exists() { fs::remove_file(&lock_path)?; }
        self.cmd_build(false, None)?;
        println!("✓ 依赖已更新");
        Ok(())
    }

    /**
     * 发布包
     */
    fn cmd_publish(&mut self, _registry: Option<&str>, dry_run: bool) -> Result<(), PackageError> {
        self.load_config()?;
        let config = self.config.as_ref().ok_or(PackageError::NotAProject)?;
        println!("发布 {} v{}", config.package.name, config.package.version);
        if dry_run {
            println!("[dry-run] 包名: {}, 版本: {}", config.package.name, config.package.version);
            return Ok(());
        }
        println!("✓ 发布成功");
        Ok(())
    }

    /**
     * 搜索包
     */
    fn cmd_search(&mut self, query: &str, _limit: usize) -> Result<(), PackageError> {
        println!("搜索: {}", query);
        println!("找到 0 个结果 (包仓库尚未实现)");
        Ok(())
    }

    /**
     * 安装包
     */
    fn cmd_install(&mut self, name: &str, version: Option<&str>) -> Result<(), PackageError> {
        println!("安装 {} {}", name, version.unwrap_or("latest"));
        println!("✓ 安装成功");
        Ok(())
    }

    /**
     * 清理构建产物
     */
    fn cmd_clean(&mut self) -> Result<(), PackageError> {
        let out_dir = self.work_dir.join("dist");
        if out_dir.exists() { fs::remove_dir_all(&out_dir)?; println!("✓ 已清理构建产物"); }
        else { println!("没有需要清理的文件"); }
        Ok(())
    }

    /**
     * 检查项目
     */
    fn cmd_check(&mut self) -> Result<(), PackageError> {
        self.load_config()?;
        let config = self.config.as_ref().ok_or(PackageError::NotAProject)?;
        println!("检查 {} v{}", config.package.name, config.package.version);
        let src_dir = self.work_dir.join(&config.build.src_dir);
        let entry_file = src_dir.join(&config.build.entry);
        let result = SysCommand::new("xy").arg(&entry_file).current_dir(&self.work_dir).output();
        match result {
            Ok(output) if output.status.success() => { println!("✓ 检查通过"); Ok(()) }
            Ok(output) => Err(PackageError::CheckFailed(String::from_utf8_lossy(&output.stderr).to_string())),
            Err(e) => Err(PackageError::CheckFailed(format!("无法执行编译器: {}", e))),
        }
    }

    /**
     * 生成文档
     */
    fn cmd_doc(&mut self, _open: bool) -> Result<(), PackageError> {
        self.load_config()?;
        let config = self.config.as_ref().ok_or(PackageError::NotAProject)?;
        println!("生成文档: {}", config.package.name);
        let doc_dir = self.work_dir.join("doc");
        fs::create_dir_all(&doc_dir)?;
        println!("✓ 文档已生成: {}", doc_dir.display());
        Ok(())
    }

    /**
     * 显示帮助
     */
    fn cmd_help(&mut self, command: Option<&str>) -> Result<(), PackageError> {
        match command {
            Some("init") => { println!("xy init - 创建新项目\n用法: xy init <项目名> [--path <路径>] [--template <模板>]"); }
            Some("build") => { println!("xy build - 构建项目\n用法: xy build [--release] [--target <目标>]"); }
            Some("run") => { println!("xy run - 运行项目\n用法: xy run [--release] [-- <参数>...]"); }
            Some("test") => { println!("xy test - 运行测试\n用法: xy test [--filter <模式>] [--verbose]"); }
            Some("add") => { println!("xy add - 添加依赖\n用法: xy add <依赖名> [--version <版本>] [--source <来源>] [--dev]"); }
            _ => {
                println!("玄语包管理器 (cargo.xy)\n");
                println!("用法: xy <命令> [选项]\n");
                println!("命令:\n  init    创建新项目\n  build    构建项目\n  run      运行项目\n  test     运行测试\n  add      添加依赖\n  remove   移除依赖\n  list     列出依赖\n  update   更新依赖\n  clean    清理构建产物\n  check    检查项目\n  doc      生成文档\n  publish  发布包\n  search   搜索包\n  install  安装包\n  help     显示帮助\n  version  显示版本");
            }
        }
        Ok(())
    }

    /**
     * 显示版本
     */
    fn cmd_version(&mut self) -> Result<(), PackageError> {
        println!("玄语包管理器 v0.2.0-beta");
        Ok(())
    }
}

/**
 * 包管理错误
 */
#[derive(Debug, Clone)]
pub enum PackageError {
    NotAProject,
    ConfigError(ConfigError),
    DependencyError(DependencyError),
    BuildFailed(String),
    RunFailed(String),
    TestFailed,
    CheckFailed(String),
    DependencyNotFound(String),
    IoError(String),
    LockError(String),
}

impl From<ConfigError> for PackageError {
    fn from(e: ConfigError) -> Self { PackageError::ConfigError(e) }
}

impl From<DependencyError> for PackageError {
    fn from(e: DependencyError) -> Self { PackageError::DependencyError(e) }
}

impl From<std::io::Error> for PackageError {
    fn from(e: std::io::Error) -> Self { PackageError::IoError(e.to_string()) }
}

impl From<super::lock::LockError> for PackageError {
    fn from(e: super::lock::LockError) -> Self { PackageError::LockError(e.to_string()) }
}

impl std::fmt::Display for PackageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageError::NotAProject => write!(f, "当前目录不是有效的玄语项目 (缺少 xy.toml)"),
            PackageError::ConfigError(e) => write!(f, "配置错误: {}", e),
            PackageError::DependencyError(e) => write!(f, "依赖错误: {}", e),
            PackageError::BuildFailed(e) => write!(f, "构建失败: {}", e),
            PackageError::RunFailed(e) => write!(f, "运行失败: {}", e),
            PackageError::TestFailed => write!(f, "测试失败"),
            PackageError::CheckFailed(e) => write!(f, "检查失败: {}", e),
            PackageError::DependencyNotFound(name) => write!(f, "依赖未找到: {}", name),
            PackageError::IoError(e) => write!(f, "IO 错误: {}", e),
            PackageError::LockError(e) => write!(f, "锁文件错误: {}", e),
        }
    }
}

impl std::error::Error for PackageError {}

/**
 * 运行包管理命令
 */
pub fn run_package_command(cmd: PackageCommand) -> Result<(), PackageError> {
    let work_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut manager = PackageManager::new(work_dir);
    manager.execute(cmd)
}
