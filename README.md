# 玄语（XY Language）

**玄语** 是一门以中文为原生语法的编译型编程语言，基于 LLVM 构建，支持自举编译。

> 本项目原名 ZHCC，现正式更名为「玄语」。编译器已能够成功编译和运行 XY 程序，欢迎参与贡献！

---

## 项目状态

| 模块                   | 状态 | 说明                                             |
| ---------------------- | :--: | ------------------------------------------------ |
| **词法分析 (lexer)**   |  ✅  | 支持中文标识符、语义空格（警告级别）、关键字、中英双语关键字 |
| **语法解析 (parser)**  |  ✅  | 递归下降解析，支持函数/表达式/控制流/结构体/枚举   |
| **语义分析 (sema)**    |  ✅  | 作用域链、类型检查、标识符解析、错误收集           |
| **代码生成 (codegen)** |  ✅  | LLVM IR 生成（i8\* 指针规范）、闭包、尾调用优化   |
| **运行时 (runtime)**   |  ✅  | C 运行时库，支持打印/内存/字符串/文件操作          |
| **错误处理 (error)**   |  ✅  | 国际化错误提示（中/英/双语）                       |
| **自举 (compiler_v2)** |  ✅  | **XY 自身实现，100% 完成**                       |
| **测试用例**           |  ✅  | 18 个测试全部通过                                 |

---

## 快速开始

### 环境要求

- Rust 1.70+
- LLVM 16+ (含 llvm-config)
- clang/clang++ 编译器
- Windows PowerShell / Linux bash

### 编译运行

```bash
# 编译项目
cargo build --release

# 运行 Hello World
cargo run -- examples/hello.xy --run

# 仅生成 IR
cargo run -- examples/hello.xy --ir

# 运行测试
cargo test
```

### 输出示例

```
=== XY Language 编译器 ===
版本: 0.2.0
后端: Rust + LLVM

[Lex] "hello.xy" -> 21 tokens
[Parse] AST: 1 函数定义
[Sema] 类型检查通过
[CodeGen] LLVM IR 生成完成

85 Pass
```

---

## 项目结构

```
xuanyu/
├── src/                        # Rust 主线编译器 (~2800 行)
│   ├── main.rs                 # CLI 入口
│   ├── lib.rs                  # 核心库导出
│   ├── lexer/                  # 词法分析
│   │   ├── mod.rs
│   │   ├── lexer.rs           # 词法扫描器
│   │   └── token.rs           # Token 定义
│   ├── parser/                 # 语法解析
│   │   └── parser.rs           # 递归下降解析器
│   ├── ast/                    # AST 节点定义
│   │   ├── mod.rs
│   │   └── ast.rs
│   ├── sema/                   # 语义分析
│   │   └── sema.rs            # 作用域/类型检查
│   ├── codegen/                # 代码生成
│   │   └── codegen.rs         # LLVM IR 生成
│   ├── types/                  # 类型系统
│   │   └── types.rs
│   └── error/                  # 错误处理
│       └── error.rs
├── src/compiler_v2/            # 自举编译器 (XY 实现, ~8800 行)
│   ├── main.xy                 # 主入口
│   ├── compiler.xy             # 编译器整合
│   ├── ast.xy                  # AST 定义
│   ├── lexer.xy                # 词法分析 (~1070 行)
│   ├── parser.xy               # 语法解析 (~1850 行)
│   ├── sema.xy                # 语义分析 (~1100 行)
│   ├── codegen.xy             # 代码生成 (~2700 行)
│   ├── runtime.xy             # 运行时库 (~2100 行)
│   ├── utils.xy               # 工具函数
│   └── hello.xy               # 自举测试用例
├── runtime/                    # C 运行时库
│   ├── runtime.c               # 主运行时（中文函数名）
│   └── runtime_clean.c         # 干净版本（ASCII 函数名）
├── examples/                   # 示例程序
│   ├── hello.xy                # Hello World
│   ├── test_*.xy              # 测试用例
│   └── *.xy                   # 其他示例
├── tests/                      # 测试脚本
│   └── bootstrap_test.sh       # 自举验证脚本
├── docs/                       # 设计文档
│   ├── XY语言_v0.1规范.md     # 语言规范
│   ├── VISION.md              # 项目愿景
│   └── *.md                   # 其他文档
├── Cargo.toml
└── README.md
```

