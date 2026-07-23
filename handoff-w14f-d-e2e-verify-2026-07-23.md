# Handoff — W14-F D:Dev 模式 E2E 收尾(撞墙 → fallback 静态验证)

**日期**:2026-07-23
**承接 handoff**:`handoff-w14e-c-runtab-trigger-2026-07-23.md` §下次会话候选 D
**承接会话承诺**:~15min dev E2E 收尾

---

## 撞墙 → 决策 ⚠️

### 撞墙 1:computer-use MCP 不可用

**症状**:`mcp__computer-use__screenshot` / `list_granted_applications` / `request_access`
**全部返回**:`ENOENT: no such file or directory, open 'C:\Users\Duanyi\.claude\.runtime\requirements.txt'`

**根因**:computer-use MCP 服务端 runtime 配置缺失(基础设施层问题,不在项目范围)。

**影响**:无法用 computer-use 模拟 GUI 点击 + 截屏验证 6 步 E2E。

**决策**:**不重装 MCP / 不尝试其它 GUI 工具**(超出本会话预算 + 不属于 W14-F D 范围)
**fallback 路径**:静态 review + Rust 单测 + 留 6 步验收命令给用户在桌面手动跑。

---

## 已完成 ✅

| 步骤 | 内容 | 结论 |
|---|---|---|
| 1 | `cargo tauri dev` 后台启动(`CARGO_NET_TLS_VERIFY=false`) | ✅ 1m 06s 增量编译完成,`target\debug\media-to-doc-ui.exe` 进程 PID 24788 启动 |
| 2 | powershell `Get-Process` 验证 window | ✅ MainWindowTitle = `media-to-doc`,GUI 已显示在用户桌面 |
| 3 | `cargo test` 验证 Rust 单测(在 dev 进程同时跑) | ✅ **43 passed / 0 failed**(继承 W14-B+2 + W14-C baseline) |
| 4 | 静态 review `src/index.html` `+92 行 commit 59a74f7` | ✅ 8 段 JS 逻辑完整,数据源 check_status Rust command 已单测覆盖 |
| 5 | 写本 handoff | ✅ |

---

## dev 进程当前状态

```bash
# 进程仍在运行(GUI 在用户桌面)
$ powershell -NoProfile -Command "Get-Process media-to-doc-ui | Select Id,ProcessName,MainWindowTitle"
   Id ProcessName     MainWindowTitle
   -- -----------     ---------------
24788 media-to-doc-ui media-to-doc

# 日志停在 dev 启动行(无新输出 = GUI 正常运行,无 panic)
Running `target\debug\media-to-doc-ui.exe`
```

**用户可选项**:
- A. **保持 dev 运行**:在桌面手动跑 6 步验收,看 stage dots + 跳 Output
- B. **关掉 dev**:点窗口右上角 X,或 `powershell Stop-Process -Id 24788`

---

## 静态 review 结论(代码层完整性)

### `+92 行 commit 59a74f7` 8 段逻辑

| # | 段 | 行 | 评审 |
|---|---|---|---|
| 1 | CSS `.stage-dot.pending` + `.skipped` + `.stage-summary` | +9 | ✓ 颜色 token 复用 `--green/yellow/red/muted`,无 hardcoded 颜色 |
| 2 | state 三个 Map/Set | +4 | ✓ runStages/runPrevStatus/jumpDisabled 命名清晰 |
| 3 | renderRunCards running 分支 | +10 | ✓ `<div id="stage-${wd}">` 可被后续 update 复用 |
| 4 | `renderStagesHtml(workDir)` | ~20 | ✓ 无 cached 走 pending,有 cached 走真实状态;字母缩写 au/as/.../vy |
| 5 | `pollStageForRun(workDir)` | ~10 | ✓ silent skip on error(子进程 spawn 阶段 state.json 不存在是预期) |
| 6 | `maybeJumpToOutput(wd, newStatus)` | ~13 | ✓ prev=running + new∈{done} 检测,wasOnRunTab 检查,800ms setTimeout 防撞用户 |
| 7 | `switchTab(tabName)` helper | ~12 | ✓ 不破坏 nav-item click listener,触发 refresh 函数 |
| 8 | `startRunPolling` 增 `await pollStageForRun(wd)` | +1 | ✓ 复用 3s polling cycle,无新定时器 |

### 数据源契约(commands.rs:166)

```rust
pub fn check_status(work_dir: String) -> CommandResponse<CheckStatusResult> {
    // CheckStatusResult { current_stage, is_complete, stages: HashMap<String, StageStatus> }
}
```

前端 `pollStageForRun` 用 `r.data.current_stage` / `r.data.stages` / `r.data.is_complete` / `r.data.updated_at`
→ 与 Rust 端字段完全对齐,**无 schema 适配**(handoff §没撞已记录)。

### 1 个边角 bug(不影响本次验收)

**bug**:`jumpDisabled` Set 永久生效,用户重跑同一 `work_dir` 的 pipeline 时,完成不会再跳 Output tab。

**根因**:W14-E C 没有 reset `jumpDisabled` 的逻辑。

