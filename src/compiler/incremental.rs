/**
 * @file incremental.rs
 * @brief 增量编译系统
 * @description 通过检测文件变更、实现模块级增量编译，加速大型项目的构建
 * 
 * 功能特性:
 * - 模块级增量编译
 * - 依赖图管理
 * - 变更检测 (文件哈希)
 * - 并行编译支持
 * - 构建缓存
 */

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

/**
 * 文件变更类型
 */
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileChange {
    /// 文件已创建
    Created,
    /// 文件已修改
    Modified,
    /// 文件已删除
    Deleted,
    /// 文件未变更
    Unchanged,
}

/**
 * 模块信息
 */
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// 模块路径
    pub path: PathBuf,
    /// 模块名称
    pub name: String,
    /// 依赖的模块
    pub dependencies: Vec<String>,
    /// 文件最后修改时间
    pub last_modified: u64,
    /// 文件内容哈希
    pub content_hash: u64,
    /// 编译产物路径
    pub output_path: Option<PathBuf>,
    /// 是否需要重新编译
    pub needs_rebuild: bool,
}

impl serde::Serialize for ModuleInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ModuleInfo", 8)?;
        state.serialize_field("path", &self.path.to_string_lossy().to_string())?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("dependencies", &self.dependencies)?;
        state.serialize_field("last_modified", &self.last_modified)?;
        state.serialize_field("content_hash", &self.content_hash)?;
        state.serialize_field("output_path", &self.output_path.as_ref().map(|p| p.to_string_lossy().to_string()))?;
        state.serialize_field("needs_rebuild", &self.needs_rebuild)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for ModuleInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct ModuleInfoHelper {
            path: String,
            name: String,
            dependencies: Vec<String>,
            last_modified: u64,
            content_hash: u64,
            output_path: Option<String>,
            needs_rebuild: bool,
        }

        let helper = ModuleInfoHelper::deserialize(deserializer)?;
        Ok(ModuleInfo {
            path: PathBuf::from(helper.path),
            name: helper.name,
            dependencies: helper.dependencies,
            last_modified: helper.last_modified,
            content_hash: helper.content_hash,
            output_path: helper.output_path.map(PathBuf::from),
            needs_rebuild: helper.needs_rebuild,
        })
    }
}

/**
 * 编译任务
 */
#[derive(Debug, Clone)]
pub struct CompileTask {
    /// 要编译的模块
    pub module: String,
    /// 依赖的任务
    pub dependencies: Vec<String>,
    /// 优先级 (数字越小优先级越高)
    pub priority: usize,
}

/**
 * 增量编译结果
 */
#[derive(Debug, Clone)]
pub struct IncrementalResult {
    /// 需要重新编译的模块列表
    pub modules_to_rebuild: Vec<String>,
    /// 可复用的模块列表
    pub modules_to_skip: Vec<String>,
    /// 构建时间统计
    pub build_stats: BuildStats,
}

/**
 * 构建统计信息
 */
#[derive(Debug, Clone, Default)]
pub struct BuildStats {
    /// 总构建时间 (毫秒)
    pub total_time_ms: u64,
    /// 编译的模块数
    pub compiled_modules: usize,
    /// 跳过的模块数
    pub skipped_modules: usize,
    /// 并行度
    pub parallelism: usize,
}

/**
 * 增量编译系统
 */
pub struct IncrementalCompiler {
    /// 模块信息表
    modules: HashMap<String, ModuleInfo>,
    /// 依赖图 (模块 -> 依赖它的模块)
    dependency_graph: HashMap<String, Vec<String>>,
    /// 编译缓存目录
    cache_dir: PathBuf,
    /// 已编译模块的最后构建时间
    last_build_time: u64,
    /// 启用增量编译
    enabled: bool,
}

