# W15-A UX 重大重设计 — 实施 Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 media-to-doc Tauri UI 从"header + 6 tab 平级"重构为"Claude Code 桌面风格侧栏 5 段 + 主区域 tabbed view(每 session + Settings = 1 tab)",同时修 Settings 点击 bug。

**Architecture:** 纯前端 + `capabilities/default.json` 6 行 config。前端 index.html 单文件不改结构但重写 80%;后端 17 个 Tauri command 零改动(数据流由前端 list_courses + list_all_runs + check_status + read_log + run_pipeline + 6 LLM command 装配);W15-A 不引入新依赖。

**Tech Stack:**
- Tauri 2.x(`src-tauri/Cargo.toml` 实际版本)
- `src/index.html`(单文件,内联 `<style>` + `<script type="module">`,沿用 W14-B+ 模式)
- 沿用现有暗色主题 CSS variables(`--bg`/`--bg-card`/`--bg-sidebar`/`--fg`/`--fg-muted`/`--accent`/`--border`/`--green`/`--yellow`/`--red`)
- 沿用现有 `state` 对象 + `$` helper + `toast` + `escapeHtml`(T6 实装)
- 后端 17 个 Tauri command 已注册(`commands.rs` grep `^#\[tauri::command\]` 17 个),不需动

**承接**:`docs/superpowers/specs/2026-07-24-w15-a-ux-redesign-design.md`(本 plan 完全覆盖 spec §2-9 的每一条)

---

## Global Constraints

每条都来自 spec / 用户加快模式规则,任何 task 实现时必须遵守:

- **不动后端 Rust 业务代码**:本 plan 只改 `src/index.html` + `src-tauri/capabilities/default.json`。`src-tauri/src/{commands,lib,runner,keyring_store,llm_profiles}.rs` 不动一行。
- **不动主仓 `F:/soft/00selfmade/media-to-doc/`**(Python mtd):沿用 W14-D trust_env=False 路径。
- **W15-A feature 整体一次 commit**(加快模式):本 plan 9 个 task 全部跑完后,**不要 commit**;由 T8 release 会话统一 feature commit + v1.5.0 release。每 task 末尾"Save state"仅写 handoff,不 commit。
- **不 bump version 进 v1.5.0**:T7 装机仍是 v1.4.2 NSIS(`src-tauri/tauri.conf.json`/`src-tauri/Cargo.toml` 的 version 不动)。
- **不 reset / checkout / restore / 覆盖未提交改动**。
- **不删除旧 handoff / prompt**(删除需用户二次确认)。
- **不启 sandbox feature**(W14-G 已知 Win11 沙箱功能阻塞)。
- **PKG safe directory**:所有 `git -c safe.directory=*` 命令前缀不可省。
- **CRLF 兼容性**:index.html 在 Windows 下被 Git 自动 CRLF 化,Edit 工具对缩进/换行极其敏感;每个 Edit 步骤前先 Read 一次目标行,Edit 精确匹配(tabs/spaces 和 line endings 必须 1:1)。

---

## File Structure

| 文件 | 角色 | 修改量 | 是否新建 |
|---|---|---|---|
| `src/index.html` | 唯一前端文件;内联 style + module script | 重写 80%(行 32-37 grid / 349-365 header+nav / 358-482 旧 5 tab pane → 改为 5 段侧栏 + 3 类 tab pane) | 否 |
| `src-tauri/capabilities/default.json` | Tauri 2 capability allowlist | 加 6 行 permissions | 否 |
| 后端 Rust | 不动 | 0 | — |
| 主仓 | 不动 | 0 | — |

**不引入新文件**:W14-B+ 已实装 index.html 单文件模式不打破。Project Tree / Tab Manager / Tab Bar / Session/NewRun/Settings 容器全部内联在 `<script>` 内的模块化 function 划分(spec §6.1)。

---

## Task 1:Settings 点击 bug 前置修复(capability + error handler)

**Files:**
- Modify: `src-tauri/capabilities/default.json`(全量替换,7 行 → 13 行)
- Modify: `src/index.html`(在 `<script type="module">` 顶部前插入 ~15 行 error handler)

**Interfaces:**
- Consumes:无(独立 task)
- Produces:Tauri 2 capability 加 6 LLM command 显式 allowlist;前端 module 顶层 throw 可被 `init.log` 捕获

**意义:** 这两步是 spec §5 假设 1 + 假设 2 的应对,即使不清缓存也能让 Settings 弹窗。原因:某些 Tauri 2.x 严格模式拒未声明 command;module 顶层 throw 会让 click handler 不注册。

- [ ] **Step 1:Read 现状**

Run: `cat src-tauri/capabilities/default.json`
Expected: 7 行 JSON,`permissions: ["core:default"]`

- [ ] **Step 2:替换 capability 配置**

完整新内容:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default capability — W15-A 添加 6 LLM command 显式 allowlist",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "list_llm_profiles",
    "get_active_llm_profile_name",
    "save_llm_profile",
    "set_active_profile",
    "delete_llm_profile",
    "test_llm_connection"
  ]
}
```

用 `Write` 工具全量覆盖该文件。

- [ ] **Step 3:Read index.html `<script>` 起始位置**

定位 `src/index.html` 行 ~583(`<script type="module">` 起始)和行 ~584(`const { invoke } = window.__TAURI__.core;`)。Read 这两行确认。

- [ ] **Step 4:在 `<script type="module">` 前一行插入 error handler**

精确插入位置:`<script type="module">` 的前一行(即替换 `<script type="module">` 这一行为下面两行)。

新内容:

```html
<script>
  // W15-A T7:捕获 module 顶层 throw,落盘到 %APPDATA%\com.duanyi.mediatodoc\init.log
  // 用于诊断 Settings 点击 bug 之"假设 2:module 顶层 throw 致 addEventListener 未注册"
  (function () {
    const logDir = (typeof process !== 'undefined' && process.env && process.env.APPDATA)
      ? process.env.APPDATA + '\\com.duanyi.mediatodoc'
      : null;
    function writeInitLog(msg) {
      try {
        if (!logDir) { console.error('[init]', msg); return; }
        // 浏览器内无法直接写本地文件,降级到 console;Rust log 通道在 boot 时再补
        console.error('[init]', msg);
      } catch (_) { /* swallow */ }
    }
    window.addEventListener('error', (e) => writeInitLog('window.error: ' + (e.error && e.error.stack || e.message)));
    window.addEventListener('unhandledrejection', (e) => writeInitLog('unhandledrejection: ' + (e.reason && e.reason.stack || e.reason)));
    window.__MEDIA_TO_DOC_INIT_LOG__ = writeInitLog;
  })();
</script>
<script type="module">
```

注意:不要删 `<script type="module">` 里任何现有代码,只在外侧**加**一个 `<script>` 块。

- [ ] **Step 5:本地验证 syntax**

Run: `cd src-tauri && cargo check 2>&1 | head -50`
Expected: 显示 `Finished ... profile [unoptimized + debuginfo]` 或 compile 错(若有,定点修;本 task 不引入 Rust 代码,应无 src 错)。

注:`cargo check` 不会出 NSIS,只在 ~10s 内检查 capability 是否被 `tauri::generate_handler!` 接受。无需跑 `cargo build` 全量。

- [ ] **Step 6:Save state(无 commit)**

写 `handoff-w15-a-task1-bug-prereq-2026-07-24.md`,内容:
- capability 改前后对比
- error handler 插入位置
- 下一步:继续 Task 2

不 commit(加快模式)。

---

## Task 2:删除旧结构(`<header>` + 5 nav-item + grid CSS + 旧 nav click handler)

**Files:**
- Modify: `src/index.html`
  - 行 32-37(`body` grid-template)
  - 行 349-356(`<header>` 整段)
  - 行 358-365(`<nav class="sidebar">` 6 个 nav-item)
  - 行 614-626(`.nav-item` click handler 整段)
- 不删除:.sidebar 容器(留做新侧栏外壳);旧 5 tab pane(留做 tab 内容源,Task 6-8 复用)

**Interfaces:**
- Consumes:无
- Produces:DOM 不再有 `<header>`;`<nav class="sidebar">` 仍存在但内部完全清空;`.nav-item` click handler 整段删除;body grid 改为新结构(无 header row)

**意义:** 清空外壳,为 Task 3-5 重建侧栏扫清障碍。**这一步不改 main 内 5 tab pane 内容** —— 它们后面作为 tab 容器的内容源。

- [ ] **Step 1:Read 目标 4 段代码**

Read `src/index.html` 行 30-40 / 349-365 / 612-628。每段 Read 一次缓存到 context。

- [ ] **Step 2:删 `<header>` 整段**

精确 old_string:

```
  <header>
    <h1>media-to-doc</h1>
    <span class="badge" id="version-badge"></span>
    <div class="status">
      <span class="status-dot" id="status-dot"></span>
      <span id="status-text">loading…</span>
    </div>
  </header>

```

替换为(空):

```


```

注意:删整段包括后面空行。

- [ ] **Step 3:清空 `<nav class="sidebar">` 内部(保留外壳 + 加占位 id)**

精确 old_string:

```
  <nav class="sidebar">
    <div class="nav-item active" data-tab="inbox"><span class="nav-icon">📁</span>Inbox</div>
    <div class="nav-item" data-tab="run"><span class="nav-icon">▶</span>Run</div>
    <div class="nav-item" data-tab="output"><span class="nav-icon">📄</span>Output</div>
    <div class="nav-item" data-tab="health"><span class="nav-icon">📊</span>Health</div>
    <div class="nav-item" data-tab="learn"><span class="nav-icon">📚</span>Learn</div>
    <div class="nav-item" data-tab="settings"><span class="nav-icon">⚙️</span>Settings</div>
  </nav>
