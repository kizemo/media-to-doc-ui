# Spec — W15-A UX 重大重设计(Claude Code 桌面风格)

**作者**:W15-A T7 验收转向 / 2026-07-24
**承接**:`handoff-w15-a-t7-failed-redesign-pivot-2026-07-24.md` + `handoff-w15-a-t6-complete-2026-07-24.md`
**目标版本**:`feat/w15a-llm-api-settings` 上累积,最终随 v1.5.0 release 一起发布(T8)
**关系**:与 `2026-07-23-w15-a-llm-api-settings-design.md` 并列(后者覆盖后端 + 9 provider + keyring;本 spec 只覆盖前端 UX 重设计)

---

## 1. 目标与范围

### 1.1 用户原始反馈(2026-07-24)

> "如附图,左侧有 Inbox/Run/Output/Health/Learn/Settings 几个选项卡。除了 setting 外,其他几个都去除,修改为类似 claude code huahua 左侧选项卡的:新建会话 定时任务,下面应为 项目列表,如附图"
>
> "去除 UI 顶部的蓝色栏"
>
> "测试点击 setting 没有反应,无法调出添加 Providers 的界面,其他项无法测试"

### 1.2 核心改动

| 项 | 当前(T6 后) | 重设计后 |
|---|---|---|
| 顶部蓝色 `<header>` 栏 | 行 349-356,带 status dot / version badge | **删除整段** |
| 5 个主 tab(Inbox/Run/Output/Health/Learn) | 行 359-363,6 选项平级 | **删除 5 个** |
| Settings tab | 第 6 个 nav-item | **保留 → 侧栏底部齿轮** |
| 主区域 | 单视图,1 个 `.tab-pane.active` | **Tabbed view(多 tab,每个 session + Settings = 1 tab)** |
| 侧栏 | 单段 6 nav-item 列表 | **5 段:Brand / Fixed / Search / Project Tree / Settings 齿轮** |

### 1.3 范围之外(本 spec 不做)

- 后端 Tauri command 零改动(T6 已实装完整 6 LLM command + 17 个原有 command)
- keyring / provider / env var 注入逻辑零改动(T1-T5 已实装)
- mtd Python 端零改动(沿用 W14-D trust_env=False)
- "技能市场" / "定时任务" 两个固定项实装:本 spec 只留占位按钮(toast "W15-B+ 实装"),W15-B+ 再实装
- 不 bump 版本到 v1.5.0(T8 才做)

---

## 2. 信息架构(侧栏 5 段 + 主区域 Tabbed View)

### 2.1 侧栏 5 段(从顶到底)

```
┌─────────────────────────────────────────────┐
│ §1 Brand Header                              │
│   📦 media-to-doc              ⮜ (collapse)  │
├─────────────────────────────────────────────┤
│ §2 Fixed Actions                            │
│   + 新建会话                                  │
│   ⏰ 定时任务        (灰显,W15-B+)            │
├─────────────────────────────────────────────┤
│ §3 Search                                   │
│   🔍 搜索项目 [⌘K]        🔄     🗑            │
├─────────────────────────────────────────────┤
│ §4 Project Tree                             │
│   📁 课程A              ▾ (展开)             │
│       • run-20260724-153012  49m ago ●       │  ← session entry(可 resume)
│       • run-20260723-201544  1d ago  ✓        │
│       • run-20260721-083015  3d ago  ✗        │
│   📁 课程B              ▸ (折叠)             │
│   📁 课程C              ▸ (折叠)             │
├─────────────────────────────────────────────┤
│ §5 Settings Gear(底部固定)                    │
│   ⚙ 设置                                     │
└─────────────────────────────────────────────┘
```

### 2.2 主区域 Tabbed View

```
┌────────────────────────────────────────────────────────────┐
│ Tab bar: [▶ 课程A · run-153012 ×] [▶ 课程A · 新建 ×] [⚙ 设置 ×] [+] │  ← tab 头部
├────────────────────────────────────────────────────────────┤
│                                                              │
│              Active tab content(下面对应 §2.2.1-2.2.3)            │
│                                                              │
└────────────────────────────────────────────────────────────┘
```

**3 类 tab 内容:**

#### 2.2.1 Session Tab(每个 pipeline run 一个)

