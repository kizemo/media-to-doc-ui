# W15-A Task 10 Handoff — New Run Tab 内容(form + 提交)

**承接日期**: 2026-07-25
**分支**: `feat/w15a-llm-api-settings` (BASE = `073b05e`,无 commit — 加快模式)
**承接**: Task 11(Settings Tab mount),可由新会话接力

## §1 改了什么

仅 `src/index.html` 一文件,**净 +79 / -6 lines**(Task 10 only)。

精确块:`buildNewRunForm` 占位 → 替换为:

1. **完整版 `buildNewRunForm(coursePath)`** — h2 + course 预览 kv + form(LLM select / Imagegen select / StopAfter select / noLongdoc + force checkbox / Run + Cancel 按钮)
2. **`function __mountNewRunTab__(container, tab)`** — FormData 收集 → invoke `run_pipeline` → 成功 toast → `closeTab(tab.id)` + `openTab({ type: 'session', workDir: r.data.work_dir })`;Cancel 按钮 → `closeTab(tab.id)`
3. **`window.__mountNewRunTab__ = __mountNewRunTab__;`** — **Defect 1 修复**,函数声明末尾立刻挂到 window(`src/index.html:1042`)

## §2 Build evidence

`cargo check` 通过(2.05s,dev profile,5 warnings 全是预先存在,0 errors)。
`cargo build --release` 未跑(纯前端改动,无 Rust 变更;Task 12 总体验收再跑)。

## §3 Defect / I-2 / IPC 签名验证

| 项 | 状态 | 说明 |
|---|---|---|
| Defect 1(mount 函数挂 window) | **已修复** | `src/index.html:1042` |
| I-2(不要绑 window listener) | **已防御** | 仅 `container.querySelector(...).addEventListener` |
| `run_pipeline` 后端签名验证 | **已 grep + 适配** | 见 §3.1 |

### §3.1 run_pipeline 签名验证

`src-tauri/src/commands.rs:999-1007` 实际签名:
```rust
pub async fn run_pipeline(
  inbox_dir: String,
  workspace_root: Option<String>,   // ← Brief 漏
  llm: Option<String>,
  imagegen: Option<String>,
  stop_after: Option<String>,
  no_longdoc: Option<bool>,
  force: Option<bool>,
) -> CommandResponse<RunPipelineResult>
```

`RunPipelineResult`(`commands.rs:1055-1060`)字段:`work_dir / pid / log_path / spec`。

**Brief opts 漏 `workspaceRoot`**,虽然 `commands.rs:1020` 注释 `let _ = workspace_root; // 暂未使用(MCP 兼容占位)` 确认是 MCP 兼容字段,Tauri serde 仍要求字段 present。

**适配**:`opts` 加 `workspaceRoot: null`(带 2 行注释说明来源)。`noLongdoc` / `force` 保持 `!!fd.get(...)`(Rust `unwrap_or(false)` 兼容)。camelCase 转换与后端 snake_case 一一对应。

## §4 下一步(Task 11)

实现 `__mountSettingsTab__`(loadProviders + providers UI 绑定,沿用 `window.__mountSettingsTab__?.(wrap)` 模式)。

参考:`src/index.html:914-916` 是当前 Task 11 占位(Settings Tab 由 `rebuildContent()` clone `$('tab-settings')` 后再调 mount)。
后端命令 grep `commands.rs`:`load_providers` / `list_providers` / `set_active_profile` 等(W15-A T2/T3/T4 实装)。

## §5 Concerns

- **IPC 签名 Brief 漏 `workspaceRoot`**:已适配 + flag。
- **Run button 重复提交未防御**:scope creep 跳过,留 Task 12 polish(2 行 `disabled = true` + 'submitting…')。
- **未真机验证**:Task 10 brief Step 2 要求 dev / build 验证,本任务没跑(沿用 Task 9 加快模式)。Task 12 总体验收时跑 `cargo tauri dev`。
- **`<form>` inline style**:Brief 严格用 inline style(沿用 W14-B+ 既有习惯),无新增 CSS class。
- **`FormData.get()` unchecked 返回 null**(而非 false):Brief `!!fd.get(...)` 显式转 boolean,Rust `unwrap_or(false)` 兼容。

## §6 必读顺序

1. task-10-brief.md(`.superpowers/sdd/2026-07-24-w15-a-ux-redesign/task-10-brief.md`)
2. task-10-report.md(刚写的)
3. `src/index.html` 行 958-1042(本 task 块)
4. `src-tauri/src/commands.rs` 行 999-1061(后端 `run_pipeline` 签名)
5. 上一个 handoff:`handoff-w15-a-task9-session-tab-2026-07-25.md`(Session Tab 模式参考)

## §7 状态

`DONE_WITH_CONCERNS`(cargo check 通过 / release build 未跑 / Tauri dev 未真机验证 / Run button 重复提交未防御)。