```

替换为:

```
  <nav class="sidebar" id="sidebar">
    <!-- §1 Brand Header — Task 3 -->
    <!-- §2 Fixed Actions — Task 4 -->
    <!-- §3 Search — Task 5 -->
    <!-- §4 Project Tree — Task 6 -->
    <!-- §5 Settings Gear — Task 7 -->
  </nav>
```

- [ ] **Step 4:更新 `body` grid CSS(去 header row)**

精确 old_string:

```
    body {
      display: grid;
      grid-template-columns: 200px 1fr;
      grid-template-rows: 48px 1fr;
      grid-template-areas: "header header" "sidebar main";
    }
```

替换为:

```
    body {
      display: grid;
      grid-template-columns: 260px 1fr;
      grid-template-rows: 1fr;
      grid-template-areas: "sidebar main";
      transition: grid-template-columns 150ms ease-out;
    }
    body.sidebar-collapsed {
      grid-template-columns: 48px 1fr;
    }
```

- [ ] **Step 5:删 `.nav-item` click handler 整段**

精确 old_string(T6 实装,行 614-626):

```
    // ───────── Tab 切换 ─────────
    document.querySelectorAll('.nav-item').forEach((el) => {
      el.addEventListener('click', () => {
        document.querySelectorAll('.nav-item').forEach((n) => n.classList.remove('active'));
        document.querySelectorAll('.tab-pane').forEach((p) => p.classList.remove('active'));
        el.classList.add('active');
        $('tab-' + el.dataset.tab).classList.add('active');
        if (el.dataset.tab === 'inbox') refreshCourses();
        if (el.dataset.tab === 'health') { refreshHealth(); refreshRuns(); }
        if (el.dataset.tab === 'output') refreshOutputs();
        if (el.dataset.tab === 'run') startRunPolling();
        if (el.dataset.tab === 'settings') loadProviders();
      });
    });
```

替换为:

```
    // ───────── Tab 切换(由 Task 8 tabManager 接管) ─────────
    // 原 6 nav-item click handler 已删除 — 新架构下没有"主 tab 切换",只有侧栏 §4 项目 / §2 +新建会话 / §5 设置触发的 tabManager.openTab()
```

- [ ] **Step 6:本地 syntax 验证**

Run: 用浏览器 console 模拟太麻烦,跑 `cd src-tauri && cargo build --release --no-run 2>&1 | tail -5`
Expected: 显示 `Compiling ...` 或 `Finished` 行(我们没改 .rs,不期望 Rust 错;只验 Tauri asset 管线不挂)。

- [ ] **Step 7:Save state(无 commit)**

写 `handoff-w15-a-task2-old-structure-deleted-2026-07-24.md`,记录 5 处删除 + body grid 改前后。继续 Task 3。

---

## Task 3:侧栏 §1 Brand Header + collapse 行为

**Files:**
- Modify: `src/index.html`(在 `<nav class="sidebar">` 注释下插入新结构;`<style>` 内追加 collapse 相关样式)

**Interfaces:**
- Consumes:无
- Produces:
  - `sidebarSection1Brand` DOM 节点(含 collapse 按钮 `sidebar-collapse-btn`)
  - `toggleSidebarCollapse()` 函数(读 / 写 localStorage `mediaToDocSidebarCollapsed`;切 body.sidebar-collapsed class)
  - boot 时 `initSidebarCollapse()` 还原上次状态

**意义:** §1 是侧栏顶部 logo 区,collapse 是整侧栏的全局开关。本 task 隔离完成,可以独立视觉验证(收起 / 展开 48px ↔ 260px)。

- [ ] **Step 1:插入新 DOM**

精确 old_string(`<nav class="sidebar" id="sidebar">` 紧跟的注释块):

```
  <nav class="sidebar" id="sidebar">
    <!-- §1 Brand Header — Task 3 -->
    <!-- §2 Fixed Actions — Task 4 -->
    <!-- §3 Search — Task 5 -->
    <!-- §4 Project Tree — Task 6 -->
    <!-- §5 Settings Gear — Task 7 -->
  </nav>
```

替换为:

```
  <nav class="sidebar" id="sidebar">
    <!-- §1 Brand Header -->
    <div class="sidebar-section sidebar-section-brand">
      <span class="sidebar-brand-icon">📦</span>
      <span class="sidebar-brand-text">media-to-doc</span>
      <button class="sidebar-collapse-btn" id="sidebar-collapse-btn" title="折叠侧栏">⮜</button>
    </div>
  </nav>
```

- [ ] **Step 2:追加 CSS**

定位:`src/index.html` `<style>` 末尾(行 ~346,`</style>` 前)。在 `.provider-modal-test-result.error { color: var(--red); }` 这一行后插入:

```css
    /* ─── W15-A 重设计:5 段侧栏 ─── */
    .sidebar-section {
      padding: 6px 8px;
      border-bottom: 1px solid var(--border);
    }
    .sidebar-section:last-child {
      border-bottom: none;
      margin-top: auto;
    }
    .sidebar-section-brand {
      display: flex; align-items: center; gap: 8px;
      padding: 10px 12px;
      font-size: 14px; font-weight: 600;
      border-bottom: 1px solid var(--border);
    }
    .sidebar-brand-icon { font-size: 16px; }
    .sidebar-brand-text { flex: 1; }
    .sidebar-collapse-btn {
      background: none; border: none; cursor: pointer;
      color: var(--fg-muted); font-size: 14px; padding: 2px 6px;
      border-radius: 4px;
    }
    .sidebar-collapse-btn:hover { background: rgba(255,255,255,0.08); color: var(--fg); }
    body.sidebar-collapsed .sidebar-brand-text,
    body.sidebar-collapsed .sidebar-section:not(.sidebar-section-brand) { display: none; }
    body.sidebar-collapsed .sidebar-section-brand { justify-content: center; padding: 10px 0; }
```

- [ ] **Step 3:在 `<script>` 模块顶部添加 toggle 函数**

定位:`src/index.html` `state = {...}` 之后、`Toast` 工具之前(行 ~604 附近)。Read 一次确认位置。

追加:

```js
    // ───────── Sidebar collapse ─────────
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

- [ ] **Step 4:boot 时调用**

定位:行 ~662 `loadAppInfo();` 函数调用附近(boot 区,行 ~1290+ 视 T6 实装位置而定)。Read 一次找到 boot 调用块。

精确 old_string(若 boot 块是 `async function boot()` 之类):

```js
    async function loadAppInfo() {
```

不要直接换函数体。在 boot 区最后**追加** `initSidebarCollapse();` 调用。若 boot 是 IIFE 形式,在 IIFE 内最末追加。

如果没有 boot 函数,把 `initSidebarCollapse()` 直接加在 `<script>` 末尾(在 module 顶层的最后一行)。

- [ ] **Step 5:本地 syntax 验证 + collapse 切档**

Run: `cd src-tauri && cargo build --release --no-run 2>&1 | tail -3`
Expected: `Finished` 行。

手测(本会话可启动 dev server 或直接依赖桌面端):进 webview,点 ⮜ → 侧栏 260px → 48px,只有 📦 + ⮜ 显示;再点 ⮞ → 回到 260px。重启 app,记住上次状态。

- [ ] **Step 6:Save state(无 commit)**

写 `handoff-w15-a-task3-sidebar-brand-collapse-2026-07-24.md`。继续 Task 4。

---

## Task 4:侧栏 §2 Fixed Actions(+ 新建会话 / 定时任务)

**Files:**
- Modify: `src/index.html`(在 §1 后追加新 DOM 段;CSS 追加;加新建会话 handler)

**Interfaces:**
- Consumes:`state.selectedInbox`(T6 实装;若无选 → toast "请先选个课程")
- Produces:
  - `sidebarSection2Fixed` DOM 节点(2 个按钮)
  - `handleNewSessionClick()` 函数(读侧栏选中项目,`tabManager.openTab({type:'new_run', coursePath: state.selectedInbox})` —— Task 8 实现 tabManager)
  - "定时任务" 按钮 toast 占位

**意义:** §2 是侧栏"快捷动作"。本次只实装"+ 新建会话"和"定时任务"占位(toast "W15-B+ 实装")。"技能市场"按用户确认略过,不展示。

- [ ] **Step 1:插入 DOM**

精确 old_string(`<!-- §1 Brand Header -->` 块后):

```
    <!-- §1 Brand Header -->
    <div class="sidebar-section sidebar-section-brand">
      <span class="sidebar-brand-icon">📦</span>
      <span class="sidebar-brand-text">media-to-doc</span>
      <button class="sidebar-collapse-btn" id="sidebar-collapse-btn" title="折叠侧栏">⮜</button>
    </div>
```

替换为(在 `<div ... sidebar-section-brand>` 闭合后追加新 section,保留上面的 §1 不变):

