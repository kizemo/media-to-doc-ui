# Spec — W15-A T7.2 第二轮产品反馈收口

**作者**:W15-A T7.2 新会话 / 2026-07-25
**承接**:`handoff-w15-a-task12-build-verify-2026-07-25.md` §0.6 / §6 + 13 项验收通过后第二轮反馈
**目标版本**:`feat/w15a-llm-api-settings` 上累积,最终随 v1.5.0 release 一起发布(T8)
**关系**:与 `2026-07-23-w15-a-llm-api-settings-design.md` + `2026-07-24-w15-a-ux-redesign-design.md` 衔接(后端 + UX 已实装,本 spec 只覆盖 P0-A/P0-B/P0-C/P1)

---

## 1. 目标与范围

### 1.1 用户第二轮真机反馈(2026-07-25)

| # | 用户原话 | 当前事实 | 优先级 |
|---|---|---|---|
| 1 | 已添加 MiniMax,但 New Run 的 LLM 下拉没有该项;Image Agent 也应能选在线大模型 | `src/index.html:967-980` 静态协议名;Rust 启动只注入"全局 active profile",无 per-run profile 参数。`imagegen` 仍是 `skip/local_sdxl`,且 `LocalSdxlProvider` 写空占位 | **P0-A** |
| 2 | New Run 应有会话框发布任务;会话框下方可选目录;所选目录自动成为左侧项目,已存在则合并 | 当前只有结构化 form;无 task text、无 directory picker、无 project registry | **P0-B** |
| 3 | Stop after 是什么 | UI 仅英文 stage 名,无中文说明 | 说明 |
| 4 | 定时任务不能点击 | spec 故意 disabled,W15-B+ 占位 | **P2 / parked** |
| 5 | 检查 `long-doc-processor` 是否整合,并保证 Claude 修改后自动同步 | Python `longdoc.py` 仅注释"参考 Skill";UI 透传 `--no-longdoc`;无 vendored snapshot / sync / verify / hook | **P1** |

### 1.2 已澄清的设计决策(用户拍板)

| 决策 | 用户选择 |
|---|---|
| 任务文本下游用途 | **落盘到 work_dir + 进入 LLM**(写 `work_dir/task.md` 与 `state.json.task_text`,LLM chapter/draft prompt 自动把这段作为前缀注入) |
| Image Agent 两层关系 | **两层完全独立可各自不同**(策划 LLM dropdown 独立,出图 provider dropdown 独立;不强制复用主 LLM) |

### 1.3 范围之外(本 spec 不做)

- ❌ 不 commit / push / release / bump / reset / checkout(加快模式仍生效,T8 才合并)
- ❌ 不实现定时调度器(W15-B+ 留待)
- ❌ 不改 Settings tab UI / 已有 6 个 LLM command / keyring_store / runner.tspawn 逻辑(只在 §1 增 2 个 flag)
- ❌ 不修改 Tasks 1-11 未提交工作区(只新增增量)
- ❌ 不修改现有 13 项验收清单(只追加 7 项新验收,见 §6)

---

## 2. 设计 — P0-A Per-run LLM Profile

### 2.1 当前问题

- `commands.rs:1128` `inject_active_llm_env` 强制读 `llm_profiles::get_active_profile()`,无视 `--llm` 已透传的 provider 名
- 这意味着用户若 Settings 加了 3 个 profile 但只在 Settings 激活 1 个,即使 New Run `--llm anthropic` 显式选 anthropic,env_vars 也只含 active profile 的 key — **并发跑不同 profile 会串号**
- 前端 `__mountNewRunTab__` 下拉写死 3 个字符串,与 `list_llm_profiles` 数据完全脱节

### 2.2 改造方案

**前端**(增量改动):

- `__mountNewRunTab__` mount 时 `await invoke('list_llm_profiles')` → 填两个 `<select>`(主 LLM + Image Agent 策划 LLM),都按 profile `name` 选,默认含 `(default)` 选项(空字符串 → 走 CLI 默认 + 不注入 env)
- 新建/编辑 profile 后(已经在 Task 11 用 `loadProviders()` 刷设置面板):再加一次 `refreshNewRunDropdowns()` 重抓 list
- 选中 profile 后展示 `provider / model`(用 `title` attr 或 readonly 副文本),让用户看得到选了哪个

**Tauri command**(`src-tauri/src/commands.rs` 增量):

