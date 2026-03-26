/**
 * @file token.rs
 * @brief CCAS 词法分析器 - Token 定义模块
 * @description 定义词法分析器产生的 Token 类型，严格区分关键字和 Token 类型
 * 
 * 设计原则 (CCAS v2.0):
 * 1. 关键字应是原子动词或连接词，通过语义空格组合成语句
 * 2. 整数字面量等不是关键字，是词法分析器识别的 Token 类型
 * 3. 多字短语应由多个关键字组合实现 (如 循环 + 从 + 到)
 */

use std::fmt;
use std::sync::LazyLock;

/**
 * 保留关键字 (Reserved Keywords)
 * 这些是源代码中必须使用的保留词
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    // ========== 控制流 (Control Flow) ==========
    若,           // if
    如果,         // if (XY语法变体)
    则,           // then (CCAS 特色)
    否则,         // else
    否则若,       // else if / elif
    或,           // || (逻辑或关键字)
    且,           // && (逻辑与关键字)
    当,           // while
    直到,         // until (do-while 后置条件)
    循环,         // loop / for 通用入口
    从,           // from (配合循环: i 从 0)
    到,           // to (配合循环: 到 10)
    取自,         // in (配合循环: 项 取自 集合)
    遍历,         // for (列表推导式: for x in list)
    在,           // in (列表推导式: for x in list)
    跳过,         // continue (比"继续"更准确)
    退出,         // break (比"中断"更准确)
    跳出,         // break (XY源码使用的关键字)
    匹配,         // match (模式匹配)
    情况,         // case
    默认,         // default

    // ========== 函数与协程 (Functions & Async) ==========
    函数,         // fn
    过程,         // proc (无返回值函数)
    返回,         // return
    异步,         // async
    等待,         // await
    启动,         // spawn (启动线程/协程)

    // ========== 数据类型 (Types) - 对应规范 4.1 ==========
    整数,         // int64
    长整数,       // int64
    浮点数,       // float64
    双精度,       // float64
    布尔,         // bool
    文本,         // String / char*
    字符,         // char
    无返回,       // void
    指针,         // pointer / void*
    列表,         // list (动态数组)
    或许,         // Option / T?

    // ========== 内存安全与所有权 (Memory & Safety) ==========
    定义,         // let (声明不可变变量)
    可变,         // mut (修饰符: 定义 可变 x = 1)
    借用,         // borrow / ref
    可变借用,     // mut borrow
    拥有,         // move / own (显式转移所有权)
    手动,         // manual (关闭 GC，进入手动内存管理)
    原生,         // unsafe (绕过安全检查)

    // ========== 可见性与修饰符 (Visibility & Modifiers) ==========
    公开,         // public (比"公共"更简洁)
    私有,         // private
    常量,         // const
    静态,         // static
    外部,         // extern (FFI 调用)

    // ========== 模块系统 (Modules) ==========
    模块,         // module
    引入,         // import / use
    导出,         // export / pub

    // ========== 错误处理 (Error Handling) ==========
    尝试,         // try
    捕获,         // catch
    抛出,         // throw / raise
    最终,         // finally

    // ========== 复合数据类型 (Composite Types) ==========
    结构体,       // struct
    枚举,         // enum
    联合,         // union
    类型别名,     // type (别名)
}

/**
 * Token 类型枚举
 * 包含所有词法单元类型，包括关键字、字面量、符号等
 */
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenType {
    // 关键字包装 (内部使用)
    Keyword(Keyword),

    // ========== 字面量 (Literals) - 不是关键字，是 Token 类型 ==========
    整数字面量,    // 整数常量: 123, 0xFF
    浮点字面量,    // 浮点常量: 3.14
    文本字面量,    // 字符串常量: "你好"
    字符字面量,    // 字符常量: 'A'
    布尔字面量,    // 布尔常量: 真/假

    // ========== 标识符 (Identifier) ==========
    标识符,       // 变量名、函数名、类型名 (支持中文: 用户年龄)

    // ========== 运算符 (Operators) - 代码生成使用 ASCII 符号 ==========
    // 算术运算符
    加,           // + 
    减,           // - 
    乘,           // * 
    除,           // / 
    取余,         // % 

    // 比较运算符
    等于,         // == 
    不等于,       // != 
    大于,         // > 
    小于,         // < 
    大于等于,     // >= 
    小于等于,     // <= 

    // 逻辑运算符
    与,           // && 
    或,           // || 
    非,           // ! 

    // 位运算符
    位与,         // & 
    位或,         // | 
    位异或,       // ^ 
    位非,         // ~ 
    左移,         // << 
    右移,         // >> 

    // 赋值运算符
    赋值,         // =

    // ========== 分隔符 (Delimiters) ==========
    左圆括号,      // ( 
    右圆括号,      // ) 
    左花括号,      // { 
    右花括号,      // } 
    左方括号,      // [  
    右方括号,      // ] 
    左尖括号,      // <  (用于泛型参数，如 或许<整数>)
    右尖括号,      // >  (用于泛型参数)
    左移等于,      // <<= 
    右移等于,      // >>=
    逗号,          // , 
    句号,          // . 
    分号,          // ; 
    冒号,          // : 

    // ========== 特殊 Token ==========
    文件结束,      // EOF
    未知,         // 未知类型
}