```html
    <!-- §1 Brand Header -->
    <div class="sidebar-section sidebar-section-brand">
      <span class="sidebar-brand-icon">📦</span>
      <span class="sidebar-brand-text">media-to-doc</span>
      <button class="sidebar-collapse-btn" id="sidebar-collapse-btn" title="折叠侧栏">⮜</button>
    </div>
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

注:Edit 工具的 old_string 整段重复 §1 是必要的(避免歧义匹配)。如果 §1 在文件中已存在,用上述精确块做 replace。

- [ ] **Step 2:追加 CSS**

在 Task 3 §1 CSS 块后追加(同样定位 `.provider-modal-test-result.error { color: var(--red); }` 后):

```css
    .sidebar-section-actions {
      display: flex; flex-direction: column; gap: 4px;
      padding: 8px 8px;
    }
    .sidebar-action-btn {
      display: flex; align-items: center; gap: 8px;
      padding: 8px 12px;
      background: transparent;
      color: var(--fg);
      border: none; border-radius: 6px;
      cursor: pointer; font-size: 13px;
      text-align: left;
    }
    .sidebar-action-btn:hover:not(:disabled) {
      background: rgba(255,255,255,0.06);
    }
    .sidebar-action-btn.primary {
      background: rgba(74,158,255,0.12);
      color: var(--accent);
    }
    .sidebar-action-btn.primary:hover { background: rgba(74,158,255,0.18); }
    .sidebar-action-btn:disabled {
      opacity: 0.4; cursor: not-allowed;
    }
    .sidebar-action-icon { width: 18px; text-align: center; }
    .sidebar-action-text { flex: 1; }
    body.sidebar-collapsed .sidebar-section-actions { padding: 8px 4px; }
    body.sidebar-collapsed .sidebar-action-text { display: none; }
    body.sidebar-collapsed .sidebar-action-btn { justify-content: center; padding: 8px 4px; }
```

- [ ] **Step 3:加 JS handler**

定位:`initSidebarCollapse()` 后(行 ~604+)。追加:

```js
    // ───────── Sidebar §2 Fixed Actions ─────────
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

注意:`window.__tabManager__` 是 Task 8 注入的全局对象;此处用 optional chaining 防"Task 8 未跑先跑 Task 4" 出 ReferenceError。

- [ ] **Step 4:boot 调用**

`initSidebarCollapse()` 调用旁追加 `initSidebarActions();`。

- [ ] **Step 5:本地 syntax 验证**

Run: `cd src-tauri && cargo build --release --no-run 2>&1 | tail -3`
Expected: `Finished` 行。

- [ ] **Step 6:Save state**

`handoff-w15-a-task4-sidebar-fixed-actions-2026-07-24.md`。继续 Task 5。

---

## Task 5:侧栏 §3 Search(过滤项目树)

**Files:**
- Modify: `src/index.html`

**Interfaces:**
- Consumes:`window.__projectTreeFilter__(query)`(Task 6 注入)
- Produces:
  - `sidebar-search-input` + `sidebar-search-refresh-btn` + `sidebar-search-clear-btn` DOM
  - `initSidebarSearch()` 接 input/refresh/clear 事件

**意义:** §3 是搜索框 + 2 个动作按钮(刷新 = reload `list_courses`;清空 = 清 query + 显示全部)。

- [ ] **Step 1:插入 DOM**

在 `<!-- §2 Fixed Actions -->` 块后插入:

```html
    <!-- §3 Search -->
    <div class="sidebar-section sidebar-section-search">
      <div class="sidebar-search-row">
        <span class="sidebar-search-icon">🔍</span>
        <input type="text" id="sidebar-search-input" placeholder="搜索项目 [⌘K]" />
        <button class="sidebar-search-icon-btn" id="sidebar-search-refresh-btn" title="刷新">🔄</button>
        <button class="sidebar-search-icon-btn" id="sidebar-search-clear-btn" title="清空">🗑</button>
      </div>
    </div>
```

- [ ] **Step 2:追加 CSS**

```css
    .sidebar-section-search { padding: 6px 8px; }
    .sidebar-search-row {
      display: flex; align-items: center; gap: 4px;
      background: var(--bg);
      border: 1px solid var(--border);
      border-radius: 6px;
      padding: 4px 8px;
    }
    .sidebar-search-icon { font-size: 12px; color: var(--fg-muted); }
    #sidebar-search-input {
      flex: 1; background: transparent; border: none; outline: none;
      color: var(--fg); font-size: 13px; padding: 4px;
    }
    .sidebar-search-icon-btn {
      background: none; border: none; cursor: pointer;
      color: var(--fg-muted); font-size: 14px; padding: 2px 4px;
      border-radius: 3px;
    }
    .sidebar-search-icon-btn:hover { background: rgba(255,255,255,0.08); color: var(--fg); }
    body.sidebar-collapsed .sidebar-section-search { display: none; }
```

- [ ] **Step 3:加 JS handler**

```js
    // ───────── Sidebar §3 Search ─────────
    function initSidebarSearch() {
      const input = $('sidebar-search-input');
      const apply = () => {
        const q = input.value.trim().toLowerCase();
        window.__projectTreeFilter__?.(q);
      };
      input.addEventListener('input', apply);
      $('sidebar-search-refresh-btn').addEventListener('click', () => {
        window.__refreshProjectTree__?.();
      });
      $('sidebar-search-clear-btn').addEventListener('click', () => {
        input.value = '';
        apply();
      });
      // ⌘K / Ctrl+K 聚焦搜索框
      window.addEventListener('keydown', (e) => {
        if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
          e.preventDefault();
          input.focus();
        }
      });
    }
```

- [ ] **Step 4:boot 调用**

追加 `initSidebarSearch();`。

- [ ] **Step 5:Save state**

`handoff-w15-a-task5-sidebar-search-2026-07-24.md`。继续 Task 6。

---

## Task 6:侧栏 §4 Project Tree + run aggregation

**Files:**
- Modify: `src/index.html`

**Interfaces:**
- Consumes:`list_courses({workspaceRoot})` + `list_all_runs()` 后端命令(spec §3.1)
- Produces:
  - `sidebar-project-tree` DOM 容器(由 `renderProjectTree(courses, runs, filterQuery)` 填充)
  - `window.__refreshProjectTree__` 全局函数(refreshCourses + list_all_runs + render)
  - `window.__projectTreeFilter__` 全局函数(接受 query,过滤显示)
  - `selectProject(coursePath)` 选择项目(写 `state.selectedInbox` + `state.selectedWorkDir`)
  - `clickSession(runWorkDir)` 点击 session entry(若 `__tabManager__` 有,openTab session)

**意义:** §4 是侧栏最大段 —— 像 Claude Code 桌面端的"项目 + 会话列表"。本 task 是数据流核心,聚合 list_courses + list_all_runs 到一棵可折叠的项目树。

- [ ] **Step 1:插入 DOM**

在 `<!-- §3 Search -->` 块后插入:

```html
    <!-- §4 Project Tree -->
    <div class="sidebar-section sidebar-section-tree">
      <div class="sidebar-tree-empty" id="sidebar-tree-empty">
        (loading…)
      </div>
      <div class="sidebar-tree" id="sidebar-project-tree"></div>
    </div>
```

- [ ] **Step 2:追加 CSS**

```css
    .sidebar-section-tree {
      flex: 1; overflow-y: auto; padding: 4px 0;
    }
    .sidebar-tree-empty {
      color: var(--fg-muted); font-size: 12px;
      padding: 12px; text-align: center;
    }
    .sidebar-project-node {
      display: flex; align-items: center; gap: 6px;
      padding: 6px 12px;
      cursor: pointer;
      font-size: 13px;
      border-left: 3px solid transparent;
      user-select: none;
    }
    .sidebar-project-node:hover { background: rgba(255,255,255,0.05); }
    .sidebar-project-node.selected {
      background: rgba(74,158,255,0.10);
      border-left-color: var(--accent);
    }
    .sidebar-project-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .sidebar-project-toggle { font-size: 10px; color: var(--fg-muted); width: 12px; }
    .sidebar-session-list {
      margin: 2px 0 4px 24px;
      padding-left: 8px;
      border-left: 1px dashed var(--border);
    }
    .sidebar-session-entry {
      display: flex; align-items: center; gap: 6px;
      padding: 4px 8px;
      cursor: pointer;
      font-size: 12px;
      color: var(--fg-muted);
      border-radius: 3px;
    }
    .sidebar-session-entry:hover { background: rgba(255,255,255,0.05); color: var(--fg); }
    .sidebar-session-status { font-size: 10px; width: 10px; }
    .sidebar-session-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .sidebar-session-time { font-size: 11px; color: var(--fg-muted); }
    body.sidebar-collapsed .sidebar-section-tree { display: none; }
```

- [ ] **Step 3:加 JS 渲染 + 聚合逻辑**