```rust
#[tauri::command]
pub async fn run_pipeline(
  inbox_dir: String,
  workspace_root: Option<String>,
  /// 用户从下拉选的 profile name;None = 走 CLI `--llm` 显式值或不传
  llm_profile_name: Option<String>,
  /// Image Agent 策划 LLM profile name;None = Image Agent 关闭
  image_agent_profile_name: Option<String>,
  imagegen: Option<String>,
  stop_after: Option<String>,
  no_longdoc: Option<bool>,
  force: Option<bool>,
  /// 用户在 New Run textarea 写的任务文本;None = 跳过
  task_text: Option<String>,
) -> CommandResponse<RunPipelineResult>
```

- **删除** `inject_active_llm_env` 调用;改成 `inject_profile_env(spec, profile_name_opt)` —— 按 name 查 profile,失败 `PROFILE_NOT_FOUND`,Ollama 空 key 走兼容分支
- `build_mtd_run_args` 增加 `llm_profile_name: Option<&str>` / `image_agent_profile_name: Option<&str>` / `task_text: Option<&str>` 参数 → 转成 `--llm-profile-name` / `--image-agent-profile-name` / `--task-text` CLI flag
- `resume_pipeline` 同样加 `llm_profile_name` 等 3 个参数(续跑也能改 profile)

**主仓 CLI**(`media-to-doc` `src/media_to_doc/cli.py` 增量):

- `mtd run` / `mtd resume` 增加 3 个 flag,与 Tauri 同名
- 落到 `WorkflowConfig.llm_profile_name` / `image_agent_profile_name` / `task_text` 字段
- LLMConfig 初始化时:若 `llm_profile_name` 非空 → 读 metadata + keyring(主仓侧实现,keyring 跨平台用 keyring 库)→ 派生 base_url/model/env;否则维持原 `LLM_*` env / `--llm` 路径
- `task_text` 字段 → `state.json.task_text` 持久化 + chapter/draft stage prompt prefix

### 2.3 并发安全

每个 run 在 `build_mtd_run_args` 时**独立** derive env_vars;spawn 时 `.env_clear() + .envs(&spec.env_vars)`(W14-D + T5 既有路径),无全局态。

### 2.4 测试(必跑)

| 测试 | RED→GREEN 路径 |
|---|---|
| `build_mtd_run_args_with_profile_name_adds_flag` | 传入 `llm_profile_name=Some("minimax-prod")` → args 含 `--llm-profile-name minimax-prod` |
| `build_mtd_run_args_with_image_agent_profile_name_adds_flag` | 传入 `image_agent_profile_name=Some("deepseek-prod")` → args 含 `--image-agent-profile-name deepseek-prod` |
| `build_mtd_run_args_with_task_text_adds_flag` | 传入 `task_text=Some("...")` → args 含 `--task-text ...` |
| `inject_profile_env_errors_on_profile_not_found` | 不存在的 name → `PROFILE_NOT_FOUND` |
| `inject_profile_env_ollama_does_not_require_keyring` | provider=Ollama + keyring 无 key → 空 key 注入,不报错 |
| `run_pipeline_with_profile_name_replaces_active_profile` | 传 `llm_profile_name=Some("minimax-prod")` 时即使 active 是 deepseek,spec.env_vars 也只含 MiniMax key |

改 `commands.rs` / `runner.rs` 必须 2 轮 review(本会话 + 上一会话 reviewer)。

---

## 3. 设计 — P0-B New Run 任务文本 + 目录选择 + 项目注册表

### 3.1 前端 New Run 改造(增量)

`__mountNewRunTab__` 当前 form(行 958-1008)改造顺序:

1. Course `<dt>` 下方插入:`<textarea name="taskText" rows="4" placeholder="说说你想怎么处理这个视频(可选)..."></textarea>`
2. Image Agent 折叠面板(默认折叠)插在 LLM 下拉后:`<details><summary>Image Agent 策划: <profile> · 出图: <provider></summary>...</details>`
3. Course 行右侧加按钮:`<button type="button" id="new-run-pick-dir-btn">选目录</button>`(点了就调 `tauri-plugin-dialog` 的 `open({directory: true})`)
4. 选完目录:显示规范化绝对路径 + `add_project` 调后端注册 + 刷新侧栏(`__projectTree__.refresh()`)+ 自动选中该项目

### 3.2 项目注册表(persistent)

