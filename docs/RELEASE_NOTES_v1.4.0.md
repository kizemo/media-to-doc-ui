# Release Notes — media-to-doc-ui v1.4.0

**发布日期**:2026-07-23
**子仓 tag**:`v1.4.0`(annotated,指向 `c780654` frontendDist-only feat)
**发布地址**:https://github.com/kizemo/media-to-doc-ui/releases/tag/v1.4.0
**主仓 handoff**:见同目录 `handoff-v140-dev-frontend-dist-2026-07-22.md`

---

## 亮点

### 1. dev 模式 frontendDist-only(免 python http.server 占位)

- **问题(W14-D E2E)**:旧 `tauri.conf.json` 有 `devUrl=http://localhost:1420` + `beforeDevCommand=""` → `cargo tauri dev` 等不到 vite frontend server → 临时 hack 用 `python -m http.server 1420` 占端口
- **修复(v1.4.0)**:Tauri 2 标准做法 — 删除 `devUrl` + `beforeDevCommand` 字段,dev 模式也走 `frontendDist=../src` builtin assets protocol
- **收益**:
  - 不再需要 dev server
  - 不再需要 `beforeDevCommand`
  - 不再需要 `python -m http.server 1420` 占位
  - `cargo tauri dev` 增量编译 38.51s,直接 Running 无 vite 卡顿

```diff
// src-tauri/tauri.conf.json build section
  "build": {
    "frontendDist": "../src",
-   "devUrl": "http://localhost:1420",
-   "beforeDevCommand": "",
+   "beforeBuildCommand": ""
  }
```

### 2. NSIS installer 同步 bump 1.3.0 → 1.4.0

- `installer.nsi` `PRODUCT_VERSION` + `OutFile` 跟随子仓版本
- 产出 `media-to-doc-1.4.0-setup.exe`(~1.58MB,同 1.3.0 安装逻辑)

### 3. 8 个 Tauri commands 持续工作(无回归)

| Command | 语义 |
|---|---|
| `list_courses` | 扫描 inbox 课程目录 |
| `run_pipeline` | 启动 11 stage pipeline |
| `resume_pipeline` | 中断后续跑 |
| `cancel_run` | 同步取消(≤2s 超时) |
| `check_status` | 读 state.json |
| `list_outputs` | 列 output_final 产物 |
| `read_lecture` | 读 md/html,优先 W12-D 布局 |
| `get_run_metrics` | LE L1 健康度查询 |
| `list_runs` | LE 跨 run 健康度 |
| `read_log` | mtd.log offset 模式 tail(W14-B+2) |

回归验证:主仓 `scripts/_w14d_e2e_verify.py` 8/8 OK(子仓 dev 模式改动不影响主仓 Python API 端到端逻辑)。

---

## Assets

| Asset | Size | SHA256 |
|---|---|---|
| `media-to-doc-1.4.0-setup.exe` | ~1.58MB | (gh release page 显示) |
| `media-to-doc-1.4.0-portable.exe` | ~6.23MB | (gh release page 显示) |

---

## 安装

### Windows(installer,推荐)

1. 下载 `media-to-doc-1.4.0-setup.exe`
2. 管理员运行(perMachine 安装)
3. 装到默认 `C:\Program Files\MediaToDoc\`
4. 桌面 / 开始菜单启动 `media-to-doc`

### Windows(portable,免安装)

1. 下载 `media-to-doc-1.4.0-portable.exe`
2. 双击运行(无需安装)

### 环境配置(必做)

```bash
# 主仓路径
setx MEDIA_TO_DOC_PROJECT "F:\soft\00selfmade\media-to-doc"

# (可选)uv 路径
setx UV_BIN "C:\Users\Duanyi\.local\bin\uv.exe"

# (可选)workspace
setx MEDIA_TO_DOC_WORKSPACE "F:\soft\00selfmade\media-to-doc\workspace"
```

### macOS / Linux

跨平台编译需 Rust 1.97+ + WebKit/GTK 依赖。Tauri 官方文档见 https://tauri.app/start/prerequisites/。

---

## 升级路径

从 v1.3.0 升级:

- installer:覆盖安装(NSIS 自动卸载 v1.3.0)
- portable:直接替换 exe
- 配置 / workspace / inbox 不需变动

---

## 测试

- **43 cargo test / 0 failed**(W14-C A baseline + W14-B+2 +9)
- 8 commands 全部经 cargo test 验证
- `cargo tauri dev` 启动时间 38.51s(增量),首次全量约 5min
- 主仓端到端 8/8 verify OK

---

## 已知问题

- Rust toolchain 需 1.97+(自带 lld-link 无需 MSVC)
- 公司 VPN 用户构建时需设 `CARGO_NET_TLS_VERIFY=false`(运行不受影响)
- macOS / Linux 编译需用户自查环境

---

## 上游

主仓 `media-to-doc` Python 后端 v1.2.1 已发布:
- PyPI:https://pypi.org/project/media-to-doc/
- GitHub:https://github.com/kizemo/media-to-doc/releases/tag/v1.2.1

UI 端调用此 Python 后端做实际 pipeline,UI 自身只做 orchestrator + log tail + 输出展示。