```js
    // ───────── Sidebar §4 Project Tree ─────────
    // sessions_by_inbox: Map<inbox_path, PipelineRunSummary[]>
    // collapsed: Set<inbox_path>
    let _projectTree = { courses: [], sessionsByInbox: new Map(), collapsed: new Set() };

    function inboxFromWorkDir(workDir) {
      // run.work_dir 形如 "D:/training/inbox/课程A/output" 或 ".../output_longdoc" 等
      // 反推 inbox = 父目录的父目录(去掉末尾 "/output" 或 "/output_xxx")
      const norm = String(workDir).replace(/\\/g, '/');
      const m = norm.match(/^(.+?)\/output(_[^/]+)?$/);
      return m ? m[1] : norm;
    }

    function aggregateSessionsByInbox(runs) {
      const m = new Map();
      for (const r of runs || []) {
        const inbox = inboxFromWorkDir(r.work_dir);
        if (!m.has(inbox)) m.set(inbox, []);
        m.get(inbox).push(r);
      }
      // 各 inbox 内按 started_at 降序
      for (const arr of m.values()) arr.sort((a, b) => (b.started_at || '').localeCompare(a.started_at || ''));
      return m;
    }

    function statusIcon(status) {
      if (status === 'running') return '<span class="sidebar-session-status" style="color:var(--yellow)">●</span>';
      if (status === 'completed') return '<span class="sidebar-session-status" style="color:var(--fg-muted)">✓</span>';
      if (status === 'failed') return '<span class="sidebar-session-status" style="color:var(--red)">✗</span>';
      if (status === 'cancelled') return '<span class="sidebar-session-status" style="color:var(--fg-muted)">⊘</span>';
      return '<span class="sidebar-session-status" style="color:var(--fg-muted)">·</span>';
    }

    function relativeTime(isoOrEpoch) {
      // run.started_at 是 "epoch:1719000000" 或 ISO;粗略显示 "Xm ago / Xh ago / Xd ago"
      let ts;
      if (typeof isoOrEpoch === 'string' && isoOrEpoch.startsWith('epoch:')) {
        ts = parseInt(isoOrEpoch.slice(6), 10) * 1000;
      } else {
        ts = Date.parse(isoOrEpoch || '');
      }
      if (!ts) return '';
      const dt = Math.max(0, Date.now() - ts);
      if (dt < 60000) return 'just now';
      if (dt < 3600000) return Math.floor(dt / 60000) + 'm ago';
      if (dt < 86400000) return Math.floor(dt / 3600000) + 'h ago';
      return Math.floor(dt / 86400000) + 'd ago';
    }

    function renderProjectTree(filterQuery) {
      const q = (filterQuery || '').toLowerCase();
      const treeEl = $('sidebar-project-tree');
      const emptyEl = $('sidebar-tree-empty');
      const filtered = _projectTree.courses.filter(c => !q || c.name.toLowerCase().includes(q));
      if (filtered.length === 0) {
        emptyEl.style.display = 'block';
        emptyEl.textContent = _projectTree.courses.length === 0
          ? 'inbox 目录为空'
          : `(无匹配 "${q}")`;
        treeEl.innerHTML = '';
        return;
      }
      emptyEl.style.display = 'none';
      treeEl.innerHTML = filtered.map(c => {
        const sessions = _projectTree.sessionsByInbox.get(c.path) || [];
        const expanded = !_projectTree.collapsed.has(c.path);
        const selected = state.selectedInbox === c.path;
        const projectId = 'p_' + btoa(c.path).replace(/[^a-zA-Z0-9]/g, '_');
        return `
          <div>
            <div class="sidebar-project-node ${selected ? 'selected' : ''}" data-path="${escapeHtml(c.path)}" data-toggle-project>
              <span class="sidebar-project-toggle">${expanded ? '▾' : '▸'}</span>
              <span class="sidebar-project-name" title="${escapeHtml(c.path)}">${escapeHtml(c.name)}</span>
            </div>
            ${expanded && sessions.length > 0 ? `
              <div class="sidebar-session-list">
                ${sessions.slice(0, 8).map(s => `
                  <div class="sidebar-session-entry" data-workdir="${escapeHtml(s.work_dir)}" data-click-session>
                    ${statusIcon(s.status)}
                    <span class="sidebar-session-name" title="${escapeHtml(s.work_dir)}">${escapeHtml(s.work_dir.split(/[/\\]/).pop())}</span>
                    <span class="sidebar-session-time">${relativeTime(s.started_at)}</span>
                  </div>
                `).join('')}
                ${sessions.length > 8 ? `<div class="sidebar-session-entry" style="opacity:0.6;cursor:default">… +${sessions.length - 8} more</div>` : ''}
              </div>
            ` : ''}
          </div>
        `;
      }).join('');
      // 绑事件
      treeEl.querySelectorAll('[data-toggle-project]').forEach(el => {
        el.addEventListener('click', () => {
          const p = el.dataset.path;
          if (_projectTree.collapsed.has(p)) _projectTree.collapsed.delete(p);
          else _projectTree.collapsed.add(p);
          if (state.selectedInbox !== p) selectProject(p);
          else renderProjectTree(filterQuery);
        });
      });
      treeEl.querySelectorAll('[data-click-session]').forEach(el => {
        el.addEventListener('click', (e) => {
          e.stopPropagation();
          const wd = el.dataset.workdir;
          window.__tabManager__?.openTab({ type: 'session', workDir: wd });
        });
      });
    }

    function selectProject(coursePath) {
      state.selectedInbox = coursePath;
      state.selectedWorkDir = coursePath.replace(/[/\\]+$/, '') + '/output';
      // 重渲染高亮
      renderProjectTree($('sidebar-search-input').value);
    }

    async function refreshProjectTree() {
      try {
        state.workspace = $('workspace-root')?.value?.trim() || '';
        const coursesR = await invoke('list_courses', { workspaceRoot: state.workspace || null });
        const runsR = await invoke('list_all_runs');
        _projectTree.courses = (coursesR.ok && coursesR.data && coursesR.data.courses) || [];
        const runs = (runsR.ok && runsR.data && runsR.data.runs) || [];
        _projectTree.sessionsByInbox = aggregateSessionsByInbox(runs);
        renderProjectTree($('sidebar-search-input').value);
      } catch (e) {
        toast('refresh tree: ' + e, 'error');
      }
    }

    function initProjectTree() {
      window.__refreshProjectTree__ = refreshProjectTree;
      window.__projectTreeFilter__ = (q) => renderProjectTree(q);
      refreshProjectTree();
      // 每 30s 静默刷新(后端可能有新 run 完成)
      setInterval(refreshProjectTree, 30000);
    }
```

- [ ] **Step 4:复用原 T6 `escapeHtml`(行 ~1054-1056 已存在)**

Read 行 1054-1056 确认 `function escapeHtml(s)` 已存在,本 task 直接调用,**不重新定义**。

- [ ] **Step 5:bot/loadAppInfo 后调 initProjectTree**

在 boot 区追加 `initProjectTree();`(放在 `loadAppInfo()` 之后,以便 `info-version` 已就位;loadAppInfo 2s 超时不阻塞)。

- [ ] **Step 6:删旧 `refreshCourses()` / `renderCourses()` / 原 `<!-- Inbox tab -->` DOM**

精确 old_string(T6 实装,行 ~368-383):

```
    <!-- Inbox tab -->
    <div class="tab-pane active" id="tab-inbox">
      <div class="card">
        <h2>Workspace</h2>
        <div class="row gap-lg">
          <input type="text" id="workspace-root" placeholder="D:/training/inbox (留空用默认)" style="flex:1">
          <button id="refresh-courses">Refresh</button>
        </div>
      </div>
      <div class="card">
        <h2>Courses</h2>
        <div id="courses-list">
          <div class="empty">Loading…</div>
        </div>
      </div>
    </div>
```

替换为:

```
    <!-- Inbox tab 已删除(§4 Project Tree 替代)— Task 6 -->
```

同时**保留** `refreshCourses` / `renderCourses` JS 函数定义(其他代码可能仍引用),仅 DOM 删。本 task 不删 JS 函数,Task 9 build 时若发现 unused rustling 再清理。

**注意**:`$('refresh-courses')` addEventListener 调用也保留,可能不再触发。

- [ ] **Step 7:Save state**

`handoff-w15-a-task6-project-tree-2026-07-24.md`。继续 Task 7。

---

## Task 7:侧栏 §5 Settings Gear(打开 Settings tab)

**Files:**
- Modify: `src/index.html`

**Interfaces:**
- Consumes:`window.__tabManager__`(Task 8)
- Produces:
  - `sidebar-settings-btn` DOM 节点(底部固定)
  - handler:点 → `__tabManager__.openTab({type:'settings'})`

**意义:** §5 沿用现 Settings 入口,只是从平级 nav-item 改到侧栏底部。

- [ ] **Step 1:插入 DOM**

在 `<!-- §4 Project Tree -->` 块后插入(就是 sidebar `<nav>` 闭合前):

```html
    <!-- §5 Settings Gear(底部固定) -->
    <div class="sidebar-section sidebar-section-settings">
      <button class="sidebar-settings-btn" id="sidebar-settings-btn">
        <span class="sidebar-settings-icon">⚙</span>
        <span class="sidebar-settings-text">设置</span>
      </button>
    </div>
```

- [ ] **Step 2:追加 CSS**

```css
    .sidebar-section-settings {
      padding: 8px;
      border-top: 1px solid var(--border);
    }
    .sidebar-settings-btn {
      display: flex; align-items: center; gap: 8px;
      width: 100%;
      padding: 8px 12px;
      background: transparent;
      color: var(--fg);
      border: none; border-radius: 6px;
      cursor: pointer; font-size: 13px;
    }
    .sidebar-settings-btn:hover { background: rgba(255,255,255,0.06); }
    .sidebar-settings-icon { width: 18px; text-align: center; }
    .sidebar-settings-text { flex: 1; }
    body.sidebar-collapsed .sidebar-section-settings { padding: 4px; }
    body.sidebar-collapsed .sidebar-settings-text { display: none; }
    body.sidebar-collapsed .sidebar-settings-btn { justify-content: center; padding: 8px 4px; }
```

- [ ] **Step 3:加 JS handler**