**位置**:`${app_config_dir}/project_registry.json`,通过 `tauri::api::path::app_config_dir()` 取(app_config_dir = `%APPDATA%/com.duanyi.mediatodoc/` on Windows)

**schema**:

```json
{
  "version": 1,
  "projects": [
    {
      "id": "<sha256(canonical_path)[:16]>",
      "path": "<规范化绝对路径>",
      "display_name": "<最后一段目录名>",
      "last_used_at": "2026-07-25T10:00:00Z",
      "added_at": "2026-07-24T08:00:00Z",
      "sessions": [
        { "work_dir": "...", "started_at": "...", "status": "running|completed|failed|cancelled" }
      ]
    }
  ]
}
```

**身份规则**:用规范化路径(`std::fs::canonicalize` + Windows 大小写归一 + NFC unicode)生成的 sha256 前 16 位做 ID。**绝不用 display_name 做 ID**(重名不同路径不误合并)。

**新建入口**:
- 目录选择按钮(picker)
- `run_pipeline(inbox_dir=...)` 入参不在注册表时自动 `add_project` 一次(幂等)

### 3.3 Tauri commands(增量,放 commands.rs 末尾)

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct ProjectEntry { id, path, display_name, last_used_at, added_at, sessions }

#[tauri::command]
pub fn list_projects() -> CommandResponse<Vec<ProjectEntry>>;

#[tauri::command]
pub fn add_project(path: String) -> CommandResponse<ProjectEntry>;
  // canonicalize → 算 id → upsert(同 id 不覆盖 last_used_at + sessions)
  // 返回新 entry

#[tauri::command]
pub fn remove_project(id: String) -> CommandResponse<()>;
  // 从 registry 删,不动 work_dir / output

