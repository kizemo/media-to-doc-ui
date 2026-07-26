# Handoff — W15-A SDD: Tasks 6-12 接力

**日期**:2026-07-24 晚
**项目**:`F:/soft/00selfmade/media-to-doc-ui`
**当前分支**:`feat/w15a-llm-api-settings`(基线 `073b05e`,**无新 commit**)
**承接**:`handoff-w15-a-t7-failed-redesign-pivot-2026-07-24.md` + `handoff-w15-a-task1-bug-prereq-2026-07-24.md`等
**状态**:**SDD 12-task plan 已完成 5 task(T1-T5);剩 7 task(T6-T12)在工作区累积,无 commit**。

---

## 0. 新会话必读(按顺序)

1. `F:/soft/00selfmade/media-to-doc-ui/.superpowers/sdd/progress.md` — 进度 + 时间账 + 2 个 pre-flight plan 缺陷
2. `F:/soft/00selfmade/media-to-doc-ui/docs/superpowers/plans/2026-07-24-w15-a-ux-redesign.md` — 实施 plan 全 12 task
3. `F:/soft/00selfmade/media-to-doc-ui/docs/superpowers/specs/2026-07-24-w15-a-ux-redesign-design.md` — UX spec
4. 本文件 — 接力说明
5. `F:/soft/00selfmade/media-to-doc-ui/prompt-sdd-w15a-tasks6-12-next.md` — 给新会话第一句话引用的 prompt
6. 当前 git 状态:`cd F:/soft/00selfmade/media-to-doc-ui && git -c safe.directory=* status --short --branch`

---

## 1. 已完成 task 摘要(T1-T5)

| Task | 关键改动 | 累计 src/index.html 行数 | 文件 commit |
|---|---|---|---|
| 1 | `capabilities/default.json` description 更新 + module-level error handler (`<script>` 块,19 行,在原 `<script type="module">` 前) | 1281 → 1300 | 无 |
| 2 | 删 `<header>` + 5 `<nav-item>` + body grid + 旧 nav click handler + `loadAppInfo()` null-guard 修复(reviewer 抓的 Critical) | 1300 → 1288 | 无 |
| 3 | §1 Brand Header + collapse toggle + localStorage 持久化(`initSidebarCollapse`) | 1288 → 1339 | 无 |
| 4 | §2 Fixed Actions(`handleNewSessionClick` + `initSidebarActions`,`window.__tabManager__?.openTab(...)` 可选链) | 1339 → 1392 | 无 |
| 5 | §3 Search(input + 🔄 + 🗑 + Cmd/Ctrl+K,`window.__projectTreeFilter__?.` / `__refreshProjectTree__?.` 可选链) | 1392 → 1450 | 无 |

**build 累计耗时**:5 task × ~2:30-2:45 = ~13 min(每次 cargo build --release 全部从 0 编译;**新会话可用 `cargo build --release --incremental`** 或 `cd src-tauri && cargo build --release 2>&1 | tail -5` 单次,确认 0 errors / 5 baseline warnings 即过)

---

## 2. 关键 background(T1-T5 已就位但新会话必知)

### 2.1 当前 src/index.html 结构概览

```
行 1-32:           <head> + <title> + marked.js 引用
行 33-46:          body grid CSS(含 body.sidebar-collapsed 260↔48px)
行 47-340:         <style> 中的全部 CSS(W14-B+ 累积 + W15-A T6 Settings UI + Task 3/4/5 新增)
行 350-355:        body 内容
行 350-353:        <nav class="sidebar" id="sidebar"> 含 §1-§5 sections
  行 354-362:        §1 Brand Header
  行 363-379:        §2 Fixed Actions
  行 380-414:        §3 Search
  行 415-418:        <!-- §4 Project Tree — Task 6 -->
  行 419-422:        <!-- §5 Settings Gear — Task 7 -->
行 423-?:           <main> 含 6 .tab-pane div(inbox/run/output/health/learn/settings 全保留)
行 ~570-590:        Task 1 error handler <script> 块
行 ~600-1450:       <script type="module"> 全部累积
```

(行号是示意图,实际以 Read 为准)

### 2.2 Working tree 文件状态(累计)

```
M src-tauri/Cargo.toml
M src-tauri/capabilities/default.json          # Task 1 改 description
M src-tauri/src/commands.rs                      # pre-existing T1-T6
M src-tauri/src/lib.rs                           # pre-existing T1-T6
M src-tauri/src/runner.rs                        # pre-existing T1-T6
M src/index.html                                 # T1-T6 + Task 1 + 2 + 3 + 4 + 5 累积
?? src-tauri/src/keyring_store.rs                # T1 untracked
?? src-tauri/src/llm_profiles.rs                 # T2 untracked
?? task.md
?? handoff-w15-a-*.md (8 个)
?? prompt-*.md (4 个)
```

**禁止**:不要 reset / checkout / restore / 覆盖任何已工作区内容。

### 2.3 Plan 中已发现的 2 个缺陷(必须 baked 到 dispatch prompts)

