/**
 * @file tests/unit/parser_test.rs
 * @brief 语法分析器单元测试
 * @description 测试 parser 对各种语法结构的正确解析
 */

#[cfg(test)]
mod parser_tests {
    use xuanyu::ast::{ASTNode, MatchPattern};
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

    // ============ 模式匹配语句测试 ============

    #[test]
    fn test_simple_match_statement() {
        // 匹配 颜色 { 情况 红 => { ... } }
        let source = "匹配 颜色 { 情况 红 => { 打印(\"红色\") } }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_statement().unwrap();

        match stmt {
            xuanyu::Stmt::Match(m) => {
                assert_eq!(m.arms.len(), 1);
                match &m.arms[0].pattern {
                    MatchPattern::EnumVariant { variant_name, .. } => {
                        assert_eq!(variant_name, "红");
                    }
                    _ => panic!("期望枚举变体模式"),
                }
            }
            _ => panic!("期望 Match 语句"),
        }
    }

    #[test]
    fn test_match_with_field_bindings() {
        // 带字段绑定的匹配：情况 加法(左, 右) => ...
        let source = "匹配 表达式 { 情况 加法(左, 右) => { 返回 左 + 右 } }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_statement().unwrap();

        match stmt {
            xuanyu::Stmt::Match(m) => {
                assert_eq!(m.arms.len(), 1);
                match &m.arms[0].pattern {
                    MatchPattern::EnumVariant { variant_name, fields, .. } => {
                        assert_eq!(variant_name, "加法");
                        assert_eq!(fields.len(), 2);
                        assert_eq!(fields[0].binding_name, "左");
                        assert_eq!(fields[1].binding_name, "右");
                    }
                    _ => panic!("期望枚举变体模式"),
                }
            }
            _ => panic!("期望 Match 语句"),
        }
    }

    #[test]
    fn test_match_with_wildcard_default() {
        // 带默认分支的模式匹配
        let source = "匹配 颜色 { 情况 红 => { 打印(1) } 默认 => { 打印(0) } }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_statement().unwrap();

        match stmt {
            xuanyu::Stmt::Match(m) => {
                assert_eq!(m.arms.len(), 2);
                assert!(matches!(m.arms[0].pattern, MatchPattern::EnumVariant { .. }));
                assert!(matches!(m.arms[1].pattern, MatchPattern::Wildcard));
            }
            _ => panic!("期望 Match 语句"),
        }
    }

    #[test]
    fn test_match_multiple_arms() {
        // 多分支 + 默认
        let source = "匹配 颜色 { 情况 红 => { 打印(1) } 情况 绿 => { 打印(2) } 情况 蓝 => { 打印(3) } 默认 => { 打印(0) } }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_statement().unwrap();

        match stmt {
            xuanyu::Stmt::Match(m) => {
                assert_eq!(m.arms.len(), 4);
                assert_eq!(m.subject.span().start_line, 1);
            }
            _ => panic!("期望 Match 语句"),
        }
    }

    #[test]
    fn test_match_named_field_bindings() {
        // 命名字段绑定: 情况 点(x: px, y: py) => ...
        let source = "匹配 点 { 情况 点(x: px, y: py) => { 返回 px } }".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_statement().unwrap();

        match stmt {
            xuanyu::Stmt::Match(m) => {
                match &m.arms[0].pattern {
                    MatchPattern::EnumVariant { fields, .. } => {
                        assert_eq!(fields.len(), 2);
                        assert_eq!(fields[0].name.as_deref(), Some("x"));
                        assert_eq!(fields[0].binding_name, "px");
                        assert_eq!(fields[1].name.as_deref(), Some("y"));
                        assert_eq!(fields[1].binding_name, "py");
                    }
                    _ => panic!("期望枚举变体模式"),
                }
            }
            _ => panic!("期望 Match 语句"),
        }
    }

    // ============ 异步 / 等待 表达式测试 ============

    #[test]
    fn test_await_expression() {
        let source = "等待 异步函数()".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression().unwrap();

        match expr {
            xuanyu::Expr::Await(_) => {}
            other => panic!("期望 Await 表达式，得到 {:?}", other),
        }
    }

    #[test]
    fn test_await_with_identifier() {
        let source = "等待 future".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression().unwrap();

        match expr {
            xuanyu::Expr::Await(await_expr) => {
                match &*await_expr.expr {
                    xuanyu::Expr::Identifier(ident) => assert_eq!(ident.name, "future"),
                    other => panic!("期望 Await 内部为标识符，得到 {:?}", other),
                }
            }
            other => panic!("期望 Await 表达式，得到 {:?}", other),
        }
    }

    #[test]
    fn test_await_statement() {
        let source = "等待 获取数据()".to_string();
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        assert!(parser.parse_statement().is_ok());
    }
}