impl IncrementalCompiler {
    /**
     * 创建新的增量编译器
     */
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            modules: HashMap::new(),
            dependency_graph: HashMap::new(),
            cache_dir,
            last_build_time: 0,
            enabled: false,
        }
    }

    /**
     * 启用/禁用增量编译
     */
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /**
     * 检查增量编译是否启用
     */
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /**
     * 检查模块是否已注册
     */
    pub fn is_defined(&self, name: &str) -> bool {
        self.modules.contains_key(name)
    }

    /**
     * 注册模块
     */
    pub fn register_module(&mut self, path: PathBuf, name: String, dependencies: Vec<String>) -> Result<(), IncrCompileError> {
        // 检查文件是否存在
        if !path.exists() {
            return Err(IncrCompileError::ModuleNotFound(path.to_string_lossy().to_string()));
        }

        // 获取文件信息
        let metadata = fs::metadata(&path)
            .map_err(|e| IncrCompileError::IoError(e.to_string()))?;
        
        let last_modified = metadata.modified()
            .map_err(|e| IncrCompileError::IoError(e.to_string()))?
            .duration_since(UNIX_EPOCH)
            .map_err(|e| IncrCompileError::IoError(e.to_string()))?
            .as_secs();

        // 计算内容哈希
        let content_hash = self.compute_file_hash(&path)?;

        // 创建模块信息
        let module_info = ModuleInfo {
            path: path.clone(),
            name: name.clone(),
            dependencies: dependencies.clone(),
            last_modified,
            content_hash,
            output_path: None,
            needs_rebuild: true,
        };

        // 更新依赖图
        for dep in &dependencies {
            self.dependency_graph
                .entry(dep.clone())
                .or_default()
                .push(name.clone());
        }

        self.modules.insert(name.clone(), module_info);

        Ok(())
    }

    /**
     * 计算文件哈希
     */
    fn compute_file_hash(&self, path: &Path) -> Result<u64, IncrCompileError> {
        let content = fs::read(path)
            .map_err(|e| IncrCompileError::IoError(e.to_string()))?;
        
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        
        Ok(hasher.finish())
    }

    /**
     * 检测文件变更
     */
    pub fn detect_changes(&mut self) -> HashMap<String, FileChange> {
        let mut changes = HashMap::new();

        // 先收集所有模块路径和名称
        let module_entries: Vec<(String, PathBuf)> = self.modules.iter()
            .map(|(name, module)| (name.clone(), module.path.clone()))
            .collect();

        // 遍历模块，检查变更
        for (name, path) in module_entries {
            let current_hash = self.compute_file_hash(&path).unwrap_or(0);
            let module = self.modules.get_mut(&name).unwrap();

            let change = if !path.exists() {
                FileChange::Deleted
            } else if current_hash != module.content_hash {
                module.content_hash = current_hash;
                FileChange::Modified
            } else {
                FileChange::Unchanged
            };

            changes.insert(name, change);
        }

        changes
    }

    /**
     * 确定需要重新编译的模块
     */
    pub fn get_modules_to_rebuild(&mut self) -> IncrementalResult {
        let start_time = SystemTime::now();

        // 检测变更
        let changes = self.detect_changes();

        // 找出需要重新编译的模块
        let mut to_rebuild = HashSet::new();
        let mut to_skip = HashSet::new();

        for (name, change) in &changes {
            match change {
                FileChange::Modified | FileChange::Deleted | FileChange::Created => {
                    // 文件变更，需要重新编译
                    to_rebuild.insert(name.clone());
                    if let Some(module) = self.modules.get_mut(name) {
                        module.needs_rebuild = true;
                    }
                }
                FileChange::Unchanged => {
                    // 检查依赖是否需要重新编译
                    let deps_need_rebuild = self.dependencies_need_rebuild(name);

                    if deps_need_rebuild {
                        to_rebuild.insert(name.clone());
                        if let Some(module) = self.modules.get_mut(name) {
                            module.needs_rebuild = true;
                        }
                    } else {
                        to_skip.insert(name.clone());
                        if let Some(module) = self.modules.get_mut(name) {
                            module.needs_rebuild = false;
                        }
                    }
                }
            }
        }

        // 计算拓扑排序，确定编译顺序
        let sorted = self.topological_sort(&to_rebuild);

        // 计算构建统计
        let elapsed = start_time.elapsed()
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let build_stats = BuildStats {
            total_time_ms: elapsed,
            compiled_modules: to_rebuild.len(),
            skipped_modules: to_skip.len(),
            parallelism: num_cpus(),
        };

        self.last_build_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        IncrementalResult {
            modules_to_rebuild: sorted,
            modules_to_skip: to_skip.into_iter().collect(),
            build_stats,
        }
    }

    /**
     * 检查依赖是否需要重新编译
     */
    fn dependencies_need_rebuild(&self, module_name: &str) -> bool {
        if let Some(deps) = self.dependency_graph.get(module_name) {
            for dep in deps {
                if let Some(dep_module) = self.modules.get(dep) {
                    if dep_module.needs_rebuild {
                        return true;
                    }
                }
            }
        }
        false
    }

    /**
     * 拓扑排序 (Kahn算法)
     */
    fn topological_sort(&self, modules: &HashSet<String>) -> Vec<String> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

        // 初始化
        for module in modules {
            in_degree.insert(module.clone(), 0);
            adjacency.insert(module.clone(), Vec::new());
        }

        // 构建依赖图
        for module in modules {
            if let Some(module_info) = self.modules.get(module) {
                for dep in &module_info.dependencies {
                    if modules.contains(dep) {
                        adjacency.get_mut(dep).unwrap().push(module.clone());
                        *in_degree.get_mut(module).unwrap() += 1;
                    }
                }
            }
        }

        // 找到入度为0的节点
        let mut queue: Vec<String> = in_degree.iter()
            .filter(|(_, &d)| d == 0)
            .map(|(n, _)| n.clone())
            .collect();

        let mut result = Vec::new();

        // BFS
        while let Some(node) = queue.pop() {
            result.push(node.clone());

            if let Some(neighbors) = adjacency.get(&node) {
                for neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(neighbor.clone());
                        }
                    }
                }
            }
        }

        result
    }

    /**
     * 获取全部模块信息的快照（用于后续与最新状态比对）
     */
    pub fn get_modules_map(&self) -> HashMap<String, ModuleInfo> {
        self.modules.clone()
    }

    /**
     * 与历史快照比对，返回自上次构建以来发生变更的模块名集合。
     * - 新出现 / 已删除 / 内容哈希或修改时间变化的模块均视为变更。
     */
    pub fn changed_since(&self, previous: &HashMap<String, ModuleInfo>) -> HashSet<String> {
        let mut changed = HashSet::new();
        for (name, cur) in &self.modules {
            match previous.get(name) {
                Some(prev) => {
                    if prev.content_hash != cur.content_hash || prev.last_modified != cur.last_modified {
                        changed.insert(name.clone());
                    }
                }
                None => {
                    changed.insert(name.clone());
                }
            }
        }
        for prev_name in previous.keys() {
            if !self.modules.contains_key(prev_name) {
                changed.insert(prev_name.clone());
            }
        }
        changed
    }

    /**
     * 获取模块信息
     */
    pub fn get_module(&self, name: &str) -> Option<&ModuleInfo> {
        self.modules.get(name)
    }

    /**
     * 列出所有模块
     */
    pub fn list_modules(&self) -> Vec<&str> {
        self.modules.keys().map(|s| s.as_str()).collect()
    }

    /**
     * 获取依赖图
     */
    pub fn get_dependency_graph(&self) -> &HashMap<String, Vec<String>> {
        &self.dependency_graph
    }

    /**
     * 清除编译缓存
     */
    pub fn clear_cache(&mut self) -> Result<(), IncrCompileError> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)
                .map_err(|e| IncrCompileError::IoError(e.to_string()))?;
        }
        fs::create_dir_all(&self.cache_dir)
            .map_err(|e| IncrCompileError::IoError(e.to_string()))?;

        // 重置所有模块的编译状态
        for module in self.modules.values_mut() {
            module.needs_rebuild = true;
        }

        Ok(())
    }

    /**
     * 保存构建状态到缓存
     */
    pub fn save_state(&self) -> Result<(), IncrCompileError> {
        // 确保缓存目录存在（首次构建时可能尚未创建）
        fs::create_dir_all(&self.cache_dir)
            .map_err(|e| IncrCompileError::IoError(e.to_string()))?;

        let state_file = self.cache_dir.join("build_state.json");
        
        let state = BuildState {
            modules: self.modules.clone(),
            last_build_time: self.last_build_time,
        };

        let content = serde_json::to_string_pretty(&state)
            .map_err(|e| IncrCompileError::SerializeError(e.to_string()))?;

        fs::write(state_file, content)
            .map_err(|e| IncrCompileError::IoError(e.to_string()))?;

        Ok(())
    }

    /**
     * 从缓存加载构建状态
     */
    pub fn load_state(&mut self) -> Result<(), IncrCompileError> {
        let state_file = self.cache_dir.join("build_state.json");
        
        if !state_file.exists() {
            return Ok(()); // 没有缓存文件，直接返回
        }

        let content = fs::read_to_string(&state_file)
            .map_err(|e| IncrCompileError::IoError(e.to_string()))?;

        let state: BuildState = serde_json::from_str(&content)
            .map_err(|e| IncrCompileError::DeserializeError(e.to_string()))?;

        self.modules = state.modules;
        self.last_build_time = state.last_build_time;

        Ok(())
    }
}

