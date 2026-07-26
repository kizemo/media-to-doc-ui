# Handoff — W15-A T7 验收失败 + UX 重设计转向

**日期**:2026-07-24
**项目**:`F:/soft/00selfmade/media-to-doc-ui`
**当前分支**:`feat/w15a-llm-api-settings`(基线 `db84639`,未 push,未 commit)
**承接 handoff**:`handoff-w15-a-t6-complete-2026-07-24.md`
**状态**:**T7 验收 1/13 PASS,12 项阻塞于 Settings 点击 bug**;同时用户要求 UX 重大重设计(Claude Code Haha 风格)

---

## 0. 下一会话先做什么(非技术说明)

**关键认知:这不是修 bug,这是重做产品形态。** 用户桌面反馈 3 条已把 W15-A 验收从"看 Settings 是否能用"推到"砍掉一半侧栏 + 删蓝色顶栏 + 仿 Claude Code Haha 重做侧栏"。下一会话顺序:

1. **删 5 个主入口 + 重做侧栏**:Inbox / Run / Output / Health / Learn 全部从主侧栏下掉,改成 Claude Code Haha 那种"新建会话 / 定时任务 + 搜索框 + 项目列表"布局。**Settings 保留**作为齿轮放底部,跟 Claude Code Haha 一致。
2. **删 UI 顶部蓝色 title bar**(`<header>` 整段),把"media-to-doc" logo 移到左上角侧栏头部(像 Claude Code Haha 那种带 logo + collapse 箭头的顶)。
3. **修 Settings 点击 bug**(见 §3 根因调查 + 验证清单):点击不切 tab、不弹 Providers modal。极可能是 capabilities 缺 LLM commands 权限 + WebView2 stale cache + module init throw 三者其一。
4. 跑通 13 项验收(spec §8)的 Settings 链路(只剩 6 个 LLM command + 9 provider + active 切换 + env var 注入)。
5. 不 commit / push / release / bump v1.5.0,继续"W15-A 整体一次 commit"加快模式。
6. Settings tab 修好后,跑 T7 全部 13 项(只过 Settings 链路即可;原 5 个 tab 内容已删,验收改成"Claude Code Haha 风格侧栏 + 项目列表工作流")。

**用户反馈原文**(2026-07-24 反馈):
- "如附图,左侧有 Inbox/Run/Output/Health/Learn/Settings 几个选项卡。除了 setting 外,其他几个都去除,修改为类似 claude code huahua 左侧选项卡的:新建会话 定时任务,下面应为 项目列表,如附图"
- "去除 UI 顶部的蓝色栏"
- "测试点击 setting 没有反应,无法调出添加 Providers 的界面,其他项无法测试。"
- "请调用 bug 检修工作流,检查安装日志/使用日志,分析 bug 原因。"
- "请撰写 handoff 文件,包含上述反馈,并撰写新开会话的 prompt,保证任务的顺利推进。"

---

## 1. T7 验收结果(用户桌面)

| # | 步骤 | 期望 | 实际 | 状态 |
|---|---|---|---|---|
| 1 | 装 v1.4.2 + 启动 | 6 tab 显示 | ✅ 6 tab 全部显示 | **PASS** |
| 2 | Settings → Providers | 空列表 + [+ 添加] | ❌ **Settings 点击无反应** | **FAIL** |
| 3-13 | 添加 DeepSeek/激活/重启/Run pipeline/env 注入/... | spec §8 全链 | 阻塞于 #2 | **BLOCKED (12 项)** |

**总评**:**1/13 PASS,12/13 BLOCKED**。build 本身成功(3m01s, exit 0, 5 warnings 与 T6 一致),T1 keyring race 修复后 98/98 测试 pass,问题在 T6 前端 UI 没有按预期工作。

---

## 2. 当前 git 工作区状态

