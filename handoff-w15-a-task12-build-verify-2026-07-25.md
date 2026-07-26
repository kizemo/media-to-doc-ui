# Handoff — W15-A Task 12: Build + 真机验证 + 最终 Handoff

**日期**: 2026-07-25 凌晨
**项目**:`F:/soft/00selfmade/media-to-doc-ui`
**当前分支**:`feat/w15a-llm-api-settings`(BASE `073b05e`,**无新 commit**,加快模式)
**承接**:`handoff-sdd-w15a-tasks6-12-2026-07-24.md` + 本会话已完成的 Tasks 6-11

---

## ⚠️ 0.5 用户真机验证反馈(2026-07-25 上午装机后)

用户已按 §2 步骤装机 1.4.2 NSIS,跑 13 项验收时报告 **3 个 visible bug + 1 个 functional bug**:

| # | 用户描述 | 实际症状(从代码 + 截图反推) |
|---|---|---|
| 1 | "顶部多余的蓝色栏未取消" | 主区域顶部有一条深灰色长条(`background: var(--bg-card)` = #252525)— 这其实是 **`<div class="tab-bar" id="tab-bar">`**(Task 8 留的空 div,min-height 36px + bg #252525)。不是蓝色栏,是 .tab-bar 占位没填充 |
| 2 | "右侧缺少会话窗口" | 主区域整片黑/空 — `<div class="tab-content-host">` 同理没内容 |
| 3 | "所有按钮都没有功能" | §1 ⮜ collapse / §2 + 新建会话 / §2 定时任务 / §3 搜索 / §3 🔄 / §3 🗑 / §5 设置 全部点击无反应 |

### 根因结论与修复(2026-07-25 复诊)

**根因已确认:H1 成立，但 throw 不在任一 init 函数内。** 实际中断点是 `src/index.html:801`：

```js
const { invoke } = window.__TAURI__.core;
```

`src-tauri/tauri.conf.json` 原先未配置 `app.withGlobalTauri`；本地 Tauri CLI 2.11.4 schema 明确该字段默认 `false`，因此 WebView 不注入 `window.__TAURI__`，module 在调用所有 init 之前即抛 `TypeError: Cannot read properties of undefined (reading 'core')`。这同时解释静态侧栏可见、tab bar/content 为空、全部按钮无 listener。

**最小修复**：只在 `src-tauri/tauri.conf.json` 添加 `"withGlobalTauri": true`。未改 `src-tauri/src/*.rs`，未 bump、未 commit。`initSidebarActions()` 两个目标 DOM 均存在，不是根因，未做无证据的 null-guard 改动。

**证据**：Node harness 复现同一 TypeError；配置断言 RED→GREEN；独立 reviewer 无 Critical/Important 阻塞；`cargo test --lib` 98/98；`cargo tauri build` exit 0。新 installer：2,630,443 bytes，2026-07-25 09:50:53，SHA256 `a84f301dd5db6b38b88a0f04d1ffd608727907a6ec467f955d22e487e82bd5b8`。

### 原诊断假设(已归档)

**H1:JS boot throw 导致全部 init 没跑**
- 证据:tab bar + tab content host 是 Task 8 已挂的 DOM,但因为 `initTabManager()` 没运行 → `rebuildTabBar()` / `rebuildContent()` 没被调 → 内容空
- 证据:所有 init 函数都没有绑 listener → 说明 boot 调用链在某一行 throw 后中断
- **最具体嫌疑**:`initSidebarActions()` 行 848-849 没 null 守卫:
  ```js
  function initSidebarActions() {
    $('sidebar-new-session-btn').addEventListener('click', handleNewSessionClick);
    $('sidebar-schedule-btn').addEventListener('click', () => toast('定时任务 — W15-B+ 实装', 'info'));
  }
  ```
  其他 init 函数(`initSidebarCollapse` / `initProjectTree` / `initSettingsGear`)都用 `if (btn)` 守卫,这个没有。如果 `#sidebar-new-session-btn` 在 DOM(已确认存在,行 541),那这个不是 throw 点。但**这是 evidence collect 的入手点**

**H2:WebView2 缓存(W14-G+ 已知问题)**
- 证据:W14-G+ 撞过 — WebView2 cache 在 `%LOCALAPPDATA%\com.duanyi.mediatodoc\EBWebView\` 可能拒不清,导致 app 加载 stale HTML
- 用户的强清缓存(§2 步骤 A)清了 LOCALAPPDATA + APPDATA 顶级目录,但 WebView2 子目录可能漏
- 修法:`Remove-Item -Recurse -Force "$env:LOCALAPPDATA\com.duanyi.mediatodoc\EBWebView"` 然后重装;或在 `index.html` 加 `?v=v1.4.2` cache-bust

**H3:某种 race condition / 时序**
- `<script type="module">` 是 deferred,但 module top-level `await loadAppInfo()` 应该 OK
- 可能性低

### 诊断优先级(新会话第一步)

1. **打开 WebView2 DevTools 看 console 错误**:
   - Tauri 2 WebView2 默认禁用 F12 / 右键菜单。需用户按 F12 看 console(若 webview 启用)
   - 替代:`%APPDATA%\com.duanyi.mediatodoc\init.log` 应有 module error handler 写的 log(行 780-797,实际只是 console.error,因 browser 不能写文件)
   - **真正诊断路径**:让用户按 F12 / Ctrl+Shift+I,如果 Tauri 没禁用
2. **检查 `index.html` 是否真带 W15-A 改动**:grep `__mountSettingsTab__` / `__mountSessionTab__` / `tab-bar` 应有内容
3. **检查 boot 链 throw 点**:从 line 2149 `buildPresetOptions()` 开始 → 2150-2158 modal listeners → 2161-2167 boot,逐行 instrument 加 try/catch console.error

### 后续修复方向

| 方向 | 修法 |
|---|---|
| H1 真 throw | 在每个 init 函数开头加 `try { ... } catch (e) { console.error('[init] <fn>', e); }` 包裹,定位 throw 点 |
| H1 防御加固 | `initSidebarActions()` / 所有 init 函数统一加 null 守卫(`if (btn) btn.addEventListener(...)`) |
| H2 cache-bust | `<script src="https://unpkg.com/marked@12.0.0/marked.min.js?v=v1.4.2">` 类似手法在 main asset 也加 `?v=v1.4.2` |
| H2 完全清 | `Remove-Item -Recurse -Force "$env:LOCALAPPDATA\com.duanyi.mediatodoc\EBWebView"` |

### 反馈截图参考

用户发的截图显示:
- §1-§5 侧栏 DOM 全部 visible(说明 HTML 正确加载)
- §4 显示 "(loading…)"(说明 list_courses IPC 还没返回 — 可能是 initProjectTree 没跑 OR 跑了但 list_courses 失败)
- 主区域顶部一条灰色横条(就是 .tab-bar 占位)+ 下方大片黑色(就是 .tab-content-host 占位)

## ⚠️ 0.6 用户第二轮真机反馈与下一阶段范围(2026-07-25)

### 已验证到的运行态

- `withGlobalTauri=true` 修复已让前端启动链恢复；用户已能打开 Settings 并成功添加一个 MiniMax provider。
- 这只证明“添加 provider”链路可达，**不等于原 13 项已完成**；Task 12 仍未达到 ≥11/13 的完成门槛。
- 新反馈改变了 T8 前置条件：**先完成下表 P0/P1，再做 v1.5.0 release**，本 handoff §6 原“直接进入 T8”已被替换。

### 用户 5 项反馈 → 事实与必交付

| 优先级 | 用户反馈 | 当前事实 | 下一会话必交付 |
|---|---|---|---|
| **P0** | 已添加 MiniMax，但 New Run 的 LLM 下拉没有该项；Image Agent 也应能选在线大模型 | `src/index.html:967-980` 两个下拉是静态协议名；Rust 启动时只注入“全局 active profile”，并没有 per-run profile 参数。`imagegen` 当前只是 `skip/local_sdxl`，且 `LocalSdxlProvider` 仍写空占位文件 | LLM 下拉改为实时读取已保存 profiles，按 **profile name** 逐次选择，不能靠切换全局 active 造成并发串号；后端从所选 profile 派生 CLI provider + env。另设 `Image Agent profile` 选择并明确“文本模型负责配图策划/prompt”与“真正出图 provider/API”两层，不把普通文本 LLM 假装成图片模型 |
| **P0** | New Run 应有会话框发布任务；会话框下方可选目录；所选目录自动成为左侧项目，已存在则合并 | 当前 New Run 只有结构化 form；左侧项目只扫描默认 workspace，没有任意目录注册表，也没有自由任务文本入口 | 做 chat-style textarea + native directory picker；持久化 project registry；用规范化绝对路径作为项目身份，同一路径去重并合并 sessions，重名不同路径要区分；定义 task text 如何进入 pipeline/LLM，而不是只在 UI 展示 |
| **说明** | Stop after 是什么 | `run_pipeline(..., stop_after=...)` 会跑完指定阶段后正常停下并保存 state，之后可 resume；不是失败或取消 | UI 改成中文说明/tooltip；`none`=完整运行。阶段含义见下表 |
| **P2 / parked** | 定时任务不能点击 | 当前按钮按 spec 故意 disabled，W15-B+ 占位 | 本阶段不实现；保持灰显并显示“后续版本提供”，不要算验收失败 |
| **P1** | 检查 `long-doc-processor` 是否整合，并保证以后在 Claude 修改后自动同步到项目 | **尚未整合**：Python `longdoc.py` 仅注释“参考 Skill”，prompt/CSS/流程是独立副本；UI 只透传 `--no-longdoc`；两仓都没有 vendored Skill、symlink、同步脚本或 hook | 建立可分发的 vendored snapshot + SHA256 manifest + sync/verify 脚本；Claude `PostToolUse(Edit|Write)` hook 检测 Skill 真身路径变化后触发同步。运行时使用项目内 snapshot，不能依赖用户机器的 `~/.claude` 路径；不要用需管理员/开发者模式的 symlink |

### Stop after 非技术说明

选择后，程序会“做到这一站就先停”，中间结果已保存，检查满意后可继续：

| 选项 | 做到这里为止 |
|---|---|
| `audio` | 从视频提取声音 |
| `asr` | 把声音转成文字 |
| `frames` | 提取关键画面 |
| `ocr` | 识别画面里的文字 |
| `asr_correct` | 用画面文字校正转写 |
| `chapters` | 整理章节结构 |
| `draft` | 生成分章讲义草稿 |
| `imagegen` | 处理 AI 配图 |
| `render` | 生成 Markdown / HTML |
| `longdoc` | 深度净化并整理最终长文 |
| `verify` | 完成最终质量检查（完整流程终点） |

### `long-doc-processor` 整合审计结论

- Skill 真身：`C:/Users/Duanyi/.claude/skills/long-doc-processor/`（当前 v4.0 系列）。
- 主仓现状：`src/media_to_doc/pipeline/longdoc.py:26-27,54-86,180` 只借鉴规则并内嵌 prompt；没有读取 Skill 文件。
- UI 现状：仅把 `longdoc` 当 stage 名并透传 `--no-longdoc`，不调用 Skill。
- 自动同步现状：`C:/Users/Duanyi/.claude/settings.json` 已有 hooks 框架，但没有 long-doc 同步 hook；两个项目均无同步/校验脚本。
- **推荐单一真相方案**：Claude Skill 为编辑真身 → 白名单同步到主仓 package data snapshot → manifest/hash 校验 → Python longdoc 运行 vendored snapshot；这样 Claude 内修改可自动同步，PyPI/Tauri 分发仍不依赖个人目录。hook 只负责触发，sync 脚本负责确定性复制与失败退出。
- 必须增加测试：源 Skill 可用时 sync 后 hash 一致；源 Skill 不存在时已打包 snapshot 仍可运行；任何漂移由 verify 脚本 exit 1。

---

## 0. 状态

- **Tasks 1-11 全部 complete**(working-tree 累积,无 commit)
- **Task 12 / T7.2 in progress**:
  - ✅ `withGlobalTauri=true` 启动根因修复 + reviewer 通过
  - ✅ `cargo test --lib` — **98/98 passed**(0 failed)
  - ✅ `cargo tauri build` — exit 0,5 个既有 warnings
  - ✅ NSIS installer:`F:\soft\00selfmade\media-to-doc-ui\src-tauri\target\release\bundle\nsis\media-to-doc_1.4.2_x64-setup.exe`(2,630,443 bytes,2026-07-25 09:50:53)
  - ✅ 用户确认 Settings 可添加 MiniMax provider
  - ⏳ P0:per-run LLM profile / Image Agent profile / chat task box / directory project registry
  - ⏳ P1:`long-doc-processor` vendored snapshot + 自动同步/漂移校验
  - ⏳ 更新后重跑新验收并写最终 `-complete-` / `-blocked-` handoff

**当前 ledger**:`.superpowers/sdd/2026-07-24-w15-a-ux-redesign/progress.md`(Tasks 1-11 全 ✅,含 2 个 fix round)

---

## 1. Build 产物路径(Build 完成后)

```
src-tauri/target/release/bundle/
├── nsis/
│   └── media-to-doc_1.4.2_x64-setup.exe        ← 主 installer
└── media-to-doc_1.4.2_x64-portable.exe          ← 便携版
```

**仍是 v1.4.2**(加快模式 — T8 release 会话才 bump 到 v1.5.0)。

---

## 2. 装机步骤(用户执行)

### 步骤 A:卸载旧版 + 清残留缓存(必须,Settings bug 修复前提)

```powershell
# 卸载旧版(走控制面板或执行 unins000.exe)
& "$env:LOCALAPPDATA\com.duanyi.mediatodoc\unins000.exe"

# 删残留数据目录(假设 3:WebView2 缓存可能拒不清)
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\com.duanyi.mediatodoc"
Remove-Item -Recurse -Force "$env:APPDATA\com.duanyi.mediatodoc"
```

### 步骤 B:装新 1.4.2

```powershell
& "F:\soft\00selfmade\media-to-doc-ui\src-tauri\target\release\bundle\nsis\media-to-doc_1.4.2_x64-setup.exe"
```

如果 build 没完成,等 build 完成后再跑这条;build 进度查 `cargo tauri build` 输出。

---

## 3. 13 项验收清单(用户桌面跑)

> **Pass 条件**:13 项中 ≥ 11 项 PASS,其余若因 spec 设计缺陷可标 FAIL 并写 handoff `-blocked-`。

| # | 步骤 | 期望 | 关联 | PASS/FAIL |
|---|------|------|------|-----------|
| 1 | 卸载旧版 + 清缓存 + 装 1.4.2 NSIS | 装机成功,启动器能找到 | §5.3 清缓存重装 | ☐ |
| 2 | 启动 app | 看到新侧栏 5 段,**无 header** | §2.1 | ☐ |
| 3 | §1 ⮜ collapse 按钮点 2 次 | 侧栏 260px ↔ 48px 切换;**重启保留** | §2.4 | ☐ |
| 4 | §3 搜索框输入关键词 | §4 项目树只显示匹配课程 | §3.1 | ☐ |
| 5 | §4 项目展开 + session entry 点击 | session 在主区域**新开 tab** | §2.3 | ☐ |
| 6 | §2 "+ 新建会话"(侧栏已选项目) | 主区域开 **New Run Tab**,课程预填 | §2.2.2 | ☐ |
| 7 | §2 "+ 新建会话"(侧栏未选项目) | toast 提示 "请先选个课程" | §2.3 | ☐ |
| 8 | New Run Tab 提交 → Run pipeline | **关该 tab + 开对应 Session Tab** | §3.3 | ☐ |
| 9 | Session Tab 取消/resume 按钮 | 调对应 command 成功 + UI 状态更新 | §2.2.1 | ☐ |
| 10 | §5 ⚙ 设置 | **Settings tab 在主区域打开** | §2.2.3 | ☐ |
| 11 | Settings → Providers → 添加 DeepSeek → 测试连接 → 保存 → 激活 | **6 步全跑通**,列表新增可见 | T6 + Task 11 I-1 修复 | ☐ |
| 12 | tab × 关闭 | tab 关闭;**最后一 tab × 灰显** | §2.3 | ☐ |
| 13 | 重启 app | 之前开的 tabs 全部恢复;collapse 状态保留;激活 provider 保留 | §3.5 | ☐ |

### 验证 #11 特别说明(I-1 修复)

**Task 11 reviewer 抓出 I-1**:`submitProviderForm` 保存后只刷 SOURCE `#provider-list`(CSS 隐藏),cloned 列表不刷新 — 用户必须手动 Refresh 才能看到新加 profile。**已修复**(`__mountSettingsTab__` 末尾追加 `window.__activeSettingsContainer__ = container` + `submitProviderForm` 末尾双刷 `loadProviders()` + `if (window.__activeSettingsContainer__) loadProvidersInto(window.__activeSettingsContainer__)`)。

**验收**:加完服务商后**立即看到列表新增**,无需手动 Refresh。

---

## 4. 已知 Parked Minor / Polish 项(不影响验收)

| # | 项目 | 来源 | 留待 |
|---|---|---|---|
| M-1 | `.sidebar` 缺 `display: flex; flex-direction: column;` → §5 不真"底部固定"(紧贴 §4) | Task 7 I-1 | T8 release 或 v1.5.0 |
| M-2 | `renderProjectTree.projectId` 变量声明但未用 | Task 6 review | cleanup |
| M-3 | `setInterval(refreshProjectTree, 30000)` 永不 clear | Task 6 review | cleanup |
| M-4 | `tabTitle()` 单段 workDir fallback(undefined · output) | Task 8 I-1 | cleanup |
| M-5 | `btoa(c.path)` 中文路径抛 InvalidCharacterError(Task 6 projectId 未用衍生) | Task 6 review | cleanup |
| M-6 | placeholder 文案 `(form mounts in Task 10)` / `(mounts in Task 9)` | Task 8 review | 已被 Task 9/10 替换 |
| M-7 | `tab-pane` source 在 `<main>` 内仍有 5 个,`rebuildContent` 用 remove('active') 隐藏 | Task 8 review | cleanup |
| M-8 | Run button 重复提交防御未加 | Task 10 review | cleanup |
| **I-2** | **`openProviderModalInto` editing 模式未更新 `providerEditingName`**(edit 时保存误判新建场景) | Task 11 review | **Task 12 polish** |

### 关于 I-2(edit 流程不完整)

- **范围**:Settings → Providers → 点 "编辑" 按钮 → 打开 modal → 改字段 → 保存
- **症状**:edit 模式打开 modal 只 reset form + 设标题,**不预填 name/base_url/model/note 字段**;保存时被 `submitProviderForm` 误判为新建场景(覆盖而不是更新)
- **Workaround**:edit 时手动重填全部字段;或点删除 + 重建
- **留待**:Task 12 polish 不修,T8 release session 修
- **验收影响**:**不在 13 项验收必跑**(#11 是 "添加 → 激活" 6 步,不走 edit 流程)

---

## 5. Reporting Template(用户回填)

### Pass 场景

```markdown
# W15-A Task 12 Verification — COMPLETE

**日期**:2026-07-25
**build 产物**:media-to-doc_1.4.2_x64-setup.exe(存在 ✅)
**装机**:成功
**13 项验收**:
1. ✅ PASS
2. ✅ PASS
3. ✅ PASS
4. ✅ PASS
5. ✅ PASS
6. ✅ PASS
7. ✅ PASS
8. ✅ PASS
9. ✅ PASS
10. ✅ PASS
11. ✅ PASS(添加 DeepSeek + 测试 + 保存 + 激活,cloned 列表自动刷新 I-1 修复生效)
12. ✅ PASS
13. ✅ PASS(tabs + collapse + active provider 全保留)

**PASS 比例**:13/13 ≥ 11 → **Task 12 通过**

**W15-A 加快模式**:不 commit,工作区累积保留;**下一步:T8 release session**(feature commit + bump v1.5.0 + sandbox-verify + PyPI/GitHub release)。
```

→ 用户写完 → 下一会话接力 T8 release。

### Fail 场景

```markdown
# W15-A Task 12 Verification — BLOCKED

**13 项验收**:
1. ✅ PASS
...
N. ❌ FAIL(具体失败描述 + console 截图 + 复现步骤)
...

**FAIL 项分析**:
- 是 spec 设计缺陷? → 留待 polish / 下一 spec
- 是 implementer 实现 bug? → 需要 fix + 重新装机

**下一步**:开新会话修特定 bug → 重新 build → 重跑 13 项。
```

→ 用户写 `handoff-w15-a-t7-1-blocked-2026-07-25.md`(参照 handoff-sdd-w15a-tasks6-12-2026-07-24.md §6)。

---

## 6. 下一会话接力点（第二轮反馈后，替代原“直接 T8 release”）

**新会话第一条正文必须先用非技术语言说明阶段计划**：先让“选模型、写任务、选课程目录”符合用户直觉，再接通后端，最后补 long-doc 自动同步和验收；不要上来先讲 Rust/IPC/schema。

### 必做顺序

1. **先调查并写 mini spec / plan，不直接改代码**：确认 task text 的下游用途，以及 `Image Agent` 是“文本模型做配图策划”还是“图片模型真正出图”；二者需要分层展示。
2. **P0-A per-run profiles**：New Run 从 `list_llm_profiles` 动态加载 profile name（MiniMax 必须出现）；新增每次 run 的 `llmProfileName`，后端按该 profile 注入 key/env 并派生 CLI provider，不能通过修改全局 active profile 实现。
3. **P0-B New Run 会话式入口**：增加任务 textarea + 目录选择按钮；持久化 project registry；同一规范化路径自动合并，重名不同路径不误合并；左侧立即刷新并选中。
4. **P0-C Image Agent**：允许选择已保存的在线 LLM profile 做配图策划；真正生成图片仍单独选择 image provider。必须先补主仓能力/接口，不能只加一个无效下拉框。
5. **P1 long-doc 同源**：主仓新增 vendored snapshot、sync/verify 脚本和测试；再通过 Claude `PostToolUse(Edit|Write)` hook 自动触发。hook 改 `settings.json` 前先用 `update-config` Skill；若该 Skill 仍报 schema 错，记录 blocker 后按现有 hooks 结构人工最小编辑并验证。
6. **P2 parked**：定时任务继续 disabled，只改清晰说明，不实现调度器。
7. TDD + reviewer：前端/后端逐项 RED→GREEN；涉及 `commands.rs` / `runner.rs` 必须两轮 review；跑 `cargo test --lib` + 主仓 `uv run pytest` + `cargo tauri build`。
8. **仍不 commit / push / release / bump / reset**；完成后强清装机，按更新后的验收清单验证，再写最终 handoff。T8 release 继续 blocked。

### 设计红线

- 不把 API key 写入 HTML、metadata、日志或命令行；继续走 keyring + child env。
- 不用 display name 作为项目唯一标识；项目 identity 必须是规范化真实路径。
- 不让 Python wheel / Tauri installer 在运行时硬依赖 `C:/Users/Duanyi/.claude/...`。
- 不把普通文本 LLM 宣称为图片生成模型；策划模型与出图模型分开。
- 不修改/覆盖现有 Tasks 1-12 未提交工作区；继续加快模式。

**新会话 prompt**：`prompt-w15-a-t7-2-product-feedback-next.md`。

### T8 release（全部前置通过后才恢复）

feature commit → bump 1.5.0 → 两仓测试/build → 强清装机 + sandbox-verify → reviewer → 等用户拍板 merge/release。

---

## 7. 风险与避坑

| 风险 | 避坑 |
|---|---|
| Build 失败(impl bug 致 src/index.html 语法错) | Build 输出会显示具体行号;若失败,先读 cargo 输出,定位 JS 语法错误 |
| 装机后 Settings 仍不弹(假设 1/2/3 错) | 13 项验收 #10/11 暴露;参考 §5.3 三步 + WebView2 cache 清 |
| 13 项验收发现 I-2 edit 模式阻断 | 已 parked,不影响 #11 验收;用户可走 "delete + re-add" 流程 |
| WebView2 cache 拒不清(假设 3 撞墙) | step 1 强清;若仍不行,在 `index.html` 加 `?v=v1.4.2` cache-bust |
| cargo build 撞 sparse HTTPS SSL 撞墙 | `default crates-io` + `CARGO_NET_TLS_VERIFY=false`(cargo registry 重设;`feedback_cargo_ssl_mitm.md`) |

---

## 8. 关键文件路径

- **Plan**:`docs/superpowers/plans/2026-07-24-w15-a-ux-redesign.md`
- **Spec**:`docs/superpowers/specs/2026-07-24-w15-a-ux-redesign-design.md` §8(13 项验收)
- **Ledger**:`.superpowers/sdd/2026-07-24-w15-a-ux-redesign/progress.md`
- **本 handoff**:`handoff-w15-a-task12-build-verify-2026-07-25.md`
- **Build 产物(待)**:`src-tauri/target/release/bundle/nsis/media-to-doc_1.4.2_x64-setup.exe`
- **sandbox-verify**:`F:\soft\00selfmade\sandbox-verify\media-to-doc-ui\mtd-verify.ps1`(T8 release 用,Task 12 不必跑)

---

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>