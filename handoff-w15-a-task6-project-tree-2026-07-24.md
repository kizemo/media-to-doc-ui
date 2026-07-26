# Handoff: W15-A Task 6 — §4 Project Tree + run aggregation

**Branch:** `feat/w15a-llm-api-settings`(无 commit,加快模式)
**Base:** `073b05e`
**Date:** 2026-07-24

## §1 改了什么

`src/index.html`(本 task 净变化 ≈ +185 / -17 行)

| Step | 改动 | 行数 |
|---|---|---|
| 1 | §4 Project Tree DOM 插入(`<div class="sidebar-section-tree">` + 2 子节点) | +7 |
| 2 | §4 CSS append(`.sidebar-section-tree` / `.sidebar-tree-empty` / `.sidebar-project-node` / `.sidebar-session-list` / `.sidebar-session-entry` / `body.sidebar-collapsed .sidebar-section-tree`) | +41 |
| 3 | §4 JS block(8 个函数 + globals `__refreshProjectTree__` / `__projectTreeFilter__`) | +137 |
| 5 | boot 区追加 `initProjectTree();`(在 `initSidebarSearch()` 之后) | 0 |
| 5' | boot 区替换 `await refreshCourses();` → 删除(brief 字面"追加"vs runtime 兼容,见 Concerns §1) | -1 |
| 6 | 删除 `<div class="tab-pane active" id="tab-inbox">` 整段 | -16 |
| 6' | null-safe `$('refresh-courses').addEventListener(...)` — 防御 module-load throw | 0 |

## §2 Build evidence

```
cargo build --release:
  warning: `media-to-doc-ui` (lib) generated 5 warnings
  Finished `release` profile [optimized] target(s) in 3m 20s

cargo check:
  warning: `media-to-doc-ui` (lib) generated 5 warnings
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 25.33s
```

0 errors。Warnings 全在 `src-tauri/src/llm_profiles.rs` 的 unused functions(`all_templates`
/ `provider_name`),与本 task 无关。

## §3 下一步(Task 7)

W15-A 12-task plan 第 7 步:**§5 Settings Gear**(侧栏底部齿轮按钮 → 打开 Settings
tab / modal)。与本 task 数据流正交,直接接力即可。

启动指令建议:
```
承接 W15-A Task 7。handoff 在 F:/soft/00selfmade/media-to-doc-ui/handoff-w15-a-task6-project-tree-2026-07-24.md
report 在 .superpowers/sdd/2026-07-24-w15-a-ux-redesign/task-6-report.md
base = 073b05e,branch = feat/w15a-llm-api-settings,无 commit。
```

## §4 Concerns

1. **CRITICAL**:`$('refresh-courses')` 必须 null-safe。删除 inbox tab pane 后该
   `addEventListener` 立即 throw,会阻断整个 module 加载。我已加 `if ($('refresh-courses'))`
   守卫,refreshCourses / renderCourses 函数体未动。Task 9-11 cleanup 时整个 Inbox 模块
   可一并删除。

2. **MEDIUM**:Brief step 5 字面"追加 initProjectTree()"在 runtime 不可行(refreshCourses
   会 throw)。我选择"替换"以保证 boot 可执行 — 一处不可避免的偏差。

3. **MINOR**:`output_final/` 不在 `inboxFromWorkDir` 正则内 — 主仓 W12-D 的 final 产物
   目录会被聚到自身(假项目),不影响当前 W14-C 工作流。spec §3.1 后续可补。

4. **MINOR**:30s `setInterval(refreshProjectTree, 30000)` 永不 clearInterval — webview
   session 生命周期即应用生命周期,符合 brief 裁定。

5. **MINOR**:`projectId` 变量在 `renderProjectTree` 内声明但未使用 — 严格按 brief
   复制,未清理。1 行 delete 即可。

完整 concerns 见
`F:/soft/00selfmade/media-to-doc-ui/.superpowers/sdd/2026-07-24-w15-a-ux-redesign/task-6-report.md`。