```
$ git -C media-to-doc-ui status --short --branch
## feat/w15a-llm-api-settings
 M docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md
 M docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md
 M src-tauri/Cargo.toml
 M src-tauri/src/commands.rs
 M src-tauri/src/lib.rs
 M src-tauri/src/runner.rs
 M src/index.html
 M src-tauri/src/keyring_store.rs         ← T1 race fix(本会话)
?? handoff-w15-a-providers-decided-v142-icon-2026-07-24.md
?? handoff-w15-a-t2-complete-2026-07-24.md
?? handoff-w15-a-t3-complete-2026-07-24.md
?? handoff-w15-a-t4-complete-2026-07-24.md
?? handoff-w15-a-t5-complete-2026-07-24.md
?? handoff-w15-a-t6-complete-2026-07-24.md
?? handoff-w15-a-t7-failed-redesign-pivot-2026-07-24.md  ← 本文件
?? handoff-w15-a-v141-release-2026-07-24.md
?? prompt-next-session.md
?? prompt-w15-a-t5-next.md
?? prompt-w15-a-t6-next.md
?? src-tauri/src/keyring_store.rs
?? src-tauri/src/llm_profiles.rs
?? task.md
```

**新增 untracked(本会话)**:
- `handoff-w15-a-t7-failed-redesign-pivot-2026-07-24.md`(本文件)
- `prompt-w15-a-t7-redesign-next.md`(接力 prompt,见本目录)
- `src-tauri/src/keyring_store.rs` **modified**:`TEST_PROFILE` 共享 race 修复(3 测试改独立 profile name),98/98 pass

**build 产物**(本会话 18:04 出炉,未分发):
```
F:/soft/00selfmade/media-to-doc-ui/src-tauri/target/release/bundle/nsis/media-to-doc_1.4.2_x64-setup.exe
2,612,797 bytes  (Jul 24 18:04)
```
**该 build 仍未含修复后的 Settings bug 调试补丁**(我没改业务代码),仅含 T1 race 修复 + 原 T1-T6 代码。用户当前桌面上装的极可能是这一版,新会话再 build 一次后用户重装即可。

---

## 3. Settings 点击 bug 根因调查(systematic-debugging 4 phase 走查)

### 3.1 Phase 1:证据收集(实际查到)

| 检查项 | 结果 | 解读 |
|---|---|---|
| `%APPDATA%\com.duanyi.mediatodoc\llm_profiles.json` | 存在 | Tauri 进程写过 settings 元数据 |
| `%APPDATA%\com.duanyi.mediatodoc\logs\` | 不存在 | Tauri 端没开 log file,只能靠 console |
| `%LOCALAPPDATA%\com.duanyi.mediatodoc\EBWebView\` | 存在(AutoLaunchProtocols / Crashpad / Default 等子目录) | WebView2 cache,**可能 stale HTML** |
| `D:/training/inbox` workspace | 用户截图显示路径,沙箱不可见 | 用户用 D 盘,工作目录正常 |
| `capabilities/default.json` | `permissions: ["core:default"]` | **未列 6 个 LLM command** |
| `index.html` 行 614-626 nav-item click handler | 存在,挂 6 个 .nav-item | 代码层正确 |
| `index.html` 行 1064 `loadProviders` function declaration | hoisted,模块顶层 | 代码层正确 |
| `index.html` 行 485 `#tab-settings` div + 行 541 `provider-modal-backdrop` | 存在 | DOM 层正确 |
| `<script type="module">` 加载顺序 | module 默认 deferred | 与 click handler 顺序无关 |
| `lib.rs` invoke_handler 行 99-104 | 6 LLM commands 已注册(T4 阶段) | Tauri 端 OK |

### 3.2 Phase 2:模式对比