```
┌─ Session View ───────────────────────────────────────────┐
│ Header:课程A · run-20260724-153012 · status: running ●   │
│   Buttons:[暂停轮询] [取消 run] [resume] [打开日志] [打开输出]│
├─────────────────────────────────────────────────────────┤
│ Stage Grid(11 stage icon:audio→asr→...→verify)            │
│   ✓  ✓  ✓  ●  …  …  …  …  …  …  …                          │
├─────────────────────────────────────────────────────────┤
│ Live Log Tail(log tail 2s 轮询)                            │
│   <pre>tail of log file...</pre>                          │
└─────────────────────────────────────────────────────────┘
```

#### 2.2.2 New Run Tab(点 "+ 新建会话"产生)

```
┌─ New Run Config ────────────────────────────────────────┐
│ Course: 课程A(pre-filled from sidebar selection)         │
│ LLM: [select ▼ (default)]                                │
│ Imagegen: [select ▼ (default)]                           │
│ Stop after: [select ▼ (none)]                            │
│ ☐ no-longdoc   ☐ force                                  │
│                                                           │
│ [▶ Run pipeline]  [取消]                                  │
└─────────────────────────────────────────────────────────┘
```

#### 2.2.3 Settings Tab(点齿轮产生)

```
┌─ Settings View ─────────────────────────────────────────┐
│ Sub-nav:Providers | General | Theme | About               │
│                                                           │
│ <providers 子页面内容保留 T6 实装>                          │
│ <general / theme 占位>                                    │
│ <about:版本 + mtd version + status dot>                   │
└─────────────────────────────────────────────────────────┘
```

### 2.3 Tab 行为规则

| 触发 | 行为 |
|---|---|
| 首次打开 app | 主区域 1 个新标签:`+ 新建会话`(若侧栏无选中项目则显示项目选择提示) |
| 点侧栏 §4 项目下 session entry | 该 session 已开 tab → focus;未开 → 新 tab 打开 |
| 点 §2 "+ 新建会话" | 无项目选中 → toast "请先选个课程";有选中 → 新 tab 打开 New Run Config(pre-fill course) |
| 点 §5 ⚙ 设置 | Settings tab 已开 → focus;未开 → 新 tab 打开 |
| 点 tab × | 关闭该 tab |
| 点 tab 头部 | focus 该 tab |
| 至少保留 1 个 tab | 不能关闭最后一个;最后一 tab 时 × 灰显 |
| 启动时恢复 | `localStorage.mediaToDocTabs` 存 [{type:'session', runId} / {type:'settings'}];每次 tab 增删写回 |

### 2.4 Collapse 行为

| 状态 | 侧栏宽 | 显示内容 |
|---|---|---|
| 展开(默认) | 260px | §1-5 全部完整文本 |
| 收起 | 48px | §1 logo + §2-5 图标(无文本) |
| 切换 | 点 §1 ⮜ | 动画 150ms 过渡;状态存 localStorage(`mediaToDocSidebarCollapsed`) |

---

## 3. 数据流(零后端改动)

### 3.1 侧栏 §4 Project Tree 数据装配

```js
// 1. 列课程(项目) — 已有 Tauri command
const courses = await invoke('list_courses', { workspaceRoot });

// 2. 列所有 run — 已有 Tauri command
const allRuns = await invoke('list_all_runs');

// 3. 按 course 聚合(纯前端):
//    - 每个 course.path 找 runs.filter(r => r.work_dir.startsWith(course.path + '/output') 或 r.inbox_dir === course.path)
//    - 注意:目前 run summary 字段仅含 work_dir / status / started_at / finished_at / log_path,
//      没有显式 inbox_dir 字段;前端用 work_dir 的前缀('/output/' 段)反推 inbox。
//    - 反推:inbox = work_dir.parent.parent  或  work_dir.replace(/\/output.*$/, '')
//    - 若反推不一致 → 该 run 不归任何课程,放"未分类"组(W15-B+ 可加 inbox_dir 字段)
```

### 3.2 Session Tab 数据装配

```js
// 已开 tab 的 run_id → 定时轮询(2s 一次):
async function refreshSession(runId) {
  const status = await invoke('check_status', { workDir: runId });
  const logTail = await invoke('read_log', { workDir: runId, offset, limit: 200 });
  const stages = computeStages(status);
  // 更新 DOM
}
```

### 3.3 New Run Tab 提交