#[tauri::command]
pub fn touch_project(id: String) -> CommandResponse<()>;  // 更新 last_used_at
```

### 3.4 目录选择(dialog 插件)

- `src-tauri/Cargo.toml` 加 `tauri-plugin-dialog = "2"`(W15-A 还没装)
- `src-tauri/src/lib.rs` 注册 `tauri::plugin::Builder::new().build::<tauri_plugin_dialog::TauriPlugin>()`(具体看 plugin 文档)
- `src-tauri/capabilities/default.json` 加 `"dialog:default"` 权限(若需要)
- 前端 `import { open } from '@tauri-apps/plugin-dialog'` 或 `window.__TAURI__.dialog.open`

### 3.5 测试(必跑)

| 测试 | 内容 |
|---|---|
| `list_projects_returns_empty_when_registry_missing` | registry 文件不存在 → `[]`,不报错 |
| `add_project_canonicalizes_and_dedupes` | 两次 add 同一路径(canonicalize 后)→ 第二条不重复,id 一致 |
| `add_project_different_paths_same_name_have_different_ids` | `D:/a/foo` + `E:/b/foo` → id 不同 |
| `remove_project_persists` | remove 后重新 list 不含 |
| `add_project_windows_path_case_insensitive` | `C:/Foo` vs `c:/foo` → 同一 id(Windows 路径大小写归一) |
| `task_text_persisted_to_state_json` | 跑 mock pipeline → `state.json.task_text` 含传入文本 |
| `task_text_injected_as_chapter_prompt_prefix` | 用 mock LLM 验证 chapter stage prompt 第一行 = task_text |

### 3.6 task_text LLM 注入

主仓 `src/media_to_doc/pipeline/chapters.py`(W12-E 已实装 LLM-driven chapter fusion):

- `chapter_prompt` 构造时若 `cfg.task_text` 非空,在最前面加 `USER_INSTRUCTION: {task_text}\n\n`(多段 task 用换行分隔,trim)
- draft stage 同样注入(`drafts.py`)
- LLM 端看得到用户原始意图,但 imagegen / asr 等阶段不注入(减少噪音)

---

## 4. 设计 — P0-C Image Agent 两层独立

### 4.1 当前事实

- `imagegen` 当前只有 `skip` / `local_sdxl`(命令层),主仓 `LocalSdxlProvider` 是占位实现
- 没有"策划 LLM"概念,prompt 是 hardcode 的
- 用户反馈"Image Agent 也应能选在线大模型" → 需要真正接入 LLM 策划

### 4.2 策划 LLM(第一层)

- 见 §2:`image_agent_profile_name` 独立 dropdown,可选 `None`(Image Agent 关闭)
- 前端折叠面板默认折叠,展开后看到「策划 LLM:<select>」 + 「出图 provider:<select>」
- 主仓 imagegen stage 加 `_plan_prompts(transcript, ocr, profile) -> list[ImagePlan]` 函数,用策划 LLM 生成每张图的:
  ```python
  @dataclass
  class ImagePlan:
      chapter_id: str
      slot: int
      prompt: str           # SDXL prompt
      negative_prompt: str
      style_hint: str       # 风格提示
  ```

### 4.3 出图 provider(第二层)

- 沿用 `--imagegen` (skip/local_sdxl),**新增 `sdxl_remote`** placeholder(stub: 报 `PROVIDER_NOT_IMPLEMENTED`,UI 隐藏或标灰)
- `local_sdxl` 真正占位问题:**补一个最小可跑实现**(不调 SDXL 真模型,返回 `ProviderNotConfigured` + 让 Image Agent 自然 fallback skip),至少让 `--imagegen local_sdxl` 不再写空占位
- `skip` 路径:只让策划 LLM 写 prompt 到 `imagegen/drafts/<stem>_plans.json`,**不出图**,与 §3.5 "落盘但不出图"语义对齐

### 4.4 主仓代码增量

- `src/media_to_doc/pipeline/imagegen.py`:
  - `LocalSdxlProvider.generate(image_plans, work_dir) -> list[Path]` 最小实现:`return []` + `logger.warning("LocalSdxlProvider not configured, returning empty")`
  - `SkipProvider.generate(...) -> list[Path]`:沿用现有(只写 plans.json)
  - `imagegen_stage(workflow)`:若有 `image_agent_profile_name` + 策划 LLM profile 可用 → 调 `_plan_prompts`;否则 fall back skip + log "策划 LLM 不可用,跳过配图策划"
- `src/media_to_doc/llm/__init__.py`:`create_planner_provider(profile_name) -> BaseLLMProvider` —— 用 profile name 派生 provider(与主 LLM 路径同形,但 model 可不同)
- `src/media_to_doc/pipeline/state.py`:`state.json.image_plans` 字段落盘 plans

### 4.5 测试(必跑)

| 测试 | 内容 |
|---|---|
| `imagegen_skip_writes_plans_json_without_calling_provider` | `imagegen=skip` + 有策划 LLM profile → 写 plans.json,不出图 |
| `imagegen_skip_without_planner_logs_skip` | `imagegen=skip` + 无策划 LLM profile → log "策划 LLM 不可用",plans.json 不写 |
| `imagegen_local_sdxl_returns_empty_with_warning` | `imagegen=local_sdxl` 无 SDXL 配置 → 返回 `[]` + warn,不抛 |
| `local_sdxl_placeholder_no_longer_writes_empty_file` | 跑完 imagegen stage 后 `output/imagegen/` 不再含 0 字节 `placeholder.txt` |
| `planner_provider_uses_image_agent_profile_name` | 传 `image_agent_profile_name="deepseek-prod"` → LLM call 走 deepseek,不走主 LLM |

---

## 5. 设计 — P1 long-doc Skill 同源(vendored + 自动同步)

### 5.1 当前事实(已审计)

- Skill 真身:`C:/Users/Duanyi/.claude/skills/long-doc-processor/`(v4.0 系列)
- 主仓 `src/media_to_doc/pipeline/longdoc.py:26-27,54-86,180` 仅借鉴规则,内嵌 prompt(独立副本)
- UI 只透传 `--no-longdoc`,不调用 Skill
- `~/.claude/settings.json` 有 hooks 框架,但无 long-doc 同步 hook
- 两仓均无同步/校验脚本

### 5.2 vendored snapshot 方案

**白名单**(不复制 evals/lessons.md 等,只复制规则 + 脚本):

```
src/media_to_doc/data/long_doc_skill/
├── SKILL.md
├── references/
│   ├── content-rules.md
│   ├── gotchas.md
│   ├── image-pipeline.md
│   ├── image-style.md
│   ├── ooxml-numbering.md
│   ├── phase-0-input.md
│   ├── phase-1-purification.md
│   ├── phase-2-merge.md
│   ├── phase-3-render-html.md
│   ├── qa-gates.md
│   ├── runtime-compatibility.md
│   └── maintenance.md
├── scripts/
│   ├── doc_to_md.py
│   ├── generate_image.py
│   ├── markdown_to_docx.py
│   ├── markdown_to_html.py
│   ├── ocr_images.py
│   ├── renumber_headings.py
│   ├── validate_skill.py
│   ├── verify_docx.py
│   ├── verify_html.py
│   └── xmind_to_md.py
└── MANIFEST.json
```

**MANIFEST.json schema**:

```json
{
  "version": "1.0.0",
  "synced_from": "C:/Users/Duanyi/.claude/skills/long-doc-processor/",
  "synced_at": "2026-07-25T10:00:00Z",
  "files": [
    { "path": "SKILL.md", "sha256": "...", "size_bytes": 12345 },
    { "path": "references/content-rules.md", "sha256": "...", "size_bytes": ... },
    ...
  ]
}
```

### 5.3 Python 加载(vendored 真身,不读 ~/.claude)

`src/media_to_doc/pipeline/longdoc.py`:

```python
import importlib.resources

