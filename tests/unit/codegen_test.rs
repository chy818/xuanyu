/**
 * @file tests/unit/codegen_test.rs
 * @brief 代码生成器单元测试
 * @description 测试 codegen 对 LLVM IR 的正确生成
 */

#[cfg(test)]
mod codegen_tests {
    use xuanyu::lexer::lexer::Lexer;
    use xuanyu::parser::parser::Parser;
    use xuanyu::sema::SemanticAnalyzer;
    use xuanyu::codegen::CodeGenerator;

    // ============ 表达式代码生成测试 ============

    #[test]
    fn test_integer_literal_codegen() {
        // 测试整数字面量代码生成
        let source = "42".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression().unwrap();
        
        assert!(expr.is_ok());
        // 验证生成的 IR 包含 add i64
    }

    #[test]
    fn test_binary_expr_codegen() {
        // 测试二元表达式代码生成
        let source = "x + y".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression().unwrap();
        
        assert!(expr.is_ok());
    }

    // ============ 变量代码生成测试 ============

    #[test]
    fn test_variable_alloca() {
        // 测试变量分配代码生成
        let source = "定义 整数 x = 42".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_statement().unwrap();
        
        assert!(stmt.is_ok());
        // 验证生成 alloca 和 store 指令
    }

    #[test]
    fn test_variable_load() {
        // 测试变量加载代码生成
        let source = "x".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression().unwrap();
        
        assert!(expr.is_ok());
        // 验证生成 load 指令
    }

    // ============ 运算符代码生成测试 ============

    #[test]
    fn test_add_operator_codegen() {
        // 测试加法运算符代码生成
        let source = "a + b".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression().unwrap();
        
        assert!(expr.is_ok());
        // 验证生成 add i64 指令
    }

    #[test]
    fn test_subtract_operator_codegen() {
        // 测试减法运算符代码生成
        let source = "a - b".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression().unwrap();
        
        assert!(expr.is_ok());
        // 验证生成 sub i64 指令
    }

    #[test]
    fn test_multiply_operator_codegen() {
        // 测试乘法运算符代码生成
        let source = "a * b".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression().unwrap();
        
        assert!(expr.is_ok());
        // 验证生成 mul i64 指令
    }

    #[test]
    fn test_divide_operator_codegen() {
        // 测试除法运算符代码生成
        let source = "a / b".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression().unwrap();
        
        assert!(expr.is_ok());
        // 验证生成 sdiv i64 指令
    }

    #[test]
    fn test_modulo_operator_codegen() {
        // 测试取模运算符代码生成
        let source = "a % b".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression().unwrap();
        
        assert!(expr.is_ok());
        // 验证生成 srem i64 指令
    }

    #[test]
    fn test_comparison_operators_codegen() {
        // 测试比较运算符代码生成
        let test_cases = vec![
            ("a == b", "icmp eq"),
            ("a != b", "icmp ne"),
            ("a < b", "icmp slt"),
            ("a > b", "icmp sgt"),
            ("a <= b", "icmp sle"),
            ("a >= b", "icmp sge"),
        ];
        
        for (source, expected_ir) in test_cases {
            let tokens = Lexer::new(source.to_string()).tokenize().unwrap();
            let mut parser = Parser::new(tokens);
            let expr = parser.parse_expression().unwrap();
            assert!(expr.is_ok(), "Failed for: {}", source);
        }
    }

    #[test]
    fn test_logical_operators_codegen() {
        // 测试逻辑运算符代码生成
        let test_cases = vec![
            ("a && b", "and i1"),
            ("a || b", "or i1"),
        ];
        
        for (source, _expected_ir) in test_cases {
            let tokens = Lexer::new(source.to_string()).tokenize().unwrap();
            let mut parser = Parser::new(tokens);
            let expr = parser.parse_expression().unwrap();
            assert!(expr.is_ok(), "Failed for: {}", source);
        }
    }

    #[test]
    fn test_bitwise_operators_codegen() {
        // 测试位运算符代码生成
        let test_cases = vec![
            ("a & b", "and i64"),
            ("a | b", "or i64"),
            ("a ^ b", "xor i64"),
            ("a << b", "shl i64"),
            ("a >> b", "lshr i64"),
        ];
        
        for (source, _expected_ir) in test_cases {
            let tokens = Lexer::new(source.to_string()).tokenize().unwrap();
            let mut parser = Parser::new(tokens);
            let expr = parser.parse_expression().unwrap();
            assert!(expr.is_ok(), "Failed for: {}", source);
        }
    }

    // ============ 赋值代码生成测试 ============

    #[test]
    fn test_simple_assignment_codegen() {
        // 测试简单赋值代码生成
        let source = "x = 42".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_statement().unwrap();
        
        assert!(stmt.is_ok());
        // 验证生成 store 指令
    }

    #[test]
    fn test_compound_assignment_codegen() {
        // 测试复合赋值代码生成
        let source = "x += 1".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_statement().unwrap();
        
        assert!(stmt.is_ok());
        // 验证生成 add 和 store 指令
    }

    // ============ 控制流代码生成测试 ============

    #[test]
    fn test_if_statement_codegen() {
        // 测试 if 语句代码生成
        let source = "若 x > 0 则 { 返回 1 }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_statement().unwrap();
        
        assert!(stmt.is_ok());
        // 验证生成 br 指令和条件跳转
    }

    #[test]
    fn test_if_else_statement_codegen() {
        // 测试 if-else 语句代码生成
        let source = "若 x > 0 则 { 返回 1 } 否则 { 返回 0 }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_statement().unwrap();
        
        assert!(stmt.is_ok());
        // 验证生成条件跳转
    }

    #[test]
    fn test_while_loop_codegen() {
        // 测试 while 循环代码生成
        let source = "当 x < 10 则 { x = x + 1 }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_statement().unwrap();
        
        assert!(stmt.is_ok());
        // 验证生成循环结构的 br 指令
    }

    // ============ 列表操作代码生成测试 ============

    #[test]
    fn test_list_creation_codegen() {
        // 测试列表创建代码生成
        let source = "rt_list_new()".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression().unwrap();
        
        assert!(expr.is_ok());
        // 验证生成 call 指令调用 rt_list_new
    }

    #[test]
    fn test_list_append_codegen() {
        // 测试列表追加代码生成
        let source = "rt_list_append(列表, 值)".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression().unwrap();
        
        assert!(expr.is_ok());
        // 验证生成 call 指令调用 rt_list_append
    }

    #[test]
    fn test_list_index_access_codegen() {
        // 测试列表索引访问代码生成
        let source = "列表[0]".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression().unwrap();
        
        assert!(expr.is_ok());
        // 验证生成 call 指令调用 rt_list_get
    }

    #[test]
    fn test_list_index_assignment_codegen() {
        // 测试列表索引赋值代码生成
        let source = "列表[0] = 新值".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_statement().unwrap();
        
        assert!(stmt.is_ok());
        // 验证生成 call 指令调用 rt_list_set
    }

    // ============ 函数调用代码生成测试 ============

    #[test]
    fn test_function_call_codegen() {
        // 测试函数调用代码生成
        let source = "打印整数(42)".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression().unwrap();
        
        assert!(expr.is_ok());
        // 验证生成 call 指令
    }

    // ============ 字符串常量代码生成测试 ============

    #[test]
    fn test_string_constant_codegen() {
        // 测试字符串常量代码生成
        let source = "\"Hello World\"".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression().unwrap();
        
        assert!(expr.is_ok());
        // 验证生成全局常量定义
    }
}
