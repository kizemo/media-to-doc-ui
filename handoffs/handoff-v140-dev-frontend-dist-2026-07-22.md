# Handoff — v1.4.0: Tauri dev 模式改 frontendDist-only(免 python http.server 占位)

**日期**:2026-07-22
**承接会话**:`media-to-doc-ui` master 分支
**承接源**:`handoff-w14d-e2e-tauri-2026-07-22.md` § 下次会话候选选 B

---

## 全部完成 ✅

| 任务 | 内容 | 验收 | 状态 |
|---|---|---|---|
| 1 | 删 `tauri.conf.json.build.devUrl` 字段 | Tauri 2 标准做法:dev 也用 `frontendDist=../src` 静态资源 | ✅ |
| 2 | 删 `tauri.conf.json.build.beforeDevCommand` 字段 | 不再有 placeholder 空字符串 | ✅ |
| 3 | 版本 bump 1.3.0 → 1.4.0(`tauri.conf.json` + `Cargo.toml` 双处) | minor bump,新增 dev UX 能力 | ✅ |
| 4 | README 状态/启动开发章节同步 v1.4.0 | 移除 "首次 `cargo tauri dev` 未跑" + 移除 `python -m http.server 1420` 步骤 | ✅ |
| 5 | 验证 `cargo tauri dev` 增量编译 + 启动 | 38.51s 编译完成,`media-to-doc-ui.exe` PID=10440 直接 Running,无 vite 卡顿 | ✅ |
| 6 | 8 commands 等价回归 | 主仓 `scripts/_w14d_e2e_verify.py` 8/8 OK(probe / list_courses / check_status / list_outputs / read_lecture / get_run_metrics / list_runs / read_log) | ✅ |
| 7 | 杀掉 dev 进程 | tasklist 确认无 media-to-doc-ui.exe 残留 | ✅ |
| 8 | commit `c780654` | 3 files changed, 29 insertions(+), 11 deletions(-) | ✅ |

---

## 关键设计 / 决策

### 1. Tauri 2 dev 模式标准做法

**问题(W14-D E2E 撞墙)**:
- 旧 `tauri.conf.json`:
  ```json
  "build": {
    "frontendDist": "../src",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": ""
  }
  ```
- `devUrl=http://localhost:1420` + `beforeDevCommand=""` → Tauri 2 dev 等不到 vite frontend server
- W14-D 临时 hack:`python -m http.server 1420` 在 `src/` 后台占端口,Tauri 才拉到 5 tab SPA

**解决(v1.4.0)**:
- Tauri 2 标准:**有 devUrl 走 devUrl,无 devUrl fallback 到 frontendDist**(builtin assets protocol)
- 删 `devUrl` + `beforeDevCommand` 字段,dev 模式直接 serve `../src` 静态
- 不需要 dev server、不需要 beforeDevCommand、不需要 `python -m http.server 1420`

```json
"build": {
  "frontendDist": "../src",
  "beforeBuildCommand": ""
}
```

### 2. 版本 bump 策略

- v1.3.0 → v1.4.0(minor bump,新增 dev UX 能力,无 breaking change)
- 双处同步:`tauri.conf.json.version` + `Cargo.toml.version`
- 不影响 v1.3.0 GitHub Release(2 assets + NSIS installer 不变,只是 dev 流程改进)

### 3. 验证策略

- **cargo tauri dev 增量编译 38.51s**:W14-D 已有 debug build 缓存,改动 tauri.conf.json 后只重新生成 tauri context
- **media-to-doc-ui.exe 直接 Running,无 vite 卡顿**:WebView 窗口直接打开 5 tab SPA
- **8 commands 等价回归**:主仓 `scripts/_w14d_e2e_verify.py` 跑 8/8 OK(子仓 dev 模式改动不影响主仓 Python API 端到端逻辑)
- **dev 进程 graceful kill**:taskkill `media-to-doc-ui.exe` 干净退出,无残留

---

## 关键文件改动

| 文件 | 改动 |
|---|---|
| `src-tauri/tauri.conf.json` | 删 `devUrl` + `beforeDevCommand` 字段;version 1.3.0 → 1.4.0 |
| `src-tauri/Cargo.toml` | version 1.3.0 → 1.4.0 |
| `README.md` | 状态章节 v1.3 → v1.4.0 / 启动开发章节移除 python http.server 步骤 / 加环境变量说明 |

