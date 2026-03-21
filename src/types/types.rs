/**
 * @file types.rs
 * @brief CCAS 类型系统
 * @description 定义类型系统和类型检查相关功能
 */

/**
 * CCAS 基本类型
 */
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CCASType {
    Int,
    Long,
    Float,
    Double,
    Bool,
    String,
    Char,
    Void,
    Array(Box<CCASType>),
    Pointer(Box<CCASType>),
    List,
    Custom(String),
}

impl CCASType {
    /**
     * 获取类型的字节大小
     */
    pub fn size(&self) -> usize {
        match self {
            CCASType::Int => 4,
            CCASType::Long => 8,
            CCASType::Float => 4,
            CCASType::Double => 8,
            CCASType::Bool => 1,
            CCASType::String => 8,  // 指针大小
            CCASType::Char => 1,
            CCASType::Void => 0,
            CCASType::Array(t) => t.size(),
            CCASType::Pointer(_) => 8,
            CCASType::List => 8,  // 列表是指针
            CCASType::Custom(_) => 8,
        }
    }

    /**
     * 检查类型是否可以隐式转换为目标类型
     */
    pub fn can_implicit_cast_to(&self, target: &CCASType) -> bool {
        match (self, target) {
            (CCASType::Int, CCASType::Long) => true,
            (CCASType::Int, CCASType::Float) => true,
            (CCASType::Int, CCASType::Double) => true,
            (CCASType::Long, CCASType::Float) => true,
            (CCASType::Long, CCASType::Double) => true,
            (CCASType::Float, CCASType::Double) => true,
            (CCASType::List, CCASType::List) => true,
            _ => self == target,
        }
    }
}

/**
 * 类型上下文
 */
pub struct TypeContext {
    types: std::collections::HashMap<String, CCASType>,
}

impl TypeContext {
    pub fn new() -> Self {
        let mut types = std::collections::HashMap::new();
        types.insert("整数".to_string(), CCASType::Int);
        types.insert("长整数".to_string(), CCASType::Long);
        types.insert("浮点数".to_string(), CCASType::Float);
        types.insert("双精度".to_string(), CCASType::Double);
        types.insert("布尔".to_string(), CCASType::Bool);
        types.insert("文本".to_string(), CCASType::String);
        types.insert("字符".to_string(), CCASType::Char);
        types.insert("无返回".to_string(), CCASType::Void);
        types.insert("列表".to_string(), CCASType::List);
        Self { types }
    }

    pub fn get_type(&self, name: &str) -> Option<CCASType> {
        self.types.get(name).cloned()
    }
}

impl Default for TypeContext {
    fn default() -> Self {
        Self::new()
    }
}
