# v0.3.0 实现计划（玄语编译器）

## 概述

v0.3.0 四大特性：**模式匹配**、**异步**、**调试器**、**发布包**。当前代码库已有部分骨架（lexer/parser/ast/sema 对 match/async 有基础支持），codegen 层是主要缺口。本计划按「依赖优先、垂直切片、每步可验证」组织，四个特性彼此独立可并行，但都依赖一个共同的**健全编译基线**。

## 现状盘点（2026-08-08 审计）

| 特性 | lexer | parser | ast | sema | codegen | runtime |
|------|:---:|:---:|:---:|:---:|:---:|:---:|
| 模式匹配 | ✅ 匹配/情况/默认 | ✅ `parse_match_statement` | ✅ MatchStmt/MatchArm | ⚠️ 基础分析 | ✅ 最小可用（icmp+br 链/字段绑定/默认） | — |
| 异步 | ✅ 异步/等待/启动 | ✅ Await 表达式 | ✅ AwaitExpr/AsyncContext/AsyncFn | ✅ Await 分析 | ⚠️ 最小 Await（rt_coro_await） | ✅ 协程调度器原语 |
| 调试器 | — | — | — | — | — | ❌ 无 |
| 发布包 | — | — | — | — | — | ⚠️ build_l2.bat 手写 |

**关键缺口**：
1. ~~codegen 对 `Stmt::Match` 直接报 unsupported~~ → 已实现最小可用（generate_match_stmt + 字段绑定 + 默认分支）
2. codegen 对 `Expr::Await` 从透传升级为调用 `rt_coro_await`（i64 协程句柄）；`异步/启动` 关键字 parser 尚未支持，状态机转换留待 v0.3.0 正式实现
3. 调试器完全空白；REPL 只有 `:变量`/`:函数` 等静态检查命令
4. 发布构建依赖手写 `build_l2.bat`，无跨平台自动化和产物打包

## 架构决策

1. **代码生成基线**：当前 codegen 用 `label_counter` + `%L{}` 标签 + `br i1` 生成控制流（见 codegen.rs:1316 的 generate_if_stmt）。Match 实现复用同一模式，无需引入新 IR 后端。
2. **枚举判别式**：枚举在 LLVM IR 中按 i64 存储判别值（enum_1001 = add i64 0, 0 等），match 通过 `icmp eq` 链对 subject 与各变体判别值比较，命中分支后绑定字段变量再执行分支体。这与 `若` 的跳转模式一致。
3. **异步采用轻量协程模型**：`异步` 函数编译为状态机（state + resume），`等待` 生成挂起/恢复。runtime 增加协程调度器（C 实现）。此为最小可行方案，避免引入完整 green-thread。
4. **调试器采用 CLI + LLVM 源码行映射**：利用现有 `--ir` 的 debug info 元数据（Line/Col），通过 `llc -O0` 生成带 DWARF 的目标文件，调试器基于 DWARF 定位行号。最小可行调试器 = 断点（行号）+ 单步 + 变量查看，全部在进程内通过信号/检查点实现，不依赖 gdb/lldb。
5. **发布包**：复用现有 `cargo build --release` + LLVM 工具链，编写跨平台发布脚本（PowerShell 对标 build_l2.bat），产出 zip/tar.gz，含 `xy`、`runtime.c`、`xyc`、示例、文档。CI workflow 已就绪，增加 release 触发。

## 任务列表

### Phase 1：健全基线（所有特性共同依赖）
- [x] Task 1: 建立 release 冒烟测试基线（现有集成测试 + CI 全绿为门禁）
- [x] Task 2: 新增 Match/Async 的 parser 与 AST 单元测试，锁定现有骨架行为（防重构回归）

### Checkpoint: 基线
- [x] cargo test 全绿
- [x] CI workflow 全绿

### Phase 2：模式匹配（独立，优先）
- [x] Task 3: codegen 生成 Match 判别跳转骨架（`icmp` + `br i1` 链，枚举变体匹配）
- [x] Task 4: 支持字段绑定（MatchFieldBinding 声明变量并注入分支作用域）
- [x] Task 5: 支持 `默认`（Wildcard）分支与穷尽性检查（sema）
- [x] Task 6: Match 表达式（返回值的 match）与测试
- [x] Task 7: 模式匹配 e2e 测试（枚举判别 + 字段绑定 + 默认分支）

