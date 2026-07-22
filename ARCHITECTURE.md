# media-to-doc UI 架构(W14-B 设计)

> 本文档定义 Tauri 2 桌面壳与 media-to-doc Python 后端的协作模型。
> 与 CLAUDE.md §10(v1.3 Phase 2)路线图对齐。

## 1. 进程模型

Tauri 2 默认一个 Rust 主进程(单实例),前端 WebView 是 WebView2 进程(系统 Edge)。
IPC 通过 Tauri 自有 JSON-RPC(走 named pipe / 共享内存,不走网络)。

```
┌──────────────────────────────────────────────────┐
│ Process 1: media-to-doc.exe (Rust)               │
│   • tauri::Builder → WebView2 host               │
│   • #[tauri::command] handlers                   │
│   • 子进程管理(tokio process::Command)            │
│   • 单实例锁(避免双开)                            │
└──────────┬───────────────────────────┬───────────┘
           │ JSON-RPC IPC              │ spawn child
┌──────────▼──────────────┐ ┌─────────▼───────────┐
│ Process 2: WebView2     │ │ Process 3+: mtd run │
│ (Microsoft Edge runtime)│ │ (uv run python ...) │
│  src/index.html         │ │  11 stage pipeline  │
│  window.__TAURI__.core  │ │  ASR/chapters/draft │
│  invoke('cmd', args)    │ │  ...                │
└─────────────────────────┘ └─────────────────────┘
```

## 2. Tauri Commands(W14-B+ 设计)

8 个命令对齐 media-to-doc MCP 8 工具(W7+W8,见 `docs/MCP_INTEGRATION.md`):

| Tauri command | MCP tool | 调用方式 | 用途 |
|---|---|---|---|
| `list_courses(root)` | `list_courses` | 直接读目录 | 列 inbox 子目录 |
| `run_pipeline(inbox, opts)` | `run_pipeline` | **spawn `uv run mtd run`** | 启动流水线(后台) |
| `check_status(work_dir)` | `check_status` | 读 state.json | 查 stage 进度 |
| `list_outputs(inbox)` | `list_outputs` | 读 output_final/ | 列产物 |
| `read_lecture(inbox, version, fmt)` | `read_lecture` | 读 .md / .html | 打开讲义 |
| `get_run_metrics(work_dir)` | `get_run_metrics` | 调 media_to_doc Python API | LE 健康度 |
| `list_runs(workspace, limit)` | `list_runs` | 同上 | 历史 run |
| `cancel_run(work_dir)` | (无对应 MCP) | kill subprocess | 用户中途取消 |

返回统一 JSON:`{"ok": true, "data": {...}}` 或 `{"ok": false, "error": "msg"}`。

## 3. 子进程管理

`run_pipeline` 必须长跑(107min 视频 ~4h),不能阻塞 Tauri command。

设计:
- `run_pipeline` 立即返回 `{"ok": true, "data": {"work_dir": "..."}}`,后台 spawn `mtd run`
- 后台 task 写 stdin/stdout 到 `<work_dir>/mtd.log`(前端轮询读)
- 用 `tokio::process::Command` + `Child` handle 存到 `tauri::State<RunRegistry>`
- `cancel_run` 通过 Child handle kill(Windows: `taskkill /T /F`)
- 启动时检查 `RunRegistry`,自动 attach 到上次未完成 run(state.json 标记)

## 4. 跨进程真相流

进度监控双轨(继承 media-to-doc 的 LE 设计):
1. **`state.json`**:`mtd run` 写,Tauri `check_status` 读,前端 5s 轮询
2. **`mtd.log`**:实时 stdout/stderr,Tauri 转发到前端 console

前端 UI 组件:
- ProgressBar:11 个 stage 圆点 + 当前 stage 高亮 + percentage
- LogPanel:折叠展开 + 自动滚到底 + grep 过滤(关键词:`LLM failure` / `gatekeeper` / `verify`)

## 5. 前端(W14-B 极简,W14-B+ 实装)

技术选型:
- **W14-B MVP**:vanilla HTML + CSS + JS(零打包,WebView2 直接解析)
- **W14-B+ 实装**:Vue 3 + TypeScript + Vite(开发体验 vs 体积权衡)
- 状态管理:Pinia(轻量,与 Vue 3 官方)
- UI 组件:Tauri 自带 WebView2 渲染,无需 Electron-style 组件库

页面布局(单 SPA + 侧边栏):
```
┌──────────────────────────────────────────────┐
│  ⬢ media-to-doc          [● running 3/11]   │
├────────┬─────────────────────────────────────┤
│ Inbox  │                                     │
│ Run    │   <主内容区>                        │
│ Output │                                     │
│ Health │                                     │
│ Learn  │                                     │
└────────┴─────────────────────────────────────┘
```

## 6. 错误处理

按 media-to-doc 的 Loop Engineering 设计:
- gatekeeper failure → toast 警告 + 红点
- LLM unhealthy → 状态栏黄色 + 提示换 provider
- imagegen skip → 提示配图为空,讲义仍可看
- 长 run 失败 → 自动写 `pipeline_run.json` + LE L1 沉淀

## 7. 安全模型(Tauri 2 capabilities)

`src-tauri/capabilities/default.json` 定义权限白名单:
- W14-B:`core:default` 最小集
- W14-B+:`core:default` + 自定义 permission(`shell:allow-execute` for mtd subprocess)
- 严格 CSP:`default-src 'self'; img-src 'self' data:;`
- 不暴露任意文件系统(只允许 `inbox_dir` / `output_dir` 范围)

## 8. NSIS 装包(Phase 3,v1.4)

W14-B+ 后续:
- `cargo tauri build` 输出 `target/release/bundle/nsis/*.exe`
- NSIS 模板:
  - 安装 `media-to-doc.exe` 到 `C:\Program Files\media-to-doc\`
  - 检测 Python(若无,引导装 embed Python)
  - 检测 Ollama(若无,引导装)
  - 桌面 + 开始菜单快捷方式
  - 添加 PATH(可选)
  - 卸载脚本:`Uninstall.exe`

## 9. 与 media-to-doc 的版本兼容

- 本仓不强制 lock media-to-doc 版本(用户自由安装任意版)
- `app_info.mtd_version` 调 `uv run mtd --version` 探测
- Tauri 启动时若 `mtd` 不可用,引导用户装:`media_to_doc[llm,asr,...]`
- MCP 协议版本与 media-to-doc 主仓同步

## 10. 已知风险 + 缓解

| 风险 | 缓解 |
|---|---|
| 首次 `cargo build` 拉数百个 Tauri crate(VPN SSL MITM 撞) | rsproxy.cn sparse mirror + git-fetch-with-cli;失败回退 vendor |
| WebView2 在老 Win10 缺失 | Tauri 启动检测,引导装 WebView2 Runtime |
| 长 run 崩溃留 zombie 子进程 | `tauri::State<RunRegistry>` 启动时 attach + 父进程退出时清理 |
| mtd 输出 stdout UTF-8 编码 | Rust 端 `read_to_string` 后 `String::from_utf8_lossy`,前端展示原文 |
| 用户多开 | `tauri-plugin-single-instance` 插件 |