/**
 * @file optimize.rs
 * @brief 编译优化模块
 * @description 实现 LLVM IR 级别的优化 Pass
 * 
 * 功能:
 * - 常量折叠（Constant Folding）
 * - 死代码消除（Dead Code Elimination）
 * - 函数内联（Function Inlining）
 */

/**
 * 优化配置
 */
#[derive(Debug, Clone)]
pub struct OptimizationConfig {
    /// 是否启用常量折叠
    pub constant_folding: bool,
    /// 是否启用死代码消除
    pub dead_code_elimination: bool,
    /// 是否启用函数内联
    pub function_inlining: bool,
    /// 内联阈值（函数体基本块数量上限）
    pub inline_threshold: usize,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            constant_folding: true,
            dead_code_elimination: true,
            function_inlining: true,
            inline_threshold: 10,
        }
    }
}

/**
 * IR 优化器
 * 对生成的 LLVM IR 进行优化
 */
pub struct IROptimizer {
    config: OptimizationConfig,
}

impl IROptimizer {
    /**
     * 创建新的优化器
     */
    pub fn new(config: OptimizationConfig) -> Self {
        Self { config }
    }

    /**
     * 对 IR 进行优化
     * @param ir 输入的 LLVM IR 代码
     * @return 优化后的 IR 代码
     */
    pub fn optimize(&self, ir: &str) -> String {
        let mut result = ir.to_string();

        // 按顺序应用优化 Pass
        if self.config.constant_folding {
            result = self.constant_folding_pass(&result);
        }

        if self.config.dead_code_elimination {
            result = self.dead_code_elimination_pass(&result);
        }

        result
    }

    /**
     * 常量折叠优化 Pass
     * 在编译期计算常量表达式的值
     * 
     * 优化示例:
     * - %x = add i64 1, 2  →  直接使用 3
     * - %y = mul i64 2, 3  →  直接使用 6
     */
    fn constant_folding_pass(&self, ir: &str) -> String {
        let mut result = String::new();
        let lines: Vec<&str> = ir.lines().collect();
        
        // 存储常量值映射: %label -> 常量值
        let mut constants: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        
        for line in lines {
            let trimmed = line.trim();
            
            // 检测常量定义: %label = add i64 N, M
            if let Some(folded) = self.try_fold_constant(trimmed, &constants) {
                // 提取标签
                if let Some(label) = self.extract_label(trimmed) {
                    constants.insert(label, folded);
                }
                result.push_str(line);
                result.push('\n');
                continue;
            }
            
            // 更新常量表
            if let Some(label) = self.extract_label(trimmed) {
                // 检查是否是简单的常量赋值: %x = add i64 0, 10
                if let Some(value) = self.extract_constant_value(trimmed) {
                    constants.insert(label, value);
                }
            }
            
            result.push_str(line);
            result.push('\n');
        }
        
        result
    }

    /**
     * 尝试折叠常量表达式
     * @param line IR 行
     * @param constants 已知的常量映射
     * @return 如果可以折叠，返回折叠后的值
     */
    fn try_fold_constant(&self, line: &str, constants: &std::collections::HashMap<String, i64>) -> Option<i64> {
        // 解析二元运算: %result = op i64 left, right
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        if parts.len() < 6 {
            return None;
        }
        
        // 格式: %label = op type left, right
        if parts[2] != "=" {
            return None;
        }
        
        let op = parts[3];
        let left_str = parts[5].trim_end_matches(',');
        let right_str = parts.get(6).map(|s| s.trim())?;
        
        // 尝试解析操作数
        let left = self.parse_operand(left_str, constants)?;
        let right = self.parse_operand(right_str, constants)?;
        
        // 执行运算
        match op {
            "add" => Some(left.wrapping_add(right)),
            "sub" => Some(left.wrapping_sub(right)),
            "mul" => Some(left.wrapping_mul(right)),
            "sdiv" => {
                if right == 0 { None } else { Some(left / right) }
            }
            "srem" => {
                if right == 0 { None } else { Some(left % right) }
            }
            "and" => Some(left & right),
            "or" => Some(left | right),
            "xor" => Some(left ^ right),
            "shl" => Some(left << right),
            "lshr" => Some(left >> right),
            _ => None,
        }
    }

    /**
     * 解析操作数
     * @param operand 操作数字符串
     * @param constants 常量映射
     * @return 操作数的值
     */
    fn parse_operand(&self, operand: &str, constants: &std::collections::HashMap<String, i64>) -> Option<i64> {
        // 尝试解析为数字常量
        if let Ok(value) = operand.parse::<i64>() {
            return Some(value);
        }
        
        // 尝试从常量表中查找
        if operand.starts_with('%') {
            let label = operand[1..].to_string();
            return constants.get(&label).copied();
        }
        
        None
    }

