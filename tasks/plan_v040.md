# v0.4.0 实现计划（玄语编译器）

## 概述

v0.4.0 主攻**编译器自身**两大方向：**增量编译**（接入主编译管线）与**自举稳定**（全模块自举进入 CI 门禁）。
依据用户决策：新增语言特性必须同步进 L2（compiler_v2），故本版本包含 L1/L2 差距收敛与同步机制化。

v0.3.0 归档：四大特性（模式匹配/异步/调试器/发布包）已完成并发布 `v0.3.0-beta`（见 `tasks/archive/plan_v030.md`）。

## 现状盘点（2026-08-11 审计）

| 项 | 现状 | 缺口 |
|---|---|---|
| 增量编译 | `src/compiler/incremental.rs`（603 行）能力较全，但**独立骨架未接入主编译管线**；`main.rs` 仅用文件级 `.cache` hack | 接入 `MultiFileCompiler` |
| 自举门禁 | CI bootstrap job 仅逐文件 `--ir-pure`，**无全模块自举 + IR 等价验证** | 全流程自举进 CI |
| L2 async | lexer 识别关键字，parser 无 `异步/等待/启动` 语义解析 | codegen 无协程状态机/`rt_coro_*` 调用，sema 无 await 分析 |
| L2 match | parser 有 match 解析骨架 | sema 无穷尽性检查；codegen 判别跳转完整性待核对 |
| 容量上限 | L2 最大单文件 `codegen_s.xy` 147KB | 自举产能上限需突破并验证 |
| 文档/版本 | `std/README.md` 等已同步 v0.4.0-alpha | 已收口于 Step 0 |

## 架构决策

1. **增量粒度分步走**：当前多文件 codegen 为「合并型」（`main.rs` `compile_multi_file` 将多模块合并后一次性生成 IR），模块无法按文件切独立 IR。
   - 第一步先落地**检测级增量**：文件变更检测（哈希）+ 模块解析缓存 + 全量 IR 产物缓存复用，二次构建跳过未变更模块的词法/解析/语义阶段。
   - 模块级独立 codegen + 链接期符号合并作为后续扩展，不阻塞本版本交付。
2. **增量缓存目录**：统一为 `.cache/xuanyu/`，`save_state`/`load_state` 持久化构建状态（替换 `main.rs` 的 `.cache` hack）。
3. **自举稳定目标**：L1 编译全部 L2 源码 → `xyc` 自编译自身 → `xyc2` 再编译自身 → **L1/L2/L3 IR 三方等价**，作为 CI 常态化门禁。
4. **L1/L2 同步机制**：编写 `docs/自举同步规范.md`，新特性落地 L1 时必须同步 L2（本版本先补齐 v0.3.0 的 async/match 缺口）。

## 任务列表

### Step 0：预处理与基线（v0.4.0 起点）
- [x] Task 0.1: Cargo.toml 版本号升级为 v0.4.0-alpha
- [x] Task 0.2: 文档/版本号同步（std/README、README 结构树、API_REFERENCE）
- [x] Task 0.3: 旧名残留清理（徐语 → 玄语）
- [x] Task 0.4: v0.3.0 计划归档（tasks/archive/），新建本 plan_v040.md
- [x] Task 0.5: 基线门禁复跑（cargo test + e2e + 自举全绿）

### Phase 1：增量编译接入主编译管线
- [ ] Task 1.1: 审计 `compile_multi_file` 合并型 codegen，确定增量切入点
- [ ] Task 1.2: 将 `IncrementalCompiler` 接入 `MultiFileCompiler.compile()`（register_module → detect_changes → get_modules_to_rebuild）
- [ ] Task 1.3: 缓存产物与状态持久化（`.cache/xuanyu/`），统一 `.cache` hack
- [ ] Task 1.4: CLI 增 `--incremental` 标志 + 构建统计输出（命中/重建/跳过）
- [ ] Task 1.5: 失效传播正确性测试 + 二次构建性能对比测试
- [ ] Task 1.6: 文档更新（README 增量编译章节）

### Checkpoint: 增量编译
- [ ] 二次构建对未变更项目显著提速（命中率/耗时可量化）
- [ ] 修改依赖模块后下游正确重建
- [ ] cargo test 全绿无回归

### Phase 2：自举稳定（本阶段核心大项）
- [ ] Task 2.1: 盘点并补齐 L2 缺失的 v0.3.0 语义（async 状态机 codegen / `启动`/`等待` / match 穷尽性）
- [ ] Task 2.2: 容量上限专项：突破自举产能上限至 ≥147KB（codegen_s.xy）
- [ ] Task 2.3: 全模块自举脚本（L1 编译全 L2 → xyc 自编译 → xyc2 再自编译 → IR 三方等价）
- [ ] Task 2.4: CI bootstrap job 升级为全流程自举等价门禁（Linux/Windows）
- [ ] Task 2.5: 自举回归用例扩充（L2 源码新增 async/match 用法）

### Checkpoint: 自举稳定
- [ ] 全模块自举 + IR 等价在 CI 常态化通过
- [ ] L2 与 L1 对 v0.3.0 特性语义对齐
- [ ] 容量上限突破验证通过

### Phase 3：L1/L2 同步机制化
- [ ] Task 3.1: 编写 `docs/自举同步规范.md`（新特性落地 L1 的 L2 同步 checklist）
- [ ] Task 3.2: 新增特性评审提醒机制

### 整体收尾
- [ ] Task 4.1: CHANGELOG/README/发布脚本版本统一 v0.4.0-alpha
- [ ] Task 4.2: 全量回归 + 自举验证 + 发布说明
- [ ] 版本号升级为 v0.4.0-alpha（已完成）

## 里程碑

- **M1**（Step 0）：基线全绿 + 文档版本同步
- **M2**（Phase 1）：增量编译接入主链路，二次构建提速可量化
- **M3**（Phase 2）：全模块自举等价进 CI 门禁；L2 补齐 v0.3.0 async/match
- **M4**（Phase 3 + 收尾）：同步机制就绪，发布 v0.4.0-alpha

## 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| 合并型 codegen 限制增量粒度 | 高 | 分步路线：先检测级缓存，后编译级独立 codegen |
| L2 补 async/match 工作量大 | 高 | 设为 Phase 2 首要任务，单独专项充分测试 |
| 自举容量/递归多态问题 | 高 | 容量专项先于语言特性扩展 |
| Phase 1/2 并行引入回归 | 中 | 每 Phase 设 checkpoint；Step 0 先锁定门禁测试 |