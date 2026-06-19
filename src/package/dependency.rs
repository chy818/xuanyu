/**
 * @file dependency.rs
 * @brief 依赖解析和管理
 * @description 实现依赖图构建、版本冲突检测、传递依赖解析
 */

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use super::config::{DependencySpec, Version};

/**
 * 依赖信息
 */
#[derive(Debug, Clone)]
pub struct Dependency {
    /// 依赖名称
    pub name: String,
    /// 版本要求
    pub version_req: String,
    /// 解析后的版本
    pub resolved_version: Option<Version>,
    /// 来源
    pub source: DependencySource,
    /// 传递依赖
    pub dependencies: Vec<Dependency>,
    /// 是否可选
    pub optional: bool,
    /// 启用的特性
    pub features: Vec<String>,
    /// 本地路径
    pub path: Option<PathBuf>,
}

/**
 * 依赖来源
 */
#[derive(Debug, Clone, PartialEq)]
pub enum DependencySource {
    /// 官方仓库
    Registry,
    /// Git 仓库
    Git { url: String, rev: Option<String> },
    /// 本地路径
    Path(PathBuf),
    /// GitHub
    GitHub { user: String, repo: String },
}

/**
 * 依赖解析器
 */
pub struct DependencyResolver {
    /// 已解析的依赖
    resolved: HashMap<String, Dependency>,
    /// 依赖图 (依赖名 -> 依赖它的包列表)
    dependency_graph: HashMap<String, Vec<String>>,
    /// 冲突检测
    conflicts: Vec<DependencyConflict>,
    /// 下载缓存目录
    cache_dir: PathBuf,
}

/**
 * 依赖冲突
 */
#[derive(Debug, Clone)]
pub struct DependencyConflict {
    /// 依赖名称
    pub name: String,
    /// 冲突的版本要求
    pub version_reqs: Vec<(String, String)>, // (包名, 版本要求)
    /// 建议解决方案
    pub suggestion: String,
}

