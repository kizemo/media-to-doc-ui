# W15-A Task 3 Handoff — 侧栏 §1 Brand Header + collapse 行为

**日期**: 2026-07-24
**Task**: Task 3 / 12 in W15-A UX redesign plan
**状态**: DONE — build 通过,5 段侧栏第 1 段已实装

---

## 完成内容

| Step | 描述 | 状态 |
|---|---|---|
| 1 | 在 `<nav class="sidebar">` 内插入 §1 Brand Header DOM | OK |
| 2 | 在 `<style>` 末尾追加 5 段侧栏 CSS + collapse 样式 | OK |
| 3 | 在 `state = {...}` 后追加 `toggleSidebarCollapse` + `initSidebarCollapse` 函数 | OK |
| 4 | 在 boot 区追加 `initSidebarCollapse();` 调用 | OK |
| 5 | `cargo build --release` 验证 | OK(Finished release,0 errors,5 pre-existing warnings) |

## 改动文件

只动了一个文件:

- `F:/soft/00selfmade/media-to-doc-ui/src/index.html`(1288 → 1339 行,+51)

`git diff --stat src/index.html`:`+518 / -41`(包含 Task 2 未 commit 的累积改动)

未 commit(W15-A accelerate mode)— 5 个 task 全部完成后再批量提交。

## 新增 DOM

```html
<nav class="sidebar" id="sidebar">
  <!-- §1 Brand Header -->
  <div class="sidebar-section sidebar-section-brand">
    <span class="sidebar-brand-icon">📦</span>
    <span class="sidebar-brand-text">media-to-doc</span>
    <button class="sidebar-collapse-btn" id="sidebar-collapse-btn" title="折叠侧栏">⮜</button>
  </div>
  <!-- §2 Fixed Actions — Task 4 -->
  <!-- §3 Search — Task 5 -->
  <!-- §4 Project Tree — Task 6 -->
  <!-- §5 Settings Gear — Task 7 -->
</nav>
```

## 新增 CSS

`.sidebar-section` / `.sidebar-section-brand` / `.sidebar-brand-icon` /
`.sidebar-brand-text` / `.sidebar-collapse-btn` +
`body.sidebar-collapsed` 三条规则(收起时折叠 text、其他 section 隐藏、品牌栏居中)。

## 新增 JS

```js
function toggleSidebarCollapse() {
  const collapsed = document.body.classList.toggle('sidebar-collapsed');
  try { localStorage.setItem('mediaToDocSidebarCollapsed', collapsed ? '1' : '0'); } catch (_) {}
  const btn = $('sidebar-collapse-btn');
  if (btn) btn.textContent = collapsed ? '⮞' : '⮜';
}
function initSidebarCollapse() {
  let collapsed = false;
  try { collapsed = localStorage.getItem('mediaToDocSidebarCollapsed') === '1'; } catch (_) {}
  if (collapsed) {
    document.body.classList.add('sidebar-collapsed');
    const btn = $('sidebar-collapse-btn');
    if (btn) btn.textContent = '⮞';
  }
  const btn = $('sidebar-collapse-btn');
  if (btn) btn.addEventListener('click', toggleSidebarCollapse);
}
```

在 boot 区(`// ───────── Boot ─────────` 下)追加 `initSidebarCollapse();` 调用,
位于 `await loadAppInfo();` 之前。

## Build 验证

```
warning: function `provider_name` is never used
   --> src\llm_profiles.rs:131:8
warning: `media-to-doc-ui` (lib) generated 5 warnings
    Finished `release` profile [optimized] target(s) in 2m 37s
```

0 errors,5 pre-existing warnings(与 baseline 一致:all_templates / provider_name 等)。

## 下一步必交付

继续 **Task 4 — 侧栏 §2 Fixed Actions**(新建 / Run / Pause / Resume / Cancel 按钮)。

预计改动:

- DOM:在 §1 下方插入 §2 段落
- CSS:`.sidebar-section-fixed` / `.sidebar-action-btn` / `.sidebar-action-icon` / `.sidebar-action-label`
- JS:`initFixedActions()` + `startNewPipeline()` 等 5 个按钮 handler

## 加速模式规则(W15-A)

- 不 commit / add / push(整个 W15-A 完成后批量提交)
- 不 bump version
- 不改 Rust .rs 文件
- 不动主仓 `F:/soft/00selfmade/media-to-doc/`
- 不删 `<main>` 内的 5 个 `<div class="tab-pane">`(Task 9-11 复用)
- 不创建新文件

## 避坑提示

1. **`<style>` 末尾插入位置**:用 `.provider-modal-test-result.error { color: var(--red); }` + `</style>` 作为锚点(原文件行 349-350),确保不会被前面的 CSS 规则匹配掉。
2. **JS 函数插入位置**:用 `jumpDisabled: new Set(),    // ...\n    };\n\n    // ───────── Toast ─────────` 作为唯一锚点,避免误插到其他区域。
3. **Boot 顺序**:`initSidebarCollapse();` 在 `await loadAppInfo();` 之前调用,
   因为 localStorage 同步读,UI 立刻反映收起状态,不等 async。
4. **CRLF 保持**:文件 CRLF,所有 Edit 用工具默认换行(自动保留),不需要手动处理。

## 必读顺序

新会话开 Task 4 前,先读:

1. `F:/soft/00selfmade/media-to-doc-ui/.superpowers/sdd/task-4-brief.md`
2. 本文件确认 Task 3 已落地
3. `git diff src/index.html` 确认 working tree 状态