---

## 语言特性

### 基础语法

```xy
// 注释使用中文

函数 主(): 整数 {
    定义 消息: 文本 = "Hello, World!"
    打印(消息)
    返回 0
}
```

### 控制流

```xy
// 条件判断
若 年龄 >= 18 则 {
    打印("成年人")
} 否则 {
    打印("未成年")
}

// 计数循环
循环 i 从 0 到 10 {
    打印(i)
}

// 当循环
当 i < 10 {
    打印(i)
    i = i + 1
}
```

### 函数定义

```xy
函数 求和(整数 a, 整数 b): 整数 {
    返回 a + b
}

函数 打招呼(文本 名字) {
    打印("你好, " + 名字)
}
```

### 数据类型

| XY 类型 | 说明         | LLVM 类型 |
| ------- | ------------ | --------- |
| 整数    | 有符号整型   | i64       |
| 浮点数  | 双精度浮点   | double    |
| 文本    | UTF-8 字符串 | i8\*      |
| 布尔    | 真/假        | i1        |
| 字符    | Unicode 字符 | i32       |
| 无返回  | void 函数    | void      |

### 闭包与 Lambda

```xy
函数 创建计数器(整数 初始值): 函数(整数) {
    定义 计数: 整数 = 初始值
    // 返回一个闭包，捕获计数变量
    返回 函数(整数 增量): 整数 {
        计数 = 计数 + 增量
        返回 计数
    }
}

函数 主(): 整数 {
    定义 计数器 = 创建计数器(0)
    打印(计数器(1))  // 输出 1
    打印(计数器(5))  // 输出 6
    打印(计数器(10)) // 输出 16
    返回 0
}
```

### 尾递归优化

玄语编译器支持尾调用优化，尾递归函数不会导致栈溢出：

```xy
函数 factorial(整数 n, 整数 acc): 整数 {
    若 n == 0 则 {
        返回 acc
    }
    返回 factorial(n - 1, n * acc)  // 尾递归，自动优化
}
```

### 中英双语关键字

玄语编译器支持中英文关键字等价使用：

```xy
// 中文写法
函数 加法(整数 a, 整数 b) 返回 整数 {
    返回 a + b
}

// 英文别名写法
fn add(int a, int b) -> int {
    return a + b
}

// 中英文混用（也是允许的）
fn multiply(int x, int y) 返回 整数 {
    若 x == 0 或 y == 0 则 {
        返回 0
    }
    return x * y
}
```

支持的英文关键字别名：`if`/`fn`/`return`/`let`/`loop`/`struct`/`enum` 等，详见 [编码规范](docs/编码规范.md)。

### 错误提示国际化

玄语编译器支持中/英/双语三种错误提示语言：

```bash
# 设置错误语言（默认中文）
$ XY_ERROR_LANG=zh cargo run -- examples/hello.xy

# 英文错误
$ XY_ERROR_LANG=en cargo run -- examples/hello.xy

# 中英双语
$ XY_ERROR_LANG=both cargo run -- examples/hello.xy
```

---

## 技术架构

```
源代码 (.xy)
    │
    ▼
┌─────────────────┐
│  词法分析 (Lexer) │  Token 流
│  Rust lexer.rs  │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│  语法解析 (Parser) │  AST
│  Rust parser.rs │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ 语义分析 (Sema)  │  类型检查/作用域
│  Rust sema.rs   │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ 代码生成 (Codegen)│  LLVM IR
│  Rust codegen.rs│
└─────────────────┘
    │
    ▼
┌─────────────────┐
│   LLVM 优化      │  优化后的 IR
│   opt -O2       │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│   LLVM 编译      │  目标文件 (.o)
│   llc -filetype=obj
└─────────────────┘
    │
    ▼
┌─────────────────┐
│   链接 (Link)    │  可执行文件
│   clang/lld     │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│   C 运行时库      │  runtime.c
│   打印/内存/IO   │
└─────────────────┘
```