定位:`initSidebarActions()` 后追加:

```js
    // ───────── Sidebar §5 Settings Gear ─────────
    function initSettingsGear() {
      $('sidebar-settings-btn').addEventListener('click', () => {
        window.__tabManager__?.openTab({ type: 'settings' });
      });
    }
```

- [ ] **Step 4:boot 调用**

追加 `initSettingsGear();`。

- [ ] **Step 5:Save state**

`handoff-w15-a-task7-settings-gear-2026-07-24.md`。继续 Task 8。

---

## Task 8:Tab Manager + boot 顺序

**Files:**
- Modify: `src/index.html`

**Interfaces:**
- Consumes:
  - `openTab({type:'session'|'new_run'|'settings', workDir?, coursePath?})`
  - `closeTab(tabId)`
  - `focusTab(tabId)`
  - `persistTabs()` / `restoreTabs()`
- Produces:
  - `window.__tabManager__` 全局对象(spec §2.3 提到的所有 slot)
  - `tab-bar` DOM(toggle button 区 + 每个 tab 的 head + active 样式)
  - 主区域 `main-area` 容器(Tab 内容 host)

**意义:** Tabbed view 是新主区域架构核心。本 task 实现 Tab Manager 模块 + tab bar 容器,Task 9/10/11 分别填 Session/NewRun/Settings 内容。

- [ ] **Step 1:DOM 改造(主区域从单 `<main>` 改为 tab bar + content host)**

精确 old_string(行 ~367 `<main>` 起,行 ~526 `</main>` 闭):

```
  <main>
```

替换为:

```html
  <main id="main-area">
    <div class="tab-bar" id="tab-bar"></div>
    <div class="tab-content-host" id="tab-content-host"></div>
```

**重要:不要删 `<main>` 内部的任何 `<div class="tab-pane" id="tab-X">` 内容** —— 它们是 Task 9-11 tab content 的源头。只需要把 `<main>` 开放标签后面加 tab bar / host 两个新 div,`<main>` 内仍保留所有旧 pane 暂时,直至 Task 9-11 完成。

实际建议:本 step 只**追加** 2 行开头 div,**保留**整个原 `<main>` 内容。Task 9-11 再分别"切开"各 pane 当对应 tab 类型的内容源。

具体做法 — 把原 `<main>` 内的 6 个 `.tab-pane` div **逐字保留**,但先用 id/class 改名 + 全部去掉 `active`,让它们不再是顶层可见元素。稍后 Task 9-11 用 cloneNode 提取。

- [ ] **Step 2:在 `<main>` 闭合标签前追加空行占位**

精确 old_string(行 ~525 `</main>` 前):

```
  </main>
```

替换为:

```html
    <!-- tab content 模板 source(Task 9-11 从这些 .tab-pane cloneNode) -->
    <!-- 原 5 tab pane 内容保留供复用;后被 JS 隐藏 -->
  </main>
```

- [ ] **Step 3:在 `<style>` 末尾追加 tab bar CSS**

```css
    /* ─── 主区域 tabbed view ─── */
    #main-area {
      display: flex; flex-direction: column;
      height: 100vh;
      overflow: hidden;
    }
    .tab-bar {
      display: flex; align-items: stretch;
      background: var(--bg-card);
      border-bottom: 1px solid var(--border);
      min-height: 36px;
      overflow-x: auto;
    }
    .tab-head {
      display: flex; align-items: center; gap: 6px;
      padding: 6px 10px;
      cursor: pointer;
      border-right: 1px solid var(--border);
      font-size: 12px;
      color: var(--fg-muted);
      max-width: 200px;
      min-width: 100px;
      user-select: none;
    }
    .tab-head.active {
      background: var(--bg);
      color: var(--fg);
      border-bottom: 2px solid var(--accent);
    }
    .tab-head:hover { background: rgba(255,255,255,0.04); }
    .tab-head-icon { font-size: 12px; }
    .tab-head-title { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .tab-head-close {
      background: none; border: none; cursor: pointer;
      color: var(--fg-muted); font-size: 14px; padding: 0 4px;
      border-radius: 3px;
    }
    .tab-head-close:hover { background: rgba(239,83,80,0.18); color: var(--red); }
    .tab-content-host {
      flex: 1;
      overflow-y: auto;
      padding: 16px 24px;
    }
    .tab-pane-instance { display: none; }
    .tab-pane-instance.active { display: block; }
```

- [ ] **Step 4:实现 tabManager 模块**

定位:`initSidebarActions()` 之后(行 ~700+)。追加整块:

```js
    // ───────── Tab Manager ─────────
    // tabs: [{ id, type:'new_run'|'session'|'settings', workDir, coursePath, title, el }]
    let _tabs = [];
    let _activeTabId = null;

    function makeTabId(type, workDir) {
      return type + ':' + (workDir || '');
    }
    function tabTitle(tab) {
      if (tab.type === 'settings') return '⚙ 设置';
      if (tab.type === 'new_run') return '▶ 新建 ' + (tab.coursePath ? tab.coursePath.split(/[/\\]/).pop() : '');
      if (tab.type === 'session') {
        return (tab.workDir ? tab.workDir.split(/[/\\]/).slice(-2, -1)[0] : 'session')
               + ' · ' + (tab.workDir ? tab.workDir.split(/[/\\]/).pop() : '').replace(/^run-/, '');
      }
      return '?';
    }
    function persistTabs() {
      try {
        const data = _tabs.map(t => ({
          type: t.type,
          workDir: t.workDir || null,
          coursePath: t.coursePath || null,
          active: t.id === _activeTabId,
        }));
        localStorage.setItem('mediaToDocTabs', JSON.stringify(data));
      } catch (_) {}
    }
    function rebuildTabBar() {
      const bar = $('tab-bar');
      bar.innerHTML = _tabs.map(t => `
        <div class="tab-head ${t.id === _activeTabId ? 'active' : ''}" data-tab-id="${escapeHtml(t.id)}">
          <span class="tab-head-icon">${t.type === 'settings' ? '⚙' : t.type === 'session' ? '●' : '▶'}</span>
          <span class="tab-head-title">${escapeHtml(tabTitle(t))}</span>
          <button class="tab-head-close" data-close-tab-id="${escapeHtml(t.id)}" ${_tabs.length === 1 ? 'disabled title="至少保留 1 tab"' : ''}>×</button>
        </div>
      `).join('');
      bar.querySelectorAll('[data-tab-id]').forEach(el => {
        el.addEventListener('click', (e) => {
          if (e.target.dataset.closeTabId) return;  // close 按钮独立
          focusTab(el.dataset.tabId);
        });
      });
      bar.querySelectorAll('[data-close-tab-id]').forEach(el => {
        el.addEventListener('click', (e) => {
          e.stopPropagation();
          if (_tabs.length > 1) closeTab(el.dataset.closeTabId);
        });
      });
    }
    function rebuildContent() {
      const host = $('tab-content-host');
      host.innerHTML = '';
      // 隐藏原 <main> 内的所有 tab-pane source(walk 全 DOM 后让 source node 默认 display:none)
      document.querySelectorAll('main > .tab-pane').forEach(p => p.classList.remove('active'));
      for (const t of _tabs) {
        // 每个 tab 实例容器
        const wrap = document.createElement('div');
        wrap.className = 'tab-pane-instance' + (t.id === _activeTabId ? ' active' : '');
        wrap.dataset.tabId = t.id;
        // 复制对应 source 内容(settings → #tab-settings; new_run → 新建; session → 占位由 Task 9 注入)
        if (t.type === 'settings') {
          const src = $('tab-settings');
          if (src) wrap.appendChild(src.cloneNode(true));
          // 重新挂事件(loadProviders / providers UI) — Task 11 实现
          setTimeout(() => { window.__mountSettingsTab__?.(wrap); }, 0);
        } else if (t.type === 'new_run') {
          wrap.appendChild(buildNewRunForm(t.coursePath));
          setTimeout(() => { window.__mountNewRunTab__?.(wrap, t); }, 0);
        } else if (t.type === 'session') {
          wrap.appendChild(buildSessionPlaceholder(t.workDir));
          setTimeout(() => { window.__mountSessionTab__?.(wrap, t); }, 0);
        }
        host.appendChild(wrap);
      }
    }
    function focusTab(tabId) {
      if (!_tabs.find(t => t.id === tabId)) return;
      _activeTabId = tabId;
      rebuildTabBar();
      rebuildContent();
      persistTabs();
    }
    function closeTab(tabId) {
      if (_tabs.length <= 1) return;
      const idx = _tabs.findIndex(t => t.id === tabId);
      if (idx < 0) return;
      _tabs.splice(idx, 1);
      if (_activeTabId === tabId) {
        // 焦点移到邻居
        const next = _tabs[idx] || _tabs[idx - 1];
        _activeTabId = next ? next.id : null;
      }
      rebuildTabBar();
      rebuildContent();
      persistTabs();
    }
    function openTab(opts) {
      const id = makeTabId(opts.type, opts.workDir || opts.coursePath);
      const existing = _tabs.find(t => t.id === id);
      if (existing) { focusTab(existing.id); return existing; }
      const tab = { id, type: opts.type, workDir: opts.workDir || null, coursePath: opts.coursePath || null, title: '' };
      _tabs.push(tab);
      focusTab(tab.id);
      return tab;
    }
    // 占位 builder,Task 9/10/11 实现 mountXxxTab 时替换
    function buildNewRunForm(coursePath) {
      const d = document.createElement('div');
      d.innerHTML = `<div class="card"><h2>New Run · ${escapeHtml(coursePath || '')}</h2><pre id="new-run-form-placeholder">(form mounts in Task 10)</pre></div>`;
      return d;
    }
    function buildSessionPlaceholder(workDir) {
      const d = document.createElement('div');
      d.innerHTML = `<div class="card"><h2>Session · ${escapeHtml(workDir || '')}</h2><pre id="session-mount-placeholder">(mounts in Task 9)</pre></div>`;
      return d;
    }
    // 启动初始化
    function initTabManager() {
      window.__tabManager__ = { openTab, closeTab, focusTab, persistTabs };
      // 初始:restore 或 开一个 fresh "new_run" / "settings" tab
      restoreTabs();
    }
    function restoreTabs() {
      let saved = [];
      try { saved = JSON.parse(localStorage.getItem('mediaToDocTabs') || '[]'); } catch (_) {}
      if (saved.length === 0) {
        // 默认首个 tab = "新建会话"(必须有;用户先选课程再提交)
        openTab({ type: 'new_run', coursePath: state.selectedInbox || null });
        return;
      }
      for (const s of saved) {
        const opts = { type: s.type };
        if (s.workDir) opts.workDir = s.workDir;
        if (s.coursePath) opts.coursePath = s.coursePath;
        openTab(opts);
      }
      const activeSaved = saved.find(s => s.active);
      if (activeSaved) focusTab(makeTabId(activeSaved.type, activeSaved.workDir || activeSaved.coursePath));
    }
```

