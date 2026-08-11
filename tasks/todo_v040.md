# v0.4.0 Todo

## Step 0: 预处理与基线
- [x] Task 0.5: 基线门禁复跑（cargo test + e2e + 自举全绿）

## Phase 1: 增量编译接入主编译管线
- [x] Task 1.1: 审计 compile_multi_file 合并型 codegen（确认「检测级增量」路线，见 plan_v040.md）
- [x] Task 1.2: IncrementalCompiler 接入 MultiFileCompiler.compile()（incremental_change_set + 全新实例比对流程）
- [x] Task 1.3: 缓存产物与状态持久化（.cache/xuanyu/<模块>-<路径哈希>/，meta.json + build_state.json）
- [x] Task 1.4: CLI --incremental + 构建统计
- [x] Task 1.5: 失效传播 + 性能对比测试（A1/B2 手动验证 + 4 个增量单测）
- [x] Task 1.6: 文档更新（README 增量编译章节 + .gitignore .cache/）

## Phase 2: 自举稳定
- [ ] Task 2.1: 补齐 L2 缺失语义（async 状态机 / 启动/等待 / match 穷尽性）
- [ ] Task 2.2: 容量上限突破（≥147KB）
- [ ] Task 2.3: 全模块自举脚本（L1→xyc→xyc2→IR 等价）
- [ ] Task 2.4: CI bootstrap job 全流程门禁
- [ ] Task 2.5: 自举回归用例扩充

## Phase 3: L1/L2 同步机制化
- [ ] Task 3.1: docs/自举同步规范.md
- [ ] Task 3.2: 新特性评审提醒

## 收尾
- [ ] Task 4.1: CHANGELOG/README 版本统一
- [ ] Task 4.2: 全量回归 + 自举验证 + 发布说明

## 依赖关系
- Task 0.5 依赖 Step 0 前四项（已完成）
- Task 1.2 依赖 1.1；Task 1.5 依赖 1.2-1.4（本迭代已完成，含删除/新增/修改/产物复用验证）
- Task 2.1-2.5 顺序执行，Task 2.4 依赖 2.3
- Phase 1 与 Phase 2 相互独立，可并行
- Task 4.x 依赖全部