- **Tauri 2 IPC permission 默认**:`#[tauri::command]` 注册的命令默认允许调用,capability 用于显式限制。但**新版本 Tauri 2.x**(项目用 2.x,具体 patch 见 `Cargo.toml`)有 stricter 默认,某些版本要求 capability 显式 allowlist。**需要新会话核对当前 Tauri 版本对 capability 的实际行为**。
- **WebView2 stale cache**:Tauri 2 dev/prod 切换时,`EBWebView/Default` 缓存可能保留旧 `index.html` 解析,导致 module 顶层 throw 后 click handler 没注册但用户没察觉。**首次新会话应让用户清缓存 + 重装**。
- **Claude Code Haha 截图分析**:参考实现侧栏结构是"logo + 折叠按钮 / 搜索框 / 项目分组 / 底部设置"四段,**没有任何子 tab**,Settings 是齿轮在底部,与 T6 设计的 6 tab + Settings 6 子页**根本是两种产品形态**。这进一步佐证用户要的是产品级重设计。

### 3.3 Phase 3:三大最可能假设(按概率)

1. **【高】Tauri 2 capability 拒绝 6 LLM IPC 调用** → `loadProviders` 内 `await invoke('list_llm_profiles')` 抛 permission error → catch 块只更新 `#provider-list` 文案,**不切 tab**。但用户报告"无反应"包括 tab 不切 → 此假设不能完全解释,除非 click handler 本身也 throw。
2. **【中】Module 顶层 throw 致 click handler 未注册**:`<script type="module">` 顶层任何 throw 都会让 `addEventListener` 那一行不执行。**验证方式**:在 `<script>` 顶部加 `try/catch` 写 `init.log`。
3. **【中】WebView2 stale cache** → 装新版后 WebView 仍跑旧版 HTML(没 Settings tab pane 的版本),所以 click 切换 tab 找不到 `#tab-settings` → `classList.add('active')` 静默无效。**验证方式**:卸载 + 删 `%LOCALAPPDATA%\com.duanyi.mediatodoc\` + 重装。

### 3.4 Phase 4:新会话必做验证(已写到 prompt)

1. **强清缓存重装**:`%LOCALAPPDATA%\com.duanyi.mediatodoc\` + `%APPDATA%\com.duanyi.mediatodoc\` 全删 + 卸载 + 重装 1.4.2 NSIS → 排除 cache
2. **显式列 6 LLM commands 到 capability**:`capabilities/default.json` permissions 数组加 `["core:default", "list_llm_profiles", "get_active_llm_profile_name", "save_llm_profile", "set_active_profile", "delete_llm_profile", "test_llm_connection"]`(或按 Tauri 2 实际语法)
3. **加 module 顶层 error handler**:`window.addEventListener('error', ...)` 写 `%APPDATA%\com.duanyi.mediatodoc\init.log`,把 init 错误留证据
4. **开发 build 启 DevTools**:`tauri.conf.json` `"app.windows[0]"` 加 `"devtools": true`(仅 dev 模式),用户 F12 看 console
5. **重 build + 装 + 跑 13 项**(注意:**只剩 Settings 链路可验**,其他 5 tab 已删)

---

## 4. UX 重设计规格(从用户截图 + 反馈提取)

### 4.1 删 5 个主入口 + 重建侧栏(Claude Code Haha 风格)

**当前侧栏**(`index.html` 行 358-365):
```
nav-item: Inbox / Run / Output / Health / Learn / Settings
```

**新侧栏**(参考附图 2 Claude Code Haha):
```
┌─ 顶部:logo(media-to-doc) + collapse arrow(<) ──────┐
│                                                     │
│  + 新建会话                                          │
│  ⏰ 定时任务                                          │
│  ⛏ 技能市场(可选)                                    │
│                                                     │
│  🔍 搜索聊天 [⌘K]    🔄    🗑                          │
│                                                     │
│  项目                                                │
│                                                     │
│  📁 rime_claude (collapsed)                          │
│     - Fluxing v0.19.0...   4分前                      │
│     - Phase L-2          1小时前                      │
│     ...                                              │
│     展开显示                                          │
│                                                     │
│  📁 media-to-doc (expanded)                          │
│     - W15-A T7           49分前  ← 当前               │
│     - W15-A T6           1小时前                      │
│     - W15-A T5           1小时前                      │
│     - w15-a-t4           2小时前                      │
│     ...                                              │
│     展开显示                                          │
│                                                     │
│  📁 00数据汇总分析                                    │
│     - pgSQL初始化_ERP   9天前                         │
│     - 补充进销存数据     10天前                        │
│                                                     │
│  ⚙ 设置 (底部)                                       │
└─────────────────────────────────────────────────────┘
```

**实现要点**:
- **3 个固定项**(`新建会话` / `定时任务` / 可选 `技能市场`)在搜索框上方
- **搜索框** + 历史按钮(🔄)+ 删除按钮(🗑)紧贴固定项下方
- **项目列表**:按项目名分组(📁 + 项目名 + 折叠按钮),每个项目下显示该项目下的"会话/任务"列表(对应现有 13 项验收里的 5 个 tab 内容,变成项目内的子视图,而不是平级 tab)
- **底部设置**齿轮,点击进 Settings(providers/llm profiles 仍可访问)
- **collapse 箭头** `(<)` 把侧栏收到左侧细条,只显示图标(media-to-doc Logo + 4 个图标 + 设置齿轮)

**W15-A 后端命令无需改**:`list_courses` / `run_pipeline` / `list_outputs` / `get_run_metrics` / `list_runs` 仍由后端提供,前端只是把它们从"主 tab 网格"重组成"项目树"。

### 4.2 删 UI 顶部蓝色 title bar

**当前**(`index.html` 行 349-356):
```html
<header>
  <h1>media-to-doc</h1>
  <span class="badge" id="version-badge"></span>
  <div class="status">
    <span class="status-dot" id="status-dot"></span>
    <span id="status-text">loading…</span>
  </div>