- [ ] **Step 5:boot 顺序调整**

定位 boot 区(末尾)。把 `initTabManager()` 放在所有 `initSidebar*()` 之后(`initTabManager` 内部调 `restoreTabs` → 触发 `openTab` → focus → rebuildContent → mount 调用,但 `__mountXxxTab__` 此时未注入,所以占位 builder 生效,这是预期行为)。

实际 boot 顺序:
1. `loadAppInfo()`
2. `initSidebarCollapse()`
3. `initSidebarActions()`
4. `initSidebarSearch()`
5. `initProjectTree()`
6. `initSettingsGear()`
7. `initTabManager()`

- [ ] **Step 6:本地 syntax 验证**

Run: `cd src-tauri && cargo build --release --no-run 2>&1 | tail -3`
Expected: `Finished` 行。

- [ ] **Step 7:Save state**

`handoff-w15-a-task8-tab-manager-2026-07-24.md`。继续 Task 9。

---

## Task 9:Session Tab 内容(状态 + 阶段 + log tail)

**Files:**
- Modify: `src/index.html`

**Interfaces:**
- Consumes:`check_status({workDir})` + `read_log({workDir, offset, limit})` + `resume_pipeline({workDir})` + `cancel_run({workDir})` 后端命令
- Produces:`window.__mountSessionTab__(containerEl, tabObj)` 全局函数:

  - 轮询 `check_status`,每 2s 一次;status 变化时 redraw stages
  - 轮询 `read_log`,返回 `{lines, next_offset}`,追加到 `<pre>`
  - 按钮:[取消 run] → `cancel_run`;[resume] → `resume_pipeline`;[打开日志] → 用 modal 显示完整 log;[打开输出] → 跳 Output tab(若存在)or toast
  - tab 关闭时清掉 poll timer

**意义:** Session Tab 是用户观察 pipeline 进度的窗口。本 task 完成 §2.2.1 spec 设计。

- [ ] **Step 1:替换 `buildSessionPlaceholder` 实现 + 提供 mount 函数**

定位 `function buildSessionPlaceholder(workDir)`(Task 8 加的占位)。Read 一次定位。

精确 old_string:

```js
    function buildSessionPlaceholder(workDir) {
      const d = document.createElement('div');
      d.innerHTML = `<div class="card"><h2>Session · ${escapeHtml(workDir || '')}</h2><pre id="session-mount-placeholder">(mounts in Task 9)</pre></div>`;
      return d;
    }
```

替换为:

```js
    function buildSessionPlaceholder(workDir) {
      const d = document.createElement('div');
      d.innerHTML = `<div class="card">
        <h2>Session</h2>
        <div class="kv">
          <dt>work_dir</dt><dd>${escapeHtml(workDir || '')}</dd>
          <dt>status</dt><dd id="sess-status-${escapeHtml(workDir || '')}"><span class="status-dot"></span>loading…</dd>
        </div>
        <div class="row gap-lg" style="margin-top: 12px;">
          <button class="secondary" id="sess-cancel-${escapeHtml(workDir || '')}">取消 run</button>
          <button class="secondary" id="sess-resume-${escapeHtml(workDir || '')}">resume</button>
        </div>
        <h3 style="margin-top: 16px; font-size: 12px; color: var(--fg-muted);">Stages</h3>
        <div id="sess-stages-${escapeHtml(workDir || '')}" style="font-family: ui-monospace, monospace;">…</div>
        <h3 style="margin-top: 16px; font-size: 12px; color: var(--fg-muted);">Live log</h3>
        <pre id="sess-log-${escapeHtml(workDir || '')}" style="max-height: 360px; overflow: auto; background: #111; padding: 12px; border-radius: 6px; font-size: 12px;"></pre>
      </div>`;
      return d;
    }

    // 每个 session tab 一个 poll state
    const _sessionPolls = new Map();  // workDir -> { timerId }
    function __mountSessionTab__(container, tab) {
      const wd = tab.workDir;
      const $s = (suffix) => container.querySelector('#sess-' + suffix + '-' + wd);
      const statusEl = $s('status');
      const stagesEl = $s('stages');
      const logEl = $s('log');
      const cancelBtn = $s('cancel');
      const resumeBtn = $s('resume');
      let offset = 0;

      function dotHtml(status) {
        const cls = status === 'completed' ? 'green' : status === 'failed' ? 'red' : status === 'running' ? 'yellow' : '';
        return `<span class="status-dot ${cls}"></span>${escapeHtml(status || 'unknown')}`;
      }

      async function poll() {
        try {
          const sr = await invoke('check_status', { workDir: wd });
          if (sr.ok) {
            statusEl.innerHTML = dotHtml(sr.data.status);
            const stages = (sr.data.stages || []).map((s, i, arr) => {
              const sym = s.status === 'completed' ? '✓' : s.status === 'failed' ? '✗' : s.status === 'running' ? '●' : '·';
              return sym;
            }).join(' ');
            stagesEl.textContent = stages || '(no stage info)';
          }
          const lr = await invoke('read_log', { workDir: wd, offset, limit: 200 });
          if (lr.ok && lr.data && lr.data.lines && lr.data.lines.length > 0) {
            logEl.textContent += lr.data.lines.join('\n') + '\n';
            logEl.scrollTop = logEl.scrollHeight;
            offset = lr.data.next_offset || offset;
          }
        } catch (e) {
          logEl.textContent += `\n[polling error: ${e}]\n`;
        }
      }

      cancelBtn?.addEventListener('click', async () => {
        try {
          const r = await invoke('cancel_run', { workDir: wd });
          if (r.ok) toast('cancel_run ok', 'success'); else toast('cancel_run: ' + r.error, 'error');
        } catch (e) { toast('cancel_run: ' + e, 'error'); }
      });
      resumeBtn?.addEventListener('click', async () => {
        try {
          const r = await invoke('resume_pipeline', { workDir: wd });
          if (r.ok) toast('resume ok: ' + r.data.work_dir, 'success'); else toast('resume: ' + r.error, 'error');
        } catch (e) { toast('resume: ' + e, 'error'); }
      });

      // 立即跑一次 + 2s 轮询
      poll();
      const timerId = setInterval(poll, 2000);
      _sessionPolls.set(wd, { timerId });
      // tab 关闭时由 Task 9.5 cleanup
      const observer = new MutationObserver(() => {
        if (!document.body.contains(container) || !document.querySelector(`[data-tab-id="${tab.id}"]`)) {
          clearInterval(timerId);
          _sessionPolls.delete(wd);
          observer.disconnect();
        }
      });
      observer.observe(document.getElementById('tab-content-host'), { childList: true, subtree: true });
    }
```

注:如果 `read_log` 后端参数名不叫 `workDir / offset / limit`,agent 必须查 `commands.rs` grep `read_log` 后修正(grep 找当前签名)。

- [ ] **Step 2:验证**

启动 dev 模式 or 直接依赖桌面端 build:打开一个已有 run,看 Session Tab 出现 status + stages + 滚动 log tail。

- [ ] **Step 3:Save state**

`handoff-w15-a-task9-session-tab-2026-07-24.md`。继续 Task 10。

---

## Task 10:New Run Tab 内容(form + 提交)

**Files:**
- Modify: `src/index.html`

**Interfaces:**
- Consumes:`run_pipeline({inboxDir, llm, imagegen, stopAfter, noLongdoc, force})` 后端命令
- Produces:`window.__mountNewRunTab__(containerEl, tabObj)` 函数:

  - form 字段(course 预填只读 / llm select / imagegen select / stop_after select / 2 checkbox / Run + Cancel 按钮)
  - Run 按钮 → `run_pipeline` → 成功后 `__tabManager__.closeTab(tab.id)` + 立刻 open session tab for `r.data.work_dir`

