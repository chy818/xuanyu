/**
 * @file tests/README.md
 * @brief 测试套件说明
 * @description 玄语编译器测试套件的结构和使用方法
 */

# 玄语编译器测试套件

## 概述

玄语编译器的测试套件分为三个层次：
1. **单元测试 (Unit Tests)** - 测试各个编译阶段
2. **集成测试 (Integration Tests)** - 测试完整程序编译和运行
3. **自展测试 (Bootstrap Tests)** - 验证编译器能编译自身

## 目录结构

```
tests/
├── README.md              # 本文档
├── run_tests.ps1          # 测试自动化脚本 (PowerShell)
├── bootstrap_test.sh      # 自展验证脚本 (Bash)
├── unit/                  # 单元测试
│   ├── lexer_test.rs      # 词法分析器测试
│   ├── parser_test.rs     # 语法分析器测试
│   └── codegen_test.rs    # 代码生成器测试
├── integration/           # 集成测试
│   ├── operator_test.xy   # 运算符测试
│   ├── control_flow_test.xy  # 控制流测试
│   └── list_test.xy       # 列表测试
└── bootstrap/             # 自展测试
    └── self_compile_test.xy  # 自展编译测试
```

## 运行测试

### 运行所有测试

```powershell
.\tests\run_tests.ps1 -All
```

### 运行特定类型的测试

```powershell
# 单元测试
.\tests\run_tests.ps1 -Unit

# 集成测试
.\tests\run_tests.ps1 -Integration

# 自展测试
.\tests\run_tests.ps1 -Bootstrap
```

### 详细输出

```powershell
.\tests\run_tests.ps1 -Verbose
```

## 单元测试

### Lexer 测试 (lexer_test.rs)

测试词法分析器对各种词法单元的正确识别：
- 关键字识别（若、则、否则、循环、函数等）
- 中文标识符
- 数字字面量（整数、浮点数、十六进制）
- 字符串字面量
- 运算符（算术、比较、逻辑、位运算）
- 界符（括号、分号等）
- 布尔字面量（真、假）
- 注释

### Parser 测试 (parser_test.rs)

测试语法分析器对各种语法结构的正确解析：
- 表达式解析
- 变量定义
- if 语句（含否则若）
- while 循环
- for 循环
- 函数定义
- 列表操作
- 运算符表达式
- 块语句和返回语句

### Codegen 测试 (codegen_test.rs)

测试代码生成器对 LLVM IR 的正确生成：
- 表达式代码生成
- 变量分配和加载
- 运算符代码生成（算术、比较、逻辑、位运算）
- 赋值代码生成
- 控制流代码生成
- 列表操作代码生成
- 函数调用代码生成
- 字符串常量代码生成

## 集成测试

### 运算符测试 (operator_test.xy)

全面测试各种运算符：
- 算术运算符 (+, -, *, /, %)
- 关系运算符 (==, !=, <, >, <=, >=)
- 逻辑运算符 (&&, ||, !)
- 位运算符 (&, |, ^, <<, >>)
- 复合赋值 (+=, -=, *=, /=, %=)

### 控制流测试 (control_flow_test.xy)

测试控制流语句：
- if-else if-else 结构
- 多层嵌套
- while 循环
- for 循环
- break 和 continue

### 列表测试 (list_test.xy)

测试列表的创建和操作：
- 列表创建 (rt_list_new)
- 元素添加 (rt_list_append)
- 列表索引访问
- 列表元素修改
- 遍历列表
- 嵌套列表
- 字符串列表和整数列表

## 自展测试

### 自展编译测试 (self_compile_test.xy)

验证编译器能够编译自身的各个模块：
- 函数定义
- 变量定义和赋值
- 条件语句
- 否则若语句
- 循环语句
- 列表操作

## 测试输出

测试运行后会生成以下输出：
- `output/` 目录包含各测试的 IR 输出
- 控制台显示测试结果统计

## 添加新测试

### 添加单元测试

在对应的测试文件中添加新的 `#[test]` 函数：

```rust
#[test]
fn test_new_feature() {
    // 测试代码
}
```

### 添加集成测试

在 `tests/integration/` 目录创建新的测试文件：

```xy
/**
 * @file tests/integration/new_feature_test.xy
 * @brief 新功能测试
 */

函数 主() : 整数 {
    // 测试代码
    返回 0
}
```

### 添加自展测试

在 `tests/bootstrap/` 目录创建新的测试文件。

## 测试最佳实践

1. **每个测试应该独立** - 测试之间不应有依赖关系
2. **测试应该有明确的预期** - 断言应该清晰
3. **测试应该快速运行** - 避免耗时操作
4. **测试应该覆盖边界情况** - 特别注意空值、极值等情况
5. **测试应该定期运行** - 建议每次提交前运行

## CI/CD 集成

测试脚本可以集成到 CI/CD 流程中：

```bash
# 在 CI 脚本中
powershell -File tests/run_tests.ps1 -All
```

## 报告问题

如果发现测试失败，请检查：
1. 编译器是否正确编译
2. 运行时环境是否配置正确
3. 测试文件是否有语法错误

如有问题，请在 GitHub 仓库中提交 issue。