</header>
```
**CSS** 让 `header` 蓝色背景,带状态点 + 状态文字 + 版本 badge。

**改造**:
- 整个 `<header>` 标签删除
- `media-to-doc` logo + collapse arrow 移到侧栏顶(像附图 2 那样)
- `version-badge` + `status-dot` + `status-text` 移到 Settings → About 子页(已存在的 app info 区)
- `app_info` IPC 调用保留(只是不挂在 header 了)

### 4.3 Settings 入口保留

- 侧栏底部齿轮,点开进 Settings tab(全屏覆盖,不是子页面)
- Settings 内保留 4 子页:Providers / General / Theme / About(实装的只有 Providers,后 3 个 W15-B+ 留)
- 这是 T6 唯一保留下来的子页面

---

## 5. 加快模式规则(沿用 T6)

- W15-A 整体一次 commit(feature commit),**不分多次 commit**
- 不 reset / checkout / restore / 覆盖未提交改动
- 不切回 `master` 直接开发
- 不删除旧 handoff / prompt(删除需用户二次确认)
- **不启 sandbox feature**(W14-G 已知 Win11 沙箱功能阻塞)
- **不 bump version 进 v1.5.0**(T8 才做)
- **T7 prompt 红线**:不 commit / push / release / reset / 改主仓

---

## 6. 下一会话任务清单(新会话直接读,无需再问)

| Task | 内容 | 必交付 | 优先级 |
|---|---|---|---|
| **T7.1** | **强清缓存重装验证 Settings 链路** | `%LOCALAPPDATA%` + `%APPDATA%` 删 + 卸载 + 重装 1.4.2 → 点 Settings → 能否进 Providers 列表 | P0 |
| **T7.2** | **加 capability 显式 allowlist** | `capabilities/default.json` permissions 加 6 LLM command 显式条目 | P0(假设 1) |
| **T7.3** | **加 module 顶层 error handler** | `index.html` `<script>` 顶部加 `window.addEventListener('error', ...)` 写 `init.log` | P0(假设 2) |
| **T7.4** | **删 5 tab + 删 header** | `index.html` 删 nav-item 5 个 + 删 `<header>` 整段 | P1 |
| **T7.5** | **重建 Claude Code Haha 风格侧栏** | 顶部 logo+collapse / 3 固定项 / 搜索框 / 项目树(按 inbox 课程名分组)/ 底部设置齿轮 | P1 |
| **T7.6** | **重 build 1.4.2 + 装机 + 走 Settings 链路 13 项** | `cargo tauri build` 一次,装机后跑 13 项;原 5 个 tab 验收改成"项目树展开/折叠 + Settings 链路" | P1 |
| **T7.7** | **写 handoff** | 写 `handoff-w15-a-t7-1-redesign-complete-2026-07-24.md`(若成功)或 `handoff-w15-a-t7-1-blocked-2026-07-24.md`(若仍卡) | P2 |
| **T8** | v1.5.0 release | 沿用原 plan T8,等 T7 全过 + 用户拍板 | P3 |

---

## 7. 必读顺序(新会话第一句之后)

1. 本文件
2. `prompt-w15-a-t7-redesign-next.md`(接力 prompt,30 行内)
3. `handoff-w15-a-t6-complete-2026-07-24.md`(T6 详情,SPA + Settings UI)
4. `handoff-w15-a-providers-decided-v142-icon-2026-07-24.md`(加快模式规则 + 9 provider 决策)
5. `src/index.html` 行 349-365(原 header + nav)+ 行 485-579(Settings tab pane + provider modal)
6. `src-tauri/capabilities/default.json`(需改)
7. `docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md` §8(13 项验收,部分项需改口径)
8. `docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md` Task 8(13 步手动验收,部分项需改口径)
9. `git status --short --branch`

---

## 8. 事实面状态

| 事实面 | 状态 | 证据 |
|---|---|---|
| 代码 | changed-and-regressed | T1 race 修复 + T6 UI 保留,Settings 点击仍坏 |
| 测试 | 98/98 pass | 本会话 `cargo test --lib` 一次过 |
| Build | v1.4.2 NSIS 出炉 | 18:04, 2,612,797 bytes, 5 warnings |
| 运行态 | failing-on-user-machine | Settings tab 点击无反应 |
| 文档 | in-flight | spec §8 / plan Task 8 旧 12-provider 描述待总同步 |
| 规则 | verified-current | feature 分支、TDD、一次性 commit 规则已遵守 |
| 工作区 | pending | W15-A 未提交改动与历史残留文件均保留 |

---

## 9. 已知避坑

- **不要 bump v1.5.0** in T7.4-T7.6:仍是 v1.4.2 装机,bump 推迟到 T8
- **不要直接改 master**:所有改动在 `feat/w15a-llm-api-settings` 上累积
- **不要删 5 tab 内容对应的后端命令**:`list_courses` / `run_pipeline` / `list_outputs` / `get_run_metrics` / `list_runs` 保留,只是前端从 tab 重组成项目树
- **不要 reset T6 的 Settings 4 子页布局**:Providers 子页 + 子页切换 CSS 完整保留,新会话只加侧栏 + 删 header
- **不要 merge master** 任何加快模式中混进来的 fix(若有)
- **不要改主仓 `media-to-doc/`**(mtd Python 端零改动 — env var 注入沿用 W14-D trust_env=False 路径)

---

## 10. 新会话第一句

> 承接 `F:/soft/00selfmade/media-to-doc-ui/handoff-w15-a-t7-failed-redesign-pivot-2026-07-24.md`,T7 验收 1/13 PASS 后用户要求 UX 重大重设计:删 5 主入口 + 删 header + 仿 Claude Code Haha 重建侧栏。Settings 点击 bug 需先修(强清缓存 + capability allowlist + module error handler 三步验证)。本会话任务 T7.1-T7.7,详见 handoff §6。

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