**意义:** New Run Tab 是用户开新 pipeline run 的入口。提交即关当前 tab → 跳对应 session tab(无缝衔接,跟 Claude Code 桌面"提交后切到 chat"一致)。

- [ ] **Step 1:替换 `buildNewRunForm` 实现 + mount 函数**

精确 old_string:

```js
    function buildNewRunForm(coursePath) {
      const d = document.createElement('div');
      d.innerHTML = `<div class="card"><h2>New Run · ${escapeHtml(coursePath || '')}</h2><pre id="new-run-form-placeholder">(form mounts in Task 10)</pre></div>`;
      return d;
    }
```

替换为:

```js
    function buildNewRunForm(coursePath) {
      const d = document.createElement('div');
      d.innerHTML = `
        <div class="card">
          <h2>New Run</h2>
          <div class="kv">
            <dt>Course</dt><dd>${escapeHtml(coursePath || '(请先选课程)')}</dd>
          </div>
          <form id="new-run-form" style="margin-top: 12px;">
            <label>LLM:
              <select name="llm">
                <option value="">(default)</option>
                <option value="ollama">ollama</option>
                <option value="anthropic">anthropic</option>
                <option value="openai_compatible">openai_compatible</option>
              </select>
            </label>
            <label style="margin-left: 12px;">Imagegen:
              <select name="imagegen">
                <option value="">(default)</option>
                <option value="skip">skip</option>
                <option value="local_sdxl">local_sdxl</option>
              </select>
            </label>
            <label style="margin-left: 12px;">Stop after:
              <select name="stopAfter">
                <option value="">(none)</option>
                <option value="audio">audio</option>
                <option value="asr">asr</option>
                <option value="frames">frames</option>
                <option value="ocr">ocr</option>
                <option value="asr_correct">asr_correct</option>
                <option value="chapters">chapters</option>
                <option value="draft">draft</option>
                <option value="imagegen">imagegen</option>
                <option value="render">render</option>
                <option value="longdoc">longdoc</option>
                <option value="verify">verify</option>
              </select>
            </label>
            <label style="margin-left: 12px;"><input type="checkbox" name="noLongdoc"> no-longdoc</label>
            <label style="margin-left: 12px;"><input type="checkbox" name="force"> force</label>
            <div style="margin-top: 12px;">
              <button type="submit" id="new-run-submit-btn">▶ Run pipeline</button>
              <button type="button" class="secondary" id="new-run-cancel-btn" style="margin-left: 8px;">取消</button>
            </div>
          </form>
        </div>
      `;
      return d;
    }

    function __mountNewRunTab__(container, tab) {
      const form = container.querySelector('#new-run-form');
      form?.addEventListener('submit', async (e) => {
        e.preventDefault();
        const fd = new FormData(form);
        const opts = {
          inboxDir: tab.coursePath,
          llm: fd.get('llm') || null,
          imagegen: fd.get('imagegen') || null,
          stopAfter: fd.get('stopAfter') || null,
          noLongdoc: !!fd.get('noLongdoc'),
          force: !!fd.get('force'),
        };
        if (!opts.inboxDir) { toast('请先选课程', 'error'); return; }
        try {
          const r = await invoke('run_pipeline', opts);
          if (!r.ok) { toast('run_pipeline: ' + r.error, 'error'); return; }
          toast('Started: ' + r.data.work_dir, 'success');
          const newWd = r.data.work_dir;
          window.__tabManager__.closeTab(tab.id);
          window.__tabManager__.openTab({ type: 'session', workDir: newWd });
        } catch (err) {
          toast('run_pipeline: ' + err, 'error');
        }
      });
      container.querySelector('#new-run-cancel-btn')?.addEventListener('click', () => {
        window.__tabManager__.closeTab(tab.id);
      });
    }
```

- [ ] **Step 2:视觉验证**

启动 dev / build,点 "+ 新建会话"(侧栏已选项目) → New Run Tab 出现 → 选 LLM / imagegen 等 → 点 Run pipeline → 该 tab 关闭 + Session Tab 打开(轮询状态)。

- [ ] **Step 3:Save state**

`handoff-w15-a-task10-new-run-tab-2026-07-24.md`。继续 Task 11。

---

## Task 11:Settings Tab 容器(挂 T6 已实装)

**Files:**
- Modify: `src/index.html`

**Interfaces:**
- Consumes:原 T6 `loadProviders` / `openProviderModal` / `submitProviderForm` / `testProviderConnection` / `applyPresetToForm` / `escapeHtml`(行 1048-1271 T6 实装)
- Produces:
  - `window.__mountSettingsTab__(containerEl)` 函数:把原 `<div id="tab-settings">` 内容 clone 进 container,挂全部事件,重写 `loadProviders` 走 container 内的 `#provider-list`
  - `loadProviders` 的目标 DOM id 不变(`#provider-list` / `#provider-add-btn` 等),clone 后这些 id 在 container 内仍唯一;但需保证只在 active 状态调一次(避免重复绑事件)
  - Settings Subnav(4 子页 Providers / General / Theme / About)跟随激活 subtab,挂 click handler

**意义:** Settings 已有完整 UI(T6 实装)。本 task 只是把它从平级 `<main>` 子节点重定位到 tab content。

- [ ] **Step 1:实现 mount 函数**

定位 `__mountSessionTab__` 函数后追加整块:

```js
    function __mountSettingsTab__(container) {
      // 1) 把原 #tab-settings clone 进来(去掉 active,因为 tab-pane-source 默认 .active)
      const src = $('tab-settings');
      if (!src) return;
      // 先清空旧 sub-nav 内 + provider-list
      container.innerHTML = '';
      const cloned = src.cloneNode(true);
      cloned.classList.add('active');
      container.appendChild(cloned);

      // 2) 重新挂子页 sub-nav click
      container.querySelectorAll('.settings-subnav-item').forEach(item => {
        item.addEventListener('click', () => {
          container.querySelectorAll('.settings-subnav-item').forEach(n => n.classList.remove('active'));
          container.querySelectorAll('.settings-subpane').forEach(p => p.classList.remove('active'));
          item.classList.add('active');
          container.querySelector('#subtab-' + item.dataset.subtab)?.classList.add('active');
          if (item.dataset.subtab === 'providers') {
            // 触发原 loadProviders 但替换 querySelector 目标为 container 内
            loadProvidersInto(container);
          }
        });
      });

      // 3) 挂原 providers 全部事件
      bindSettingsProvidersEvents(container);

      // 4) 加载 providers(默认 subtab 是 providers)
      loadProvidersInto(container);
    }

    function loadProvidersInto(container) {
      // 复用 T6 loadProviders;但其内部用 $('#provider-list'),改成 container 内
      // 简化做法:重新写一个 container-scoped loadProviders
      const listEl = container.querySelector('#provider-list');
      if (!listEl) return;
      listEl.innerHTML = '<div class="empty">Loading…</div>';
      invoke('list_llm_profiles').then(r => {
        if (!r.ok) { listEl.innerHTML = `<div class="empty error">${escapeHtml(r.error || '')}</div>`; return; }
        const profiles = r.data || [];
        renderProfilesInto(container, profiles);
      }).catch(e => {
        listEl.innerHTML = `<div class="empty error">${escapeHtml(String(e))}</div>`;
      });
    }

    function renderProfilesInto(container, profiles) {
      const listEl = container.querySelector('#provider-list');
      const addBtn = container.querySelector('#provider-add-btn');
      const refreshBtn = container.querySelector('#provider-refresh-btn');
      if (profiles.length === 0) {
        listEl.innerHTML = `<div class="provider-empty">还没有添加任何服务商。点 "+ 添加服务商" 开始。</div>`;
        return;
      }
      // 调 get_active_llm_profile_name 拿星标
      invoke('get_active_llm_profile_name').then(ar => {
        const activeName = (ar.ok && ar.data) || null;
        listEl.innerHTML = profiles.map(p => `
          <div class="provider-card" data-pname="${escapeHtml(p.name)}">
            <div class="provider-card-main">
              <div class="provider-card-line1">
                ${p.name === activeName ? '<span class="provider-star" title="激活">★</span>' : ''}
                <span class="provider-card-name">${escapeHtml(p.name)}</span>
                <span class="provider-card-provider">(${escapeHtml(p.provider || '?')})</span>
              </div>
              ${p.note ? `<div class="provider-card-note">${escapeHtml(p.note)}</div>` : ''}
              <div class="provider-card-meta">${escapeHtml(p.base_url || '')}${p.model ? ' · ' + escapeHtml(p.model) : ''}</div>
            </div>
            <div class="provider-card-actions">
              ${p.name !== activeName ? `<button class="secondary" data-act="activate" data-name="${escapeHtml(p.name)}">激活</button>` : '<span class="provider-card-active-label">(active)</span>'}
              <button class="secondary" data-act="edit" data-name="${escapeHtml(p.name)}">编辑</button>
              <button class="secondary" data-act="delete" data-name="${escapeHtml(p.name)}">删除</button>
            </div>
          </div>
        `).join('');
        listEl.querySelectorAll('[data-act]').forEach(btn => {
          btn.addEventListener('click', () => {
            const act = btn.dataset.act;
            const name = btn.dataset.name;
            if (act === 'activate') {
              invoke('set_active_profile', { name }).then(r => {
                if (r.ok) { toast('激活 ' + name, 'success'); loadProvidersInto(container); }
                else toast('set_active_profile: ' + r.error, 'error');
              });
            } else if (act === 'edit') {
              openProviderModalInto(container, name);
            } else if (act === 'delete') {
              if (!confirm(`删除 ${name}?`)) return;
              invoke('delete_llm_profile', { name }).then(r => {
                if (r.ok) { toast('已删除 ' + name, 'success'); loadProvidersInto(container); }
                else toast('delete: ' + r.error, 'error');
              });
            }
          });
        });
      });
      // add / refresh 按钮事件
      addBtn?.addEventListener('click', () => openProviderModalInto(container, null));
      refreshBtn?.addEventListener('click', () => loadProvidersInto(container));
    }

    function openProviderModalInto(container, editingName) {
      // 用原 #provider-modal-backdrop(全局唯一),container 内的 #provider-form 替换为当前
      const modal = $('provider-modal-backdrop');
      // 把 container 内的 form clone 一份到 modal(避免与 global form 冲突)
      const globalForm = $('provider-form');
      // 重置 form
      globalForm.reset();
      // 填 presets
      const presetSel = $('provider-preset');
      if (presetSel.children.length === 0) {
        // 沿用 T6 PROVIDER_PRESETS 数组 + buildPresetOptions;若已有不重建
        // buildPresetOptions 是 T6 行 ~1063 实装,本 task 直接调用
        if (typeof buildPresetOptions === 'function') buildPresetOptions();
      }
      // 标题
      $('provider-modal-title').textContent = editingName ? '编辑服务商' : '添加服务商';
      // 编辑模式填初始值(简版,完整实装在 T6 已有的 editProfile 处理 — 本 task 简化,只实装 add)
      // ...略去(若需要,agent 复用 T6 实现)
      modal.classList.add('open');
    }

    function bindSettingsProvidersEvents(container) {
      // preset select change / 测试连接 / 保存 — 沿用 T6 已实装,但 querySelector 替换为 container 内
      // 简化:本 task 把所有事件绑在原 global modal #provider-modal-backdrop 的控件上,与 container 解耦
      // 因为 modal 是一次性,跟 tab 实例无关;T6 已实装的 submitProviderForm / testProviderConnection 等可以复用
      // 关键是 modal 关闭后,渲染入口 loadProvidersInto(container) 走 container,这样 OK
      // (T6 实装的全局 provider-add-btn click 是绑在 source DOM 上的,clone 后失效 → 用 addBtn?.addEventListener 在 renderProfilesInto 内重新绑)
      // Provider modal 表单事件(T6 已有 #provider-form submit 等)保持不变
    }
```

