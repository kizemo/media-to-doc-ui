# Handoff — W15-A Task 2：旧结构已删除

**日期:** 2026-07-24  
**工作区:** `F:\soft\00selfmade\media-to-doc-ui`  
**分支:** `feat/w15a-llm-api-settings`  
**状态:** working tree，未 commit

## 已完成

仅针对 `src/index.html` 完成 Task 2 的旧结构清理：

1. 删除旧蓝色 `<header>` markup（标题、version badge、status dot/text）。
2. 清空旧 `<nav class="sidebar">` 中的 6 个旧 nav items：Inbox、Run、Output、Health、Learn、Settings。
3. 保留 sidebar 外壳并加 `id="sidebar"`，放入 Task 3-7 的占位注释。
4. 将 body grid 从 `200px + 48px header row` 改为 `260px sidebar + 1fr main`，增加 `body.sidebar-collapsed` 的 `48px` 侧栏布局与 transition。
5. 删除旧 `.nav-item` click handler，替换为 Task 8 `tabManager` 接管说明。

## 明确保留

- `<main>` 内 6 个 `.tab-pane` 没有删除：
  `tab-inbox`、`tab-run`、`tab-output`、`tab-health`、`tab-learn`、`tab-settings`。
- 后续 Task 9-11 仍可通过 cloneNode / 新 tabManager 复用这些 tab contents。
- `switchTab`、`maybeJumpToOutput` 中旧 `.nav-item` 查询及 `.nav-item` CSS 暂未清理；这是后续 tabManager 重构需要处理的规划点，不属于本 Task 的 exact deletion 范围。
- 现有 Task 1 module-level error handler 与此前 W15-A Settings 代码未被本 Task 改动。

## 验证证据

- `git diff --check`：通过。
- `src/index.html` 的 `.tab-pane` 数量：6。
- `<header>` markup 与旧 nav-item markup：已不存在。
- `cargo build --release --no-run`：Cargo 1.97.1 报 `unexpected argument '--no-run'`，这是 brief 命令错误，不是代码编译错误。
- 替代验证 `cargo build --release`：通过，`Finished release profile [optimized]`，0 errors，生成 5 个 warning。

## 下一步必交付

- Task 3-7 在保留的 `<nav id="sidebar">` 中重建 Claude-Code-style sidebar。
- Task 8-11 实装新的 tabManager / tab 内容挂载，并替换仍依赖 `.nav-item` 的旧 helper。
- 之后执行父任务要求的完整 `cargo tauri build`、sandbox 安装验证与 13 项验收。

## Planning concerns

- 当前运行完成自动跳 Output 的逻辑仍通过 `.nav-item.active` 判断；旧 nav DOM 已不存在，因此新 tabManager 必须提供等价的 active-tab 状态或更新该逻辑。
- body 的 `header { grid-area: header; }` 与 `.nav-item` CSS 尚存。Task 2 brief 未要求删除它们，建议在新侧栏结构稳定后再做定向清理，避免提前影响后续任务。
- 本次未修改任何 Rust `.rs` 文件、版本号或主仓；没有执行任何 Git 写操作。

## 必读文件

1. `F:\soft\00selfmade\media-to-doc-ui\.superpowers\sdd\task-2-report.md`
2. `F:\soft\00selfmade\media-to-doc-ui\src\index.html`
3. `F:\soft\00selfmade\media-to-doc-ui\.superpowers\sdd\task-2-brief.md`

