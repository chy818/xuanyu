# 玄语（XY Language）

**玄语** 是一门以中文为原生语法的编译型编程语言，基于 LLVM 构建，支持自举编译。

> 本项目原名 ZHCC，现正式更名为「玄语」。代码处于早期阶段，架构可能随时调整。欢迎提 Issue 或 PR 共同完善！

---

## 项目状态

| 模块                   | 状态 | 说明                                             |
| ---------------------- | :--: | ------------------------------------------------ |
| **词法分析 (lexer)**   |  ✅  | 支持中文标识符、语义空格（警告级别）、关键字     |
| **语法解析 (parser)**  |  ✅  | 递归下降解析，支持函数/表达式/控制流/结构体/枚举 |
| **语义分析 (sema)**    |  ✅  | 作用域链、类型检查、标识符解析、错误收集         |
| **代码生成 (codegen)** |  ✅  | LLVM IR 生成（i8\* 指针规范）                    |
| **运行时 (runtime)**   |  ✅  | C 运行时库，支持打印/内存/字符串/文件操作        |
| **自举 (compiler_v2)** |  🔨  | XY 自身实现，框架完成，约 30-70% 功能            |
| **测试用例**           |  ✅  | 7 个测试通过                                     |

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
cargo run -- examples\hello.xy --run

# 仅生成 IR
cargo run -- examples\hello.xy --ir

# 运行测试
cargo test
```

### 输出示例

```
=== XY Language 编译器 ===
版本: 0.1.0
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
├── src/                        # Rust 主线编译器
│   ├── main.rs                 # CLI 入口
│   ├── lib.rs                  # 核心库导出
│   ├── lexer/                  # 词法分析
│   │   ├── mod.rs
│   │   ├── lexer.rs            # 词法扫描器
│   │   └── token.rs            # Token 定义
│   ├── parser/                 # 语法解析
│   │   └── parser.rs           # 递归下降解析器
│   ├── ast/                    # AST 节点定义
│   │   ├── mod.rs
│   │   └── ast.rs
│   ├── sema/                   # 语义分析
│   │   └── sema.rs             # 作用域/类型检查
│   ├── codegen/                # 代码生成
│   │   └── codegen.rs          # LLVM IR 生成 (1785 行)
│   ├── types/                  # 类型系统
│   │   └── types.rs
│   └── error/                  # 错误处理
│       └── error.rs
├── src/compiler_v2/            # 自举编译器 (XY 实现)
│   ├── main.xy                 # 主入口
│   ├── compiler.xy             # 编译器整合
│   ├── lexer.xy                # 词法分析 (~60%)
│   ├── parser.xy               # 语法解析 (~30%)
│   ├── codegen.xy              # 代码生成 (~30%)
│   ├── runtime.xy              # 运行时占位
│   └── hello.xy                # 自举测试用例
├── runtime/                    # C 运行时库
│   ├── runtime.c               # 主运行时（中文函数名）
│   └── runtime_clean.c         # 干净版本（ASCII 函数名）
├── examples/                   # 示例程序
│   ├── hello.xy                # Hello World
│   └── *.xy                    # 其他示例
├── tests/                      # 测试脚本
│   └── bootstrap_test.sh       # 自举验证脚本
├── Cargo.toml
├── VISION.md                   # 愿景文档
└── CCAS_SPEC_V0.0.md          # 中文计算体系结构规范
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
| 整数    | 有符号整型   | i32       |
| 长整数  | 有符号长整型 | i64       |
| 浮点数  | 单精度浮点   | float     |
| 文本    | UTF-8 字符串 | i8\*      |
| 布尔    | 真/假        | i1        |
| 无返回  | void 函数    | void      |

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

## 自举路线图

```
Phase 1 ✅ 已完成
├── Rust 实现完整编译器
├── 支持基础语法和控制流
└── 生成可运行的 LLVM IR

Phase 2 🔨 当前阶段
├── compiler_v2 自举实现
├── lexer.xy 框架完成 (~60%)
├── parser.xy 框架完成 (~30%)
└── codegen.xy 框架完成 (~30%)

Phase 3 📋 待完成
├── 完善 parser.xy 递归下降解析
├── 实现 sema.xy 语义分析
├── 完成 codegen.xy IR 生成
└── 实现 runtime.xy 运行时库

Phase 4 📋 自举验证
├── xy.exe 编译 src/compiler_v2/*.xy
├── 生成 IR 并执行
└── 验证自举成功
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
cargo test lexer::test_xxx    # 运行特定测试
cargo test --test lexer        # 运行 lexer 模块测试
cargo test --lib               # 运行库单元测试
```

---

## 贡献指南

1. Fork 项目并创建分支：`git checkout -b feat/你的功能`
2. 遵循项目编码规范(详情查看编码规范文档)
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
