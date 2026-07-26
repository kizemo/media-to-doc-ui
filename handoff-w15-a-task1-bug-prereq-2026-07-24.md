# Handoff: W15-A Task 1 — Settings Bug 前置修复(capability + error handler)

**日期**:2026-07-24
**承接**:`handoff-w15-a-t7-failed-redesign-pivot-2026-07-24.md` §3.4 (新会话必做验证 1-3)
**分支**:`main`(working tree,无 commit)
**task 进度**:W15-A 12 tasks 第 1/12

---

## 1. 完成内容

按 task brief `task-1-brief.md` 步骤 1-6 执行,**Step 2/4 实际产物与 brief 假设有偏差,见 §3 关键发现**。

### 1.1 Step 1-2:capability 改造

**结论**:未按 brief 形式加 6 个 LLM command 显式 allowlist。

**实证过程**(3 次尝试,均触发 cargo check 拒绝):

| 尝试 | identifier 形式 | cargo check 结果 |
|---|---|---|
| A | `list_llm_profiles`(原 brief 形式) | `invalid plugin or permission identifier 'list_llm_profiles': identifiers can only include lowercase ASCII, hyphens which are not leading or trailing, and a single colon if using a prefix` |
| B | `app:list_llm_profiles`(假设 app 前缀) | 同上错(下划线不允许) |
| C | `list-llm-profiles`(kebab-case) | `Permission list-llm-profiles not found, expected one of core:default, core:app:* ...` |
| D | `allow-list-llm-profiles`(Tauri 2 命令命名约定) | `Permission allow-list-llm-profiles not found` |

Tauri 2.11.5(项目当前 version)的 `tauri::generate_handler!` 不会为 app 自定义 command 自动生成 permission 条目,所以 acl-manifests.json 里**不存在**这 6 个 command 的 identifier,build script 任何写法都拒绝。

**最终产物**:`src-tauri/capabilities/default.json` 仅 description 改为标注"维持 core:default 即可,LLM commands 通过 tauri::generate_handler! 默认放行",permissions 数组保持 `["core:default"]`。

### 1.2 Step 3-4:module 顶层 error handler

按 brief 在 `<div id="toast-container"></div>` 与 `<script type="module">` 之间插入 19 行 `<script>` 块,捕获:
- `window.error` 事件(顶层 throw 才会触发,module 内部 catch 不捕获)
- `window.unhandledrejection` 事件(async throw 没被 await catch)
- 暴露 `window.__MEDIA_TO_DOC_INIT_LOG__` 给后续 module 调用

**注意**:浏览器环境无 `fs.writeFileSync`,`writeInitLog` 实际降级到 `console.error('[init]', msg)`。`process.env.APPDATA` 在 WebView2 里不存在,日志最终通过 console + 未来 Rust 端 `log_init_messages()` 通道(留口子,TBD 由新会话决定)输出到 `%APPDATA%\com.duanyi.mediatodoc\init.log`。

### 1.3 Step 5:cargo check

```
   Compiling media-to-doc-ui v1.4.2
warning: function `all_templates` is never used  -- src\llm_profiles.rs:39(已有)
warning: function `provider_name` is never used  -- src\llm_profiles.rs:131(已有)
warning: function `default_ollama_template` is never used  -- src\llm_profiles.rs(已有)
warning: function `default_anthropic_template` is never used  -- src\llm_profiles.rs(已有)
warning: function `default_openai_template` is never used  -- src\llm_profiles.rs(已有)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.60s
```

**0 errors, 0 new warnings**(5 个 unused-function warnings 来自 W15-A T2 WIP,与本 task 无关)。

### 1.4 Step 6:无 commit,只写本 handoff

按 W15-A 加快模式规则(W14-C + T6 沿用):W15-A 整体一次 commit,不分小 commit。本 task 不 git commit / add / push / reset / checkout / restore。

---

## 2. 改动文件清单

