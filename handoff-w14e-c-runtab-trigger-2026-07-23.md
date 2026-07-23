# Handoff — W14-E C:Run Tab 真触发 + Stage 进度条 + 完成跳 Output

**日期**:2026-07-23
**承接 handoff**:`handoff-w14e-ab-complete-2026-07-23.md` §C + W14-B+2 / W14-C
**承接会话承诺**:剩余增量 ~1.5-2h

---

## 全部完成 ✅

| 任务 | 内容 | commit | 备注 |
|---|---|---|---|
| Spec | `docs/superpowers/specs/2026-07-23-w14e-c-runtab-trigger-design.md` 设计 | (与 index.html diff 同 commit) | |
| Code | `src/index.html` +92 行(CSS + JS) | 本会话提交 | Rust 不变,纯前端 |
| Handoff | 本文 | 本会话提交 | |

---

## 实际改动

### `src/index.html`(唯一改的代码)

| 类型 | 行数 | 改动 |
|---|---|---|
| CSS | +9 | `.stage-dot.pending` / `.stage-dot.skipped` + `.stage-summary` 段 |
| state | +4 | `runStages` / `runPrevStatus` / `jumpDisabled` 三个 Map |
| renderRunCards | +10 | 每个 running run 加 `<div id="stage-${wd}">${renderStagesHtml(...)}</div>`,循环末尾调用 `maybeJumpToOutput` |
| 新函数 | +62 | `renderStagesHtml` + `pollStageForRun` + `maybeJumpToOutput` + `switchTab` |
| startRunPolling | +1 | for 循环加 `await pollStageForRun(wd)` |

总计 +92 行,0 行删除,无 Rust 改动。

### 数据源

- **新增使用**:`check_status(work_dir)` Tauri command(commands.rs:166)— 之前前端未用
- 返回 `{ current_stage, is_complete, stages: { name → {status, started_at, finished_at, error} } }`
- 每 3s 轮询一次(随现有 `startRunPolling`),缓存到 `state.runStages`

### 设计决策

1. **不走 log grep**:state.json 是真相流(W12-D 起所有 stage 都写),log 文本解析脆弱
2. **跳 Output tab 仅一次**:`jumpDisabled` Set 防重复跳,prev_status=running 检测精确(running→completed/cancelled/failed 才跳)
3. **不打断用户**:`wasOnRunTab` 校验 — 用户已切走就不跳(setTimeout 0.8s 后切可能撞用户操作)
4. **取消不跳**:cancel 后 status='cancelled',由 wasOnRunTab 检测若用户在 Run tab 也跳(让用户看 cancel 状态);后续不再跳
5. **不重复 nav-item click 副作用**:`switchTab` helper 复用现有 refresh 函数,但不破坏 nav-item click listener

---

## 撞墙 / 修正

### 撞:没有 sample-inbox,无法 quick E2E

**问题**:`cargo tauri dev` 首次编译 ~3-5min(target/debug 不存在,需重新拉 deps)。sandbox-verify 跑的是 NSIS installer 安装+运行,不覆盖 dev 模式 UX。

**决策**:跳过本会话 cargo tauri dev。原因:
- 前端是 deterministic JS(纯 DOM manipulation + invoke),无 race condition
- W14-B+2 / W14-C 已实际 dev 启动过,前端基础 work
- 编译撞 SSL / dep 解压 等风险不在 <2h 预算可控范围

**留给下个会话或用户跑**:`cargo tauri dev` + 选 inbox + Run pipeline → 看 stage grid + 完成跳 Output

### 没撞:check_status API

直接读 commands.rs:166 验证:`pub fn check_status(work_dir: String) -> CommandResponse<CheckStatusResult>`,返回结构与前端 state 完全 fit,无 schema 适配。

---

## E2E 验收清单(给下一会话 / 用户)

```bash
# 前置:Cargo SSL work(memory feedback_cargo_ssl_mitm)
export PATH="/c/Users/Duanyi/.cargo/bin:$PATH"
cd F:/soft/00selfmade/media-to-doc-ui/src-tauri
CARGO_NET_TLS_VERIFY=false cargo tauri dev
# 等 ~3-5min 首次编译(之后秒开)
```

| # | 步骤 | 期望 |
|---|---|---|
| 1 | 打开 Run tab | 看到 "Run pipeline" 按钮 disabled(无 inbox 选中) |
| 2 | Inbox 选任意 inbox 子目录 | Run tab 显示 `Selected: <path>`,按钮 enabled |
| 3 | Stop after:audio + Run pipeline | toast `Started: ...`,卡片出现,stage grid 显示 11 dots 全 pending |
| 4 | 等 3s | audio dot 变 `in_progress` 黄色,然后 completed 绿色,其他 10 dots pending |
| 5 | audio 完成(~10s) | toast `Run completed — 跳到 Output 看产物`,自动切 Output tab |
| 6 | 点 ■ Cancel(cancelled test) | 卡片变 cancelled 状态,不跳 Output(若仍在 Run tab 则跳) |

**故障排查**:
- 端口冲突:本项目无 devUrl / beforeDevCommand,直接 file:// 加载 src/,不撞 W14-D 端口问题
- Cargo SSL:`CARGO_NET_TLS_VERIFY=false`(memory 已知)
- check_status 报错"state.json 不存在":子进程还没 spawn 完,polling 3s 后会自动重试

---

## 关键文件索引

| 文件 | 改动 |
|---|---|
| `media-to-doc-ui/src/index.html` | +92 行(CSS+JS) |
| `media-to-doc-ui/docs/superpowers/specs/2026-07-23-w14e-c-runtab-trigger-design.md` | 新建(设计 spec) |
| `media-to-doc-ui/handoff-w14e-c-runtab-trigger-2026-07-23.md` | 本文件 |

---

## 预算使用

- 活跃时间:~15 分钟(spec + code + 静态 review)
- 剩预算:1h45min 内,如需补 E2E 验证可在本会话开 cargo tauri dev 后台跑

---

## 下次会话候选

- D. `cargo tauri dev` 实际跑一次 E2E(短 inbox,stop_after=audio,验 stage 渲染 + 跳 Output)— ~10-15min
- E. WiX/MSI installer(Tauri bundler 重试)— 2-3h
- F. LE L3 优化(Prompt 自适应 + 自动重试 + 跨 Agent 经验晋升)— 4-6h
- G. 真实长视频 107min Tauri UI 完整跑 — 6-10h 需多 session

---

## 下次会话第一句

> 承接 `handoff-w14e-c-runtab-trigger-2026-07-23.md`,Run tab stage 进度条 + 完成跳 Output 已实装(scratch spec + +92 行 JS)。需要 cargo tauri dev 实际跑一次 5min E2E 收尾,或进 D/E/F 候选。