```js
async function submitNewRun(opts) {
  // opts 来自 tab 内表单
  const r = await invoke('run_pipeline', {
    inboxDir: opts.inboxDir,
    llm: opts.llm || null,
    imagegen: opts.imagegen || null,
    stopAfter: opts.stopAfter || null,
    noLongdoc: opts.noLongdoc || false,
    force: opts.force || false,
  });
  if (!r.ok) { toast('run_pipeline: ' + r.error, 'error'); return; }
  // 关闭 New Run Tab → 打开对应 Session Tab(work_dir 是 r.data.work_dir)
}
```

### 3.4 Settings Tab 数据

沿用 T6 已实装的 `loadProviders` / `applyPresetToForm` / `testProviderConnection` / `submitProviderForm`(索引 1064-1271)。Settings tab 切换到 active 时 `loadProviders()`。

### 3.5 Tab 状态持久化

```js
// 每当 tabs 集合变化(增/删/重排/focus),序列化到 localStorage:
function persistTabs() {
  const data = tabs.map(t => ({
    type: t.type,                    // 'new_run' | 'session' | 'settings'
    workDir: t.workDir || null,      // 仅 session
    coursePath: t.coursePath || null, // 仅 new_run
    active: t.id === activeTabId,
  }));
  localStorage.setItem('mediaToDocTabs', JSON.stringify(data));
}
// 启动时 restore(只 restore type + 关键标识,内容每次启动重拉)
```

---

## 4. UI 细节

### 4.1 视觉风格(对齐 Claude Code Haha 截图)

- **暗色主题**(沿用现有 CSS variable,不变)
- **侧栏**:背景 `#1f1f1f`,hover `rgba(255,255,255,0.05)`(已用),active `rgba(74,158,255,0.1)` + 左侧 3px accent
- **session entry**:课程名次行(缩进 16px),run 名次行(缩进 32px);running 状态点用 ●(黄/绿),completed ✓ 灰,failed ✗ 红
- **tab header**:每个 tab 头部 max-width 180px,文本溢出省略;× 按钮 hover 变红;active tab 底部 2px accent line
- **typography**:沿用现有 -apple-system + PingFang SC + Microsoft YaHei
- **spacing**:侧栏内 padding 8px;tab 间 gap 1px(border-style)

### 4.2 防 XSS

所有从后端读出的 user-controlled 字符串(course name / run work_dir / log text)渲染到 innerHTML 前过 `escapeHtml`(已有,行 1054-1056)。**log tail 渲染改用 `<pre>` + `textContent`(不是 innerHTML),天然防 XSS**。

### 4.3 collapse 动画

```css
.sidebar {
  transition: width 150ms ease-out;
  overflow: hidden;  /* 防止收起时内容溢出 */
}
.sidebar.collapsed { width: 48px; }
```

---

## 5. Settings 点击 bug 修复(3 步)

| 步骤 | 文件 | 改动 | 假设应对 |
|---|---|---|---|
| 5.1 capability allowlist | `src-tauri/capabilities/default.json` | permissions 加 6 个 LLM command 显式条目 | 假设 1:Tauri 2 默认拒未声明 command |
| 5.2 module 顶层 error handler | `src/index.html` `<script>` 顶部 | `window.addEventListener('error', ...)` 写 `%APPDATA%\com.duanyi.mediatodoc\init.log` | 假设 2:顶层 throw 致 addEventListener 未执行 |
| 5.3 引导用户强清缓存重装 | (无文件改动) | 用户卸载 + 删 `%LOCALAPPDATA%\com.duanyi.mediatodoc\` + `%APPDATA%\com.duanyi.mediatodoc\` + 重装 1.4.2 NSIS | 假设 3:WebView2 stale cache |

```json
// capabilities/default.json 改后:
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

> 注:Tauri 2 capability 语法以当前 `Cargo.toml` / `tauri = "2.x"` 实际要求为准。若某个版本需要 `core:` 前缀或 widget 形式,以 build 报错为准调整。下游新会话 build 一次验证。

```js
// index.html <script> 顶部加:
window.addEventListener('error', (e) => {
  try {
    const dir = '%APPDATA%/com.duanyi.mediatodoc';  // 实际通过 Rust log 通道写更好
    // 用 invoke 调一个专门的 log_init_error command;若无就降级到 console + toast
    console.error('[init-error]', e.error || e.message);
  } catch (_) {}
});
// + module 顶层 try/catch 包住后续 IIFE
```