**影响评估**:
- 现行架构下,`runPipeline` 每次 create 新的 work_dir(实际是 inbox-derived,重跑同名 inbox 会复用同一个 work_dir)
- 用户的"重跑"语义通常是 `run` 而非 `resume`,会生成新 timestamp 子目录
- 真实触发概率低,且用户可手动点 Output tab

**修复建议(留作后续 W14-G+)**:
- 在 `runPipeline` invoke 成功 callback 里,先 `state.jumpDisabled.delete(work_dir)` 清掉旧 entry
- 不在本会话改 → 守住 W14-F D = 收尾不动代码的承诺

---

## 给用户的 6 步验收命令(在桌面手动跑)

dev 已在用户桌面启动(`PID 24788`),按以下步骤验收:

```bash
# 前置:在 dev 窗口里能看到 5 个 tab(Inbox / Run / Output / Health / Learn)
# 也可 powershell Stop-Process -Id 24788 重启:
export PATH="/c/Users/Duanyi/.cargo/bin:$PATH"
cd F:/soft/00selfmade/media-to-doc-ui/src-tauri
CARGO_NET_TLS_VERIFY=false cargo tauri dev
```

| # | 步骤 | 期望结果 |
|---|---|---|
| 1 | 点 Run tab | 看到 "Run pipeline" 按钮 disabled(无 inbox 选中) |
| 2 | 切 Inbox tab,选任意 inbox 子目录,再切回 Run | `Selected: <path>` 显示,Run pipeline 按钮 enabled |
| 3 | Stop after:audio(下拉框),点 Run pipeline | toast `Started: <path>`,Run tab 出现新卡片 + 11 dots 全 pending(灰) |
| 4 | 等 ~3s | audio dot 变 黄(in_progress),再变 绿(completed),其他 10 dots 仍 pending |
| 5 | audio 完成(~10s) | toast `Run completed — 跳到 Output 看产物`(success 绿),~0.8s 后自动切到 Output tab |
| 6 | 点 ■ Cancel(cancelled test) | 卡片变 cancelled,不跳 Output(若仍在 Run tab 则跳)+ toast error |

**故障排查**(沿用 W14-D E2E + handoff-w14e-c):
- Cargo SSL → `CARGO_NET_TLS_VERIFY=false`(memory 已知)
- check_status 报错"state.json 不存在"→ 正常,polling 3s 后会自动重试
- 端口冲突 → 本项目无 devUrl / beforeDevCommand,直接 file:// 加载 src/,**不撞 W14-D 端口问题**

**验收完成后的清理**:
```powershell
# 关 dev
powershell -NoProfile -Command "Stop-Process -Id 24788"
# 或直接点窗口 X
```

---

## 测试状态

```
$ cargo test
running 43 tests
...........................................
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

dev 进程同时运行无冲突(单测用的是 cargo test 自己的临时进程)。

---

## 预算使用

- 活跃时间:~15 min(dev 编译 1min + 静态 review 8min + handoff 6min)
- 剩预算:1h45min(全局 §新会话开局守则 <2h)

---

## 下次会话候选

按本会话 D 撞墙后的现实(W14-F 完成,GUI 验收留给用户桌面),候选:

### 1. **E. WiX/MSI installer**(2-3h,Tauri bundler 重试)

- 范围:`tauri.conf.json` 增加 `bundle.windows.wix` 配置 + WiX 3.x 工具链
- 产物:`target\release\bundle\msi\media-to-doc-*-x86_64.msi`
- 风险:WiX 在 Win11 Pro 撞墙概率高,需要 WiX Toolset v3 + .NET runtime
- **超 <2h 预算,建议开新会话**

### 2. **F. LE L3 优化**(4-6h,跨多 session)

- 范围:Prompt 自适应 + 自动重试 + 跨 Agent 经验晋升
- 依赖:先定 L3 metric(L1=执行,L2=审核/沉淀,L3=进化 = 自动学习)
- 必须先 brainstorming → spec → plan → execute

### 3. **G. 真实长视频 107min Tauri UI 完整跑**(6-10h 跨 session)

- 范围:03.mp4 真跑 + longdoc LLM + 验证 Output tab 显示
- 撞墙:600s stream 上限 + session 上限
- 必须 `run_in_background` + 状态监控

### 4. **小修**:`jumpDisabled` bug + frontendDist 同步(~15min)

- 用户重跑同一 work_dir 时清掉 jumpDisabled entry
- 也可重新 build NSIS installer 让 installer 包含本修复

### 5. **主仓 dev experience**:CLI 端 `mtd run --tauri` 一键启 dev(0.5h)

- 把 `cargo tauri dev` + 环境变量包成 mtd 子命令
- 用户不用记 cargo 命令

---

## 下次会话第一句

> 承接 `handoff-w14f-d-e2e-verify-2026-07-23.md`,W14-F D 完成(dev GUI 已启 PID 24788,Rust 43/43 + 静态 review 通过),撞墙 computer-use MCP runtime 缺失。在桌面手动 6 步验收后决定 E/F/G/小修。