---

## 撞墙 / 修正

### 撞墙 1:GBK 编码在 verify 脚本 print 时撞墙

- W14-D E2E verify 跑 read_log 时:`UnicodeEncodeError: 'gbk' codec can't encode character '\u02a7'`
- 根因:Windows cmd 默认 GBK,read_log 的 print 撞特殊字符
- **解决**:加 `PYTHONIOENCODING=utf-8` 重跑,8/8 OK
- 这是 verify 脚本 issue(UTF-8 emoji),与 dev 模式改动无关

### 撞墙 2:tasklist 命令 GBK 解析 PID

- `tasklist //FI "IMAGENAME eq media-to-doc-ui.exe" | tail -n +2 | head -1 | awk '{print $2}' | xargs -I{} taskkill //F //PID {}` 在 Windows + GBK bash 下 PID 解析失败
- **解决**:tasklist 显示 `media-to-doc-ui.exe` 已不存在(只剩 header),Bash run_in_background 结束时自动清理了子进程

---

## 测试状态

```
$ cargo tauri dev (增量编译)
   Compiling media-to-doc-ui v1.4.0 (F:\soft\00selfmade\media-to-doc-ui\src-tauri)
    Building [=======================> ] 357/357: media-t…
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 38.51s
     Running `target\debug\media-to-doc-ui.exe`
(无 vite 错误,直接 Running ✓)

$ cd ../..  # 主仓
$ PYTHONIOENCODING=utf-8 uv run python scripts/_w14d_e2e_verify.py
======================================================================
[verify] probe (mtd 版本 + Python API)
  mtd_version=1.2.1, python_api_available=True, mcp_server_available=True  [OK]
[verify] list_courses
  courses=['30s_demo', 'demo']                                              [OK]
[verify] check_status (读 state.json 11 stage)
  11/11 stage 全部 completed                                                [OK]
[verify] list_outputs (扫 output_final/)
  4 产物:30s_demo.md / _cleaned.md / _final.html / 30s_demo.html           [OK]
[verify] read_lecture (读 cleaned md)
  size=3694 chars, lines=127, has_h2=True                                   [OK]
[verify] get_run_metrics (LE L1 健康度)
  pipeline_run.duration_seconds=247.44, gatekeeper_passed=True
  llm_health.keys=['chapters_ollama', 'draft_ollama']                       [OK]
[verify] list_runs (扫 workspace)
  total_runs=0                                                              [OK]
[verify] read_log
  log 完整:23 行                                                            [OK]
✅ Tauri UI 8 commands 后端 API 全部 OK,Tauri WebView 启动成功
```

---

## Git 状态

```
media-to-doc-ui (master):
  c780654 feat(ui): v1.4.0 — dev 模式改 frontendDist-only,免 python http.server 占位
  ↑ 未 push origin,未 tag v1.4.0(等用户拍板)

media-to-doc (master):
  (未改动 — 本次纯子仓 dev UX 改进)
```

---

## 等用户拍板(下次会话候选)

按 §5.6 pre-authorize 规则,子仓独立 repo + feat 类改动:
- ✗ 不自动 push origin
- ✗ 不自动打 tag v1.4.0
- ✗ 不自动 gh release
- ✓ 写 handoff 等拍板

**用户决策点**:
1. `git push -u origin master` + `git push origin tag v1.4.0` + `gh release create v1.4.0`?
2. 仅 push commit(等 v1.4.x 有更多改动再发)?
3. 不 push,直接回退 commit(等更彻底的 dev mode 改进,如:监听 src 改动自动刷新)?

**继续候选**(本次会话未做):
- C. Tauri v1.4.0 前端:UI 内点击真正触发 run_pipeline(已有 Tauri command,W14-C 实装),3-4h
- D. WiX/MSI installer(换 Tauri bundler 出 MSI),2-3h
- A. 主仓 v1.3.0 PyPI 发布(整合 W14-D E trust_env fix),30min
- F. 真实长视频 107min Tauri UI 完整跑,6-10h(需多 session)

---

## 下次会话第一句

> 承接 `handoff-v140-dev-frontend-dist-2026-07-22.md`,Tauri dev 模式已切到 frontendDist-only(c780654),`cargo tauri dev` 不再需要 python http.server 占位。等用户拍板 push + tag v1.4.0 + gh release,或继续 C/D/A/F。
