# W14-E C — Run Tab 真触发 + Stage 进度条 + 完成跳 Output

**日期**:2026-07-23
**承接 handoff**:`handoff-w14e-ab-complete-2026-07-23.md` §C
**承接会话承诺**:Run tab UI 前端真触发 + 反馈(W14-B+2/W14-C 已实装大部分代码)

---

## 1. 目标

W14-B+2 + W14-C 已实现 Run pipeline 按钮、`run_pipeline` Tauri command 后台 spawn、
3s 状态轮询、cancel 按钮、per-run log tail、多卡片视图。本 spec 收尾剩余 UX 反馈:

1. **Stage 进度条 + 当前 stage 高亮** — 用户能看见流水线跑到哪一步
2. **完成时自动跳 Output tab + toast** — 用户不用手动切回去看产物

不动 Rust 代码(沿用已有 commands),仅改 `src/index.html`。

---

## 2. 数据源选择

`check_status(work_dir)` Tauri command(commands.rs:166)已实装返回:

```rust
pub struct CheckStatusResult {
  pub course: String,
  pub inbox_path: String,
  pub current_stage: String,   // ← 当前正在跑哪个 stage
  pub started_at: String,
  pub updated_at: String,
  pub is_complete: bool,       // ← 全部 completed/skipped
  pub stages: BTreeMap<String, StageStatus>,  // ← 11 stage × status
}
```

**StageStatus.status** 取值:`pending` | `in_progress` | `completed` | `skipped` | `failed`

**前端目前没用 `check_status`**(只用了 `get_run_metrics`),本 spec 引入。

**为什么不用 log 文本 grep**:
- log 是 stdout free text,解析脆弱(阶段名出现在不同上下文)
- state.json 是真相流,W12-D 起普及,所有阶段都写

---

## 3. 设计

### 3.1 Stage 进度条渲染(11 stage grid)

CSS 已有 `.stage-grid` + `.stage-dot` + 3 个状态 class(`completed`/`in_progress`/`failed`)。**复用 + 补 `pending` 与 `skipped`**。

rendered in `run-card-body`:

```html
<div class="stage-grid">
  <div class="stage-dot completed" title="audio · completed">a</div>
  <div class="stage-dot in_progress" title="asr · in_progress">r</div>
  <div class="stage-dot pending" title="frames · pending">f</div>
  ...
</div>
```

11 stages 顺序:audio / asr / frames / ocr / asr_correct / chapters / draft / imagegen / render / longdoc / verify

CSS 增量:`.stage-dot.pending` 用现有 .pill.pending 背景

### 3.2 Polling 拓展:check_status per running run

现状 `startRunPolling` 每 3s 只 refreshRunCards + tail log。扩展:

```js
for (const [wd, tracker] of state.activeRuns) {
  await tailLogForRun(wd, tracker.logPath);   // 原有
  await pollStageForRun(wd);                  // 新增
}
```

`pollStageForRun(work_dir)`:
- invoke('check_status', { work_dir })
- 失败:静默(进程可能刚 spawn,state.json 还没写)
- 成功:更新该 work_dir 的 stage-grid 渲染缓存(下一次 renderRunCards 用)

### 3.3 完成时跳 Output tab

每次 renderRunCards 检查每条 run 的 status 转变:
- 维护 `state.runPrevStatus: Map<work_dir, string>`
- 如果 prev = 'running' 且 new ∈ {'completed','cancelled','failed'} → 触发 jump
- **只跳一次**:jump 之后该 work_dir 出 Map,不再触发
- 跳的同时 toast:`Run {status}: 跳到 Output 看产物`
- 触发条件:用户当前在 Run tab(避免打断用户在 Output/Health 操作)

```js
function maybeJumpToOutput(workDir, status) {
  if (state.runPrevStatus.get(workDir) === 'running'
      && ['completed','cancelled','failed'].includes(status)) {
    const wasRunningTab = document.querySelector('.nav-item.active')?.dataset.tab === 'run';
    if (wasRunningTab) {
      toast(`Run ${status} — 跳到 Output`);
      setTimeout(() => switchTab('output'), 800);  // 0.8s 缓冲
    }
  }
  state.runPrevStatus.set(workDir, status);
}
```

---

## 4. E2E 验收清单

| # | 验证项 | 期望 |
|---|---|---|
| 1 | cargo tauri dev 启动 | 窗口打开,Inbox/Run/Output/Health/Learn 5 tab |
| 2 | Inbox 选 inbox 子目录 | Run tab `run-inbox` 显示 selected path,Run pipeline 按钮 enabled |
| 3 | 点 Run pipeline(stop_after=audio) | toast 显示 Started,卡片出现 running 状态 |
| 4 | 几秒内 stage-grid 显示 | `audio` in_progress / completed 高亮 |
| 5 | 完成时(几秒后) | toast 显示 completed,自动跳 Output tab,显示产物 |
| 6 | Cancel 按钮 | click 后卡片状态变 cancelled,不跳 Output |
| 7 | 并发选项 | 不在本 spec 范围 |

---

## 5. 改动清单

| 文件 | 改动 | 备注 |
|---|---|---|
| `src/index.html` | +110 行 JS(进度条 + polling + jump tab)+ 6 行 CSS(`.stage-dot.pending` + `.stage-dot.skipped`)| 唯一改动 |
| `docs/superpowers/plans/2026-07-23-w14e-c-runtab-trigger.md` | 实施 plan | |
| `handoff-w14e-c-runtab-trigger-2026-07-23.md` | 完成时写 | |

无 Rust 改动,无新 Tauri command,无测试改动(cargo tauri dev 手测)。

---

## 6. 风险

- **cargo tauri dev 端口冲突**:`devUrl=1420`(`tauri.conf.json`)。W14-D memory
  `feedback_tauri_dev_static_server` 已知用 `python -m http.server 1420` 占端口
- **Cargo 编译撞 SSL**:`CARGO_NET_TLS_VERIFY=false` 已在 env(memory
  `feedback_cargo_ssl_mitm`)
- **快速 E2E**:用 stop_after=audio(只跑 audio stage,~10s 完成)避免真实 ASR
- **可能不存在 inbox 样例**:用户自建 sample-inbox 或选现有真实 inbox 目录
  (若都没有,前端可手动选 workspace 父目录)