### Checkpoint: 模式匹配
- [x] match 集成测试全部通过
- [x] 与 if/loop 组合无回归

### Phase 3：异步（与 Phase 2 并行）
- [x] Task 8: runtime.c 增加协程调度器（spawn/resume/yield 原语）
- [ ] Task 9: `异步` 函数编译为状态机（挂起点 = await 位置）
- [x] Task 10: `等待` 表达式 codegen（任务完成获取结果，i64 句柄等待）
- [ ] Task 11: `启动` 表达式 codegen（spawn 到运行时）
- [ ] Task 12: 异步 e2e 测试（spawn + await 并发执行）

### Checkpoint: 异步
- [ ] 异步示例可编译运行
- [ ] 并发协程执行顺序正确

### Phase 4：调试器（与 Phase 2/3 并行）
- [x] Task 13: CLI 增加 `--debug` 模式 + 调试命令集（断点/继续/单步/退出）— 最小版：标志 + 命令解析 + REPL 占位
- [x] Task 14: 行号映射表（源码行 → 函数/IR label 骨架，通过 AST span 构建 `LineMapping`）
- [ ] Task 15: 变量查看（在当前作用域上下文读取）
- [x] Task 16: REPL 集成调试命令（`:断点`/`:继续`/`:单步` 占位）
- [ ] Task 17: 调试器测试（断点命中、单步步进、变量值正确）— 已有 LineMapping/命令解析单测

### Checkpoint: 调试器
- [x] --debug 模式可输出行号映射（单测 + CLI 冒烟验证）
- [ ] 变量值在断点处正确

### Phase 5：发布包（独立，可与 2-4 并行）
- [x] Task 18: 跨平台发布脚本（PowerShell `build/release.ps1` 产出 dist 目录 + zip，含产物自检）
- [ ] Task 19: CI release job（cargo build --release + 打包 zip/tar.gz + artifact 上传）
- [x] Task 20: 产物自检（打包后的 xy 可编译 hello.xy）

### Checkpoint: 发布包
- [x] release 脚本在 Windows 本机产出自检通过
- [x] 产物内含 runtime.c / 示例 / 文档 / 版本号

### Phase 6：整体收尾
- [x] Task 21: v0.3.0 文档更新（README 发布打包/--debug 章节 + CHANGELOG 开发版记录 + 版本号统一宏）
- [ ] Task 22: 全量回归 + 自举验证 + 发布说明

### Checkpoint: v0.3.0
- [ ] 四特性全部落地并有测试覆盖
- [ ] 自举验证通过
- [ ] 发布包可交付

## 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| Match codegen 与现有枚举 IR 表示不兼容 | 高 | Phase 2 先验证单一枚举的判别跳转（Task 3），再扩展字段绑定 |
| 异步状态机转换复杂，超出文本 IR 可维护性 | 高 | 采用最小协程模型，不引入完整运行时；Phase 3 独立并行，失败不影响其他特性 |
| 调试器 DWARF 映射工作量超预期 | 中 | 最小可行 = 行号断点 + 单步，不做表达式求值；用 `llc -O0` 保留行号 |
| 发布脚本在三个平台行为不一致 | 中 | PowerShell 为主 + CI 矩阵逐平台验证（沿用现有 workflow 矩阵） |
| 四个特性并行引入回归 | 中 | 每个 Phase 结束均有 Checkpoint；Phase 1 先锁定基线测试 |

## 待决问题

1. 异步采用「轻量协程（状态机）」还是「完整线程/Green Thread」？本文档默认协程，如需线程模型工作量显著增大。
2. 调试器是否需要 REPL 深度集成，还是独立 `xy --debug` CLI？本文档默认 CLI + 轻量 REPL 命令。
3. 模式匹配是否需要支持**结构化模式**（如 `情况 (a, b)` 匹配元组）？当前 AST 仅支持枚举变体 + 通配，本期建议聚焦枚举 + 字面量，结构化留待后续。
