# media-to-doc UI

> **Phase 2 / v1.3+** — Tauri 2 桌面壳,把 `media-to-doc` Python 流水线包成"3 次点击跑通"的桌面应用。

## 状态

W14-B+ → v1.4.0 dev 模式改进(2026-07-22):
- Tauri CLI 2.11.4 已装(`~/.cargo/bin/tauri.exe`)
- 项目骨架就位:`src-tauri/`(Rust 后端)+ `src/`(前端,纯 HTML/CSS/JS,无框架)
- 12 个 Tauri commands 实装(W14-B+ hello world + W14-B+ 8 commands + W14-B+2 read_log + W14-C list_all_runs)
- **`cargo tauri dev` 直接启动**(W14-D E2E 撞墙根因:旧 config `devUrl=http://localhost:1420` + `beforeDevCommand=""` 需 `python -m http.server 1420` 占位;W14-D v1.4.0 删 `devUrl` 字段,dev 模式也走 `frontendDist=../src` 静态资源,无需 dev server 也不需 beforeDevCommand)
- 8 commands 等价验证 8/8 OK(W14-D E2E 回归)
- 多课程并发(W14-C,`max_concurrent=3` + LRU 100 + 2s cancel)
- NSIS installer + portable 已发(W14-D C,GitHub Release v1.3.0)

## 架构(详见 [ARCHITECTURE.md](./ARCHITECTURE.md))

```
┌─────────────────────────────────────────────┐
│ Tauri WebView (Chromium)                    │
│   src/index.html — vanilla TS + IPC         │
│   • Inbox page   → 选视频                   │
│   • Run page     → 调 backend.run_pipeline  │
│   • Outputs page → 看 output_final/        │
└─────────────────┬───────────────────────────┘
                  │ Tauri IPC (JSON-RPC)
┌─────────────────▼───────────────────────────┐
│ Rust 后端 (src-tauri/src/lib.rs)            │
│   #[tauri::command]                         │
│   • app_info() / ping()                     │
│   • W14-B+ : run_pipeline / check_status    │
└─────────────────┬───────────────────────────┘
                  │ subprocess / MCP stdio
┌─────────────────▼───────────────────────────┐
│ media-to-doc Python 包                      │
│   • CLI: uv run mtd run/resume/list         │
│   • MCP server: mtd mcp (8 tools)           │
│   • Python API: from media_to_doc import ...│
└─────────────────────────────────────────────┘
```

## 为什么 Tauri(而非 Electron)

| 维度 | Tauri 2 | Electron |
|---|---|---|
| 运行时大小 | ~10MB(WebView2 系统组件) | ~150MB(打包 Chromium) |
| 内存 | 30-50MB | 100-300MB |
| 启动 | <500ms | 1-3s |
| 后端 | Rust(系统级) | Node.js |
| IPC | 同步 + Promise | async |

主要动机:**Tauri 后端用 Rust,可直接 spawn 子进程调 mtd CLI,不绕 Node 层**;前端 WebView2 复用系统 Edge,无打包负担。

## 工具链(W14-B 已就位)

```bash
# 已装(本机 ~/.cargo/bin/)
rustc 1.97.1
cargo 1.97.1
tauri 2.11.4 (cargo-tauri.exe → tauri.exe)

# 镜像配置(~/.cargo/config.toml)
[source.crates-io]
replace-with = 'rsproxy-sparse'
[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"
[net]
git-fetch-with-cli = true
```

## 启动开发

```bash
# 1. 切到项目
cd F:/soft/00selfmade/media-to-doc-ui

# 2. 启动 dev(增量编译 ~30-60s,首次会拉数百个 Tauri 依赖 crate,5-15min)
cargo tauri dev

# 3. 浏览器窗口自动开 → 看 5 tab SPA(Inbox / Run / Output / Health / Learn)
#
# 不需要任何 dev server / python http.server 占位:
# - tauri.conf.json 已删 devUrl(W14-D v1.4.0 改进)
# - dev 模式直接走 frontendDist=../src 静态资源
# - 之前 W14-D E2E 撞墙的 `python -m http.server 1420` hack 不再需要
```

环境变量(可选,主仓路径 / 工具定位):

```bash
# 主仓路径(默认 F:/soft/00selfmade/media-to-doc,可省略)
export MEDIA_TO_DOC_PROJECT="F:/soft/00selfmade/media-to-doc"

# uv 二进制路径(默认 ~/.local/bin/uv 或 PATH 中)
export UV_BIN=/c/Users/Duanyi/.local/bin/uv

# workspace 根目录(默认主仓 workspace/)
export MEDIA_TO_DOC_WORKSPACE="F:/soft/00selfmade/media-to-doc/workspace"
```

## 与 media-to-doc 主仓的关系

- 主仓:`F:/soft/00selfmade/media-to-doc/`(Python 后端 + LE 闭环 + 11 stage)
- 本仓:`F:/soft/00selfmade/media-to-doc-ui/`(Rust + WebView 桌面壳)
- **本仓不入主仓**(Tauri 编译产物 ~50MB+ 不适合 wheel/sdist)
- 本仓不发布 PyPI,只发 GitHub Release

## 下一步(W14-B+ / v1.3)

- 实装 8 个 Tauri commands(对齐 media-to-doc MCP 8 工具):`run_pipeline` / `resume_pipeline` / `check_status` / `list_outputs` / `read_lecture` / `get_run_metrics` / `list_runs` / `list_courses`
- 进度条组件(轮询 `state.json`)
- 系统托盘 + 通知
- 多视频目录选择(W12-D layout)
- NSIS 装包(Phase 3,v1.4)