def _skill_root() -> Path:
    """返回 vendored snapshot 真身路径,不读 ~/.claude。"""
    root = importlib.resources.files("media_to_doc.data.long_doc_skill")
    return Path(str(root))  # 或 with as_file() 走 tempfile(开发态友好)

def load_purification_prompt() -> str:
    return (_skill_root() / "references" / "phase-1-purification.md").read_text(encoding="utf-8")
```

**pyproject.toml**:

```toml
[tool.setuptools.package-data]
media_to_doc = ["data/long_doc_skill/**/*"]

# 或 hatch:
[tool.hatch.build.targets.wheel.force-include]
"src/media_to_doc/data/long_doc_skill" = "media_to_doc/data/long_doc_skill"
```

### 5.4 Tauri 打包(NSIS 自包含)

`src-tauri/tauri.conf.json`:

```json
{
  "bundle": {
    "resources": [
      "../../src/media_to_doc/data/long_doc_skill/**/*"
    ]
  }
}
```

NSIS installer 自带 snapshot → 运行时 `importlib.resources` 从 wheel 内读 → **不依赖用户机器的 `~/.claude` 路径**。

### 5.5 同步 / 校验脚本(主仓根目录)

**`scripts/sync_long_doc_skill.py`**:

- 读 `CLAUDE_SKILLS_PATH` env var / 默认 `~/.claude/skills/long-doc-processor/`
- 不存在 → 报 `SOURCE_MISSING`,exit 1(允许首次 bootstrap: `--bootstrap` 从同仓备份恢复)
- 复制白名单 → 写 `MANIFEST.json`(sha256 + size + synced_at + synced_from)
- 输出 diff 报告(新增/修改/删除)
- 退出码:成功 0,源缺失 1(可被 hook 容忍)

**`scripts/verify_long_doc_skill.py`**:

- 重算 snapshot 内每个文件 sha256
- 与 MANIFEST 对比 → 不一致 `exit 1`(CI / pre-commit 用)
- 一致 `exit 0`

### 5.6 Claude hook(自动同步)

**`~/.claude/settings.json`** 新增:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "python F:/soft/00selfmade/media-to-doc/scripts/sync_long_doc_skill.py"
          }
        ],
        "condition": {
          "file_path_matches": "C:/Users/Duanyi/.claude/skills/long-doc-processor/**"
        }
      }
    ]
  }
}
```

**首次实施流程**(用户必交付):
1. 先调 `update-config` Skill(`Skill(skill="update-config")`)尝试 schema-aware 改 settings.json
2. 若 `update-config` 报错(已知 settings.json schema 可能对 hooks 嵌套挑剔):记录 blocker 到 handoff,然后**手编最小 JSON 增量**,diff 现有 hooks → 加新 hook → 保存
3. 触发验证:在 Skill 真身随便改一个文件,看 snapshot 是否同步(对比 sha256)
4. 不要删除或重排已有 hook

### 5.7 测试(必跑)

| 测试 | 内容 |
|---|---|
| `sync_long_doc_skill_copies_whitelisted_files` | mock 源目录 → 跑 sync → snapshot 含预期文件,不含 evals/lessons.md |
| `sync_long_doc_skill_writes_manifest_with_correct_sha256` | 跑 sync → 读 MANIFEST → 验 hash |
| `sync_long_doc_skill_source_missing_exits_1` | 源不存在 → exit 1 |
| `verify_long_doc_skill_passes_when_match` | sync 后 verify → exit 0 |
| `verify_long_doc_skill_fails_on_drift` | 手动改 snapshot 一个文件 → verify → exit 1 |
| `longdoc_module_reads_vendored_snapshot` | import media_to_doc.pipeline.longdoc → load_purification_prompt() 返回真实内容(集成测试) |
| `pyproject_wheel_includes_long_doc_skill_data` | `python -m zipfile -l dist/*.whl` 含 `media_to_doc/data/long_doc_skill/SKILL.md` |
| `tauri_conf_resources_includes_long_doc_skill` | `grep` 确认 `tauri.conf.json` `bundle.resources` 含 snapshot 路径 |
| `claude_post_tool_use_hook_triggers_sync` | 模拟 Edit/Write 命中 long-doc-processor 路径 → sync 跑过(集成测试,可手动验证) |