impl DependencyResolver {
    /**
     * 创建新的依赖解析器
     */
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            resolved: HashMap::new(),
            dependency_graph: HashMap::new(),
            conflicts: Vec::new(),
            cache_dir,
        }
    }

    /**
     * 解析依赖
     * @param dependencies 依赖列表
     * @return 解析后的依赖图
     */
    pub fn resolve(
        &mut self,
        dependencies: &HashMap<String, DependencySpec>,
    ) -> Result<Vec<Dependency>, DependencyError> {
        // 清空之前的状态
        self.resolved.clear();
        self.dependency_graph.clear();
        self.conflicts.clear();

        // 构建待解析队列
        let mut queue: VecDeque<(String, DependencySpec, Option<String>)> = dependencies
            .iter()
            .map(|(name, spec)| (name.clone(), spec.clone(), None))
            .collect();

        // BFS 解析依赖
        while let Some((name, spec, parent)) = queue.pop_front() {
            // 检查是否已解析
            if let Some(existing) = self.resolved.get(&name) {
                // 检查版本兼容性
                if !self.check_version_compatibility(&name, &spec, existing) {
                    self.conflicts.push(DependencyConflict {
                        name: name.clone(),
                        version_reqs: vec![
                            (parent.clone().unwrap_or_default(), spec.version().to_string()),
                            ("已解析".to_string(), existing.version_req.clone()),
                        ],
                        suggestion: format!("请统一 {} 的版本要求", name),
                    });
                }
                continue;
            }

            // 解析依赖来源
            let source = self.parse_source(&spec)?;

            // 解析版本
            let resolved_version = Version::parse(spec.version()).ok();

            // 创建依赖对象
            let dep = Dependency {
                name: name.clone(),
                version_req: spec.version().to_string(),
                resolved_version,
                source,
                dependencies: Vec::new(),
                optional: spec.is_optional(),
                features: self.get_features(&spec),
                path: None,
            };

            // 记录依赖关系
            if let Some(ref p) = parent {
                self.dependency_graph
                    .entry(name.clone())
                    .or_default()
                    .push(p.clone());
            }

            // 添加到已解析
            self.resolved.insert(name.clone(), dep);

            // TODO: 获取传递依赖并加入队列
        }

        // 检查冲突
        if !self.conflicts.is_empty() {
            return Err(DependencyError::Conflicts(self.conflicts.clone()));
        }

        // 返回拓扑排序后的依赖列表
        Ok(self.topological_sort())
    }

    /**
     * 解析依赖来源
     */
    fn parse_source(&self, spec: &DependencySpec) -> Result<DependencySource, DependencyError> {
        if let Some(source) = spec.source() {
            // 解析来源字符串
            if source.starts_with("git:") || source.starts_with("git+") {
                let url = source.trim_start_matches("git+").to_string();
                Ok(DependencySource::Git { url, rev: None })
            } else if source.starts_with("path:") {
                let path = source.trim_start_matches("path:");
                Ok(DependencySource::Path(PathBuf::from(path)))
            } else if source.starts_with("github:") {
                let parts: Vec<&str> = source.trim_start_matches("github:").split('/').collect();
                if parts.len() >= 2 {
                    Ok(DependencySource::GitHub {
                        user: parts[0].to_string(),
                        repo: parts[1].to_string(),
                    })
                } else {
                    Err(DependencyError::InvalidSource(source.to_string()))
                }
            } else {
                Ok(DependencySource::Registry)
            }
        } else {
            Ok(DependencySource::Registry)
        }
    }

    /**
     * 获取特性列表
     */
    fn get_features(&self, spec: &DependencySpec) -> Vec<String> {
        match spec {
            DependencySpec::Simple(_) => Vec::new(),
            DependencySpec::Detailed(d) => d.features.clone(),
        }
    }

    /**
     * 检查版本兼容性
     */
    fn check_version_compatibility(
        &self,
        _name: &str,
        new_spec: &DependencySpec,
        existing: &Dependency,
    ) -> bool {
        let new_version = Version::parse(new_spec.version());
        let existing_version = existing.resolved_version.clone();

        match (new_version, existing_version) {
            (Ok(new), Some(existing)) => {
                // 检查新版本是否满足现有版本
                existing.satisfies(new_spec.version())
                    || new.satisfies(&existing.to_string())
            }
            _ => true,
        }
    }

    /**
     * 拓扑排序
     */
    fn topological_sort(&self) -> Vec<Dependency> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_marks = HashSet::new();

        for name in self.resolved.keys() {
            if !visited.contains(name) {
                self.visit(name, &mut visited, &mut temp_marks, &mut result);
            }
        }

        result
    }

    /**
     * 访问节点 (DFS)
     */
    fn visit(
        &self,
        name: &str,
        visited: &mut HashSet<String>,
        temp_marks: &mut HashSet<String>,
        result: &mut Vec<Dependency>,
    ) {
        if visited.contains(name) {
            return;
        }
        if temp_marks.contains(name) {
            // 检测到循环依赖
            return;
        }

        temp_marks.insert(name.to_string());

        // 访问依赖
        if let Some(deps) = self.dependency_graph.get(name) {
            for dep_name in deps {
                self.visit(dep_name, visited, temp_marks, result);
            }
        }

        temp_marks.remove(name);
        visited.insert(name.to_string());

        if let Some(dep) = self.resolved.get(name) {
            result.push(dep.clone());
        }
    }

    /**
     * 获取依赖冲突
     */
    pub fn get_conflicts(&self) -> &[DependencyConflict] {
        &self.conflicts
    }

    /**
     * 检查是否有循环依赖
     */
    pub fn has_cycles(&self) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for name in self.resolved.keys() {
            if self.detect_cycle(name, &mut visited, &mut rec_stack) {
                return true;
            }
        }
        false
    }

    /**
     * 检测循环
     */
    fn detect_cycle(
        &self,
        name: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        if rec_stack.contains(name) {
            return true;
        }
        if visited.contains(name) {
            return false;
        }

        visited.insert(name.to_string());
        rec_stack.insert(name.to_string());

        if let Some(deps) = self.dependency_graph.get(name) {
            for dep_name in deps {
                if self.detect_cycle(dep_name, visited, rec_stack) {
                    return true;
                }
            }
        }

        rec_stack.remove(name);
        false
    }

    /**
     * 获取缓存目录
     */
    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    /**
     * 下载依赖
     */
    pub fn download(&self, dep: &Dependency) -> Result<PathBuf, DependencyError> {
        let target_dir = self.cache_dir.join(&dep.name);

        match &dep.source {
            DependencySource::Registry => {
                // TODO: 从官方仓库下载
                println!("[下载] {} {} 从官方仓库", dep.name, dep.version_req);
                Ok(target_dir)
            }
            DependencySource::Git { url, rev: _ } => {
                println!("[下载] {} 从 Git: {}", dep.name, url);
                // TODO: git clone
                Ok(target_dir)
            }
            DependencySource::Path(path) => {
                println!("[本地] {} 从 {}", dep.name, path.display());
                Ok(path.clone())
            }
            DependencySource::GitHub { user, repo } => {
                println!("[下载] {} 从 GitHub: {}/{}", dep.name, user, repo);
                // TODO: 从 GitHub 下载
                Ok(target_dir)
            }
        }
    }
}