/**
 * 构建状态 (用于序列化)
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BuildState {
    modules: HashMap<String, ModuleInfo>,
    last_build_time: u64,
}

/**
 * 获取CPU核心数
 */
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1)
}

/**
 * 增量编译错误
 */
#[derive(Debug, Clone)]
pub enum IncrCompileError {
    /// 模块未找到
    ModuleNotFound(String),
    /// IO错误
    IoError(String),
    /// 序列化错误
    SerializeError(String),
    /// 反序列化错误
    DeserializeError(String),
    /// 循环依赖
    CircularDependency(String),
    /// 缓存错误
    CacheError(String),
}

impl std::fmt::Display for IncrCompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IncrCompileError::ModuleNotFound(m) => write!(f, "模块未找到: {}", m),
            IncrCompileError::IoError(e) => write!(f, "IO错误: {}", e),
            IncrCompileError::SerializeError(e) => write!(f, "序列化错误: {}", e),
            IncrCompileError::DeserializeError(e) => write!(f, "反序列化错误: {}", e),
            IncrCompileError::CircularDependency(d) => write!(f, "循环依赖: {}", d),
            IncrCompileError::CacheError(e) => write!(f, "缓存错误: {}", e),
        }
    }
}

impl std::error::Error for IncrCompileError {}