### 5.8 Wheel / NSIS 自包含验证

- `uv build` 后 `python -m zipfile -l dist/*.whl | grep long_doc_skill` 必有
- `cargo tauri build` 后 NSIS installer 用 `7z l` 看含 snapshot
- 主仓 `uv run pytest -k longdoc` 全过(集成测试用 vendored 真身)

---

## 6. 设计 — 说明:Stop after 中文 tooltip + 定时任务保持 parked

### 6.1 Stop after

- 当前 11 个 `<option>` 标签不改值,不改顺序
- 每个 `<option>` 改用 `<option>` 后跟的 `<span title="...">` 在 select 旁显示;或在 select 上挂 `<span class="tooltip" data-tip-target="stopAfter">?</span>`,hover/click 弹中文说明
- 说明文本见 handoff §0.6 的中文 stage 表(audio=提取声音 / asr=转文字 / frames=关键帧 / ocr=识别画面字 / asr_correct=校正转写 / chapters=章节结构 / draft=分章草稿 / imagegen=AI 配图 / render=生成 md/html / longdoc=深度净化 / verify=质量检查)
- `(none)` = 完整运行到 verify

### 6.2 定时任务按钮

- 保持 `disabled`,`title="后续版本提供(W15-B+)"`
- 不算验收失败

---

## 7. 测试与验证

### 7.1 测试矩阵

| 层 | 命令 | 期望 |
|---|---|---|
| Rust 单元 + 集成 | `cargo test --lib` | 98(既有) + ≥11(新增 P0-A §2.4 6 + P0-B §3.5 Rust 子集 5) = **≥109** 全过 |
| Rust 构建 | `cargo tauri build` | exit 0,5 个既有 warnings 不增 |
| 主仓 pytest | `cd F:/soft/00selfmade/media-to-doc && uv run pytest` | 604(既有) + ≥14(P0-B §3.5 Python 2 + P0-C §4.5 5 + P1 §5.7 Python 7) = **≥618** 全过 |
| 长文档 vendored 集成 | `uv run pytest -k longdoc` | 全过,确认 snapshot 真身可读 |

### 7.2 沙箱真机验收(7 项新增,13 项既有保留)

**新增项**(2026-07-25 起,append 到 13 项后):

| # | 步骤 | 期望 |
|---|---|---|
| 14 | New Run tab 打开 → 看 LLM 下拉 | 列出已保存 profile name(含 MiniMax) |
| 15 | Image Agent 折叠面板展开 | 显示策划 LLM + 出图 provider 两个独立下拉 |
| 16 | 选完 LLM + imagegen + Stop after 各阶段看 tooltip | 显示中文说明 |
| 17 | New Run 顶部 task textarea 输入"测试任务引导" | 输入后 form 仍可提交 |
| 18 | 提交后 `output/state.json.task_text` | 含 "测试任务引导" |
| 19 | New Run 点 "选目录" 选 D:/另一处课程 → 左侧立即出现 | 路径出现在 §4 项目树 |
| 20 | 同路径再次 add(重复点) | 同一项目合并 sessions,不重复 |

**P1 验收(自动,不装机)**:

| # | 步骤 | 期望 |
|---|---|---|
| 21 | 手动改 `~/.claude/skills/long-doc-processor/SKILL.md` 一行 | 同步脚本跑过,主仓 `data/long_doc_skill/SKILL.md` 更新,MANIFEST hash 更新 |
| 22 | 跑 `verify_long_doc_skill.py` | exit 0 |
| 23 | 手动改主仓 snapshot 一文件 | `verify_long_doc_skill.py` → exit 1 |

### 7.3 加快模式红线

- ❌ 不 commit / push / release / bump
- ❌ 不 reset / checkout / restore / 覆盖
- ❌ 不修改 `commands.rs` / `runner.rs` 之外已有逻辑(只增不改)
- ✅ 改 `commands.rs` / `runner.rs` 必须 2 轮 review