**缺陷 1**:Task 8 `rebuildContent()` 用 `window.__mountSessionTab__?.(wrap, t)` 等,但 Task 9/10/11 定义 `function __mountSessionTab__(container, tab)` 后**默认只在 module scope 可用**,不会挂到 `window`,导致 mount 不调用、tab 内容空白。

修复:在 Task 9/10/11 实现末尾**追加**:
```js
window.__mountSessionTab__ = __mountSessionTab__;
window.__mountNewRunTab__ = __mountNewRunTab__;
window.__mountSettingsTab__ = __mountSettingsTab__;
```

**缺陷 2**:Task 5 的 Cmd/Ctrl+K listener 在 `window.addEventListener('keydown', ...)`,**全局** 监听。如果 Task 8 或后续再加同类 listener 会双触发。建议 Task 8 移除 Task 5 的 listener,统一在 `initTabManager()` 里加一个全局 listener。

---

## 3. 接力 Task 6-12 必交付清单

| Task | 必交付 | 预计时长 | 优先级 |
|---|---|---|---|
| **6** | §4 Project Tree:DOM + `aggregateSessionsByInbox` + `inboxFromWorkDir` 反推 + `renderProjectTree` + `refreshProjectTree` + `selectProject` + 30s 静默刷新 + 注入 `window.__refreshProjectTree__` 和 `window.__projectTreeFilter__`(Task 5 的 search 现已激活) | ~25 min | P0 |
| **7** | §5 Settings Gear 按钮 + click handler → `window.__tabManager__?.openTab({type:'settings'})` | ~10 min | P0(解锁 Settings tab) |
| **8** | Tab Manager:`openTab / closeTab / focusTab / persistTabs / restoreTabs` + `<main>` 改造为 `<div id="tab-bar">` + `<div id="tab-content-host">` +3 类 tab 占位 builder;**额外做缺陷 2(移除 Task 5 Cmd/Ctrl+K)** + **修复缺陷 1(为 9/10/11 的 globals 提前占位)** | ~35 min | P0 |
| **9** | Session Tab:状态点 + 11 stage 符号 + log tail(`check_status`/`read_log` 2s 轮询) + cancel/resume 按钮;末尾追加 `window.__mountSessionTab__ = __mountSessionTab__`(缺陷 1) | ~20 min | P1 |
| **10** | New Run Tab:form(LLM/Imagegen/StopAfter/2 checkbox) + 提交 → `run_pipeline` → 关 tab + 开 session tab;末尾追加 `window.__mountNewRunTab__ = __mountNewRunTab__`(缺陷 1) | ~12 min | P1 |
| **11** | Settings Tab mount:把 `<div id="tab-settings">` cloneNode 进 container + 重挂 subnav 4 子页 + `loadProvidersInto(container)` + `renderProfilesInto` + provider card actions(activate/edit/delete) + `openProviderModalInto`(复用原 modal) + 末尾追加 `window.__mountSettingsTab__ = __mountSettingsTab__`(缺陷 1) | ~25 min | P1 |
| **12** | cargo test --lib(应 98/98 保留)+ cargo tauri build → NSIS + 装 1.4.2 + 13 项验收(spec §8)+ 写 `handoff-w15-a-t7-1-{complete,blocked}-2026-07-24.md` | ~15 min | P2 |

---

## 4. Session Health 预算

- **本会话已用 ~95 min**,剩 2 hour 上限 ≈ 25 min
- **新会话**:建议完整 2-hour budget 跑 Task 6 + Task 8(两个大件)
- 完成后若还有 budget,继续 Task 7 → 9 → 10 → 11 → 12
- 撞墙立即写 `handoff-sdd-w15a-2nd-{blocked}-2026-07-24.md`,不超时续命

---

## 5. 加快模式规则(沿用 W15-A)

- 不要 commit / push / release / bump version / reset 未提交
- 不切回 master 直接开发
- 不删除旧 handoff / prompt
- 不启 sandbox feature
- 不动主仓 `media-to-doc/`
- **每个 task 末尾"Save state"** = 写 handoff-*.md,**不 git commit**(W15-A 整体一次 feature commit 推到 T8 release)

---

## 6. 完成标准 + 必交付证据(Task 12)

13 项验收全部走通(spec §8):
- 1 强清缓存重装 1.4.2 NSIS
- 2-3 collapse + collapse 记忆
- 4 search 过滤
- 5-6 §4 项目树 + session entry + 新建会话
- 7 新建会话未选项目 toast
- 8 提交 → 切 Session
- 9 Session 按钮(cancel/resume)
- 10 §5 ⚙ Settings
- 11 Providers 6 步全跑
- 12 tab × 关闭
- 13 重启恢复

**≥11/13 PASS 才算 T7 通过**,写 handoff-w15-a-t7-1-redesign-complete-2026-07-24.md。否则写 handoff-w15-a-t7-1-blocked-2026-07-24.md,记录失败项 + console log。

不管通过/失败,本次会话**不 commit**(加快模式 → T8 release)。

---

## 7. 下一会话第一句话 prompt

见 `prompt-sdd-w15a-tasks6-12-next.md`(同目录,30 行内)。

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