降级方案(若 5.2 写文件不便):只在 console error + 加一个红色 toast "Init error,看 console",新会话再决定要不要扩。

---

## 6. 组件 / 模块划分

### 6.1 前端模块(index.html 内 `<script>` 分块,沿用 W14-B+ 结构)

```
┌─────────────────────────────────────────────────────────┐
│ 0. Tauri IPC 入口 + state                                │
│ 1. Toast 工具                                            │
│ 2. App info 加载(loadAppInfo)                            │
│ 3. Sidebar 5 段渲染 + collapse toggle                    │
│ 4. Sidebar §4 project tree(refreshCourses + run agg)      │
│ 5. Sidebar §2 fixed actions handler                      │
│ 6. Tab manager(openTab / closeTab / focusTab / persist)   │
│ 7. Session tab content(check_status + read_log + stages)  │
│ 8. New Run tab content(form submit → invoke run_pipeline) │
│ 9. Settings tab content(沿用 T6 loadProviders + 4 子页)     │
│ 10. boot(依次调用 loadAppInfo / refreshCourses /         │
│         restoreTabs / focus first tab)                   │
└─────────────────────────────────────────────────────────┘
```

### 6.2 后端模块

**零改动**。所有已有 command 已就位:`list_courses` / `list_all_runs` / `check_status` / `read_log` / `run_pipeline` / `resume_pipeline` / `cancel_run` / `app_info` / `list_llm_profiles` / `get_active_llm_profile_name` / `save_llm_profile` / `set_active_profile` / `delete_llm_profile` / `test_llm_connection`。

唯一非业务改动:`capabilities/default.json` 加 6 行 permissions(§5.1)。

---

## 7. 测试策略

| 层 | 覆盖 | 工具 |
|---|---|---|
| Rust 单元测试 | 零改动;现有 98 / 98 pass 应保留(本 spec 不动 Rust 业务代码,只改 `capabilities/default.json`) | `cargo test --lib` |
| 前端逻辑 | 不引入新 JS 测试(沿用 W14-B+ 决策:纯前端改由手动验收覆盖);新会话可在 tab manager / collapse 这两个函数上加 console.assert 单测,不强求 | 手动验证 + console |
| Build 验证 | `cargo tauri build` 必须 exit 0;新会话第一轮 build 必看 stderr | `cargo tauri build` |
| 端到端 | NSIS 装机 → 13 项验收(§8) | 用户桌面手测 |

**理由**:W14-B+ 已确立"前端改动靠手测,Rust 改动靠单测"的边界。本 spec 仅前端 + capabilities 改动,Rust 业务零改动,所以验证策略就是手测 + build。

---

## 8. 验收清单(13 项,装机后用户桌面跑)

> 替代原 spec `2026-07-23-w15-a-llm-api-settings-design.md` §8 的 13 项(原列表已 BLOCKED 在 Settings 点击 bug 上)。本表只覆盖本次重设计 + Settings 链路。