/**
 * Token 位置信息
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

impl Span {
    pub fn new(start_line: usize, start_column: usize, end_line: usize, end_column: usize) -> Self {
        Self {
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    pub fn merge(self, other: Span) -> Span {
        Span {
            start_line: self.start_line.min(other.start_line),
            start_column: if self.start_line <= other.start_line {
                self.start_column
            } else {
                other.start_column
            },
            end_line: self.end_line.max(other.end_line),
            end_column: if self.end_line >= other.end_line {
                self.end_column
            } else {
                other.end_column
            },
        }
    }

    pub fn dummy() -> Self {
        Self {
            start_line: 0,
            start_column: 0,
            end_line: 0,
            end_column: 0,
        }
    }
}

/**
 * Token 结构体
 */
#[derive(Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub literal: String,
    pub span: Span,
}

impl Token {
    pub fn new(token_type: TokenType, literal: String, span: Span) -> Self {
        Self {
            token_type,
            literal,
            span,
        }
    }

    pub fn keyword(keyword: Keyword, span: Span) -> Self {
        Self {
            token_type: TokenType::Keyword(keyword),
            literal: String::new(),
            span,
        }
    }

    pub fn is_keyword(&self, keyword: &Keyword) -> bool {
        if let TokenType::Keyword(k) = &self.token_type {
            k == keyword
        } else {
            false
        }
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Token({:?}, {:?}, {:?})",
            self.token_type, self.literal, self.span
        )
    }
}

/**
 * 关键字映射表 (String -> Keyword)
 */
static KEYWORD_MAP: LazyLock<std::collections::HashMap<&'static str, Keyword>> = LazyLock::new(|| {
    let mut map = std::collections::HashMap::new();
    
    // ========== 控制流 ==========
    map.insert("若", Keyword::若);
    map.insert("则", Keyword::则);
    map.insert("否则", Keyword::否则);
    map.insert("否则若", Keyword::否则若);
    map.insert("当", Keyword::当);
    map.insert("直到", Keyword::直到);
    map.insert("循环", Keyword::循环);
    map.insert("从", Keyword::从);
    map.insert("到", Keyword::到);
    map.insert("取自", Keyword::取自);
    map.insert("跳过", Keyword::跳过);
    map.insert("退出", Keyword::退出);
    map.insert("跳出", Keyword::跳出);
    map.insert("遍历", Keyword::遍历);
    map.insert("在", Keyword::在);
    map.insert("匹配", Keyword::匹配);
    map.insert("情况", Keyword::情况);
    map.insert("默认", Keyword::默认);

    // ========== 函数与协程 ==========
    map.insert("函数", Keyword::函数);
    map.insert("过程", Keyword::过程);
    map.insert("返回", Keyword::返回);
    map.insert("异步", Keyword::异步);
    map.insert("等待", Keyword::等待);
    map.insert("启动", Keyword::启动);

    // ========== 数据类型 ==========
    map.insert("整数", Keyword::整数);
    map.insert("长整数", Keyword::长整数);
    map.insert("浮点数", Keyword::浮点数);
    map.insert("双精度", Keyword::双精度);
    map.insert("布尔", Keyword::布尔);
    map.insert("文本", Keyword::文本);
    map.insert("字符", Keyword::字符);
    map.insert("无返回", Keyword::无返回);
    map.insert("指针", Keyword::指针);
    map.insert("列表", Keyword::列表);
    map.insert("或许", Keyword::或许);
    map.insert("如果", Keyword::如果);
    map.insert("或", Keyword::或);
    map.insert("且", Keyword::且);

    // ========== 内存安全与所有权 ==========
    map.insert("定义", Keyword::定义);
    map.insert("可变", Keyword::可变);
    map.insert("借用", Keyword::借用);
    map.insert("可变借用", Keyword::可变借用);
    map.insert("拥有", Keyword::拥有);
    map.insert("手动", Keyword::手动);
    map.insert("原生", Keyword::原生);

    // ========== 可见性与修饰符 ==========
    map.insert("公开", Keyword::公开);
    map.insert("私有", Keyword::私有);
    map.insert("常量", Keyword::常量);
    map.insert("静态", Keyword::静态);
    map.insert("外部", Keyword::外部);

    // ========== 模块系统 ==========
    map.insert("模块", Keyword::模块);
    map.insert("引入", Keyword::引入);
    map.insert("导出", Keyword::导出);

    // ========== 错误处理 ==========
    map.insert("尝试", Keyword::尝试);
    map.insert("捕获", Keyword::捕获);
    map.insert("抛出", Keyword::抛出);
    map.insert("最终", Keyword::最终);

    // ========== 复合数据类型 ==========
    map.insert("结构体", Keyword::结构体);
    map.insert("枚举", Keyword::枚举);
    map.insert("联合", Keyword::联合);
    map.insert("类型", Keyword::类型别名);

    // ========== 布尔字面量 (特殊: 是字面量但用中文表示) ==========
    // 真/假 在词法阶段作为标识符处理，解析阶段识别为布尔字面量

    map
});

static BOOLEAN_LITERALS: LazyLock<std::collections::HashSet<&'static str>> = LazyLock::new(|| {
    let mut set = std::collections::HashSet::new();
    set.insert("真");
    set.insert("假");
    set
});

/**
 * 根据字面量查找关键字
 */
pub fn lookup_keyword(literal: &str) -> TokenType {
    KEYWORD_MAP
        .get(literal)
        .map(|&k| TokenType::Keyword(k))
        .unwrap_or(TokenType::标识符)
}

/**
 * 检查是否为关键字
 */
pub fn is_keyword(literal: &str) -> bool {
    KEYWORD_MAP.contains_key(literal)
}

/**
 * 检查是否为布尔字面量
 */
pub fn is_boolean_literal(literal: &str) -> bool {
    BOOLEAN_LITERALS.contains(literal)
}

/**
 * 获取关键字数量 (用于调试)
 */
pub fn keyword_count() -> usize {
    KEYWORD_MAP.len()
}