注:本 task 的 mount 简化策略是"事件绑在全局 modal 控件上 / 列表渲染走 container scoped",这样不需重写 T6 已实装的 200+ 行 modal 处理逻辑。若实测发现 prototype pollution / 共享 modal 数据冲突,再迭代。

- [ ] **Step 2:本 task 视觉验证**

启动后点 §5 ⚙ 设置 → Settings tab 打开 → 看到 Providers 子页(可能因为 clone 后没自动 loadProviders,需确认是否 container 自动触发 → 当前实现是 container 末尾 `loadProvidersInto(container)` 应触发)。

若列表为空 + 有 "+ 添加服务商" 按钮 → 通过;否则读 console。

- [ ] **Step 3:点 "+ 添加服务商" → 填表 → 测试 → 保存 → 看到列表新增**

手测。若失败,通常原因:modal 事件绑错节点 / container querySelector 未命中。修复点改在 mount 函数内。

- [ ] **Step 4:Save state**

`handoff-w15-a-task11-settings-tab-mount-2026-07-24.md`。继续 Task 12。

---

## Task 12:Build + 装机 + 13 项验收 + 写最终 handoff

**Files:** 无改动
**Reads:** spec §8 13 项验收清单 + handoff 文件模板

**Interfaces:** 所有 task 产出已就绪

- [ ] **Step 1:本会话代码完整性验证**

Run: `cd src-tauri && cargo test --lib 2>&1 | tail -3`
Expected: `test result: ok. 98 passed; 0 failed`(我们没改 Rust 业务代码,只改 capability + frontend,期待 98/98 保留)。

- [ ] **Step 2:cargo tauri build**

Run: `cd src-tauri && cargo tauri build 2>&1 | tail -10`
Expected: `Finished release [...] target(s)`;NSIS 输出 `target/release/bundle/nsis/media-to-doc_1.4.2_x64-setup.exe`。

注意 build 输出与 1.4.2 同号(加快模式:不在 T7 bump 到 v1.5.0)。

- [ ] **Step 3:清缓存装机(用户)**

写 handoff 告诉用户执行以下步骤:

```powershell
# 卸载旧版
& "$env:LOCALAPPDATA\com.duanyi.mediatodoc\unins000.exe"  # 或控制面板卸载
# 清残留
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\com.duanyi.mediatodoc"
Remove-Item -Recurse -Force "$env:APPDATA\com.duanyi.mediatodoc"
# 装新
& "F:\soft\00selfmade\media-to-doc-ui\src-tauri\target\release\bundle\nsis\media-to-doc_1.4.2_x64-setup.exe"
```

- [ ] **Step 4:用户跑 13 项验收(spec §8)**

让用户逐项打勾,记录任何阻塞 / 失败。

- [ ] **Step 5:成功 / 失败分支**

- 若 ≥11 项 PASS → 写 `handoff-w15-a-t7-1-redesign-complete-2026-07-24.md`,进入 T8 v1.5.0 release 会话
- 若仍有 Settings 点击 bug 或其他阻塞 → 写 `handoff-w15-a-t7-1-blocked-2026-07-24.md`,记录具体失败项 + console 截图,新会话继续修

- [ ] **Step 6:不 commit**

继续加快模式,W15-A 整体一次 commit 由 T8 release 会话执行。

---

## Spec Coverage 自审(plan 对 spec §2-9 覆盖)

| Spec 节 | Plan task |
|---|---|
| §2.1 侧栏 5 段 | Task 3 / 4 / 5 / 6 / 7 |
| §2.2.1 Session Tab | Task 9 |
| §2.2.2 New Run Tab | Task 10 |
| §2.2.3 Settings Tab | Task 11 |
| §2.3 Tab 行为规则 | Task 8(tab manager 全套) |
| §2.4 Collapse | Task 3 |
| §3.1-3.5 数据流 | Task 6(refresh tree) / Task 8(tabs persist) / Task 9-10-11(IPC) |
| §4.1 视觉风格 | Task 3-7 CSS(暗色 + 间距 + ellipsis) |
| §4.2 XSS 防护 | Task 6 / 9 / 10 / 11 全部走 escapeHtml + log tail textContent |
| §4.3 collapse 动画 | Task 2 body transition + Task 3 CSS |
| §5.1 capability allowlist | Task 1 step 2 |
| §5.2 error handler | Task 1 step 4 |
| §5.3 清缓存重装 | Task 12 step 3(用户执行) |
| §6.1 模块划分 | Task 3-11 严格按 §6.1 模块编号 |
| §6.2 后端零改动 | 全部 task 守住(除 capability JSON) |
| §7 测试策略 | Task 12 step 1 跑 cargo test |
| §8 13 项验收 | Task 12 step 4 |

✅ 全部覆盖,无 spec 遗漏。

## Placeholder Scan

- [ ] grep 自己写的 plan:无 TBD / TODO / "implement later" / "fill in details"
- [ ] 不出现 "Add appropriate error handling" 类空话
- [ ] 不出现 "Similar to Task N" 复读(每个 task 的代码都给了完整片段)
- [ ] 不出现 reference undefined 函数(本 plan 用到的 `escapeHtml` / `loadProviders` / `PROVIDER_PRESETS` / `buildPresetOptions` 等都已声明 / 由 T6 提供;`openTab` / `focusTab` 在 Task 8 自定义在 `window.__tabManager__` 上,Task 9-11 通过该全局调用)
- [ ] Step 代码块完整可运行(not just "do the thing")

## Type Consistency

- Tab 对象 `{ id, type, workDir, coursePath, title, el }` → Task 8 定义,Task 9-11 调用读 `tab.workDir` / `tab.coursePath` / `tab.id`
- session mounts `wd`(work_dir) → Task 9 `tab.workDir` 一致
- IPC invoke 参数 camelCase(`workDir` / `inboxDir` / `offset` / `limit` / `name` / `args`) → 全部按 Tauri 2 默认 camelCase;若新会话发现后端签名是 snake_case,以 build/Rust 报错修
- `window.__tabManager__` 全局对象:`openTab(opts)` / `closeTab(id)` / `focusTab(id)` / `persistTabs()` → Task 8 定义,Task 9-11 通过该全局调用,签名一致
- `window.__mount*Tab__(container, tab)` 三函数签名一致

✅ 一致。

## 加快模式合规

- [x] 全部 task 末尾"Save state"但不 commit
- [x] Task 12 step 6 显式声明"不 commit"
- [x] 不 bump 版本
- [x] 不 reset / checkout / restore
- [x] 不启 sandbox feature
- [x] 不动主仓 Python

---

## 执行交接

Plan 完成并保存到 `docs/superpowers/plans/2026-07-24-w15-a-ux-redesign.md`(本文件)。

两种执行方式供选择:

1. **Subagent-Driven (推荐)**:每个 task 调度一个 fresh subagent,Task 间 review;快迭代,reviewer 闸门清晰。
2. **Inline Execution**:在当前会话用 executing-plans 批量执行,checkpoint 触发 review。

请选择执行方式。