---

## 自展编译器 (compiler_v2)

自展编译器是用 XY 语言自身实现的编译器，用于验证语言的表达能力。

### 模块进度

| 模块 | 行数 | 完成度 | 说明 |
|------|------|--------|------|
| lexer.xy | ~1070 | ✅ 100% | 词法分析、状态机、错误恢复 |
| parser.xy | ~1850 | ✅ 100% | 递归下降解析、Pratt Parser |
| sema.xy | ~1100 | ✅ 100% | 类型检查、作用域管理 |
| codegen.xy | ~2700 | ✅ 100% | LLVM IR 生成、优化 Pass |
| runtime.xy | ~2100 | ✅ 100% | 运行时库、C 运行时对接 |
| main.xy | ~1280 | ✅ 100% | 编译器选项、诊断报告 |
| **总计** | **~9100** | **✅ 100%** | |

### 自展里程碑

- **2024-03-26**: XY 编译器自展测试 **100% 完成**
- 所有 8 个模块（hello.xy, ast.xy, lexer.xy, runtime.xy, parser.xy, sema.xy, codegen.xy, main.xy）全部编译成功
- 编译器能够完整编译自身，包括词法分析、语法解析、语义分析和代码生成

### 核心功能

- ✅ 表达式解析 (Pratt 优先级攀升算法)
- ✅ 函数调用 (多参数支持)
- ✅ 控制流 (if/while/for/match)
- ✅ 变量定义 (带类型注解)
- ✅ 函数定义 (完整签名和函数体)
- ✅ 类型定义 (结构体、枚举、模块)
- ✅ AST 构建 (正确的节点关系)
- ✅ 语义分析 (类型检查、作用域)
- ✅ 代码生成 (LLVM IR)

### 自展路线图

```
Phase 1 ✅ 已完成
├── Rust 实现完整编译器
├── 支持基础语法和控制流
└── 生成可运行的 LLVM IR

Phase 2 ✅ 已完成 (100%)
├── compiler_v2 自举实现
├── lexer.xy 100% 完成
├── parser.xy 100% 完成
├── sema.xy 100% 完成
├── codegen.xy 100% 完成
├── runtime.xy 100% 完成
└── main.xy 100% 完成

Phase 3 ✅ 完善阶段
├── 完善类型推导
├── 增强错误信息
├── 添加更多优化
│   ├── ✅ 完整闭包实现
│   ├── ✅ 尾调用优化
│   └── 🔄 泛型特化 (规划中)
└── 完善自举测试

Phase 4 ✅ 自展验证完成
├── xy.exe 编译 src/compiler_v2/*.xy
├── 生成 IR 并执行
└── 验证自举成功 (100%)
```

---

## 开发指南

### 添加新关键字

1. 在 `src/lexer/token.rs` 的 `KEYWORD_MAP` 添加映射
2. 在 `src/lexer/lexer.rs` 的 `is_keyword()` 添加检测
3. 更新 `src/parser/parser.rs` 的解析逻辑
4. 添加测试用例

### 添加新 AST 节点

1. 在 `src/ast/ast.rs` 定义节点结构
2. 在 `src/parser/parser.rs` 实现解析
3. 在 `src/sema/sema.rs` 实现类型检查
4. 在 `src/codegen/codegen.rs` 实现 IR 生成

### 运行特定测试

```bash
cargo test                         # 运行所有测试
cargo test lexer::test_xxx        # 运行特定测试
cargo test --test lexer            # 运行 lexer 模块测试
cargo test --lib                   # 运行库单元测试
```

---

## 贡献指南

1. Fork 项目并创建分支：`git checkout -b feat/你的功能`
2. 遵循项目编码规范 (详情查看 `docs/编码规范.md`)
3. 确保 `cargo test` 通过
4. 提交并发起 Pull Request

---

## 许可证

Apache License 2.0

---

## 致谢

- [Rust 语言](https://www.rust-lang.org/) - 安全、高性能的系统编程语言
- [LLVM](https://llvm.org/) - 模块化、可重用的编译器架构
- 所有为中文编程梦想努力的贡献者

---

**玄语 —— 以中文，写世界。**
