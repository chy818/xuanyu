/**
 * @file tests/unit/parser_test.rs
 * @brief 语法分析器单元测试
 * @description 测试 parser 对各种语法结构的正确解析
 */

#[cfg(test)]
mod parser_tests {
    use xuanyu::lexer::lexer::Lexer;
    use xuanyu::parser::parser::Parser;

    // ============ 表达式解析测试 ============

    #[test]
    fn test_simple_expression() {
        let source = "x + y".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_expression().is_ok());
    }

    #[test]
    fn test_number_expression() {
        let source = "42".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_expression().is_ok());
    }

    #[test]
    fn test_string_expression() {
        let source = "\"Hello\"".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_expression().is_ok());
    }

    // ============ 变量定义测试 ============

    #[test]
    fn test_let_statement() {
        let source = "定义 x: 整数".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    #[test]
    fn test_let_with_initializer() {
        let source = "定义 x: 整数 = 42".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    #[test]
    fn test_mutable_variable() {
        let source = "定义 可变 x: 整数 = 0".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    // ============ if 语句测试 ============

    #[test]
    fn test_simple_if_statement() {
        let source = "若 x > 0 则 { 返回 1 }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    #[test]
    fn test_if_else_statement() {
        let source = "若 x > 0 则 { 返回 1 } 否则 { 返回 0 }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    #[test]
    fn test_if_else_if_statement() {
        let source = "若 x > 90 则 { 返回 \"优秀\" } 否则若 x > 60 则 { 返回 \"及格\" } 否则 { 返回 \"不及格\" }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    #[test]
    fn test_nested_if_else() {
        let source = "若 x > 0 则 { 若 y > 0 则 { 返回 1 } } 否则 { 返回 0 }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    // ============ while 循环测试 ============

    #[test]
    fn test_simple_while_loop() {
        let source = "当 x < 10 则 { x = x + 1 }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    #[test]
    fn test_while_with_break() {
        let source = "当 真 则 { 若 x > 10 则 { 退出 } x = x + 1 }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    #[test]
    fn test_while_with_continue() {
        let source = "当 i < 10 则 { 若 i % 2 == 0 则 { 跳过 } x = x + i }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    // ============ for 循环测试 ============

    #[test]
    fn test_for_loop() {
        let source = "循环 i 从 0 到 10 { x = x + i }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    // ============ 函数定义测试 ============

    #[test]
    fn test_simple_function() {
        let source = "函数 主(): 整数 { 返回 0 }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    #[test]
    fn test_function_with_params() {
        let source = "函数 加(x: 整数, y: 整数): 整数 { 返回 x + y }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    #[test]
    fn test_void_function() {
        let source = "函数 打印消息(msg: 文本): 无返回 { }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    // ============ 列表操作测试 ============

    #[test]
    fn test_list_creation() {
        let source = "定义 我的列表: 列表 = rt_list_new()".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    #[test]
    fn test_list_append() {
        let source = "rt_list_append(我的列表, \"元素\")".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    #[test]
    fn test_list_index_access() {
        let source = "列表[0]".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_expression().is_ok());
    }

    #[test]
    fn test_list_index_assignment() {
        let source = "列表[0] = 新值".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    // ============ 运算符测试 ============

    #[test]
    fn test_arithmetic_operators() {
        let source = "a + b * c - d / e % f".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_expression().is_ok());
    }

    #[test]
    fn test_comparison_operators() {
        let source = "x == y && a != b || m < n && p > q".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_expression().is_ok());
    }

    #[test]
    fn test_logical_operators() {
        let source = "真 && 假 || !假".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_expression().is_ok());
    }

    #[test]
    fn test_bitwise_operators() {
        let source = "a & b | c ^ d << e >> f".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_expression().is_ok());
    }

    #[test]
    fn test_compound_assignment() {
        let source = "x += 1".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    // ============ 块语句测试 ============

    #[test]
    fn test_block_statement() {
        let source = "{ 定义 x = 1; 定义 y = 2; 返回 x + y }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    // ============ 返回语句测试 ============

    #[test]
    fn test_return_statement() {
        let source = "返回 42".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    #[test]
    fn test_return_expression() {
        let source = "返回 x + y * 2".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }
}