# W15-A T7.2 第二轮反馈接力

承接：`F:/soft/00selfmade/media-to-doc-ui/handoff-w15-a-task12-build-verify-2026-07-25.md`，先读 §0.6、§6，再读 `task.md`。
分支：`feat/w15a-llm-api-settings`；Tasks 1-12 工作区均未 commit，严禁 reset/checkout/restore 覆盖。

## 第一条回复（强制）
先用非技术语言向用户说明本阶段计划：让用户能选已添加模型、像聊天一样发布任务并选择课程目录，再接通后台与 long-doc 自动同步；不要先讲 Rust、IPC、schema。

## 必交付
1. 先用 brainstorming + mini spec/plan；确认 task text 下游用途，并区分 Image Agent 的“配图策划 LLM”与“真正出图 provider”。
2. New Run 动态读取 `list_llm_profiles`，MiniMax 等按 profile name 可选；每次 run 独立传 `llmProfileName`，禁止靠切换全局 active profile。
3. 增加任务 textarea + native directory picker + project registry；规范化真实路径去重，同路径合并 sessions，重名不同路径分开。
4. Image Agent 可选已保存在线 LLM profile 做配图策划；真实图片生成另选 image provider，不能只做无效下拉框。
5. `long-doc-processor` 当前仅“参考”、未整合；实现 vendored snapshot + SHA256 manifest + sync/verify tests。
6. Claude 修改 Skill 后用 `PostToolUse(Edit|Write)` hook 自动同步；先尝试 `update-config` Skill，若 schema 报错则记录并按现有 hooks 结构最小编辑。
7. Stop after 增加中文说明/tooltip；定时任务继续 parked/disabled。
8. TDD；改 `commands.rs`/`runner.rs` 必须两轮 review；跑主仓 pytest、UI 98 tests、tauri build 和更新后的真机验收。

## 禁止
不 commit/push/release/bump；不泄露 key；不让发布包运行时依赖个人 `~/.claude` 路径；不启用或实现定时调度；不触碰主仓/子仓无关改动。
T8 v1.5.0 release 继续 blocked，完成后写新的 complete/blocked handoff + 下一会话 prompt。
