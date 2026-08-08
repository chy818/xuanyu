# v0.3.0 Todo

## Phase 1: 健全基线
- [ ] Task 1: 建立 release 冒烟测试基线（集成测试 + CI 全绿为门禁）
- [ ] Task 2: Match/Async 的 parser 与 AST 单元测试锁定骨架行为

## Phase 2: 模式匹配（独立优先）
- [ ] Task 3: codegen Match 判别跳转骨架（icmp + br i1 链）
- [ ] Task 4: 字段绑定（MatchFieldBinding 声明变量注入分支作用域）
- [ ] Task 5: 默认分支 + 穷尽性检查（sema）
- [ ] Task 6: Match 表达式（返回值）+ 测试
- [ ] Task 7: 模式匹配 e2e 测试

## Phase 3: 异步（与 Phase 2 并行）
- [ ] Task 8: runtime.c 协程调度器（spawn/resume/yield）
- [ ] Task 9: 异步函数编译为状态机（await 为挂起点）
- [ ] Task 10: 等待表达式 codegen（挂起/恢复）
- [ ] Task 11: 启动表达式 codegen（spawn）
- [ ] Task 12: 异步 e2e 测试（并发执行）

## Phase 4: 调试器（与 Phase 2/3 并行）
- [ ] Task 13: CLI --debug 模式 + 调试命令集
- [ ] Task 14: 行号映射表（源码行 → IR label）
- [ ] Task 15: 变量查看
- [ ] Task 16: REPL 集成调试命令
- [ ] Task 17: 调试器测试

## Phase 5: 发布包（独立）
- [ ] Task 18: 跨平台发布脚本（对标 build_l2.bat）
- [ ] Task 19: CI release job（打包 + artifact 上传）
- [ ] Task 20: 产物自检（打包后 xy 可编译 hello.xy）

## Phase 6: 整体收尾
- [ ] Task 21: v0.3.0 文档更新（README/CHANGELOG/API_REFERENCE）
- [ ] Task 22: 全量回归 + 自举验证 + 发布说明

## 依赖关系
- Task 2 依赖 Task 1
- Task 3-7 顺序执行，Task 7 依赖 3-6
- Task 8-12 顺序执行，Task 12 依赖 8-11
- Task 13-17 顺序执行，Task 17 依赖 13-16
- Task 18-20 顺序执行，Task 20 依赖 18-19
- Task 21-22 依赖全部
- Phase 2/3/4/5 相互独立，可并行
