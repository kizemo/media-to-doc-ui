# Release Notes — media-to-doc-ui v1.5.0

**发布日期**:2026-07-26
**子仓 tag**:`v1.5.0`(annotated)
**发布地址**:https://github.com/kizemo/media-to-doc-ui/releases/tag/v1.5.0
**主仓 handoff**:见主仓 `handoff-w15-a-t7-2-product-feedback-COMPLETE-2026-07-26.md`

---

## 亮点

### 1. 二轮产品反馈 12-task 全部实装(W15-A T7.2)

| Task | 内容 | 落点 |
|---|---|---|
| T1 | per-run profile(`WorkflowState.profile_name`) | 主仓 CLI + MCP + Tauri |
| T3 | task text 注入 chapter / draft prompt | 主仓 `mtd run --task "..."` |
| T4 | imagegen 策划 LLM + LocalSdxlProvider 最小实现 | 主仓 `imagegen.py` |
| **T9** | **long-doc skill snapshot bootstrap**(23 文件 vendored) | 主仓 `data/long_doc_skill/` |
| **T10** | **longdoc.py 读 vendored**(5 新函数 + importlib.resources) | 主仓 `pipeline/longdoc.py` |
| **T11** | **Tauri bundle.resources + Claude PostToolUse hook** | 子仓 + `~/.claude/settings.json` |

### 2. long-doc skill snapshot 完整 vendored(T9 + T11)

**关键设计**:把 long-doc-processor Skill 真身(SKILL.md + 12 references + 10 scripts + MANIFEST,sha256 校验)作为 `src/media_to_doc/data/long_doc_skill/` 子包,跟随 wheel + Tauri NSIS installer 一起分发。**NSIS 运行时不再依赖 `C:/Users/Duanyi/.claude/` 路径**。

- hatchling 默认打包(snapshot 23 文件进 wheel + sdist)
- `pyproject.toml` 无需 `force-include`(默认 packages 含 data/)
- `importlib.resources.files()` 返真实 Path(`__init__.py` 让 data 成 regular package)
- **T11 自定义 NSIS**:绕过 Tauri bundler(系统 NSIS 3.12),`bundle.resources` 不会自动 copy → `installer.nsi` `File /r` 显式把 snapshot 打到 `$INSTDIR\long_doc_skill\`
- Claude 全局 PostToolUse hook:改 SKILL.md 自动 sync → snapshot 同步

### 3. per-run profile + task text(T1 + T3)

用户原痛点:"每个 run 需要不同 LLM profile(批量场景)"、"想给每个 run 写明确目标"。

- `mtd run --profile <name>` 选 LLM profile(profile 作 gate,不创建新 profile)
- `mtd run --task "目标"` 注入 chapter / draft prompt
- `WorkflowState.profile_name` + `WorkflowState.task_text` 持久化,resume 自动恢复
- Tauri UI `New Run` tab:dynamic dropdowns + task textarea

### 4. imagegen planner LLM(T4)

`SkipProvider` 落 `image_plans.json`(策划阶段),`LocalSdxlProvider` 真出图(实现阶段),profile name 作 gate 复用现有 LLM env/config。

### 5. Project Registry + Keyring(T7-Tauri)

- 5 个新 Tauri commands(`list_projects` / `add_project` / `remove_project` / `open_project` / `update_project_metadata`)
- `src/keyring_store.rs` 用 Windows Credential Locker 安全存 API key
- 多项目并发 + 持久化 registry

### 6. New Run Tab 重做(T8-Tauri)

- `__mountNewRunTab__` 大改:dynamic dropdowns(LLM / Image Agent profile)+ task textarea + 选目录按钮
- 透传到 `runner.rs` `build_mtd_run_args` 加 3 参数(llm_profile_name / image_agent_profile_name / task_text)
- `commands.rs` `run_pipeline` / `resume_pipeline` 透传

### 7. LLM API Settings + Keyring 注入

- `src/llm_profiles.rs` 8 个 providers preset 端到端 UI 配置 + 探测(env vars 透传 env_process spawn)
- keyring 注入 env vars(替代 `inject_active_llm_env`)

---

## Assets

| Asset | Size | SHA256 |
|---|---|---|
| `media-to-doc-1.5.0-setup.exe` | 2,780,157 bytes (2.65 MiB) | `a470f2a84b6099248eac33aac1996c5e459692b64d7144d156d247ccf2b04fb1` |

> 注:v1.5.0 只发布 NSIS installer(无便携版)。便携版如需要,从 `cargo tauri build --target portable` 单独打。

---

## 安装

### Windows(installer,推荐)

1. 下载 `media-to-doc-1.5.0-setup.exe`
2. 管理员运行(perMachine 安装)
3. 装到默认 `D:\Program Files\MediaToDoc\`
4. 桌面 / 开始菜单启动 `media-to-doc`
5. 验证 long_doc_skill 已装:`ls "D:\Program Files\MediaToDoc\long_doc_skill\"`(应见 23 文件)

### 环境配置(必做)

```bash
# 主仓路径
setx MEDIA_TO_DOC_PROJECT "F:\soft\00selfmade\media-to-doc"

# (可选)uv 路径
setx UV_BIN "C:\Users\Duanyi\.local\bin\uv.exe"

# (可选)workspace
setx MEDIA_TO_DOC_WORKSPACE "F:\soft\00selfmade\media-to-doc\workspace"
```

### 验证 long-doc skill snapshot(可选)

```bash
# 在主仓跑 sync 校验
python F:\soft\00selfmade\media-to-doc\scripts\verify_long_doc_skill.py
# 期望:exit 0,无 drift(23 文件 hash 一致)
```

### macOS / Linux

跨平台编译需 Rust 1.97+ + WebKit/GTK 依赖。Tauri 官方文档见 https://tauri.app/start/prerequisites/。

---

## 测试

- **632 pytest 用例 / 0 跳过**(1.3.0 604 → 1.5.0 632,+28)
  - 7 个 T3 task_text
  - 5 个 T4 imagegen planner
  - 5 个 T9 sync_long_doc_skill
  - 7 个 T10 longdoc integration
  - 4 个 T1 profile flags
- **111 cargo test / 0 失败**(Tauri 子仓)
- **ruff**:All checks passed
- **cargo tauri build** exit 0(2m 51s)
- **7z 验证 NSIS**:23 long_doc_skill 文件全含 + 无 __pycache__/ 污染

---

## 已知问题

- Rust toolchain 需 1.97+(自带 lld-link 无需 MSVC)
- 公司 VPN 用户构建时需设 `CARGO_NET_TLS_VERIFY=false`(运行不受影响)
- macOS / Linux 编译需用户自查环境
- 主仓 PyPI 仍为 v1.2.1(子仓独立 release,主仓 wheel 跟随子仓 release 同步重打)

---

## 上游

主仓 `media-to-doc` v1.2.1(`pip install media_to_doc` 装的 Python 后端)已发布:
- PyPI:https://pypi.org/project/media-to-doc/
- GitHub:https://github.com/kizemo/media-to-doc/releases/tag/v1.2.1

UI 端调用此 Python 后端做实际 pipeline,UI 自身只做 orchestrator + log tail + 输出展示。
v1.5.0 包含的主仓新功能(per-run profile / task text / imagegen planner / long-doc skill)随 wheel 一同分发;NSIS installer 内嵌 snapshot,无需运行时读 `~/.claude/`。