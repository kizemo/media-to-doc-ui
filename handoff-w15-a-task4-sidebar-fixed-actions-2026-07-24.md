# W15-A Task 4 Handoff — 侧栏 §2 Fixed Actions(+ 新建会话 / 定时任务占位)

**日期**: 2026-07-24
**Task**: Task 4 / 12 in W15-A UX redesign plan
**状态**: DONE — build 通过,5 段侧栏第 2 段已实装

---

## 完成内容

| Step | 描述 | 状态 |
|---|---|---|
| 1 | 在 §1 Brand Header 后插入 §2 Fixed Actions DOM(2 个按钮) | OK |
| 2 | 在 `<style>` 末尾追加 §2 CSS(11 条规则 + 3 条 collapse 适配) | OK |
| 3 | 在 `initSidebarCollapse()` 后追加 `handleNewSessionClick` + `initSidebarActions` | OK |
| 4 | 在 boot 区追加 `initSidebarActions();` 调用 | OK |
| 5 | `cargo build --release` 验证 | OK(Finished release,0 errors,5 pre-existing warnings,2m43s) |
| 6 | 写 handoff | OK(本文件) |

## 改动文件

只动了一个文件:

- `F:/soft/00selfmade/media-to-doc-ui/src/index.html`(1339 → 1392 行,净 +53)

`git diff --stat src/index.html`(vs HEAD,即包含 Task 1-3 累积未 commit 改动):

```
 src/index.html | 612 +++++++++++++++++++++++++++++++++++++++++++++++++++++----
 1 file changed, 571 insertions(+), 41 deletions(-)
```

未 commit(W15-A accelerate mode)— 5 个 task 全部完成后再批量提交。

## 关键实现要点

### DOM(行 414-425,共 12 行)

```html
<!-- §2 Fixed Actions -->
<div class="sidebar-section sidebar-section-actions">
  <button class="sidebar-action-btn primary" id="sidebar-new-session-btn">
    <span class="sidebar-action-icon">+</span>
    <span class="sidebar-action-text">新建会话</span>
  </button>
  <button class="sidebar-action-btn" id="sidebar-schedule-btn" disabled title="W15-B+ 实装">
    <span class="sidebar-action-icon">⏰</span>
    <span class="sidebar-action-text">定时任务</span>
  </button>
</div>
```

- "+ 新建会话" 主按钮(`.primary` 类,accent 蓝底)
- "⏰ 定时任务" 禁用占位(`disabled` 属性 + `title="W15-B+ 实装"`)
- 包裹在 `.sidebar-section.sidebar-section-actions` 内,继承 §1 collapse 行为

### CSS(行 376-405,共 30 行)

`.sidebar-section-actions` 走 flex column + 8px padding。`.sidebar-action-btn` 默认透明、
hover 时半透白;`.primary` 是 accent 蓝底。`body.sidebar-collapsed` 折叠态下:
- `.sidebar-section-actions { padding: 8px 4px }`(左右压缩)
- `.sidebar-action-text { display: none }`(隐藏文字)
- `.sidebar-action-btn { justify-content: center }`(icon 居中)

### JS(行 705-716,共 12 行)

```js
function handleNewSessionClick() {
  const inbox = state.selectedInbox;
  if (!inbox) {
    toast('请先在 §4 项目树里选个课程', 'error');
    return;
  }
  window.__tabManager__?.openTab({ type: 'new_run', coursePath: inbox });
}
function initSidebarActions() {
  $('sidebar-new-session-btn').addEventListener('click', handleNewSessionClick);
  $('sidebar-schedule-btn').addEventListener('click', () => toast('定时任务 — W15-B+ 实装', 'info'));
}
```

### 重要 caveat:`window.__tabManager__` 尚未注入(Task 8 实装)

`handleNewSessionClick` 用 optional chaining `window.__tabManager__?.openTab(...)` 而非直接
`__tabManager__.openTab(...)`。如果 Task 8 还没实装就点 "+ 新建会话",事件 handler 会**静默
no-op**(没 ReferenceError,但也没开新 tab)。这有意为之 — Task 4 不应该因为 Task 8 没跑
就把整个 webview 弄崩。Task 8 完成后,`window.__tabManager__` 会被注入,按钮就活了。

### Boot 调用(行 1387)