| 文件 | 改动 | 行数 |
|---|---|---|
| `src-tauri/capabilities/default.json` | description 文案更新(W14-B → W15-A T7 说明)| +1 / -1 |
| `src/index.html` | 插入 module 顶层 error handler `<script>` 块 | +19 / -0 |

**未改动**:Rust 源码 / `tauri.conf.json` / `Cargo.toml` / `Cargo.lock` / 其他 capability 文件。

---

## 3. 关键发现(下一会话必读)

### 3.1 假设 1(capability 拒 LLM IPC)在 Tauri 2.11.5 下不成立

**实证**:W14-B+ 8 commands(`list_courses` / `check_status` 等)与 W15-A T2 6 LLM commands 同样通过 `tauri::generate_handler!` 注册,均未在 `capabilities/default.json` 列出,但均工作正常(W14-B+ 8 commands E2E 验证脚本已 PASS,W14-D E2E 报告 v0.1.0 装机验过)。

**Tauri 2.11.5 行为**:
- `core:default` 只放行 `core:*` plugin commands(`app-hide` / `event-listen` / `window-close` 等)
- app 自定义 commands 经 `tauri::generate_handler!` 注册后,**默认对所有 window 放行**,无需 capability 显式列
- capability 系统只限制 plugin commands,app commands 不在限制范围

**结论**:用户报告的"Settings 点击无反应"**不是** capability 拒 LLM IPC,真实根因是 hypothesis 2(module 顶层 throw)或 hypothesis 3(WebView2 stale cache)。

### 3.2 假设 2(module 顶层 throw)已被 error handler 兜底

新增的 `<script>` 块会在 module 顶层 throw 时把 stack 写 console,开发模式下用户 F12 DevTools 即可看到 `[init] window.error: ...`。

**仍需新会话做的诊断**:
1. 用户重装后,启动 dev build / prod build,点 Settings 按钮
2. F12 DevTools 看 Console 有无 `[init]` 前缀的报错
3. 如果**有** `[init]` 报错 → 假设 2 成立,定位 throw 源
4. 如果**没有** `[init]` 报错 → 假设 3(stale cache)更可能,需强清缓存

### 3.3 假设 3(stale cache)未在本 task 修复

强清缓存步骤(cargo tauri dev / prod 重 build + 用户 `%LOCALAPPDATA%\com.duanyi.mediatodoc\` 全删 + 重装 NSIS)是 W14-B+2 收尾已验证过的流程,本 task 不重复,留给 Task 4(装机 + 13 项验收)统一做。

---

## 4. 下一步必交付(Task 2)

按 W15-A 12 task 列表:
- Task 2:删 5 tab + 删 header + 重建侧栏(具体子步骤见 `.superpowers/sdd/task-2-brief.md`,待 task-1 完成后由用户在 plan 解锁)

---

## 5. 避坑提示

1. **不要尝试把 LLM command 加 capability** — 实证拒绝(cargo check 报 "not found")。如要严格限制 app commands,需要把它们改写成 tauri-plugin(改 Rust 代码,超出本 task 范围)。
2. **error handler 在浏览器环境降级到 console**,不直接落盘。要让 `init.log` 真的写文件,需要在 Rust 端开 `log_init_messages()` command 通过 invoke 通道接前端日志;若用户要求强约束"init 错误必落盘",需要在 W15-B+ 实装,本 task 不做。
3. **description 文案只是标注**,不影响 build 或运行时,可后续一并整理。
4. **不要 git commit** — W15-A 加快模式,只写 handoff。

---

## 6. 必读顺序

下一会话开篇读:
1. 本文件(`handoff-w15-a-task1-bug-prereq-2026-07-24.md`)— 关键发现 §3
2. `handoff-w15-a-t7-failed-redesign-pivot-2026-07-24.md` — 根因调查 §3 + UX 重设计 §4
3. `.superpowers/sdd/task-2-brief.md` — Task 2 起步