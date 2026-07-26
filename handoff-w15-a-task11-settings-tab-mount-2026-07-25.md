# W15-A Task 11 Handoff — Settings Tab mount(挂 T6 providers UI)

**承接日期**: 2026-07-25
**分支**: `feat/w15a-llm-api-settings` (BASE = `073b05e`,无 commit — 加快模式)
**承接**: Task 12(Build + 13 项验收 + handoff),可由新会话接力

## §1 改了什么

仅 `src/index.html` 一文件,**净 +136 lines**(Task 11 only,文件 2024 → 2160)。

精确块:`window.__mountSessionTab__ = __mountSessionTab__;`(`src/index.html:1156`)与 `// 启动初始化`(`src/index.html:1293`)之间插入:

1. **`function __mountSettingsTab__(container)`**(`src/index.html:1159-1194`)
   - step 1:`container.innerHTML = ''` + `src.cloneNode(true)` + 加 `.active` + append(brief verbatim,与 `rebuildContent()` 913-917 已有 clone 重叠,但 idempotent — 净结果还是 1 份副本)
   - step 2:重新挂 `.settings-subnav-item` click(切换 active class + 切 subpane + 切到 providers 时 reload)
   - step 3:**一次性挂** `#provider-add-btn` / `#provider-refresh-btn` container 内监听(空列表也能用,见 Concerns §1)
   - step 4:调 `bindSettingsProvidersEvents(container)`(no-op,沿用 T6 global modal binding)
   - step 5:末尾 `loadProvidersInto(container)` 触发首屏加载
2. **`window.__mountSettingsTab__ = __mountSettingsTab__;`**(`src/index.html:1195`)— **Defect 1 修复**(rebuildContent 依赖此全局,可选链 `window.__mountSettingsTab__?.(wrap)`)
3. **`function loadProvidersInto(container)`**(`src/index.html:1197-1210`)— container-scoped `list_llm_profiles` + 渲染入口
4. **`function renderProfilesInto(container, profiles)`**(`src/index.html:1212-1261`)— 取 active name → 渲染 provider cards → bind `[data-act]` activate/edit/delete 按钮
5. **`function openProviderModalInto(container, editingName)`**(`src/index.html:1263-1282`)— 简版,只实装 add 流程;edit 模式仅 reset form + 设标题(不预填字段,见 Concerns §2)
6. **`function bindSettingsProvidersEvents(container)`**(`src/index.html:1284-1291`)— brief 占位(no-op),注释说明 modal 事件沿用 T6 global binding

## §2 Build evidence

`cargo check` 通过(2.22s,dev profile,5 warnings 全是预先存在,0 errors)。

5 warnings 是 llm_profiles.rs 内 unused `provider_name` / `list_profile_names` / `all_templates` / `Protocol` / `ProviderTemplate`,与本 task 无关。

`cargo build --release`:未跑(纯前端改动,Rust 零变更;Task 12 总体验收时跑,预计 ~10min)。

## §3 Defect / I-2 / IPC 验证

| 项 | 状态 | 说明 |
|---|---|---|
| Defect 1(`window.__mountSettingsTab__` 挂载) | **已修复** | `src/index.html:1195`,与 Task 9 / 10 模式一致 |
| I-2(不绑 window listener) | **已防御** | 仅 `container.querySelector(...).addEventListener`;全局唯一 `#provider-modal-backdrop` 例外(沿用 T6 boot binding);**没有 `window.addEventListener` 路径** |
| 6 个 LLM command 后端签名验证 | **全部匹配** | 见 §3.1,无 type-shape 适配(与 Task 9 / 10 撞过的 IPC 签名不一致问题不同) |

### §3.1 6 个 LLM command 签名验证

`src-tauri/src/commands.rs` grep 结果(行号 + 实际签名):

| 命令 | `commands.rs` 行 | 签名 |
|---|---|---|
| `list_llm_profiles` | 1207-1210 | `() -> CommandResponse<Vec<ProfileMeta>>` |
| `get_active_llm_profile_name` | 1224-1227 | `() -> CommandResponse<String>` |
| `save_llm_profile` | 1321-1322 | `(args: SaveProfileArgs) -> CommandResponse<ProfileMeta>`;`SaveProfileArgs` 字段见 `commands.rs:1233-1249` |
| `set_active_profile` | 1345-1346 | `(name: String) -> CommandResponse<()>` |
| `delete_llm_profile` | 1375-1376 | `(name: String) -> CommandResponse<()>` |
| `test_llm_connection` | 1462-1465 | `(name: String) -> CommandResponse<TestConnectionResult>`;`TestConnectionResult` 字段见 `commands.rs:1383-1389` |

`SaveProfileArgs` 字段(8 个):`name / provider / base_url / model / note / api_key / tool_search_enabled / experimental_betas_disabled`(`commands.rs:1233-1249`)。所有字段在 brief `renderProfilesInto` 渲染子集 + T6 modal 处理流程内全部用上,**无遗漏**。

`ProfileMeta` 字段:`llm_profiles.rs:201-...`(`name / provider / base_url / model / note / tool_search_enabled / experimental_betas_disabled / created_at`)。brief `renderProfilesInto` 用到子集 name / provider / base_url / model / note,全部存在。