impl Default for IncrementalCompiler {
    fn default() -> Self {
        Self::new(PathBuf::from(".cache/xuanyu"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_registration() {
        let cache_dir = std::env::temp_dir().join("xuanyu_test");
        let mut compiler = IncrementalCompiler::new(cache_dir);

        // 创建一个临时文件
        let temp_file = std::env::temp_dir().join("test_module.xy");
        fs::write(&temp_file, "函数 测试() { }").unwrap();

        compiler.register_module(
            temp_file,
            "test_module".to_string(),
            vec![]
        ).unwrap();

        assert!(compiler.is_defined("test_module"));
    }

    #[test]
    fn test_topological_sort() {
        let cache_dir = std::env::temp_dir().join("xuanyu_test");
        let compiler = IncrementalCompiler::new(cache_dir);

        let mut modules = HashSet::new();
        modules.insert("a".to_string());
        modules.insert("b".to_string());
        modules.insert("c".to_string());

        // a depends on b, b depends on c
        // 简化测试，不注册依赖关系
        let sorted = compiler.topological_sort(&modules);

        // 由于没有依赖关系，结果顺序可能不同
        assert_eq!(sorted.len(), 3);
    }

    #[test]
    fn test_save_load_state_roundtrip() {
        // 使用唯一缓存目录，避免多个测试并发互相干扰
        let cache_dir = std::env::temp_dir().join(format!("xuanyu_cache_roundtrip_{}", std::process::id()));
        let _ = fs::remove_dir_all(&cache_dir);

        let temp_file = std::env::temp_dir().join("roundtrip_module.xy");
        fs::write(&temp_file, "函数 测试() { 返回 0 }").unwrap();

        // 第一次：注册并保存状态
        {
            let mut compiler = IncrementalCompiler::new(cache_dir.clone());
            compiler.register_module(temp_file.clone(), "roundtrip".to_string(), vec![]).unwrap();
            compiler.save_state().unwrap();
        }

        // 第二次：重新加载，模块应存在于状态中且无需重建
        {
            let mut compiler = IncrementalCompiler::new(cache_dir.clone());
            compiler.load_state().unwrap();
            let stored = compiler.get_modules_map();
            assert!(stored.contains_key("roundtrip"), "状态应包含已注册模块");
        }

        let _ = fs::remove_dir_all(&cache_dir);
    }

    #[test]
    fn test_changed_since_detects_modification() {
        let cache_dir = std::env::temp_dir().join(format!("xuanyu_cache_modify_{}", std::process::id()));
        let _ = fs::remove_dir_all(&cache_dir);

        let temp_file = std::env::temp_dir().join("changed_module.xy");
        fs::write(&temp_file, "函数 A() { 返回 1 }").unwrap();

        let mut compiler = IncrementalCompiler::new(cache_dir.clone());
        compiler.register_module(temp_file.clone(), "mod_a".to_string(), vec![]).unwrap();
        compiler.save_state().unwrap();

        // 载入历史快照，再重新注册（内容已修改）→ 应检测到变更
        {
            compiler.load_state().unwrap();
            let prev = compiler.get_modules_map();
            compiler.register_module(temp_file.clone(), "mod_a".to_string(), vec![]).unwrap();
            let changed = compiler.changed_since(&prev);
            assert!(changed.is_empty(), "内容未变时不应检测到变更");
        }

        // 修改文件内容后重新注册 → 应检测到变更
        {
            fs::write(&temp_file, "函数 A() { 返回 2 }").unwrap();
            compiler.load_state().unwrap();
            let prev = compiler.get_modules_map();
            compiler.register_module(temp_file.clone(), "mod_a".to_string(), vec![]).unwrap();
            let changed = compiler.changed_since(&prev);
            assert!(changed.contains("mod_a"), "内容修改后应检测到变更，got {:?}", changed);
        }

        let _ = fs::remove_dir_all(&cache_dir);
    }

    #[test]
    fn test_changed_since_detects_deletion() {
        let cache_dir = std::env::temp_dir().join(format!("xuanyu_cache_deletion_{}", std::process::id()));
        let _ = fs::remove_dir_all(&cache_dir);

        // 历史状态：登记 mod_b 并保存
        let temp_file = std::env::temp_dir().join("deleted_module.xy");
        fs::write(&temp_file, "函数 B() { 返回 0 }").unwrap();

        let keeper = std::env::temp_dir().join("kept_module.xy");
        fs::write(&keeper, "函数 K() { 返回 1 }").unwrap();

        let mut compiler = IncrementalCompiler::new(cache_dir.clone());
        compiler.register_module(temp_file.clone(), "mod_b".to_string(), vec![]).unwrap();
        compiler.save_state().unwrap();

        // 下一次构建：mod_b 已被删除，仅登记存活模块 → 历史中的 mod_b 应视为变更
        // （与 MultiFileCompiler::incremental_change_set 的比对流程保持一致：
        //   历史快照与「当前存活模块集」分别用两个实例承载）
        let mut snap = IncrementalCompiler::new(cache_dir.clone());
        snap.load_state().unwrap();
        let prev = snap.get_modules_map();

        let mut current = IncrementalCompiler::new(cache_dir.clone());
        current.register_module(keeper, "mod_kept".to_string(), vec![]).unwrap();
        let changed = current.changed_since(&prev);
        assert!(changed.contains("mod_b"), "已删除模块应视为变更，got {:?}", changed);
        assert!(changed.contains("mod_kept"), "新增模块应视为变更，got {:?}", changed);

        let _ = fs::remove_dir_all(&cache_dir);
    }
}
