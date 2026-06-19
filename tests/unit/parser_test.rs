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
        // 测试简单表达式
        let source = "x + y".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_expression().unwrap();
        assert!(ast.is_ok());
    }

    #[test]
    fn test_number_expression() {
        // 测试数字表达式
        let source = "42".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_expression().unwrap();
        assert!(ast.is_ok());
    }

    #[test]
    fn test_string_expression() {
        // 测试字符串表达式
        let source = "\"Hello\"".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_expression().unwrap();
        assert!(ast.is_ok());
    }

    // ============ 变量定义测试 ============

    #[test]
    fn test_let_statement() {
        // 测试变量定义语句
        let source = "定义 x: 整数".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_statement().unwrap();
        assert!(ast.is_ok());
    }

    #[test]
    fn test_let_with_initializer() {
        // 测试带初始值的变量定义
        let source = "定义 整数 x = 42".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_statement().unwrap();
        assert!(ast.is_ok());
    }

    #[test]
    fn test_mutable_variable() {
        // 测试可变变量定义
        let source = "定义 可变 x: 整数 = 0".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_statement().unwrap();
        assert!(ast.is_ok());
    }

    // ============ if 语句测试 ============

    #[test]
    fn test_simple_if_statement() {
        // 测试简单 if 语句
        let source = "若 x > 0 则 { 返回 1 }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_statement().unwrap();
        assert!(ast.is_ok());
    }

    #[test]
    fn test_if_else_statement() {
        // 测试 if-else 语句
        let source = "若 x > 0 则 { 返回 1 } 否则 { 返回 0 }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_statement().unwrap();
        assert!(ast.is_ok());
    }

    #[test]
    fn test_if_else_if_statement() {
        // 测试 if-else if-else 语句（否则若）
        let source = "若 x > 90 则 { 返回 \"优秀\" } 否则若 x > 60 则 { 返回 \"及格\" } 否则 { 返回 \"不及格\" }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_statement().unwrap();
        assert!(ast.is_ok());
    }

    #[test]
    fn test_nested_if_else() {
        // 测试嵌套 if-else 语句
        let source = "若 x > 0 则 { 若 y > 0 则 { 返回 1 } } 否则 { 返回 0 }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_statement().unwrap();
        assert!(ast.is_ok());
    }

    // ============ while 循环测试 ============

    #[test]
    fn test_simple_while_loop() {
        // 测试简单 while 循环
        let source = "当 x < 10 则 { x = x + 1 }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_statement().unwrap();
        assert!(ast.is_ok());
    }

    #[test]
    fn test_while_with_break() {
        // 测试带 break 的 while 循环
        let source = "当 真 则 { 若 x > 10 则 { 退出 } x = x + 1 }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_statement().unwrap();
        assert!(ast.is_ok());
    }

    #[test]
    fn test_while_with_continue() {
        // 测试带 continue 的 while 循环
        let source = "当 i < 10 则 { 若 i % 2 == 0 则 { 跳过 } x = x + i }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_statement().unwrap();
        assert!(ast.is_ok());
    }

    // ============ for 循环测试 ============

    #[test]
    fn test_for_loop() {
        // 测试 for 循环
        let source = "循环 i 从 0 到 10 { x = x + i }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_statement().unwrap();
        assert!(ast.is_ok());
    }

    // ============ 函数定义测试 ============

    #[test]
    fn test_simple_function() {
        // 测试简单函数定义
        let source = "函数 主(): 整数 { 返回 0 }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_statement().unwrap();
        assert!(ast.is_ok());
    }

    #[test]
    fn test_function_with_params() {
        // 测试带参数的函数定义
        let source = "函数 加(x: 整数, y: 整数): 整数 { 返回 x + y }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_statement().unwrap();
        assert!(ast.is_ok());
    }

    #[test]
    fn test_void_function() {
        // 测试无返回函数
        let source = "函数 打印消息(msg: 文本): 无返回 { }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_statement().unwrap();
        assert!(ast.is_ok());
    }

    // ============ 列表操作测试 ============

    #[test]
    fn test_list_creation() {
        // 测试列表创建
        let source = "定义 我的列表: 列表 = rt_list_new()".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_statement().unwrap();
        assert!(ast.is_ok());
    }

    #[test]
    fn test_list_append() {
        // 测试列表追加
        let source = "rt_list_append(我的列表, \"元素\")".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_statement().unwrap();
        assert!(ast.is_ok());
    }

    #[test]
    fn test_list_index_access() {
        // 测试列表索引访问
        let source = "列表[0]".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_expression().unwrap();
        assert!(ast.is_ok());
    }

    #[test]
    fn test_list_index_assignment() {
        // 测试列表索引赋值
        let source = "列表[0] = 新值".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_statement().unwrap();
        assert!(ast.is_ok());
    }

    // ============ 运算符测试 ============

    #[test]
    fn test_arithmetic_operators() {
        // 测试算术运算符
        let source = "a + b * c - d / e % f".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_expression().unwrap();
        assert!(ast.is_ok());
    }

    #[test]
    fn test_comparison_operators() {
        // 测试比较运算符
        let source = "x == y && a != b || m < n && p > q".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_expression().unwrap();
        assert!(ast.is_ok());
    }

    #[test]
    fn test_logical_operators() {
        // 测试逻辑运算符
        let source = "真 && 假 || !假".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_expression().unwrap();
        assert!(ast.is_ok());
    }

    #[test]
    fn test_bitwise_operators() {
        // 测试位运算符
        let source = "a & b | c ^ d << e >> f".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_expression().unwrap();
        assert!(ast.is_ok());
    }

    #[test]
    fn test_compound_assignment() {
        // 测试复合赋值运算符
        let source = "x += 1; y -= 2; z *= 3".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_statement().unwrap();
        assert!(ast.is_ok());
    }

    // ============ 块语句测试 ============

    #[test]
    fn test_block_statement() {
        // 测试块语句
        let source = "{ 定义 x = 1; 定义 y = 2; 返回 x + y }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_block().unwrap();
        assert!(ast.is_ok());
    }

    // ============ 返回语句测试 ============

    #[test]
    fn test_return_statement() {
        // 测试返回语句
        let source = "返回 42".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_statement().unwrap();
        assert!(ast.is_ok());
    }

    #[test]
    fn test_return_expression() {
        // 测试返回表达式
        let source = "返回 x + y * 2".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        let ast = parser.parse_statement().unwrap();
        assert!(ast.is_ok());
    }
}
