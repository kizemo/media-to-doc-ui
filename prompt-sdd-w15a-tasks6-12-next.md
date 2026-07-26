# 接力 prompt — W15-A SDD Tasks 6-12

**承接**:`F:/soft/00selfmade/media-to-doc-ui/handoff-sdd-w15a-tasks6-12-2026-07-24.md`

## 当前状态(2026-07-24 末)
- 分支:`feat/w15a-llm-api-settings`,无新 commit(`073b05e` 还是 plan commit)
- Tasks 1-5 已完成(工作区累积,未 commit):capability description + module error handler + 删旧结构 + §1 §2 §3 侧栏
- Tasks 6-12 剩 7 个:§4 Project Tree / §5 Settings Gear / Tab Manager / Session Tab / New Run Tab / Settings mount / Build + 13 项验收

## 必交付清单(顺序)
按 `.superpowers/sdd/progress.md` 顺序跑 Task 6 → 7 → 8 → 9 → 10 → 11 → 12(每 task 用 `subagent-driven-development` skill:dispatch implementer + reviewer + fix 循环)。每 task 末尾"Save state" = 写 `handoff-w15-a-taskN-*.md`,**不 git commit**。

**2 个 plan 缺陷必须 inject 到 dispatch prompts**:
1. **缺陷 1** (Task 9/10/11): 实现 `function __mountXxxTab__` 后必须追加 `window.__mountXxxTab__ = __mountXxxTab__;`,否则 Task 8 rebuildContent 调用空白
2. **缺陷 2** (Task 8): 删除 Task 5 加的 Cmd/Ctrl+K `window.addEventListener`,在 `initTabManager()` 内统一注册

## 绝不要做
- 不 commit / push / release / reset / checkout / restore / 改主仓
- 不 bump 版本(T7 还是 1.4.2,T8 才 bump 1.5.0)
- 不启 sandbox feature
- 不删 5 + Settings 6 个 `.tab-pane` div(Task 9-11 通过 cloneNode 复用)
- 不动 Rust `.rs` 业务代码(`commands.rs` / `lib.rs` / `runner.rs` / `keyring_store.rs` / `llm_profiles.rs`)
- 不删旧 handoff / prompt

## 关键参考
- Plan:`docs/superpowers/plans/2026-07-24-w15-a-ux-redesign.md`(特别 §6.1 模块划分 + §6.2 后端零改动)
- Spec:`docs/superpowers/specs/2026-07-24-w15-a-ux-redesign-design.md` §2-9
- 加快模式规则:`handoff-w15-a-providers-decided-v142-icon-2026-07-24.md` §1 + §3
- 13 项验收:`docs/superpowers/specs/2026-07-24-w15-a-ux-redesign-design.md` §8
- Settings bug 修复情况:`handoff-w15-a-task1-bug-prereq-2026-07-24.md` §3(已知 capability allowlist 在 Tauri 2.11.5 不需要,只有 error handler + 强清缓存是修复手段)
- 命令清单:`grep -c "#\[tauri::command\]" src-tauri/src/commands.rs` 应得 17(6 LLM + 7 pipeline + 4 utility)

## Task 12(终)产物
- `target/release/bundle/nsis/media-to-doc_1.4.2_x64-setup.exe`(仍是 v1.4.2)
- handoff-w15-a-t7-1-{complete|blocked}-2026-07-24.md
- 不 commit(加快模式 → T8 release 会话统一提交)

## Session Health
- <2 hour 新会话 budget,撞墙立即写 handoff,新会话接力
- 每 task 行数变化:`task-brief` 脚本 extract brief 到 `.superpowers/sdd/task-N-brief.md`
- 每 task review diff 生成:`scripts/review-package BASE HEAD`(**W15-A 注意**:因为无 commit,需手动生成 working-tree diff 给 reviewer — 见 handoff §1 末尾说明)

## 第一步
1. 读本 prompt + handoff-sdd-w15a-tasks6-12-2026-07-24.md 全文件
2. 读 `.superpowers/sdd/progress.md` 看 5/12 已完成进度 + 时间账
3. `cd F:/soft/00selfmade/media-to-doc-ui && git status --short --branch` 确认状态
4. 跑 `/c/Users/Duanyi/.claude/skills/subagent-driven-development/scripts/task-brief <plan> 6` 提取 Task 6 brief
5. 派 implementer subagent(Sonnet)→ 等报告 → 派 reviewer(Haiku)→ 循环到 ✅
6. 重复 Task 7-12