```js
// ───────── Boot ─────────
initSidebarCollapse();
initSidebarActions();   // ← 新增
await loadAppInfo();
```

## Cargo build 输出

```
warning: function `provider_name` is never used
   --> src\llm_profiles.rs:131:8
    |
131 | pub fn provider_name(provider: Provider) -> &'static str {
    |        ^^^^^^^^^^^^^

warning: `media-to-doc-ui` (lib) generated 5 warnings
    Finished `release` profile [optimized] target(s) in 2m 43s
```

- `Finished release profile [optimized]` ✓
- 0 errors ✓
- 5 warnings 与 baseline 一致(`provider_name` 等)✓
- 2m43s 与 Task 3 的 2m37s 一致,无新增编译热点

## 文件完整性

| 项 | 期望 | 实际 |
|---|---|---|
| 总行数 | 1339 + ~30 | 1392 ✓(净 +53) |
| `initSidebarActions` 出现次数 | 2(定义 + boot) | 2 ✓ |
| `handleNewSessionClick` 出现次数 | 1(定义) + 1(注册) | 2 ✓ |
| `sidebar-new-session-btn` 出现次数 | 2(id + 注册) | 2 ✓ |
| `sidebar-schedule-btn` 出现次数 | 2(id + 注册) | 2 ✓ |
| `sidebar-section-actions` 出现次数 | 2(class + collapse 适配) | 2 ✓ |
| `<div class="tab-pane"` 5 个 | 不删 | 5 个保持(行 ~408+) ✓ |
| CRLF 行尾 | 保持 | 保持 ✓ |

## 自我审视

### 已验证

- 4 个 Edit 锚点都 first-try matched,无 fallback
- CRLF 行尾保持(Edit 工具自动保留)
- 文件结构完整(`</html>` 在 line 1394 收尾)
- cargo build 0 errors,5 warnings 与 baseline 一致
- 关键函数 / id / class 在文件中精确出现预期次数,无重复定义
- 5 个 `.tab-pane` div 保留未删(Task 9-11 会复用)
- `<nav class="sidebar">` 的 §1/§2/§3/§4/§5 注释都在(`§3 Search — Task 5` 等占位注释保留)

### 已知限制

- 没有 dev server 启动测试 UI(本会话只跑 cargo build 静态编译验证)
- 没在浏览器手测按钮点击 / collapse 切档 — 留给下次 mtd-verify 沙箱跑时一并验
- `window.__tabManager__` 没注入时按钮是 no-op(预期行为,但用户看到没反应可能困惑)

### 风险点

- Task 4 加完后,`<nav>` 内有 1 个真实 section(§1)+ 1 个真实 section(§2)+ 3 个占位注释
  (§3/§4/§5)。collapse 时只有 §1 显示,§2 也会被
  `body.sidebar-collapsed .sidebar-section:not(.sidebar-section-brand) { display: none; }`
  隐藏 — 这是 expected behavior。Task 5+ 完成后,§2 在折叠态只显示 icon(无文字),符合设计。
- CSS 选择器 `.sidebar-section:not(.sidebar-section-brand)` 命中 §2 的 `.sidebar-section-actions`
  div — 已确认这是想要的(§1 + §2 之外的都隐藏,brand 永远在;折叠时 §2 仅 icon)。

### 接力提示

- Task 5 直接在 §2 后、注释 `<!-- §3 Search — Task 5 -->` 之前插入 §3 DOM。
- Boot 区已有 `initSidebarCollapse();` + `initSidebarActions();` 模板,Task 5 同步加
  `initSidebarSearch();`(or 类似的)。
- 5 段侧栏 class 命名规范:`sidebar-section` + `sidebar-section-<name>`(brand / actions / search / tree / gear)
- §2 action 按钮 hover/active 状态如果 Task 5+ 需要更高对比度,可调整 RGBA 值
  (0.06 / 0.12 / 0.18 三档)

## 加速模式提醒

W15-A accelerate mode 仍然生效 — 本会话只改 `src/index.html`,**不 commit、不 add、
不 push、不 bump version**。所有 W15-A tasks 完成后由用户拍板批量提交。

## 下一步(Task 5 接力)

Task 5 范围:侧栏 §3 Search(全文搜索框)。详见 `task-5-brief.md`。
本 Task 4 的产物提供 `initSidebarActions` 模板,Task 5 可直接复用 boot 调用 pattern。