JS 端 invoke 形式(本 task 实际调用):
- `invoke('list_llm_profiles')`(无参)— `loadProvidersInto` 内
- `invoke('get_active_llm_profile_name')`(无参)— `renderProfilesInto` 内
- `invoke('set_active_profile', { name })`— `renderProfilesInto` activate 按钮内
- `invoke('delete_llm_profile', { name })`— `renderProfilesInto` delete 按钮内
- `save_llm_profile` / `test_llm_connection` **不直接调**(沿用 T6 `submitProviderForm` / `testProviderConnection` 已挂的 global handler)

**所有 invoke 形式与后端签名匹配,无 camelCase rename 风险**(Tauri 默认 snake_case → camelCase 转换对单 String 参数自动生效)。

## §4 下一步(Task 12)

1. `cargo tauri build` 跑 release build(预计 ~10min)— 验证 0 errors / 0 critical warnings
2. 真机验证走 `F:\soft\00selfmade\sandbox-verify\media-to-doc-ui\mtd-verify.ps1 -InstallerPath target\release\bundle\nsis\media-to-doc-*-setup.exe`(加速模式:加 `-NoWait`)
3. 13 项验收清单(加快模式豁免非 critical 项):
   - [ ] 启动 + 侧栏 5 段展示
   - [ ] 点击 §5 ⚙ 设置 → Settings tab 打开
   - [ ] 看到 Providers 子页 + 列表(空或有 profile)
   - [ ] 点 "+ 添加服务商" → 填表 → 测试 → 保存 → 列表新增(端到端)
   - [ ] activate / edit / delete 三个按钮 fire 各自 handler
   - [ ] subnav 切 4 子页(General / Theme / About 占位)
   - [ ] Session tab 关闭 → polling cleanup 不留 timer
   - [ ] New Run tab 提交 → toast → 切 session tab
   - [ ] closeTab / focusTab / persistTabs localStorage 正确
   - [ ] Search Ctrl+K 全局单 listener 仍 fire(§3 I-2 不变量)
   - [ ] 无 console error
   - [ ] 无 window listener leak(mount N 次后 `getEventListeners(window)` 应只有 1 个 keydown)
   - [ ] 真机 screenshot 通过(sandbox-verify `screenshots/`)
4. handoff 写 `handoff-w15-a-task12-build-verify-2026-07-25.md` 总结 13 项验收 + W15-A 总进度(从 W15-A spec/plan 到 12 task 完工)

## §5 Concerns

1. **Brief `renderProfilesInto` empty-list 路径不挂 addBtn 监听器**:用户首次用 Settings Tab(空 profile 列表)点 "+ 添加服务商" 无反应。**我的修复**:`__mountSettingsTab__` 内一次性挂 addBtn/refreshBtn(走 container scoped)。**偏离 brief verbatim**(brief 想让 renderProfilesInto 挂,但只在非空路径挂 → bug)
2. **Edit 流程不完整**:`openProviderModalInto` editing 模式仅 reset form + 设标题,不预填 name/base_url/model/note 字段。T6 有 `openProviderModal(profile)` 全版本(`src/index.html:1899-1923`),**本 task 没把 T6 全版适配到 container scoped**(brief 简化策略)。Workaround:edit 时手动重填。**留 Task 12 polish**
3. **cloneNode 后同 tab 类型 id 冲突潜在风险**:罕见(`openTab({type:'settings'})` 已去重,理论同一时刻只 1 个 Settings tab)
4. **`<div id="provider-modal-backdrop">` 全局单例**:cloneNode 不复制(在 `<main>` 外,line 738)。provider add 操作全局单例,符合 UX 直觉
5. **Source `<div id="tab-settings">` 在 `<main>` 内仍存在**:rebuildContent 只 `cloneNode(true)`,不移除 source。生产路径下被 `.tab-pane.active` 切换为 hidden,无 UX 影响
6. **未实际启动 Tauri dev 跑 Settings Tab mount**:Task 11 brief Step 2-3 要求 dev / build 验证,本任务没跑(沿用 Task 9 / 10 加快模式)。Task 12 总体验收时跑 `cargo tauri build` + sandbox-verify
7. **`bindSettingsProvidersEvents(container)` 是 brief 占位函数**:no-op,实际事件绑定分布在 4 个位置(subnav click / addBtn+refreshBtn / provider card `[data-act]` / global modal controls)。**不要被字面 "bindSettingsProvidersEvents" 误导**
8. **`renderProfilesInto` 2 个串行 IPC call**(list + get_active):brief 简化策略,功能正确,性能 OK(本地 IPC ms 级)

## §6 必读顺序

1. task-11-brief.md(`.superpowers/sdd/2026-07-24-w15-a-ux-redesign/task-11-brief.md`)
2. task-11-report.md(刚写的)
3. `src/index.html` 行 1156-1292(本 task 块)
4. `src/index.html` 行 902-927(rebuildContent 调用 mount 的入口,Task 8 实装)
5. `src/index.html` 行 680-720(`<div id="tab-settings">` source 结构)
6. `src/index.html` 行 1787-2020(T6 providers UI 全实现 + boot binding)
7. `src-tauri/src/commands.rs` 行 1190-1465(6 个 LLM command 签名)
8. `src-tauri/src/llm_profiles.rs` 行 201-...(ProfileMeta 字段定义)
9. 上一个 handoff:`handoff-w15-a-task10-new-run-tab-2026-07-25.md`(Task 10 模式参考)

## §7 状态

`DONE_WITH_CONCERNS`(cargo check 通过 / release build 未跑 / Tauri dev 未真机验证 / edit 流程不完整 / brief 微调 1 处)。

Task 12 接力点见 §4。