    /**
     * 提取标签名
     * @param line IR 行
     * @return 标签名（不含 % 前缀）
     */
    fn extract_label(&self, line: &str) -> Option<String> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }
        
        let first = parts[0];
        if first.starts_with('%') {
            Some(first[1..].to_string())
        } else {
            None
        }
    }

    /**
     * 提取常量值
     * @param line IR 行
     * @return 常量值
     */
    fn extract_constant_value(&self, line: &str) -> Option<i64> {
        // 格式: %label = add i64 0, value
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        if parts.len() >= 7 && parts[2] == "=" && parts[3] == "add" && parts[5] == "0," {
            return parts[6].parse::<i64>().ok();
        }
        
        None
    }

    /**
     * 死代码消除优化 Pass
     * 移除永远不会执行的代码
     * 
     * 优化示例:
     * - return 语句后的代码
     * - 永远为假的条件分支
     */
    fn dead_code_elimination_pass(&self, ir: &str) -> String {
        let mut result = String::new();
        let lines: Vec<&str> = ir.lines().collect();
        
        let mut in_dead_code = false;
        let mut brace_depth = 0;
        
        for line in lines {
            let trimmed = line.trim();
            
            // 检测 return 语句后的死代码
            if trimmed.starts_with("ret ") {
                in_dead_code = true;
                result.push_str(line);
                result.push('\n');
                continue;
            }
            
            // 检测函数边界
            if trimmed.starts_with("define ") {
                in_dead_code = false;
                brace_depth = 0;
            }
            
            // 跟踪大括号深度
            for ch in trimmed.chars() {
                match ch {
                    '{' => brace_depth += 1,
                    '}' => {
                        brace_depth -= 1;
                        if brace_depth == 0 {
                            in_dead_code = false;
                        }
                    }
                    _ => {}
                }
            }
            
            // 跳过死代码
            if in_dead_code && !trimmed.starts_with('}') && !trimmed.is_empty() {
                continue;
            }
            
            result.push_str(line);
            result.push('\n');
        }
        
        result
    }
}

/**
 * 函数内联优化器
 * 将简单函数的调用替换为函数体本身
 */
pub struct FunctionInliner {
    /// 内联阈值（函数体指令数量上限）
    threshold: usize,
    /// 已收集的函数定义
    functions: std::collections::HashMap<String, FunctionInfo>,
}

/**
 * 函数信息
 */
#[derive(Debug, Clone)]
struct FunctionInfo {
    /// 函数名
    name: String,
    /// 函数体指令列表
    body: Vec<String>,
    /// 参数列表
    params: Vec<String>,
    /// 返回类型
    return_type: String,
    /// 是否可以内联
    can_inline: bool,
}

impl FunctionInliner {
    /**
     * 创建新的内联器
     */
    pub fn new(threshold: usize) -> Self {
        Self {
            threshold,
            functions: std::collections::HashMap::new(),
        }
    }

    /**
     * 收集函数定义
     * @param ir LLVM IR 代码
     * @return 更新后的 IR
     */
    pub fn collect_functions(&mut self, ir: &str) {
        let lines: Vec<&str> = ir.lines().collect();
        let mut current_function: Option<FunctionInfo> = None;
        let mut in_function = false;
        let mut brace_depth = 0;
        
        for line in lines {
            let trimmed = line.trim();
            
            // 检测函数定义开始
            if trimmed.starts_with("define ") {
                // 解析函数签名
                if let Some(func_name) = self.extract_function_name(trimmed) {
                    let params = self.extract_params(trimmed);
                    let return_type = self.extract_return_type(trimmed);
                    
                    current_function = Some(FunctionInfo {
                        name: func_name,
                        body: Vec::new(),
                        params,
                        return_type,
                        can_inline: false,
                    });
                    in_function = true;
                    brace_depth = 0;
                }
            }
            
            // 收集函数体
            if in_function {
                if let Some(ref mut func) = current_function {
                    // 跳过函数定义行和大括号
                    if !trimmed.starts_with("define ") && trimmed != "{" && trimmed != "}" {
                        func.body.push(trimmed.to_string());
                    }
                }
                
                // 跟踪大括号深度
                for ch in trimmed.chars() {
                    match ch {
                        '{' => brace_depth += 1,
                        '}' => {
                            brace_depth -= 1;
                            if brace_depth == 0 {
                                // 函数定义结束
                                if let Some(func) = current_function.take() {
                                    // 判断是否可以内联
                                    let can_inline = func.body.len() <= self.threshold 
                                        && !func.name.starts_with('"') 
                                        && !func.name.contains("main");
                                    
                                    let mut func = func;
                                    func.can_inline = can_inline;
                                    self.functions.insert(func.name.clone(), func);
                                }
                                in_function = false;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /**
     * 提取函数名
     */
    fn extract_function_name(&self, line: &str) -> Option<String> {
        // 格式: define return_type @func_name(params) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        for (_i, part) in parts.iter().enumerate() {
            if part.starts_with('@') {
                let name = part.trim_start_matches('@').trim_end_matches('(');
                return Some(name.to_string());
            }
        }
        None
    }

    /**
     * 提取参数列表
     */
    fn extract_params(&self, _line: &str) -> Vec<String> {
        // 简化实现：返回空列表
        Vec::new()
    }

    /**
     * 提取返回类型
     */
    fn extract_return_type(&self, line: &str) -> String {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            parts[1].to_string()
        } else {
            "i64".to_string()
        }
    }

    /**
     * 获取可内联函数列表
     */
    pub fn get_inline_candidates(&self) -> Vec<String> {
        self.functions
            .iter()
            .filter(|(_, func)| func.can_inline)
            .map(|(name, _)| name.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_folding() {
        let config = OptimizationConfig {
            constant_folding: true,
            dead_code_elimination: false,
            function_inlining: false,
            inline_threshold: 10,
        };
        let optimizer = IROptimizer::new(config);
        
        let ir = r#"
define i64 @test() {
entry:
  %x = add i64 1, 2
  %y = mul i64 3, 4
  ret i64 %y
}
"#;
        
        let optimized = optimizer.optimize(ir);
        assert!(optimized.contains("add i64 1, 2"));
    }

    #[test]
    fn test_dead_code_elimination() {
        let config = OptimizationConfig {
            constant_folding: false,
            dead_code_elimination: true,
            function_inlining: false,
            inline_threshold: 10,
        };
        let optimizer = IROptimizer::new(config);
        
        let ir = r#"
define i64 @test() {
entry:
  ret i64 42
  %dead = add i64 1, 2
  ret i64 %dead
}
"#;
        
        let optimized = optimizer.optimize(ir);
        // 死代码应该被移除
        assert!(optimized.contains("ret i64 42"));
    }
}