/**
 * 依赖错误
 */
#[derive(Debug, Clone)]
pub enum DependencyError {
    /// 版本冲突
    Conflicts(Vec<DependencyConflict>),
    /// 无效来源
    InvalidSource(String),
    /// 下载失败
    DownloadFailed(String),
    /// 循环依赖
    CircularDependency(String),
    /// 解析失败
    ParseError(String),
}

impl std::fmt::Display for DependencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DependencyError::Conflicts(conflicts) => {
                writeln!(f, "依赖冲突:")?;
                for c in conflicts {
                    writeln!(f, "  {}: {}", c.name, c.suggestion)?;
                }
                Ok(())
            }
            DependencyError::InvalidSource(s) => write!(f, "无效的依赖来源: {}", s),
            DependencyError::DownloadFailed(s) => write!(f, "下载失败: {}", s),
            DependencyError::CircularDependency(s) => write!(f, "检测到循环依赖: {}", s),
            DependencyError::ParseError(s) => write!(f, "解析错误: {}", s),
        }
    }
}

impl std::error::Error for DependencyError {}

/**
 * 依赖图
 */
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    /// 节点
    pub nodes: HashMap<String, DependencyNode>,
    /// 边 (from -> to)
    pub edges: Vec<(String, String)>,
}

/**
 * 依赖节点
 */
#[derive(Debug, Clone)]
pub struct DependencyNode {
    /// 依赖名称
    pub name: String,
    /// 版本
    pub version: String,
    /// 深度 (距离根节点的距离)
    pub depth: usize,
    /// 入度
    pub in_degree: usize,
    /// 出度
    pub out_degree: usize,
}

impl DependencyGraph {
    /**
     * 创建空图
     */
    pub fn new() -> Self {
        Self::default()
    }

    /**
     * 添加节点
     */
    pub fn add_node(&mut self, name: &str, version: &str, depth: usize) {
        self.nodes.insert(name.to_string(), DependencyNode {
            name: name.to_string(),
            version: version.to_string(),
            depth,
            in_degree: 0,
            out_degree: 0,
        });
    }

    /**
     * 添加边
     */
    pub fn add_edge(&mut self, from: &str, to: &str) {
        self.edges.push((from.to_string(), to.to_string()));
        
        if let Some(node) = self.nodes.get_mut(from) {
            node.out_degree += 1;
        }
        if let Some(node) = self.nodes.get_mut(to) {
            node.in_degree += 1;
        }
    }

    /**
     * 获取依赖树字符串
     */
    pub fn to_tree_string(&self) -> String {
        let mut output = String::new();
        let mut visited = HashSet::new();
        
        // 找到根节点 (入度为 0)
        let roots: Vec<&str> = self.nodes.values()
            .filter(|n| n.in_degree == 0)
            .map(|n| n.name.as_str())
            .collect();

        for root in roots {
            self.print_tree(root, "", &mut output, &mut visited);
        }

        output
    }

    /**
     * 打印树结构
     */
    fn print_tree(
        &self,
        name: &str,
        prefix: &str,
        output: &mut String,
        visited: &mut HashSet<String>,
    ) {
        if visited.contains(name) {
            output.push_str(&format!("{}{} (已显示)\n", prefix, name));
            return;
        }
        visited.insert(name.to_string());

        if let Some(node) = self.nodes.get(name) {
            output.push_str(&format!("{}{} v{}\n", prefix, name, node.version));
        }

        // 找到子节点
        let children: Vec<&str> = self.edges.iter()
            .filter(|(from, _)| from == name)
            .map(|(_, to)| to.as_str())
            .collect();

        for (i, child) in children.iter().enumerate() {
            let is_last = i == children.len() - 1;
            let new_prefix = if is_last {
                format!("{}  ", prefix)
            } else {
                format!("{}│ ", prefix)
            };
            let connector = if is_last { "└─ " } else { "├─ " };
            output.push_str(&format!("{}{}", prefix, connector));
            self.print_tree(child, &new_prefix, output, visited);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver() {
        let mut deps = HashMap::new();
        deps.insert("std".to_string(), DependencySpec::Simple("0.1.0".to_string()));
        
        let cache_dir = std::env::temp_dir().join("xuanyu_cache");
        let mut resolver = DependencyResolver::new(cache_dir);
        
        let result = resolver.resolve(&deps);
        assert!(result.is_ok());
    }
}