| # | 步骤 | 期望 | 关联 |
|---|---|---|---|
| 1 | 卸载旧版 + 删 `%LOCALAPPDATA%\com.duanyi.mediatodoc\` + `%APPDATA%\com.duanyi.mediatodoc\` + 装 1.4.2 NSIS | 装机成功 | 清缓存(假设 3) |
| 2 | 启动 app | 看到新侧栏 5 段,无 header | §2.1 |
| 3 | §1 ⮜ collapse 按钮 → 再点 | 侧栏 260px ↔ 48px 切换;状态保留到重启 | §2.4 |
| 4 | §3 搜索框输入 "课程A" | §4 项目树只显示匹配课程(其它折叠/隐藏) | §3.1 |
| 5 | §4 项目展开按钮 + session entry 点击 | session 在主区域开新 tab | §2.3 |
| 6 | §2 "+ 新建会话"(侧栏已选项目) | 主区域开 New Run Tab,课程预填 | §2.2.2 |
| 7 | §2 "+ 新建会话"(侧栏未选项目) | toast 提示 "请先选个课程" | §2.3 |
| 8 | New Run Tab 提交 → run_pipeline | 关闭该 tab,开对应 Session Tab | §3.3 |
| 9 | Session Tab 暂停/取消/resume 按钮 | 调对应 command 成功 + UI 状态更新 | §2.2.1 |
| 10 | §5 ⚙ 设置 | Settings tab 在主区域打开 | §2.2.3 |
| 11 | Settings → Providers → 添加 DeepSeek → 测试连接 → 保存 → 激活 | 6 步全跑通(对应 T6 实装的 UI) | T6 handoff §3 |
| 12 | tab × 关闭 | tab 关闭;最后一 tab × 灰显 | §2.3 |
| 13 | 重启 app | 之前开的 tabs 全部恢复;collapse 状态保留;激活 provider 保留 | §3.5 |

**通过条件**:13 项中 ≥ 11 项 PASS,其余若因本 spec 设计缺陷导致可标 FAIL 并写 handoff `-blocked-`。

---

## 9. 风险与回避

| 风险 | 影响 | 回避 |
|---|---|---|
| Settings 点击 bug 根因误判(3 假设之一错) | 装机后 Settings 仍不弹 | §5 三步全部做:**capability + error handler + 清缓存**;任意一步修了即可 |
| WebView2 cache 拒不清 | 装机后仍跑旧 HTML | §8.1 强制引导;若仍不行,把 index.html 加 cache-bust query(`?v=v1.4.2`) |
| Tauri 2 capability 语法误 | build 失败 | §5.1 已注明"以 Cargo.toml 实际版本为准,build 报错调整" |
| session entry 反推 inbox 不准 | run 归错课程 | §3.1 注明"反推不一致 → 未分类组",而不是归错;W15-B+ 加 inbox_dir 字段 |
| tab 数量爆炸(用户连开 10 run) | 侧栏 + 主区域渲染慢 | §2.3 没限制 tab 数;若性能成问题,W15-B+ 加 LRU + 关闭最早已完成的 tab |
| Collapse 收起时文字溢出 | 视觉破碎 | CSS `overflow: hidden` + `text-overflow: ellipsis` 仅图标区显示 |
| Log tail XSS | 注入风险 | §4.2 强制 textContent 渲染 + escapeHtml 兜底 |

---

## 10. 加快模式规则(沿用 W15-A)

- 不 commit / push / release / reset 未提交改动
- 不切回 master 直接开发
- 不删除旧 handoff / prompt
- 不启 sandbox feature(W14-G 已知阻塞)
- 不 bump version 进 v1.5.0(T8)
- **本会话不 commit**(T7 prompt 红线)
- **不动主仓 `F:/soft/00selfmade/media-to-doc/`**

---

## 11. 历史与关系

- **承接 handoff**:`handoff-w15-a-t7-failed-redesign-pivot-2026-07-24.md`(本 spec 直接响应 §4 UX 重设计规格)
- **T6 handoff**:`handoff-w15-a-t6-complete-2026-07-24.md`(保留 Settings 4 子页 + 9 provider UI + keyring)
- **原 spec**:`2026-07-23-w15-a-llm-api-settings-design.md`(本 spec 不修改;它的 §8 13 项验收由本 spec §8 替代)
- **plan**:`2026-07-23-w15-a-llm-api-settings.md` T7 验收口径由本 spec §8 替代
- **新会话接力**:`handoff-w15-a-t7-1-redesign-complete-2026-07-24.md`(成功后)或 `handoff-w15-a-t7-1-blocked-2026-07-24.md`(仍卡)

---

## 12. Spec 自审(写完立即做)

1. **Placeholder scan**:无 TBD / TODO。所有 §2-9 都是具体决策。
2. **Internal consistency**:§2.2.1 / §2.2.2 / §2.2.3 三类 tab 内容互不重叠;§3 数据流说明与 §2 UI 需求一一对应。
3. **Scope check**:聚焦"前端 UX 重设计 + Settings bug 3 步修"。后端零改动。W15-B+ 留出"定时任务 / 技能市场 / inbox_dir 字段 / 完整 General & Theme & About 子页 / LRU tab"。
4. **Ambiguity check**:
   - "session entry 反推 inbox":§3.1 已给反推规则 + fallback "未分类"。
   - "tab 状态恢复":§3.5 已给 localStorage key 命名。
   - "跑 run 时 New Run Tab 是否要 confirm":§2.2.2 提交后自动关闭 + 开 Session tab,无需 confirm。

✅ Self-review 通过,提交用户 review。