---

## 8. 风险与避坑

| 风险 | 避坑 |
|---|---|
| `update-config` Skill 报 settings.json schema 错(已知 hooks 嵌套挑剔) | 记录 blocker 到 handoff,手编最小 JSON 增量;不删/重排已有 hook |
| `dunce` 没装 → canonicalize 在 Windows 给 `\\?\` 前缀破坏可比性 | 手写规范化:strip `\\?\` 前缀 + Windows `to_lowercase()` + `Path::canonicalize` |
| `tauri-plugin-dialog` 权限未加 → 选目录按钮弹权限错误 | `src-tauri/capabilities/default.json` 加 `dialog:allow-open` |
| `importlib.resources` 在 dev 模式读 wheel 内文件 → 路径错 | 验证时先 `uv build` 出 wheel,再装 wheel 跑 longdoc 测试 |
| 主仓 `LocalSdxlProvider` 旧占位写 `placeholder.txt` 0 字节文件 | 同步删旧 placeholder 逻辑,改返回 `[]` + warn |
| task_text 含特殊字符 → CLI 透传转义错 | argparse `type=str` 即可;主仓内部不解析 CLI,直接进 state.json |
| Profile name 含中文 → `from_name("...")` 校验失败 | name 校验只 trim 空,字符不限(已实装) |
| 2 个 run 并发跑不同 profile → 串号 | §2.3 已堵:每个 run 独立 derive env_vars;新增测试 `run_pipeline_concurrent_profiles_dont_cross` |

---

## 9. 关键文件路径

- **本 spec**:`docs/superpowers/specs/2026-07-25-w15a-t7-2-product-feedback-design.md`
- **承接 handoff**:`handoff-w15-a-task12-build-verify-2026-07-25.md` §0.6 / §6
- **Plan(下一步写)**:`docs/superpowers/plans/2026-07-25-w15a-t7-2-product-feedback.md`
- **Tauri 后端**:`F:/soft/00selfmade/media-to-doc-ui/src-tauri/src/{commands.rs,runner.rs,llm_profiles.rs,types.rs}`
- **Tauri 前端**:`F:/soft/00selfmade/media-to-doc-ui/src/index.html`
- **主仓 CLI**:`F:/soft/00selfmade/media-to-doc/src/media_to_doc/{cli.py,config.py}`
- **主仓 pipeline**:`F:/soft/00selfmade/media-to-doc/src/media_to_doc/pipeline/{chapters.py,drafts.py,imagegen.py,longdoc.py}`
- **主仓 scripts**:`F:/soft/00selfmade/media-to-doc/scripts/sync_long_doc_skill.py`(新)+ `verify_long_doc_skill.py`(新)
- **Skill 真身**:`C:/Users/Duanyi/.claude/skills/long-doc-processor/`
- **Snapshot**:`F:/soft/00selfmade/media-to-doc/src/media_to_doc/data/long_doc_skill/`(新)
- **Claude settings.json**:`C:/Users/Duanyi/.claude/settings.json`(改 PostToolUse hooks)
- **Build 产物(待)**:`F:/soft/00selfmade/media-to-doc-ui/src-tauri/target/release/bundle/nsis/media-to-doc_1.4.2_x64-setup.exe`(T7.2 不必重建,等所有 P0/P1 过完一次重建)
- **sandbox-verify**:`F:/soft/00selfmade/sandbox-verify/media-to-doc-ui/mtd-verify.ps1`(T8 release 用,T7.2 不必跑)

---

## 10. 验收门槛

**T7.2 通过条件**:
- 7 项新验收(§7.2 #14-#20)全部 PASS
- P1 验收(#21-#23)自动跑过
- Rust 测试 ≥113 / 主仓测试 ≥613 全过
- `cargo tauri build` exit 0
- 2 轮 review 通过(改 `commands.rs` / `runner.rs` 必须)

**T7.2 不通过处理**:
- bug → 修 + 重 build + 重跑 §7.2 全部
- spec 设计缺陷 → 写 `handoff-w15-a-t7-2-blocked-2026-07-25.md` 等下版
- 不允许:留半成品进 T8,T8 release 仍 blocked

**T8 release(全部前置通过后)**:
- feature commit + bump v1.5.0 + 强清装机 + sandbox-verify + reviewer + 等用户拍板 merge/release