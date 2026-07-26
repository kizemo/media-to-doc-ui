# W15-A T7.2 第三轮接力(post 2026-07-26 T7+T8 done)

承接：`F:/soft/00selfmade/media-to-doc-ui/handoff-w15-a-t7-t8-complete-2026-07-26.md` + `docs/superpowers/plans/2026-07-25-w15a-t7-2-product-feedback.md` plan §Task 9-12。

分支：
- Tauri UI：`feat/w15a-llm-api-settings`（working tree，本会话 T7+T8 已改）
- 主仓：`feat/w15a-t7-2-task4-imagegen`（无本会话改动）

本会话完成：
- **T7-Tauri**：Cargo.toml 加 sha2 + tauri-plugin-dialog、lib.rs 注册 plugin + 4 project commands、commands.rs 加 4 *_impl + 5 tests；并行污染用 static Mutex 串行化
- **T8-Tauri**：src/index.html `__mountNewRunTab__` + `buildNewRunForm` 大改，加 task textarea + 工作目录行 + 选目录按钮 + Image Agent 折叠面板 + `__projectTree__.refresh/selectProject`
- **总计 111 passed / 0 failed**（106 baseline + 5 T7；主仓 620 passed 不变）

任务（优先级降序，任选）：
1. **T9-main long-doc snapshot + sync/verify**（~15 min）— 主仓独立，snapshot bootstrap + sync/verify 脚本 + 5 tests。读 plan §T9 + spec §5
2. **T10-main longdoc.py 读 vendored + pyproject**（~10 min）— importlib.resources 读 snapshot + `[tool.hatch.build]` force-include。读 plan §T10 + spec §5.3-5.4
3. **T11-Tauri bundle.resources + Claude hook**（~10 min）— `tauri.conf.json` resources + `~/.claude/settings.json` PostToolUse hook（update-config Skill 优先）。读 plan §T11 + spec §5.4-5.6
4. **T12-verify + handoff-complete**（~10 min）— `cargo test --lib` ≥113 + `uv run pytest` ≥620 + `cargo tauri build` exit 0 + 写 complete handoff

加速模式：每个 task 末尾「不 commit」，工作区累积保留。T8 release session 才一次 feature commit + bump v1.5.0。

禁止：
- ❌ 不 commit / push / release / bump / reset / checkout / restore
- ❌ 不实现定时调度器
- ❌ 不动 Tauri UI `feat/w15a-llm-api-settings` 之外的无关文件
- ❌ 不让 NSIS 运行时依赖 `C:/Users/Duanyi/.claude/`
- ❌ 不把 API key 写 HTML / log / CLI
- ❌ 不让主仓尝试解析 profile name

期望：本会话挑 1-3 个 task 跑，留剩余给后续会话。建议跑 T9+T10（独立可快速），时间允许再做 T11 hook；T12 留 release session。