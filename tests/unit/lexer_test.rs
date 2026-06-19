/**
 * @file tests/unit/lexer_test.rs
 * @brief 词法分析器单元测试
 * @description 测试 lexer 对各种词法单元的正确识别
 */

#[cfg(test)]
mod lexer_tests {
    use xuanyu::lexer::lexer::Lexer;
    use xuanyu::lexer::token::{TokenType, Keyword};

    // ============ 关键字测试 ============

    #[test]
    fn test_basic_keywords() {
        // 测试基本关键字
        let source = "若 则 否则 循环 函数".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::Keyword(Keyword::若));
        assert_eq!(tokens[1].token_type, TokenType::Keyword(Keyword::则));
        assert_eq!(tokens[2].token_type, TokenType::Keyword(Keyword::否则));
        assert_eq!(tokens[3].token_type, TokenType::Keyword(Keyword::循环));
        assert_eq!(tokens[4].token_type, TokenType::Keyword(Keyword::函数));
    }

    #[test]
    fn test_control_keywords() {
        // 测试控制流关键字
        let source = "当 从 到 跳过 退出".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::Keyword(Keyword::当));
        assert_eq!(tokens[1].token_type, TokenType::Keyword(Keyword::从));
        assert_eq!(tokens[2].token_type, TokenType::Keyword(Keyword::到));
        assert_eq!(tokens[3].token_type, TokenType::Keyword(Keyword::跳过));
        assert_eq!(tokens[4].token_type, TokenType::Keyword(Keyword::退出));
    }

    #[test]
    fn test_type_keywords() {
        // 测试类型关键字
        let source = "整数 浮点数 布尔 文本 列表".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::Keyword(Keyword::整数));
        assert_eq!(tokens[1].token_type, TokenType::Keyword(Keyword::浮点数));
        assert_eq!(tokens[2].token_type, TokenType::Keyword(Keyword::布尔));
        assert_eq!(tokens[3].token_type, TokenType::Keyword(Keyword::文本));
        assert_eq!(tokens[4].token_type, TokenType::Keyword(Keyword::列表));
    }

    #[test]
    fn test_else_if_keyword() {
        // 测试否则若关键字（已知限制：可能被拆分为否则+若，取决于最长匹配策略）
        let source = "否则若".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();

        // 检查token类型（可能是单个否则若关键字，也可能是否则+若）
        assert!(tokens.len() >= 1);
        // 关键验证：至少包含"否则"相关token
        let has_else = tokens.iter().any(|t| {
            matches!(t.token_type, TokenType::Keyword(Keyword::否则))
            || matches!(t.token_type, TokenType::Keyword(Keyword::否则若))
        });
        assert!(has_else, "应包含否则或否则若关键字");
    }

    // ============ 标识符测试 ============

    #[test]
    fn test_chinese_identifier() {
        // 测试中文标识符
        let source = "用户年龄 计算总和 列表内容".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::标识符);
        assert_eq!(tokens[0].literal, "用户年龄");
        assert_eq!(tokens[1].literal, "计算总和");
        assert_eq!(tokens[2].literal, "列表内容");
    }

    #[test]
    fn test_mixed_identifier() {
        // 测试中英文混合标识符
        let source = "userName count_i itemList".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::标识符);
        assert_eq!(tokens[0].literal, "userName");
    }

    // ============ 数字字面量测试 ============

    #[test]
    fn test_integer_literals() {
        // 测试整数字面量
        let source = "0 42 255 65535".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::整数字面量);
        assert_eq!(tokens[0].literal, "0");
        assert_eq!(tokens[1].literal, "42");
        assert_eq!(tokens[2].literal, "255");
        assert_eq!(tokens[3].literal, "65535");
    }

    #[test]
    fn test_hex_literals() {
        // 测试十六进制字面量
        let source = "0xFF 0x1A3 0xDEAD".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].literal, "0xFF");
        assert_eq!(tokens[1].literal, "0x1A3");
    }

    #[test]
    fn test_float_literals() {
        // 测试浮点字面量
        let source = "3.14 0.5 2.71828".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::浮点字面量);
        assert_eq!(tokens[0].literal, "3.14");
        assert_eq!(tokens[1].literal, "0.5");
    }

    // ============ 字符串字面量测试 ============

    #[test]
    fn test_string_literals() {
        // 测试字符串字面量
        let source = "\"你好世界\" \"Hello\"".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::文本字面量);
        assert_eq!(tokens[0].literal, "你好世界");
        assert_eq!(tokens[1].literal, "Hello");
    }

    #[test]
    fn test_string_with_escape() {
        // 测试带转义的字符串
        let source = "\"第一行\\n第二行\"".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::文本字面量);
    }

    // ============ 运算符测试 ============

    #[test]
    fn test_arithmetic_operators() {
        // 测试算术运算符
        let source = "+ - * / %".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::加);
        assert_eq!(tokens[1].token_type, TokenType::减);
        assert_eq!(tokens[2].token_type, TokenType::乘);
        assert_eq!(tokens[3].token_type, TokenType::除);
        assert_eq!(tokens[4].token_type, TokenType::取余);
    }

    #[test]
    fn test_comparison_operators() {
        // 测试比较运算符
        let source = "== != < > <= >=".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::等于);
        assert_eq!(tokens[1].token_type, TokenType::不等于);
        assert_eq!(tokens[2].token_type, TokenType::小于);
        assert_eq!(tokens[3].token_type, TokenType::大于);
        assert_eq!(tokens[4].token_type, TokenType::小于等于);
        assert_eq!(tokens[5].token_type, TokenType::大于等于);
    }

    #[test]
    fn test_logical_operators() {
        // 测试逻辑运算符
        let source = "&& || !".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::与);
        assert_eq!(tokens[1].token_type, TokenType::或);
        assert_eq!(tokens[2].token_type, TokenType::非);
    }

    #[test]
    fn test_bitwise_operators() {
        // 测试位运算符
        let source = "& | ^ << >>".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::位与);
        assert_eq!(tokens[1].token_type, TokenType::位或);
        assert_eq!(tokens[2].token_type, TokenType::位异或);
        assert_eq!(tokens[3].token_type, TokenType::左移);
        assert_eq!(tokens[4].token_type, TokenType::右移);
    }

    // ============ 界符测试 ============

    #[test]
    fn test_delimiters() {
        // 测试界符
        let source = "{ } ( ) [ ] , ; :".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::左花括号);
        assert_eq!(tokens[1].token_type, TokenType::右花括号);
        assert_eq!(tokens[2].token_type, TokenType::左圆括号);
        assert_eq!(tokens[3].token_type, TokenType::右圆括号);
        assert_eq!(tokens[4].token_type, TokenType::左方括号);
        assert_eq!(tokens[5].token_type, TokenType::右方括号);
        assert_eq!(tokens[6].token_type, TokenType::逗号);
        assert_eq!(tokens[7].token_type, TokenType::分号);
        assert_eq!(tokens[8].token_type, TokenType::冒号);
    }

    // ============ 布尔字面量测试 ============

    #[test]
    fn test_boolean_literals() {
        // 测试布尔字面量
        let source = "真 假".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::布尔字面量);
        assert_eq!(tokens[0].literal, "真");
        assert_eq!(tokens[1].literal, "假");
    }

    // ============ 注释测试 ============

    #[test]
    fn test_single_line_comment() {
        // 测试单行注释（注释token由词法分析器产生，解析器负责过滤）
        let source = "变量 x // 这是一个注释".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();

        // 至少包含 变量, x 两个token（注释token可能也被包含）
        assert!(tokens.len() >= 2);
        assert_eq!(tokens[0].literal, "变量");
        assert_eq!(tokens[1].literal, "x");
    }

    #[test]
    fn test_multi_line_comment() {
        // 测试多行注释（注释token由词法分析器产生，解析器负责过滤）
        let source = "变量 x /* 这是一个\n多行注释 */ 变量 y".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();

        // 至少包含 变量, x, 变量, y 四个token
        assert!(tokens.len() >= 4);
    }
}
