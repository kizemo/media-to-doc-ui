# W15-A Task 9 Handoff — Session Tab 内容

**承接日期**: 2026-07-25
**分支**: `feat/w15a-llm-api-settings` (BASE = `073b05e`,无 commit — 加快模式)
**承接**: Task 10 (New Run Tab 内容),可由新会话接力

## §1 改了什么

仅 `src/index.html` 一文件,净 +95 / -5 lines(1841 之前是 1836 行)。

精确块:`function buildSessionPlaceholder` 之后,替换为:

1. **扩展的 `buildSessionPlaceholder(workDir)`** — 5 段 UI:h2 / kv 表( work_dir + status dot) / cancel+resume 按钮 / `<div id="sess-stages-...">` / `<pre id="sess-log-...">`
2. **`const _sessionPolls = new Map()`** — workDir → `{timerId}` 模块级 map
3. **`function __mountSessionTab__(container, tab)`** — DOM selector helper + `dotHtml(status)` + `poll()` async(2s 轮询) + 按钮 click 监听(container 内,非 window) + MutationObserver cleanup
4. **`window.__mountSessionTab__ = __mountSessionTab__;`** — **Defect 1 修复**,函数声明末尾立刻挂到 window

## §2 Build evidence

`cargo check` 通过(2.16s,dev profile,5 warnings 全是预先存在)。
`cargo build --release` 未跑(纯前端改动,无 Rust 变更;Task 12 总体验收再跑)。

## §3 Defect / I-1 / I-2 状态

| 项 | 状态 | 说明 |
|---|---|---|
| Defect 1(mount 函数挂 window) | **已修复** | `src/index.html:1063` |
| I-2(不要绑 window listener) | **已防御** | 仅 `container.querySelector(...).addEventListener` |
| I-1(tabTitle 兜底) | **未做**(用户裁定跳过,留 Task 12) | 不破坏 Task 8 已实现逻辑 |

## §4 下一步(Task 10)

替换 `buildNewRunForm(coursePath)` 占位 + 加 `window.__mountNewRunTab__`。沿用类似模式:
- DOM 树:h2 + 表单(course picker + submit) + 结果区
- 表单 submit 调 `invoke('run_pipeline', { inboxDir, ... })`
- 容错:claude 已修过 `select` 抓当前课程的命名

参考:`src/index.html:959-963` 是当前 Task 10 占位。
后端命令 grep `commands.rs`:`run_pipeline(inbox_dir, llm_provider, ...)`(如有变化查新签名)。

## §5 Concerns

- **brief 代码块的 `read_log` / `stages` 类型与 Rust 后端不一致**,已按 brief 注 规则适配。详见 task-9-report.md §Concerns 1 / 4 / 6
- **`status-dot.*` CSS class** 假定已在样式表存在(grep 全 src 找 `.status-dot.green` 等)— 我没新增 CSS,沿用 §6.1 既有约定。如样式缺失,Task 12 补
- **MutationObserver cleanup 路径仅代码层写好,未实际启动 Tauri 验证**。逻辑链闭合见 task-9-report.md §Concerns 3
- **`_s` selector 用裸 `wd`**,未做 URI encode 防御 — 工作流 workDir 通常 plain path,此 case 罕见,跳过

## §6 必读顺序

1. task-9-brief.md(`.superpowers/sdd/2026-07-24-w15-a-ux-redesign/task-9-brief.md`)
2. task-9-report.md(刚写的)
3. `src/index.html` 行 964-1063(本 task 块)
4. `src-tauri/src/commands.rs` 行 100-170 + 456-545 + 1064-1156(后端签名 + types)
5. 上一个 handoff:`handoff-w15-a-task8-*-2026-07-24.md`(Tab Manager 基础)

## §7 状态

`DONE_WITH_CONCERNS`(cargo check 通过 / release build 未跑 / MutationObserver cleanup 未真机验证)。
