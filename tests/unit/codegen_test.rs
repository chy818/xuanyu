/**
 * @file tests/unit/codegen_test.rs
 * @brief 代码生成器单元测试
 * @description 测试 codegen 对 LLVM IR 的正确生成
 */

#[cfg(test)]
mod codegen_tests {
    use xuanyu::generate_ir;
    use xuanyu::lexer::lexer::Lexer;
    use xuanyu::parser::parser::Parser;

    // ============ 表达式代码生成测试 ============

    #[test]
    fn test_integer_literal_codegen() {
        let source = "42".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_expression().is_ok());
    }

    #[test]
    fn test_binary_expr_codegen() {
        let source = "x + y".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_expression().is_ok());
    }

    // ============ 变量代码生成测试 ============

    #[test]
    fn test_variable_alloca() {
        let source = "定义 x: 整数 = 42".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    #[test]
    fn test_variable_load() {
        let source = "x".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_expression().is_ok());
    }

    // ============ 运算符代码生成测试 ============

    #[test]
    fn test_add_operator_codegen() {
        let source = "a + b".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_expression().is_ok());
    }

    #[test]
    fn test_subtract_operator_codegen() {
        let source = "a - b".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_expression().is_ok());
    }

    #[test]
    fn test_multiply_operator_codegen() {
        let source = "a * b".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_expression().is_ok());
    }

    #[test]
    fn test_divide_operator_codegen() {
        let source = "a / b".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_expression().is_ok());
    }

    #[test]
    fn test_modulo_operator_codegen() {
        let source = "a % b".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_expression().is_ok());
    }

    #[test]
    fn test_comparison_operators_codegen() {
        let test_cases = vec![
            "a == b",
            "a != b",
            "a < b",
            "a > b",
            "a <= b",
            "a >= b",
        ];
        
        for source in test_cases {
            let tokens = Lexer::new(source.to_string()).tokenize().unwrap();
            let mut parser = Parser::new(tokens);
            assert!(parser.parse_expression().is_ok(), "Failed for: {}", source);
        }
    }

    #[test]
    fn test_logical_operators_codegen() {
        let test_cases = vec![
            "a && b",
            "a || b",
        ];
        
        for source in test_cases {
            let tokens = Lexer::new(source.to_string()).tokenize().unwrap();
            let mut parser = Parser::new(tokens);
            assert!(parser.parse_expression().is_ok(), "Failed for: {}", source);
        }
    }

    #[test]
    fn test_bitwise_operators_codegen() {
        let test_cases = vec![
            "a & b",
            "a | b",
            "a ^ b",
            "a << b",
            "a >> b",
        ];
        
        for source in test_cases {
            let tokens = Lexer::new(source.to_string()).tokenize().unwrap();
            let mut parser = Parser::new(tokens);
            assert!(parser.parse_expression().is_ok(), "Failed for: {}", source);
        }
    }

    // ============ 赋值代码生成测试 ============

    #[test]
    fn test_simple_assignment_codegen() {
        let source = "x = 42".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    #[test]
    fn test_compound_assignment_codegen() {
        let source = "x += 1".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    // ============ 控制流代码生成测试 ============

    #[test]
    fn test_if_statement_codegen() {
        let source = "若 x > 0 则 { 返回 1 }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    #[test]
    fn test_if_else_statement_codegen() {
        let source = "若 x > 0 则 { 返回 1 } 否则 { 返回 0 }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    #[test]
    fn test_while_loop_codegen() {
        let source = "当 x < 10 则 { x = x + 1 }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    // ============ 列表操作代码生成测试 ============

    #[test]
    fn test_list_creation_codegen() {
        let source = "rt_list_new()".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_expression().is_ok());
    }

    #[test]
    fn test_list_append_codegen() {
        let source = "rt_list_append(列表, 值)".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_expression().is_ok());
    }

    #[test]
    fn test_method_call_list_append_codegen() {
        let source = "函数 主(): 整数 { 定义 可变 xs: 列表 = 列表() xs.追加(1) 返回 0 }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let module = parser.parse_module().unwrap();
        let ir = generate_ir(&module).unwrap();

        assert!(ir.contains("call void @rt_list_append"), "generated IR did not contain list append call:\n{}", ir);
    }

    #[test]
    fn test_list_index_access_codegen() {
        let source = "列表[0]".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_expression().is_ok());
    }

    #[test]
    fn test_list_index_assignment_codegen() {
        let source = "列表[0] = 新值".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_statement().is_ok());
    }

    // ============ 函数调用代码生成测试 ============

    #[test]
    fn test_function_call_codegen() {
        let source = "打印整数(42)".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_expression().is_ok());
    }

    // ============ 字符串常量代码生成测试 ============

    #[test]
    fn test_string_constant_codegen() {
        let source = "\"Hello World\"".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        
        assert!(parser.parse_expression().is_ok());
    }

    // ============ 模式匹配代码生成测试 ============

    #[test]
    fn test_match_statement_codegen() {
        // 简单枚举 + 匹配：验证生成 icmp eq 判别跳转
        let source = "枚举 颜色 { 红, 绿, 蓝 }
函数 主(): 整数 {
    定义 颜色值 = 红
    定义 可变 结果: 整数 = 0
    匹配 颜色值 {
        情况 红 => { 结果 = 1 }
        情况 绿 => { 结果 = 2 }
        默认 => { 结果 = 0 }
    }
    返回 结果
}".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let module = parser.parse_module().unwrap();
        let ir = generate_ir(&module).unwrap();

        assert!(ir.contains("icmp eq i64"), "生成的 IR 应包含 icmp eq 判别比较:\n{}", ir);
        assert!(ir.contains("br i1"), "生成的 IR 应包含 br i1 跳转:\n{}", ir);
        assert!(ir.contains("%match_cmp"), "生成的 IR 应包含 match_cmp 标签:\n{}", ir);
    }

    #[test]
    fn test_match_syntax_unit() {
        // 校验 Match 语句不再报 unsupported_feature
        let source = "匹配 颜色 { 情况 红 => { 打印(1) } 默认 => { 打印(0) } }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_statement().unwrap();
        assert!(matches!(stmt, xuanyu::Stmt::Match(_)));
    }
}