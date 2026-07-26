# W15-A T7.2 第二轮产品反馈收口 — 实施 Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 media-to-doc Tauri UI 的 New Run 改为「按 profile 选 LLM + Image Agent 策划 LLM 与出图 provider 两层独立 + 任务文本落盘喂 LLM + 选课程目录自动注册项目」,并把 long-doc Skill vendored 进主仓 / 装进 NSIS,Claude 编辑 Skill 后 PostToolUse hook 自动同步。

**Architecture:**
- 后端(Tauri):`run_pipeline` / `resume_pipeline` 加 `llm_profile_name` / `image_agent_profile_name` / `task_text` 三参,删 `inject_active_llm_env` 改 `inject_profile_env(spec, name_opt)`;`build_mtd_run_args` 同步加 3 flag;新增 5 个 `*_project` 命令 + `tauri-plugin-dialog` 选目录
- 主仓(Python):`mtd run` / `mtd resume` CLI 加 3 flag;`WorkflowConfig` 加 3 字段;`LLMConfig` 按 profile name 派生 provider/base_url/model + keyring key;`task_text` 落 state.json + 注入 chapter/draft prompt;`imagegen` 加策划 LLM(planner)生成 image plans;`LocalSdxlProvider` 改最小可跑(返 `[]`+ warn,不再写 0 字节 placeholder);`longdoc.py` 改读 vendored snapshot(`importlib.resources`),不读 `~/.claude`
- 前端(Tauri webview):`__mountNewRunTab__` mount 时拉 `list_llm_profiles` 填两个 `<select>`,新增 task `<textarea>` + 「选目录」按钮;新增 project registry `list_projects` / `add_project` / `remove_project` / `touch_project` IPC
- 打包:`pyproject.toml` `[tool.hatch.build.targets.wheel.force-include]` 含 snapshot;`tauri.conf.json` `bundle.resources` 含 snapshot(NSIS 自包含)
- 同步:`scripts/sync_long_doc_skill.py`(读源 → 复制白名单 → 写 MANIFEST sha256)+ `scripts/verify_long_doc_skill.py`(对 hash,漂移 exit 1);Claude `~/.claude/settings.json` `PostToolUse(Edit|Write)` hook 触发 sync(`update-config` Skill 优先,schema 报错手编 fallback)

**Tech Stack:**
- Tauri 2.x(`src-tauri/Cargo.toml` 实际版本)+ `tauri-plugin-dialog = "2"`(新增)
- Rust 既有依赖:serde / serde_json / reqwest / tokio / once_cell(全已用,不增)
- Python 主仓既有:argparse / pydantic / json / pathlib / importlib.resources
- Python 主仓新增依赖:**无**(keyring 走 stdlib + 简单 JSON;image planner 走既有 LLM provider 抽象)
- 前端:沿用 T6 单文件 `src/index.html` + 暗色主题 CSS variables + `state` / `$` / `toast` / `escapeHtml`

**承接**:`docs/superpowers/specs/2026-07-25-w15a-t7-2-product-feedback-design.md`(本 plan 完全覆盖 spec §2-9)

---

## Global Constraints

每条都来自 spec / 用户加快模式规则,任何 task 实现时必须遵守:

- **W15-A feature 整体一次 commit**(加快模式):本 plan 12 个 task 全部跑完后,**不要 commit**;由 T8 release 会话统一 feature commit + v1.5.0 release。每个 task 末尾"Save state"仅追加 task.md 进度行,**不 git commit**。
- **不 bump version 进 v1.5.0**:`src-tauri/tauri.conf.json` / `src-tauri/Cargo.toml` 的 version 不动(保持 1.4.2),T8 才 bump。
- **不 reset / checkout / restore / 覆盖未提交改动**(Tasks 1-11 工作区累积)。
- **不删除旧 handoff / prompt**(删除需用户二次确认)。
- **改 `commands.rs` / `runner.rs` 必须 2 轮 review**(本会话 + 上一会话 reviewer)——覆盖 T6 / T7。
- **不硬编码 API key / 凭据**:继续走 keyring(Windows Credential Manager via `keyring` crate)+ `spec.env_vars` 注入。
- **项目身份用规范化绝对路径**(`canonicalize` + Windows `to_lowercase` + NFC),**不是 display name**。
- **NSIS 自包含**:long-doc snapshot 必须打包进 wheel + NSIS,运行时**不允许依赖用户机器的 `C:/Users/Duanyi/.claude/`**。
- **不实现定时调度器**:parked,W15-B+ 再做。
- **TDD**:每个 task 必须先写失败测试再写实现;`cargo test --lib` ≥109 全过(98 既有 + ≥11 新);`uv run pytest` ≥618 全过(604 既有 + ≥14 新)。
- **不把普通文本 LLM 宣称为图片生成模型**:Image Agent 第一层(text LLM 策划 prompt)与第二层(image provider)分层独立,**且第一层不是出图模型**。
- **`update-config` Skill 优先**:改 `~/.claude/settings.json` 前先调 `Skill(skill="update-config")`;schema 报错记录 blocker 到 handoff,手编最小 JSON 增量(不删/重排已有 hook)。
- **PKG safe directory**:所有 `git -c safe.directory=*` 命令前缀不可省。
- **CRLF 兼容性**:`src/index.html` 在 Windows 下 Git 自动 CRLF 化,Edit 工具对缩进/换行极其敏感;每个 Edit 步骤前先 Read 一次目标行,Edit 精确匹配(tabs/spaces + line endings 1:1)。

---

## File Structure

| 文件 | 角色 | 修改量 | 是否新建 |
|---|---|---|---|
| `F:/soft/00selfmade/media-to-doc/src/media_to_doc/cli.py` | 主仓 CLI 入口 | 加 3 个 flag `--llm-profile-name` / `--image-agent-profile-name` / `--task-text`,传递到 WorkflowConfig | 否 |
| `F:/soft/00selfmade/media-to-doc/src/media_to_doc/config.py` | WorkflowConfig / LLMConfig dataclass | 加 3 字段;`LLMConfig.from_profile_name()` 派生 provider/base_url/model + 读 keyring key 写 env | 否 |
| `F:/soft/00selfmade/media-to-doc/src/media_to_doc/pipeline/state.py` | state.json IO | 加 `task_text` 字段 | 否 |
| `F:/soft/00selfmade/media-to-doc/src/media_to_doc/pipeline/chapters.py` | chapter stage prompt 构造 | prompt 前缀注入 task_text | 否 |
| `F:/soft/00selfmade/media-to-doc/src/media_to_doc/pipeline/drafts.py` | draft stage prompt 构造 | prompt 前缀注入 task_text | 否 |
| `F:/soft/00selfmade/media-to-doc/src/media_to_doc/pipeline/imagegen.py` | imagegen stage + LocalSdxlProvider | 新增策划 LLM `_plan_prompts`;`LocalSdxlProvider.generate` 改返 `[]`+ warn;Skip 路径写 plans.json | 否 |
| `F:/soft/00selfmade/media-to-doc/src/media_to_doc/pipeline/longdoc.py` | longdoc stage | 改读 vendored snapshot(`importlib.resources.files`),不读 `~/.claude` | 否 |
| `F:/soft/00selfmade/media-to-doc/src/media_to_doc/data/long_doc_skill/` | vendored Skill 真身(SKILL.md + references/ + scripts/ + MANIFEST.json) | 12 + 13 文件 + MANIFEST | **是** |
| `F:/soft/00selfmade/media-to-doc/scripts/sync_long_doc_skill.py` | Skill → snapshot 同步脚本 | ~80 行 | **是** |
| `F:/soft/00selfmade/media-to-doc/scripts/verify_long_doc_skill.py` | snapshot hash 校验脚本 | ~30 行 | **是** |
| `F:/soft/00selfmade/media-to-doc/pyproject.toml` | wheel 打包配置 | `[tool.hatch.build.targets.wheel.force-include]` 加 snapshot 路径 | 否 |
| `F:/soft/00selfmade/media-to-doc/tests/test_*.py` | 主仓 pytest | ≥14 新增测试 | 否(可能新建 longdoc integration 测试文件) |
| `F:/soft/00selfmade/media-to-doc-ui/src-tauri/Cargo.toml` | Rust 依赖 | 加 `tauri-plugin-dialog = "2"` | 否 |
| `F:/soft/00selfmade/media-to-doc-ui/src-tauri/capabilities/default.json` | Tauri 2 capability allowlist | 加 `"dialog:default"` / `"dialog:allow-open"` | 否 |
| `F:/soft/00selfmade/media-to-doc-ui/src-tauri/src/lib.rs` | Tauri app builder | 注册 dialog plugin | 否 |
| `F:/soft/00selfmade/media-to-doc-ui/src-tauri/src/runner.rs` | spawn / args builder | `build_mtd_run_args` 加 3 参数(llm_profile_name/image_agent_profile_name/task_text) | 否 |
| `F:/soft/00selfmade/media-to-doc-ui/src-tauri/src/commands.rs` | Tauri commands | `run_pipeline` / `resume_pipeline` 加 3 参数;删 `inject_active_llm_env`,新增 `inject_profile_env`;新增 5 个 project registry 命令 | 否 |
| `F:/soft/00selfmade/media-to-doc-ui/src-tauri/tauri.conf.json` | Tauri 配置 + 打包 | `bundle.resources` 加 snapshot 路径 | 否 |
| `F:/soft/00selfmade/media-to-doc-ui/src/index.html` | 前端单文件 | `__mountNewRunTab__` 大改(dynamic dropdowns + textarea + 选目录按钮);新增 `__projectTree__.refresh()` + `addProject`;Stop after 加中文 tooltip | 否 |
| `C:/Users/Duanyi/.claude/settings.json` | Claude Code hooks 配置 | `PostToolUse(Edit\|Write)` 加新 hook 触发 sync_long_doc_skill.py | 否(用 `update-config` Skill 或手编最小增量) |

**文件总数变化**:+5 新建(snapshot dir 含 26 文件 + 1 MANIFEST;sync_long_doc_skill.py;verify_long_doc_skill.py;可能 1 新测试文件)

---

## Task 1: 主仓 CLI 加 3 flag + WorkflowConfig 加 3 字段

**Files:**
- Modify: `F:/soft/00selfmade/media-to-doc/src/media_to_doc/cli.py`(读 `mtd run` / `mtd resume` 当前签名;argparse 加 3 flag)
- Modify: `F:/soft/00selfmade/media-to-doc/src/media_to_doc/config.py`(`WorkflowConfig` dataclass 加 3 字段)
- Modify: `F:/soft/00selfmade/media-to-doc/tests/test_cli.py`(若存在;否则新建 `tests/test_cli_profile_flags.py`)
- Read: `F:/soft/00selfmade/media-to-doc/src/media_to_doc/cli.py`(完整 argparse 段落)

**Interfaces:**
- Consumes:无(独立 task)
- Produces:
  ```python
  # cli.py
  parser.add_argument("--llm-profile-name", default=None, help="LLM profile name from registry (overrides --llm)")
  parser.add_argument("--image-agent-profile-name", default=None, help="Image Agent planner LLM profile name")
  parser.add_argument("--task-text", default=None, help="User task text (persisted to state.json, injected into chapter/draft prompt)")
  # config.py
  @dataclass
  class WorkflowConfig:
      ...existing...
      llm_profile_name: Optional[str] = None
      image_agent_profile_name: Optional[str] = None
      task_text: Optional[str] = None
  ```

- [ ] **Step 1:读 `cli.py` 当前 `mtd run` argparse 段落**

Run: `Read("F:/soft/00selfmade/media-to-doc/src/media_to_doc/cli.py", offset=<grep "add_argument('--llm" 行号 - 5>, limit=40)`
Expected: 看到 `argparse.ArgumentParser` 块 + `add_argument('--llm', ...)` / `--imagegen` / `--stop-after` 等

- [ ] **Step 2:读 `config.py` 当前 `WorkflowConfig` dataclass**

Run: `Read("F:/soft/00selfmade/media-to-doc/src/media_to_doc/config.py", offset=<grep "class WorkflowConfig" 行号 - 2>, limit=60)`
Expected: 看到 dataclass 字段定义

- [ ] **Step 3:写失败测试**(`tests/test_cli_profile_flags.py` 新建)

```python
"""W15-A T7.2:CLI profile + task_text flag 解析。"""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def test_mtd_run_accepts_llm_profile_name_flag():
    """`mtd run --llm-profile-name foo` 不报错(参数透传)。"""
    # 用 --help 验证 flag 注册;避免触发真 pipeline
    result = subprocess.run(
        [sys.executable, "-m", "media_to_doc.cli", "run", "--help"],
        cwd=str(ROOT), capture_output=True, text=True, timeout=10,
    )
    assert "--llm-profile-name" in result.stdout, (
        f"--llm-profile-name 未注册到 argparse: stdout={result.stdout!r}"
    )


def test_mtd_run_accepts_image_agent_profile_name_flag():
    result = subprocess.run(
        [sys.executable, "-m", "media_to_doc.cli", "run", "--help"],
        cwd=str(ROOT), capture_output=True, text=True, timeout=10,
    )
    assert "--image-agent-profile-name" in result.stdout


def test_mtd_run_accepts_task_text_flag():
    result = subprocess.run(
        [sys.executable, "-m", "media_to_doc.cli", "run", "--help"],
        cwd=str(ROOT), capture_output=True, text=True, timeout=10,
    )
    assert "--task-text" in result.stdout


def test_mtd_resume_accepts_three_flags():
    result = subprocess.run(
        [sys.executable, "-m", "media_to_doc.cli", "resume", "--help"],
        cwd=str(ROOT), capture_output=True, text=True, timeout=10,
    )
    for flag in ("--llm-profile-name", "--image-agent-profile-name", "--task-text"):
        assert flag in result.stdout, f"resume 缺 {flag}"
```

- [ ] **Step 4:跑测试验证失败**

Run: `cd F:/soft/00selfmade/media-to-doc && uv run pytest tests/test_cli_profile_flags.py -v`
Expected: 4 个 FAIL,assert 报 `未注册到 argparse`

- [ ] **Step 5:`config.py` `WorkflowConfig` 加 3 字段**

在 `WorkflowConfig` dataclass 内(找最近一个 `Optional` 字段附近),加:

```python
# W15-A T7.2:per-run profile + task_text
llm_profile_name: Optional[str] = None
image_agent_profile_name: Optional[str] = None
task_text: Optional[str] = None
```

- [ ] **Step 6:`cli.py` `mtd run` argparse 加 3 flag**

在 `--llm` 现有 `add_argument` 后插入:

```python
parser.add_argument("--llm-profile-name", default=None, help="LLM profile name (overrides --llm; reads metadata + keyring)")
parser.add_argument("--image-agent-profile-name", default=None, help="Image Agent planner LLM profile name (None=disable image agent)")
parser.add_argument("--task-text", default=None, help="User task text (persisted to state.json, injected into chapter/draft prompt)")
```

然后在 `args = parser.parse_args()` 之后,`WorkflowConfig(...)` 构造处加 3 个 keyword arg:

```python
llm_profile_name=args.llm_profile_name,
image_agent_profile_name=args.image_agent_profile_name,
task_text=args.task_text,
```

`mtd resume` 子命令重复同样 3 行 `add_argument` + 3 个 keyword arg(代码复制,不抽 helper——spec 不要求 DRY 此处)。

- [ ] **Step 7:跑测试验证通过**

Run: `cd F:/soft/00selfmade/media-to-doc && uv run pytest tests/test_cli_profile_flags.py -v`
Expected: 4 个 PASS

- [ ] **Step 8:跑主仓全测试,确认无回归**

Run: `cd F:/soft/00selfmade/media-to-doc && uv run pytest -q`
Expected: 604 passed(原数 + 0 跳过 + 4 新)

- [ ] **Step 9:Save state**

追加到 `F:/soft/00selfmade/media-to-doc-ui/task.md` §进度:
```
| T1 | 主仓 CLI 3 flag + WorkflowConfig 3 字段 | ✅ | 4/4 | (W15-A feature commit,未做) |
```
**不 commit**。

---

## Task 2: 主仓 LLMConfig 按 profile name 派生 + keyring 集成

**Files:**
- Modify: `F:/soft/00selfmade/media-to-doc/src/media_to_doc/config.py`(加 `LLMConfig.from_profile_name()` 静态方法 + `profile_name` 字段)
- Modify: `F:/soft/00selfmade/media-to-doc/src/media_to_doc/llm/__init__.py`(或 `llm/registry.py`;找现有 provider 注册位置)确保 `LLMConfig.from_profile_name()` 能解析 provider 名
- Modify: `F:/soft/00selfmade/media-to-doc/src/media_to_doc/pipeline/runner.py`(或 `cli.py`:`WorkflowConfig` → `LLMConfig` 转换处)按 `llm_profile_name` 优先派生
- Modify: `F:/soft/00selfmade/media-to-doc/tests/test_config.py`(新建若不存在)

**Interfaces:**
- Consumes:`WorkflowConfig.llm_profile_name`(T1 产出)
- Produces:
  ```python
  # config.py
  @dataclass
  class LLMConfig:
      provider: str
      base_url: str
      model: str
      api_key: Optional[str] = None  # 从 keyring 读
      profile_name: Optional[str] = None  # 新增,记录派生来源

      @staticmethod
      def from_profile_name(name: str, *, profiles_meta_path: Path | None = None) -> "LLMConfig":
          """从 metadata JSON + keyring 派生;Ollama 不需要 key,NoEntry 走空串。"""
          ...
  ```

- [ ] **Step 1:读 `LLMConfig` 当前定义 + metadata IO**

Run: `Grep("class LLMConfig", path="F:/soft/00selfmade/media-to-doc/src/media_to_doc")` + `Read` LLMConfig 完整段落 + 找 metadata JSON 路径(`workspace_dir / "llm_profiles.json"` 或类似)

- [ ] **Step 2:读 keyring 现有使用模式**

Run: `Grep("import keyring|keyring.get_password|from keyring", path="F:/soft/00selfmade/media-to-doc/src")`
Expected: 主仓是否已用 `keyring` 库?若无 → 本 task 加 `keyring = ">=24"` 到 `pyproject.toml` `dependencies`(spec 不禁止新依赖,只是要登记)

- [ ] **Step 3:写失败测试**

```python
# tests/test_config.py(append 或新建)
"""W15-A T7.2:LLMConfig.from_profile_name() 派生。"""
import json
from pathlib import Path
import pytest
from media_to_doc.config import LLMConfig


@pytest.fixture
def fake_profiles_json(tmp_path: Path) -> Path:
    p = tmp_path / "llm_profiles.json"
    p.write_text(json.dumps({
        "profiles": [
            {"name": "minimax-prod", "provider": "MiniMax",
             "base_url": "https://api.minimaxi.com/v1", "model": "MiniMax-M3",
             "note": None, "created_at": "2026-07-25T00:00:00Z"},
            {"name": "ollama-local", "provider": "Ollama",
             "base_url": "http://localhost:11434", "model": "llama3.1",
             "note": None, "created_at": "2026-07-25T00:00:00Z"},
        ],
        "active": None,
    }), encoding="utf-8")
    return p


def test_from_profile_name_loads_minimax_meta(monkeypatch, fake_profiles_json):
    """MiniMax profile 派生 provider/base_url/model,api_key 走 monkeypatch keyring。"""
    monkeypatch.setattr(
        "media_to_doc.config.keyring.get_password",
        lambda service, name: "sk-mm-test" if name == "minimax-prod" else None,
    )
    cfg = LLMConfig.from_profile_name("minimax-prod", profiles_meta_path=fake_profiles_json)
    assert cfg.provider == "MiniMax"
    assert cfg.base_url == "https://api.minimaxi.com/v1"
    assert cfg.model == "MiniMax-M3"
    assert cfg.api_key == "sk-mm-test"
    assert cfg.profile_name == "minimax-prod"


def test_from_profile_name_ollama_uses_empty_key_on_no_entry(monkeypatch, fake_profiles_json):
    """Ollama keyring NoEntry → api_key 走空串,不报错。"""
    def fake_get_password(service, name):
        import keyring as _k
        raise _k.errors.PasswordDeleteError("no entry")
    monkeypatch.setattr("media_to_doc.config.keyring.get_password", fake_get_password)
    cfg = LLMConfig.from_profile_name("ollama-local", profiles_meta_path=fake_profiles_json)
    assert cfg.provider == "Ollama"
    assert cfg.api_key == ""


def test_from_profile_name_unknown_name_raises(monkeypatch, fake_profiles_json):
    from media_to_doc.config import ProfileNotFoundError
    with pytest.raises(ProfileNotFoundError) as ei:
        LLMConfig.from_profile_name("nope", profiles_meta_path=fake_profiles_json)
    assert "nope" in str(ei.value)
```

- [ ] **Step 4:跑测试验证失败**

Run: `cd F:/soft/00selfmade/media-to-doc && uv run pytest tests/test_config.py::test_from_profile_name_loads_minimax_meta -v`
Expected: FAIL `from_profile_name` not defined

- [ ] **Step 5:实现 `LLMConfig.from_profile_name`**

`config.py` 顶部加 import:

```python
import json
from pathlib import Path
from typing import Optional
import keyring
import keyring.errors

class ProfileNotFoundError(KeyError):
    """从 metadata 找不到指定 name 时抛。"""

class ProfileKeyringError(RuntimeError):
    """keyring 读取失败且 provider 需要 key 时抛。"""
```

`LLMConfig` dataclass 加 `profile_name: Optional[str] = None` 字段(默认值 `None` 保 backwards compat)。

在 `LLMConfig` 类体内加:

```python
@staticmethod
def from_profile_name(name: str, *, profiles_meta_path: Path | None = None) -> "LLMConfig":
    """从 metadata JSON 派生 LLMConfig;keyring 读 api_key;Ollama 不需要 key 走空串。

    Args:
        name: profile name(精确匹配 metadata.profiles[].name)。
        profiles_meta_path: metadata JSON 路径;None 时走默认 `<workspace>/llm_profiles.json`
                           (本 task 用 None 默认仅用于测试,生产由 cli.py 传实际路径)。

    Raises:
        ProfileNotFoundError: name 不在 metadata 中。
        ProfileKeyringError: keyring 读取失败且 provider != Ollama。
    """
    if profiles_meta_path is None or not Path(profiles_meta_path).is_file():
        raise ProfileNotFoundError(f"profile metadata 不存在: {profiles_meta_path}")
    data = json.loads(Path(profiles_meta_path).read_text(encoding="utf-8"))
    meta = next((p for p in data.get("profiles", []) if p["name"] == name), None)
    if meta is None:
        raise ProfileNotFoundError(f"profile 不存在: {name}")
    provider = meta["provider"]
    # keyring 读 key
    api_key = ""
    try:
        k = keyring.get_password("media_to_doc_llm", name)
        api_key = k if k else ""
    except keyring.errors.PasswordDeleteError:
        api_key = ""
    except Exception as e:
        if provider != "Ollama":
            raise ProfileKeyringError(f"keyring 读 {name} 失败: {e}") from e
        api_key = ""
    return LLMConfig(
        provider=provider,
        base_url=meta["base_url"],
        model=meta["model"],
        api_key=api_key,
        profile_name=name,
    )
```

- [ ] **Step 6:`pyproject.toml` 加 `keyring` 依赖**

在 `[project] dependencies` 列表加 `"keyring>=24"`。读当前 dependencies 段后 Edit 插入。

- [ ] **Step 7:`WorkflowConfig → LLMConfig` 转换处,优先用 profile_name**

读 `cli.py` 或 `pipeline/runner.py` 里构造 `LLMConfig` 的位置(`Grep("LLMConfig(", path=...)`)。如果当前是:

```python
llm_cfg = LLMConfig(provider=args.llm, base_url=os.environ["LLM_BASE_URL"], ...)
```

改为(伪代码,具体看现有结构):

```python
if cfg.llm_profile_name:
    llm_cfg = LLMConfig.from_profile_name(
        cfg.llm_profile_name,
        profiles_meta_path=workspace / "llm_profiles.json",
    )
else:
    llm_cfg = LLMConfig.from_env_or_args(args.llm, ...)  # 现有路径
```

**保留原 fallback 路径**(LLM env / `--llm` 显式值),不删。

- [ ] **Step 8:跑测试验证通过**

Run: `cd F:/soft/00selfmade/media-to-doc && uv run pytest tests/test_config.py -v`
Expected: 3 个 PASS

- [ ] **Step 9:跑主仓全测试,确认无回归**

Run: `cd F:/soft/00selfmade/media-to-doc && uv run pytest -q`
Expected: 608 passed(604 + 4 T1 + 3 T2 - 3 fixture 重叠 → 实测 ~611;以实际为准)

- [ ] **Step 10:Save state**

追加到 `task.md` §进度:
```
| T2 | LLMConfig.from_profile_name + keyring 集成 | ✅ | 3/3 | 同上 |
```
**不 commit**。

---

## Task 3: 主仓 task_text 落 state.json + 注入 chapter/draft prompt

**Files:**
- Modify: `F:/soft/00selfmade/media-to-doc/src/media_to_doc/pipeline/state.py`(`WorkflowState` dataclass 加 `task_text` 字段;`save()` / `load()` 同步)
- Modify: `F:/soft/00selfmade/media-to-doc/src/media_to_doc/pipeline/chapters.py`(chapter prompt 构造时读 `cfg.task_text`,若有则在 prompt 前缀注入)
- Modify: `F:/soft/00selfmade/media-to-doc/src/media_to_doc/pipeline/drafts.py`(draft prompt 同)
- Modify: `F:/soft/00selfmade/media-to-doc/tests/test_state.py`(新建若不存在)+ `tests/test_chapters.py` / `tests/test_drafts.py`(若已存在则 append)

**Interfaces:**
- Consumes:`WorkflowConfig.task_text`(T1 产出)
- Produces:
  ```python
  # state.py
  @dataclass
  class WorkflowState:
      ...existing...
      task_text: Optional[str] = None
  # chapters.py / drafts.py 内部
  def build_chapter_prompt(cfg: WorkflowConfig, transcript: str) -> str:
      prefix = f"USER_INSTRUCTION: {cfg.task_text}\n\n" if cfg.task_text else ""
      return prefix + <原有 prompt 模板>
  ```

- [ ] **Step 1:读 `WorkflowState` dataclass + state.json 序列化**

Run: `Grep("class WorkflowState", path="F:/soft/00selfmade/media-to-doc/src/media_to_doc/pipeline")` + Read 完整段落

- [ ] **Step 2:读 `chapters.py` 当前 prompt 构造函数**

Run: `Grep("def.*chapter.*prompt|def.*build.*prompt", path="F:/soft/00selfmade/media-to-doc/src/media_to_doc/pipeline/chapters.py")` + Read

- [ ] **Step 3:写失败测试(state.json 落 task_text)**

```python
# tests/test_state_task_text.py(新建)
"""W15-A T7.2:task_text 落 state.json + 序列化往返。"""
import json
from pathlib import Path
from media_to_doc.pipeline.state import WorkflowState


def test_state_includes_task_text_field():
    s = WorkflowState(course="x", inbox_path="/tmp/x", task_text="突出客户案例")
    d = s.to_dict()  # 或 asdict(s)
    assert d["task_text"] == "突出客户案例"


def test_state_round_trip_preserves_task_text(tmp_path: Path):
    p = tmp_path / "state.json"
    s = WorkflowState(course="x", inbox_path="/tmp/x", task_text="重点在第 2 节")
    s.save(p)
    s2 = WorkflowState.load(p)
    assert s2.task_text == "重点在第 2 节"


def test_state_task_text_default_none():
    s = WorkflowState(course="x", inbox_path="/tmp/x")
    assert s.task_text is None
    d = s.to_dict()
    assert d["task_text"] is None
```

- [ ] **Step 4:跑测试验证失败**

Run: `cd F:/soft/00selfmade/media-to-doc && uv run pytest tests/test_state_task_text.py -v`
Expected: FAIL `task_text` not a field

- [ ] **Step 5:实现 `WorkflowState.task_text`**

在 `WorkflowState` dataclass 加字段:

```python
task_text: Optional[str] = None
```

若 `to_dict()` / `save()` / `load()` 是手写(非 `dataclasses.asdict`),同步加 `task_text` 序列化。若用 `asdict`,**只需加字段,自动生效**。

- [ ] **Step 6:写失败测试(chapter prompt 注入 task_text)**

```python
# tests/test_chapters_prompt.py(新建)
"""W15-A T7.2:chapter prompt 注入 task_text。"""
from media_to_doc.pipeline.chapters import build_chapter_prompt  # 或实际函数名
from media_to_doc.config import WorkflowConfig


def test_chapter_prompt_with_task_text_injects_prefix():
    cfg = WorkflowConfig(inbox=..., work=..., task_text="突出客户案例")  # 用实际最小构造
    prompt = build_chapter_prompt(cfg, transcript="<asr jsonl>")
    assert prompt.startswith("USER_INSTRUCTION: 突出客户案例")


def test_chapter_prompt_without_task_text_no_prefix():
    cfg = WorkflowConfig(inbox=..., work=..., task_text=None)
    prompt = build_chapter_prompt(cfg, transcript="<asr jsonl>")
    assert not prompt.startswith("USER_INSTRUCTION:")
```

具体函数名 / `WorkflowConfig` 构造参数以 `chapters.py` 现状为准;读 Step 2 的结果调整 import + 字段。

- [ ] **Step 7:`chapters.py` prompt 构造函数加前缀**

找到 `build_chapter_prompt(cfg, transcript)`(或类似名)。在 prompt 字符串最前面:

```python
prefix = f"USER_INSTRUCTION: {cfg.task_text}\n\n" if cfg.task_text else ""
prompt = prefix + <原 prompt 模板>
```

若 prompt 模板已是 `f"""..."""`,改成 `f"{prefix}..."` 形式。

- [ ] **Step 8:同手法改 `drafts.py`**

测试 + 实现同 §7,prompt 前缀加 `cfg.task_text`。

- [ ] **Step 9:跑测试验证通过**

Run: `cd F:/soft/00selfmade/media-to-doc && uv run pytest tests/test_state_task_text.py tests/test_chapters_prompt.py -v`
Expected: 5 个 PASS(state 3 + chapter 2)

- [ ] **Step 10:`cli.py` / `pipeline/runner.py` 在 stage 启动时把 `cfg.task_text` 写到 state**

读 `runner.py` 找 `state = WorkflowState.load(work / "state.json")` 或 `state.save(...)` 位置。找到 `state.task_text = cfg.task_text`(若 WorkflowState 实例化);或 `state = WorkflowState(..., task_text=cfg.task_text)`(若 dataclass 实例化)。**不要新建 state,只在已构造 state 上设字段**。

- [ ] **Step 11:跑主仓全测试**

Run: `cd F:/soft/00selfmade/media-to-doc && uv run pytest -q`
Expected: 既有 + ≥5 新增,无回归

- [ ] **Step 12:Save state**

追加到 `task.md` §进度:
```
| T3 | task_text 落 state.json + chapter/draft prompt 注入 | ✅ | ≥5/≥5 | 同上 |
```
**不 commit**。

---

## Task 4: 主仓 imagegen 加策划 LLM + LocalSdxlProvider 最小实现

**Files:**
- Modify: `F:/soft/00selfmade/media-to-doc/src/media_to_doc/pipeline/imagegen.py`(加 `_plan_prompts()` + `LocalSdxlProvider.generate()` 最小实现 + `imagegen_stage()` 调用 planner)
- Modify: `F:/soft/00selfmade/media-to-doc/tests/test_imagegen.py`(新建或 append)
- Read: `imagegen.py` 完整内容(找现有 `SkipProvider` / `LocalSdxlProvider` 实现)

**Interfaces:**
- Consumes:`WorkflowConfig.image_agent_profile_name` + `imagegen` provider(T1 产出)
- Produces:
  ```python
  # imagegen.py
  @dataclass
  class ImagePlan:
      chapter_id: str
      slot: int
      prompt: str
      negative_prompt: str
      style_hint: str

  def _plan_prompts(
      cfg: WorkflowConfig,
      transcript: list[dict],
      ocr_results: list[dict],
  ) -> list[ImagePlan]:
      """策划 LLM 生成每张图 prompt;profile 不存在时返 [],log warning。"""

  class LocalSdxlProvider:
      def generate(self, plans: list[ImagePlan], work_dir: Path) -> list[Path]:
          # 不调 SDXL,返 [] + logger.warning("LocalSdxlProvider not configured")
  ```

- [ ] **Step 1:读 `imagegen.py` 完整内容**

Run: `Read("F:/soft/00selfmade/media-to-doc/src/media_to_doc/pipeline/imagegen.py")`
Expected: 看到 `SkipProvider` / `LocalSdxlProvider` 类 + `imagegen_stage()` 函数

- [ ] **Step 2:写失败测试(LocalSdxlProvider 最小实现)**

```python
# tests/test_imagegen_provider.py(新建)
"""W15-A T7.2:LocalSdxlProvider 不再写 0 字节 placeholder;Skip 写 plans.json。"""
import json
from pathlib import Path
from media_to_doc.pipeline.imagegen import LocalSdxlProvider, SkipProvider, ImagePlan


def test_local_sdxl_provider_returns_empty_with_warning(tmp_path: Path, caplog):
    provider = LocalSdxlProvider()
    plans = [ImagePlan(chapter_id="ch1", slot=0, prompt="a cat", negative_prompt="", style_hint="")]
    with caplog.at_level("WARNING"):
        result = provider.generate(plans, tmp_path)
    assert result == []
    assert "LocalSdxlProvider not configured" in caplog.text


def test_local_sdxl_no_longer_writes_empty_placeholder_file(tmp_path: Path):
    provider = LocalSdxlProvider()
    provider.generate([], tmp_path)
    # 不再有 placeholder.txt 0 字节文件
    assert not (tmp_path / "placeholder.txt").exists()


def test_skip_provider_writes_plans_json(tmp_path: Path):
    plans = [ImagePlan(chapter_id="ch1", slot=0, prompt="a cat", negative_prompt="", style_hint="")]
    provider = SkipProvider()
    provider.generate(plans, tmp_path)
    p = tmp_path / "image_plans.json"
    assert p.exists()
    data = json.loads(p.read_text(encoding="utf-8"))
    assert len(data["plans"]) == 1
    assert data["plans"][0]["prompt"] == "a cat"
```

- [ ] **Step 3:跑测试验证失败**

Run: `cd F:/soft/00selfmade/media-to-doc && uv run pytest tests/test_imagegen_provider.py -v`
Expected: FAIL `placeholder.txt` 存在 / `LocalSdxlProvider` 行为错

- [ ] **Step 4:`LocalSdxlProvider.generate()` 改最小实现**

```python
class LocalSdxlProvider:
    def generate(self, plans: list[ImagePlan], work_dir: Path) -> list[Path]:
        logger.warning(
            "LocalSdxlProvider 未配置 SDXL 模型,本次不出图;%d 个 plan 已写 metadata",
            len(plans),
        )
        return []
```

**删** 任何 `fs.write(work_dir / "placeholder.txt", b"")` 或类似 0 字节占位逻辑。

- [ ] **Step 5:`SkipProvider.generate()` 写 plans.json**

```python
class SkipProvider:
    def generate(self, plans: list[ImagePlan], work_dir: Path) -> list[Path]:
        out = work_dir / "image_plans.json"
        out.write_text(
            json.dumps(
                {"plans": [asdict(p) for p in plans]},
                ensure_ascii=False,
                indent=2,
            ),
            encoding="utf-8",
        )
        return []
```

顶部 `from dataclasses import asdict` import 视情况加。

- [ ] **Step 6:`ImagePlan` dataclass 定义**

```python
from dataclasses import dataclass, asdict

@dataclass
class ImagePlan:
    chapter_id: str
    slot: int
    prompt: str
    negative_prompt: str = ""
    style_hint: str = ""
```

- [ ] **Step 7:写失败测试(planner LLM)**

```python
# tests/test_imagegen_planner.py(新建)
"""W15-A T7.2:imagegen planner LLM 集成。"""
from unittest.mock import MagicMock, patch
from media_to_doc.pipeline.imagegen import _plan_prompts
from media_to_doc.config import WorkflowConfig


def test_plan_prompts_without_image_agent_profile_returns_empty(caplog):
    cfg = WorkflowConfig(inbox="/tmp", work="/tmp", image_agent_profile_name=None)
    with caplog.at_level("WARNING"):
        plans = _plan_prompts(cfg, transcript=[], ocr_results=[])
    assert plans == []
    assert "策划 LLM 不可用" in caplog.text


def test_plan_prompts_with_profile_uses_image_agent_not_main_llm():
    """策划 LLM 用 image_agent_profile_name 派生,不读主 LLM。"""
    cfg = WorkflowConfig(
        inbox="/tmp", work="/tmp",
        llm_profile_name="main-llm",
        image_agent_profile_name="image-agent",
    )
    fake_provider = MagicMock()
    fake_provider.chat.return_value = '{"plans": []}'
    with patch("media_to_doc.pipeline.imagegen._create_planner_provider", return_value=fake_provider):
        plans = _plan_prompts(cfg, transcript=[{"text": "hi"}], ocr_results=[])
    # 验证 create_planner_provider 被传 "image-agent" 不是 "main-llm"
    from media_to_doc.pipeline.imagegen import _create_planner_provider
    _create_planner_provider.assert_called_with("image-agent")  # type: ignore
    assert fake_provider.chat.called
```

- [ ] **Step 8:跑测试验证失败**

Run: `cd F:/soft/00selfmade/media-to-doc && uv run pytest tests/test_imagegen_planner.py -v`
Expected: FAIL `_plan_prompts` / `_create_planner_provider` not defined

- [ ] **Step 9:`_plan_prompts` 实现 + `_create_planner_provider` helper**

`imagegen.py` 加:

```python
def _create_planner_provider(profile_name: str) -> "BaseLLMProvider":
    """从 profile_name 派生策划 LLM provider(与主 LLM 路径同形,但 model 可不同)。

    实现:复用 `media_to_doc.llm.get_provider(LLMConfig.from_profile_name(profile_name))`。
    """
    from media_to_doc.config import LLMConfig
    from media_to_doc.llm import get_provider
    cfg = LLMConfig.from_profile_name(profile_name)
    return get_provider(cfg)


def _plan_prompts(
    cfg: WorkflowConfig,
    transcript: list[dict],
    ocr_results: list[dict],
) -> list[ImagePlan]:
    if not cfg.image_agent_profile_name:
        logger.warning("策划 LLM 不可用(image_agent_profile_name=None),跳过配图策划")
        return []
    try:
        provider = _create_planner_provider(cfg.image_agent_profile_name)
    except Exception as e:
        logger.warning("策划 LLM 创建失败(%s): %s;跳过配图策划", cfg.image_agent_profile_name, e)
        return []
    # 构造 prompt:简化版,实际可调优
    transcript_text = "\n".join(t.get("text", "") for t in transcript)
    prompt = (
        f"USER_INSTRUCTION: 根据以下视频转写与画面文字,生成最多 3 张配图 plan。\n"
        f"转写: {transcript_text[:2000]}\n"
        f"OCR: {json.dumps(ocr_results, ensure_ascii=False)[:1000]}\n"
        f"输出 JSON: {{\"plans\": [{{\"chapter_id\": \"...\", \"slot\": 0, \"prompt\": \"...\", "
        f"\"negative_prompt\": \"...\", \"style_hint\": \"...\"}}]}}\n"
    )
    raw = provider.chat(prompt)
    try:
        data = json.loads(raw)
        return [ImagePlan(**p) for p in data.get("plans", [])]
    except (json.JSONDecodeError, TypeError) as e:
        logger.warning("策划 LLM 输出解析失败: %s", e)
        return []
```

- [ ] **Step 10:`imagegen_stage()` 集成 planner**

找 `imagegen_stage(cfg, work_dir, ...)`,在调 provider 前:

```python
# 1. 策划
plans = _plan_prompts(cfg, transcript, ocr_results)
# 2. 落 plans.json 提前(供 provider 跳过时也能拿到)
(work_dir / "image_plans.json").write_text(
    json.dumps({"plans": [asdict(p) for p in plans]}, ensure_ascii=False, indent=2),
    encoding="utf-8",
)
# 3. 选 provider 出图
if cfg.imagegen == "skip":
    provider = SkipProvider()
elif cfg.imagegen == "local_sdxl":
    provider = LocalSdxlProvider()
else:
    raise ValueError(f"未知 imagegen provider: {cfg.imagegen}")
image_paths = provider.generate(plans, work_dir)
# 4. state.json 更新 image_plans
state.image_plans = [asdict(p) for p in plans]
state.save()
```

- [ ] **Step 11:跑测试验证通过**

Run: `cd F:/soft/00selfmade/media-to-doc && uv run pytest tests/test_imagegen_provider.py tests/test_imagegen_planner.py -v`
Expected: 5 个 PASS

- [ ] **Step 12:跑主仓全测试**

Run: `cd F:/soft/00selfmade/media-to-doc && uv run pytest -q`
Expected: 既有 + ≥5 新增,无回归

- [ ] **Step 13:Save state**

追加到 `task.md` §进度:
```
| T4 | imagegen 策划 LLM + LocalSdxlProvider 最小实现 | ✅ | ≥5/≥5 | 同上 |
```
**不 commit**。

---

## Task 5: Tauri `runner.rs` `build_mtd_run_args` 加 3 参数 + 测试

**Files:**
- Modify: `F:/soft/00selfmade/media-to-doc-ui/src-tauri/src/runner.rs`(`build_mtd_run_args` 加 3 参数 + 输出 args 加 3 flag;`build_mtd_resume_args` 同步加 3 参数)
- Modify: `F:/soft/00selfmade/media-to-doc-ui/src-tauri/src/runner.rs`(同文件,append 测试模块)

**Interfaces:**
- Consumes:无(Tauri 内部)
- Produces:
  ```rust
  // runner.rs
  pub fn build_mtd_run_args(
      project_root: &Path,
      inbox: &Path,
      llm: Option<&str>,
      imagegen: Option<&str>,
      stop_after: Option<&str>,
      no_longdoc: bool,
      force: bool,
      llm_profile_name: Option<&str>,        // 新增
      image_agent_profile_name: Option<&str>,// 新增
      task_text: Option<&str>,               // 新增
  ) -> SpawnSpec
  pub fn build_mtd_resume_args(
      project_root: &Path,
      work_dir: &Path,
      force: bool,
      stop_after: Option<&str>,
      llm_profile_name: Option<&str>,
      image_agent_profile_name: Option<&str>,
      task_text: Option<&str>,
  ) -> SpawnSpec
  ```

- [ ] **Step 1:读 `runner.rs` `build_mtd_run_args` 当前签名**

Run: `Read("F:/soft/00selfmade/media-to-doc-ui/src-tauri/src/runner.rs", offset=50, limit=130)`
Expected: 看到现有 `build_mtd_run_args` + `build_mtd_resume_args`(行 53-130)

- [ ] **Step 2:写失败测试(append 到 `runner.rs` 末尾 `#[cfg(test)] mod tests`)**

```rust
// runner.rs 末尾 mod tests 段(若不存在则新建;存在则 append)
#[cfg(test)]
mod t7_2_args_tests {
    use super::*;

    #[test]
    fn build_run_args_with_profile_name_adds_flag() {
        let spec = build_mtd_run_args(
            Path::new("/proj"),
            Path::new("/proj/inbox/x"),
            None, None, None, false, false,
            Some("minimax-prod"), None, None,
        );
        assert!(spec.args.contains(&"--llm-profile-name".to_string()));
        let idx = spec.args.iter().position(|a| a == "--llm-profile-name").unwrap();
        assert_eq!(spec.args[idx + 1], "minimax-prod");
    }

    #[test]
    fn build_run_args_with_image_agent_profile_name_adds_flag() {
        let spec = build_mtd_run_args(
            Path::new("/proj"),
            Path::new("/proj/inbox/x"),
            None, None, None, false, false,
            None, Some("deepseek-prod"), None,
        );
        assert!(spec.args.contains(&"--image-agent-profile-name".to_string()));
    }

    #[test]
    fn build_run_args_with_task_text_adds_flag() {
        let spec = build_mtd_run_args(
            Path::new("/proj"),
            Path::new("/proj/inbox/x"),
            None, None, None, false, false,
            None, None, Some("突出客户案例"),
        );
        assert!(spec.args.contains(&"--task-text".to_string()));
        let idx = spec.args.iter().position(|a| a == "--task-text").unwrap();
        assert_eq!(spec.args[idx + 1], "突出客户案例");
    }

    #[test]
    fn build_run_args_without_new_params_no_flag() {
        let spec = build_mtd_run_args(
            Path::new("/proj"),
            Path::new("/proj/inbox/x"),
            Some("ollama"), Some("skip"), None, false, false,
            None, None, None,
        );
        assert!(!spec.args.contains(&"--llm-profile-name".to_string()));
        assert!(!spec.args.contains(&"--image-agent-profile-name".to_string()));
        assert!(!spec.args.contains(&"--task-text".to_string()));
    }

    #[test]
    fn build_resume_args_with_all_three_new_flags() {
        let spec = build_mtd_resume_args(
            Path::new("/proj"),
            Path::new("/proj/work"),
            false, None,
            Some("minimax-prod"), Some("deepseek-prod"), Some("task"),
        );
        assert!(spec.args.contains(&"--llm-profile-name".to_string()));
        assert!(spec.args.contains(&"--image-agent-profile-name".to_string()));
        assert!(spec.args.contains(&"--task-text".to_string()));
    }
}
```

- [ ] **Step 3:跑测试验证失败**

Run: `cd F:/soft/00selfmade/media-to-doc-ui/src-tauri && cargo test --lib t7_2_args_tests -v`
Expected: 5 个 FAIL(`function takes X args but Y provided` / `this function takes ...`)

- [ ] **Step 4:改 `build_mtd_run_args` 签名 + 加 3 flag**

在 `runner.rs` 现有 `build_mtd_run_args` 函数,改:

```rust
pub fn build_mtd_run_args(
  project_root: &Path,
  inbox: &Path,
  llm: Option<&str>,
  imagegen: Option<&str>,
  stop_after: Option<&str>,
  no_longdoc: bool,
  force: bool,
  // W15-A T7.2:per-run profile + task_text
  llm_profile_name: Option<&str>,
  image_agent_profile_name: Option<&str>,
  task_text: Option<&str>,
) -> SpawnSpec {
  // ... 现有 args 构造 ...
  if let Some(n) = llm_profile_name {
    args.extend(["--llm-profile-name".to_string(), n.to_string()]);
  }
  if let Some(n) = image_agent_profile_name {
    args.extend(["--image-agent-profile-name".to_string(), n.to_string()]);
  }
  if let Some(t) = task_text {
    args.extend(["--task-text".to_string(), t.to_string()]);
  }
  // ... 现有 if llm / imagegen / stop_after / no_longdoc / force 不动 ...
  // SpawnSpec { ... } 构造不变
}
```

- [ ] **Step 5:改 `build_mtd_resume_args` 签名 + 加 3 flag**

```rust
pub fn build_mtd_resume_args(
  project_root: &Path,
  work_dir: &Path,
  force: bool,
  stop_after: Option<&str>,
  // W15-A T7.2
  llm_profile_name: Option<&str>,
  image_agent_profile_name: Option<&str>,
  task_text: Option<&str>,
) -> SpawnSpec {
  // 现有 args 构造 + 在 --stop-after 后追加 3 个 if Some
}
```

- [ ] **Step 6:跑测试验证通过**

Run: `cd F:/soft/00selfmade/media-to-doc-ui/src-tauri && cargo test --lib t7_2_args_tests -v`
Expected: 5 个 PASS

- [ ] **Step 7:跑全部 Rust 测试,确认既有 98 不回归**

Run: `cd F:/soft/00selfmade/media-to-doc-ui/src-tauri && cargo test --lib -q`
Expected: 103 passed(98 + 5 new)

- [ ] **Step 8:Save state**

追加到 `task.md` §进度:
```
| T5 | Tauri runner.rs build_mtd_*_args 加 3 参数 | ✅ | 5/5 | 同上 |
```
**不 commit**。

---

## Task 6: Tauri `commands.rs` `run_pipeline` / `resume_pipeline` per-run profile 注入

**Files:**
- Modify: `F:/soft/00selfmade/media-to-doc-ui/src-tauri/src/commands.rs`(`run_pipeline` / `resume_pipeline` 签名加 3 参数 + 改 `inject_active_llm_env` → `inject_profile_env` + 5 个新 project commands;tests append)
- Modify: `F:/soft/00selfmade/media-to-doc-ui/src-tauri/src/runner.rs`(`inject_active_llm_env` 调用点改用 `inject_profile_env` —— 或在 commands.rs 内部定义 helper)

**Interfaces:**
- Consumes:`build_mtd_run_args` 3 新参数(T5 产出)
- Produces:
  ```rust
  // commands.rs
  #[tauri::command]
  pub async fn run_pipeline(
      inbox_dir: String,
      workspace_root: Option<String>,
      llm_profile_name: Option<String>,        // 替 llm: Option<String> 中的 `llm` 由 caller 派生;或保留 llm + 新增 llm_profile_name(spec §2.2:llm_profile_name 覆盖 --llm)
      image_agent_profile_name: Option<String>,
      task_text: Option<String>,
      imagegen: Option<String>,
      stop_after: Option<String>,
      no_longdoc: Option<bool>,
      force: Option<bool>,
  ) -> CommandResponse<RunPipelineResult>;

  fn inject_profile_env(spec: &mut SpawnSpec, profile_name: Option<&str>) -> Result<(), String> {
      // None = 空 env_vars(走 CLI 默认)
      // Some(name) = 查 profile,失败 PROFILE_NOT_FOUND
      // Ollama + NoEntry = 空 key
  }
  ```

- [ ] **Step 1:读 `commands.rs` 当前 `run_pipeline` 签名 + `inject_active_llm_env` 实现**

Run: `Read("F:/soft/00selfmade/media-to-doc-ui/src-tauri/src/commands.rs", offset=998, limit=170)`
Expected: 看到 `run_pipeline` / `resume_pipeline` / `inject_active_llm_env`

- [ ] **Step 2:写失败测试(append 到 `commands.rs` 末尾 `#[cfg(test)] mod runner_tests`)**

```rust
// commands.rs 末尾 mod runner_tests 段 append
#[test]
fn t7_2_inject_profile_env_errors_on_profile_not_found() {
    let mut spec = crate::runner::SpawnSpec {
        program: "uv".into(),
        args: vec![],
        work_dir: "/tmp".into(),
        log_path: "/tmp/mtd.log".into(),
        env_vars: Default::default(),
    };
    let r = inject_profile_env(&mut spec, Some("nope-this-profile"));
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("PROFILE_NOT_FOUND"));
}

#[test]
fn t7_2_inject_profile_env_none_leaves_env_empty() {
    let mut spec = crate::runner::SpawnSpec {
        program: "uv".into(),
        args: vec![],
        work_dir: "/tmp".into(),
        log_path: "/tmp/mtd.log".into(),
        env_vars: Default::default(),
    };
    inject_profile_env(&mut spec, None).unwrap();
    assert!(spec.env_vars.is_empty());
}

#[test]
fn t7_2_inject_profile_env_ollama_with_no_keyring_entry_does_not_error() {
    // Ollama 不需要 key,NoEntry 视为空 key 注入
    let mut spec = crate::runner::SpawnSpec {
        program: "uv".into(),
        args: vec![],
        work_dir: "/tmp".into(),
        log_path: "/tmp/mtd.log".into(),
        env_vars: Default::default(),
    };
    // 用 monkeypatched env + 临时 profiles metadata 跑全链路过于复杂;
    // 直接验证 inject_profile_env 内部走 `provider == "Ollama" && keyring_err_is_no_entry → empty key`
    // 的分支。用真实 list_llm_profiles + ollama profile(若全局 metadata 没有 ollama → 用 monkeypatch)。
    // 简化:用 mock `keyring_store::read_key` 返 NoEntry,`llm_profiles::load_profiles` 返 ollama profile。
    // 本 task 简化处理:用 list_llm_profiles_impl 真实返回(若有 ollama profile)。
    let profiles = list_llm_profiles_impl();
    if let Ok(mut ps) = profiles.data {
        if let Some(ollama) = ps.iter().find(|p| p.provider == "Ollama").cloned() {
            let r = inject_profile_env(&mut spec, Some(&ollama.name));
            assert!(r.is_ok(), "Ollama 不应要求 keyring: error={:?}", r);
            return;
        }
    }
    // skip 若全局 metadata 无 ollama profile(不影响覆盖率)
}

#[test]
fn t7_2_build_run_args_with_profile_name_replaces_active() {
    // 即使全局 active profile 是 deepseek,显式传 minimax 也走 minimax
    let spec = build_mtd_run_args(
        std::path::Path::new("/proj"),
        std::path::Path::new("/proj/inbox/x"),
        None, None, None, false, false,
        Some("minimax-prod"), None, None,
    );
    assert!(spec.args.contains(&"--llm-profile-name".to_string()));
    let idx = spec.args.iter().position(|a| a == "--llm-profile-name").unwrap();
    assert_eq!(spec.args[idx + 1], "minimax-prod");
}

#[test]
fn t7_2_build_run_args_with_all_three_preserves_existing_flags() {
    let spec = build_mtd_run_args(
        std::path::Path::new("/proj"),
        std::path::Path::new("/proj/inbox/x"),
        Some("ollama"), Some("skip"), Some("chapters"), true, true,
        Some("minimax-prod"), Some("deepseek-prod"), Some("task text"),
    );
    assert!(spec.args.contains(&"--llm".to_string()));
    assert!(spec.args.contains(&"--imagegen".to_string()));
    assert!(spec.args.contains(&"--stop-after".to_string()));
    assert!(spec.args.contains(&"--no-longdoc".to_string()));
    assert!(spec.args.contains(&"--force".to_string()));
    assert!(spec.args.contains(&"--llm-profile-name".to_string()));
    assert!(spec.args.contains(&"--image-agent-profile-name".to_string()));
    assert!(spec.args.contains(&"--task-text".to_string()));
}
```

(注意:前 3 个测试需要 `inject_profile_env` 是 `pub fn` 或 `pub(crate) fn`,Step 4 实现时确认可见性。)

- [ ] **Step 3:跑测试验证失败**

Run: `cd F:/soft/00selfmade/media-to-doc-ui/src-tauri && cargo test --lib t7_2_ -v`
Expected: 5 个 FAIL

- [ ] **Step 4:`inject_active_llm_env` → `inject_profile_env` 改写**

`commands.rs` 删 `inject_active_llm_env`(行 1122-1134),改:

```rust
/// W15-A T7.2:按 profile_name 查 profile + keyring → 写 spec.env_vars。
///
/// `profile_name=None` → 空 env_vars(走 CLI 默认)。
/// `profile_name=Some(name)`:
///   - profile 不存在 → `PROFILE_NOT_FOUND`
///   - provider=Ollama + keyring NoEntry → 空 key 注入
///   - provider=其它 + keyring NoEntry → `KEYRING_ERROR`
///   - 成功 → `spec.env_vars = to_env_vars(profile, key)`
fn inject_profile_env(
  spec: &mut crate::runner::SpawnSpec,
  profile_name: Option<&str>,
) -> Result<(), String> {
  let Some(name) = profile_name else {
    spec.env_vars.clear();
    return Ok(());
  };
  let m = llm_profiles::load_profiles().map_err(|e| format!("LLM_PROFILES_LOAD_ERROR: {e}"))?;
  let meta = m
    .profiles
    .into_iter()
    .find(|p| p.name == name)
    .ok_or_else(|| format!("PROFILE_NOT_FOUND: {name}"))?;
  let is_ollama = meta.provider == "Ollama";
  let key = match keyring_store::read_key(&meta.name) {
    Ok(k) => k,
    Err(e) => {
      if is_ollama {
        String::new()
      } else {
        return Err(format!("KEYRING_ERROR: {e}"));
      }
    }
  };
  let env_vars = llm_profiles::to_env_vars(&meta, &key)
    .map_err(|e| format!("ENV_VARS_BUILD_ERROR: {e}"))?;
  spec.env_vars = env_vars;
  Ok(())
}
```

注:`llm_profiles::to_env_vars` 已在 W15-A T2 实装(若签名不同,以实际为准;读 `llm_profiles.rs:pub fn to_env_vars` 调整)。

- [ ] **Step 5:`run_pipeline` 签名加 3 参数 + 改用 `inject_profile_env`**

```rust
#[tauri::command]
pub async fn run_pipeline(
  inbox_dir: String,
  workspace_root: Option<String>,
  llm_profile_name: Option<String>,         // W15-A T7.2:替 llm 显式 provider(若 profile_name 非空,CLI 不传 --llm,让主仓按 profile 派生)
  image_agent_profile_name: Option<String>, // W15-A T7.2
  task_text: Option<String>,                // W15-A T7.2
  imagegen: Option<String>,
  stop_after: Option<String>,
  no_longdoc: Option<bool>,
  force: Option<bool>,
) -> CommandResponse<RunPipelineResult> {
  // ... 现有 resolve_inbox + resolve_media_to_doc_project 不动 ...
  let mut spec = build_mtd_run_args(
    &project,
    &inbox,
    llm.as_deref(),  // 保留 --llm 透传(spec §2.2:profile_name 覆盖 --llm,主仓优先级自己定)
    imagegen.as_deref(),
    stop_after.as_deref(),
    no_longdoc.unwrap_or(false),
    force.unwrap_or(false),
    llm_profile_name.as_deref(),
    image_agent_profile_name.as_deref(),
    task_text.as_deref(),
  );
  // 替原 inject_active_llm_env
  if let Err(e) = inject_profile_env(&mut spec, llm_profile_name.as_deref()) {
    return CommandResponse::err(e);
  }
  // ... 后续 registry.is_running + spawn_mtd 不动 ...
}
```

`llm: Option<String>` 参数若 spec §2.2 要求「profile_name 覆盖 --llm 时不传 --llm」,则把 `llm.as_deref()` 改为 `if llm_profile_name.is_none() { llm.as_deref() } else { None }`。**用 spec §2.2 + 用户拍板的「per-run profile 与 --llm 共存,profile 优先」决定**;简化版本是 profile 非空 → 忽略 `llm` arg。

- [ ] **Step 6:`resume_pipeline` 同样改**

签名加 3 参数;`build_mtd_resume_args` 多传 3 参数;`inject_profile_env` 调用。

- [ ] **Step 7:跑测试验证通过**

Run: `cd F:/soft/00selfmade/media-to-doc-ui/src-tauri && cargo test --lib t7_2_ -v`
Expected: 5 个 PASS

- [ ] **Step 8:跑全部 Rust 测试**

Run: `cd F:/soft/00selfmade/media-to-doc-ui/src-tauri && cargo test --lib -q`
Expected: 108 passed(103 + 5 new);既有 98 + 既有 5 = 103 不动

- [ ] **Step 9:跑 `cargo tauri build`,确认命令面编译过**

Run: `cd F:/soft/00selfmade/media-to-doc-ui/src-tauri && cargo tauri build 2>&1 | tail -30`
Expected: `Finished` + `Bundling ...`;warnings 不增(5 既有)

- [ ] **Step 10:Save state**

追加到 `task.md` §进度:
```
| T6 | Tauri commands.rs per-run profile 注入 | ✅ | 5/5 | 同上 |
```
**不 commit**。

---

## Task 7: Tauri `tauri-plugin-dialog` + 5 个 project registry commands

**Files:**
- Modify: `F:/soft/00selfmade/media-to-doc-ui/src-tauri/Cargo.toml`(加 `tauri-plugin-dialog = "2"`)
- Modify: `F:/soft/00selfmade/media-to-doc-ui/src-tauri/src/lib.rs`(注册 plugin)
- Modify: `F:/soft/00selfmade/media-to-doc-ui/src-tauri/capabilities/default.json`(加 `"dialog:default"`)
- Modify: `F:/soft/00selfmade/media-to-doc-ui/src-tauri/src/commands.rs`(加 5 个 project registry 命令 + tests)
- Modify: `F:/soft/00selfmade/media-to-doc-ui/src-tauri/src/types.rs`(可能加 `ProjectEntry` / `RegistryFile` 结构 —— 视实际看)

**Interfaces:**
- Consumes:无(Tauri 内部)
- Produces:
  ```rust
  // commands.rs
  #[derive(Serialize, Deserialize, Clone)]
  pub struct ProjectEntry {
      pub id: String,           // sha256(canonical_path)[:16]
      pub path: String,
      pub display_name: String,
      pub last_used_at: String, // RFC3339-ish
      pub added_at: String,
      pub sessions: Vec<SessionRef>,
  }

  #[tauri::command]
  pub fn list_projects() -> CommandResponse<Vec<ProjectEntry>>;
  #[tauri::command]
  pub fn add_project(path: String) -> CommandResponse<ProjectEntry>;
  #[tauri::command]
  pub fn remove_project(id: String) -> CommandResponse<()>;
  #[tauri::command]
  pub fn touch_project(id: String) -> CommandResponse<()>;
  ```

- [ ] **Step 1:读 `Cargo.toml` `[dependencies]` 段**

Run: `Read("F:/soft/00selfmade/media-to-doc-ui/src-tauri/Cargo.toml")`

- [ ] **Step 2:写失败测试(project registry 5 个函数)**

```rust
// commands.rs 末尾 mod runner_tests append(或新建 mod project_registry_tests)
fn tmpdir(name: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("ui_proj_{name}"));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn override_registry_dir(dir: &std::path::Path) {
    // SAFETY: test-only, single-threaded
    unsafe { std::env::set_var("MEDIA_TO_DOC_PROJECT_REGISTRY_DIR", dir); }
}

#[test]
fn list_projects_empty_when_registry_missing() {
    let tmp = tmpdir("list_proj_empty");
    override_registry_dir(&tmp);
    let r = list_projects_impl();
    assert!(r.ok, "{:?}", r);
    assert_eq!(r.data.unwrap().len(), 0);
    unsafe { std::env::remove_var("MEDIA_TO_DOC_PROJECT_REGISTRY_DIR"); }
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn add_project_canonicalizes_and_dedupes() {
    let tmp = tmpdir("add_proj_dedupe");
    override_registry_dir(&tmp);
    // 真路径 + 软链 → 应识别为同一
    let real = tmp.join("real");
    let link = tmp.join("link");
    std::fs::create_dir_all(&real).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&real, &link).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&real, &link).unwrap();
    let r1 = add_project_impl(real.to_string_lossy().into_owned());
    let r2 = add_project_impl(link.to_string_lossy().into_owned());
    assert!(r1.ok && r2.ok);
    assert_eq!(r1.data.as_ref().unwrap().id, r2.data.as_ref().unwrap().id);
    let list = list_projects_impl().data.unwrap();
    assert_eq!(list.len(), 1);
    unsafe { std::env::remove_var("MEDIA_TO_DOC_PROJECT_REGISTRY_DIR"); }
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn add_project_windows_path_case_insensitive() {
    // 仅 Windows 上跑
    if !cfg!(windows) { return; }
    let tmp = tmpdir("add_proj_case");
    override_registry_dir(&tmp);
    let r1 = add_project_impl(tmp.join("Foo").to_string_lossy().into_owned());
    let r2 = add_project_impl(tmp.join("foo").to_string_lossy().into_owned());
    if r1.ok && r2.ok {
        assert_eq!(r1.data.as_ref().unwrap().id, r2.data.as_ref().unwrap().id);
    }
    unsafe { std::env::remove_var("MEDIA_TO_DOC_PROJECT_REGISTRY_DIR"); }
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn add_project_different_paths_same_name_have_different_ids() {
    let tmp = tmpdir("add_proj_diff_paths");
    override_registry_dir(&tmp);
    let d1 = tmp.join("a"); std::fs::create_dir_all(&d1).unwrap();
    let d2 = tmp.join("b"); std::fs::create_dir_all(&d2).unwrap();
    let r1 = add_project_impl(d1.join("foo").to_string_lossy().into_owned());
    let r2 = add_project_impl(d2.join("foo").to_string_lossy().into_owned());
    assert_ne!(
        r1.data.unwrap().id,
        r2.data.unwrap().id,
        "重名不同路径应区分"
    );
    unsafe { std::env::remove_var("MEDIA_TO_DOC_PROJECT_REGISTRY_DIR"); }
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn remove_project_persists() {
    let tmp = tmpdir("rm_proj");
    override_registry_dir(&tmp);
    let r = add_project_impl(tmp.to_string_lossy().into_owned());
    let id = r.data.unwrap().id;
    let _ = remove_project_impl(id);
    let list = list_projects_impl().data.unwrap();
    assert!(list.iter().all(|p| p.id != id));
    unsafe { std::env::remove_var("MEDIA_TO_DOC_PROJECT_REGISTRY_DIR"); }
    let _ = std::fs::remove_dir_all(&tmp);
}
```

- [ ] **Step 3:跑测试验证失败**

Run: `cd F:/soft/00selfmade/media-to-doc-ui/src-tauri && cargo test --lib list_projects -v`
Expected: FAIL `list_projects_impl` not defined

- [ ] **Step 4:实现 4 个 `*_impl` 纯函数 + 4 个 `#[tauri::command]`**

`commands.rs` 末尾加:

```rust
// ────────────────────────────────────────────────────────────
// W15-A T7.2: project registry(persistent JSON)
// ────────────────────────────────────────────────────────────

use serde::Deserialize;
use sha2::{Digest, Sha256};  // 顶部 import

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRef {
    pub work_dir: String,
    pub started_at: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEntry {
    pub id: String,
    pub path: String,
    pub display_name: String,
    pub last_used_at: String,
    pub added_at: String,
    #[serde(default)]
    pub sessions: Vec<SessionRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryFile {
    pub version: u32,
    pub projects: Vec<ProjectEntry>,
}

/// 规范化:Windows 大小写归一 + NFC unicode + canonicalize。
fn canonicalize_path(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    // NFC unicode 归一(简易版:依赖 OS;Windows 已 NFC)
    let normalized = if cfg!(windows) {
        // to_lowercase 路径部分
        s.to_lowercase()
    } else {
        s.to_string()
    };
    let pb = PathBuf::from(&normalized);
    std::fs::canonicalize(&pb).unwrap_or(pb)
}

fn canonical_id(p: &Path) -> String {
    let canon = canonicalize_path(p);
    let mut hasher = Sha256::new();
    hasher.update(canon.to_string_lossy().as_bytes());
    let bytes = hasher.finalize();
    let hex: String = bytes.iter().take(8).map(|b| format!("{b:02x}")).collect();
    hex
}

fn registry_path() -> PathBuf {
    // 优先 env MEDIA_TO_DOC_PROJECT_REGISTRY_DIR(测试用);
    // fallback 到 Tauri app_config_dir
    if let Ok(v) = std::env::var("MEDIA_TO_DOC_PROJECT_REGISTRY_DIR") {
        return PathBuf::from(v).join("project_registry.json");
    }
    // Tauri 2 标准:用 tauri::api::path::app_config_dir()
    // 但 impl 阶段为单测友好,用 USERPROFILE/AppData/Roaming 推断
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata).join("com.duanyi.mediatodoc").join("project_registry.json");
    }
    PathBuf::from("project_registry.json")
}

fn load_registry() -> RegistryFile {
    let p = registry_path();
    if !p.is_file() {
        return RegistryFile { version: 1, projects: vec![] };
    }
    match std::fs::read_to_string(&p).ok().and_then(|s| serde_json::from_str::<RegistryFile>(&s).ok()) {
        Some(r) => r,
        None => RegistryFile { version: 1, projects: vec![] },
    }
}

fn save_registry(r: &RegistryFile) -> Result<(), String> {
    let p = registry_path();
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {} 失败: {e}", parent.display()))?;
    }
    let raw = serde_json::to_string_pretty(r).map_err(|e| format!("serialize 失败: {e}"))?;
    std::fs::write(&p, raw).map_err(|e| format!("写 {} 失败: {e}", p.display()))?;
    Ok(())
}

pub fn list_projects_impl() -> CommandResponse<Vec<ProjectEntry>> {
    let r = load_registry();
    CommandResponse::ok(r.projects)
}

pub fn add_project_impl(path: String) -> CommandResponse<ProjectEntry> {
    let p = PathBuf::from(&path).expand();
    if !p.is_dir() {
        return CommandResponse::err(format!("目录不存在: {}", p.display()));
    }
    let canon = canonicalize_path(&p);
    let id = canonical_id(&canon);
    let display_name = canon
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| canon.to_string_lossy().into_owned());
    let now = chrono_like_now();
    let mut r = load_registry();
    if let Some(existing) = r.projects.iter_mut().find(|e| e.id == id) {
        existing.last_used_at = now.clone();
        existing.sessions = existing.sessions.clone();  // 不动
        save_registry(&r).map_err(CommandResponse::err)?;
        return CommandResponse::ok(existing.clone());
    }
    let entry = ProjectEntry {
        id: id.clone(),
        path: canon.to_string_lossy().into_owned(),
        display_name,
        last_used_at: now.clone(),
        added_at: now,
        sessions: vec![],
    };
    r.projects.push(entry.clone());
    save_registry(&r).map_err(CommandResponse::err)?;
    CommandResponse::ok(entry)
}

pub fn remove_project_impl(id: String) -> CommandResponse<()> {
    let mut r = load_registry();
    let before = r.projects.len();
    r.projects.retain(|e| e.id != id);
    if r.projects.len() == before {
        return CommandResponse::err(format!("PROJECT_NOT_FOUND: {id}"));
    }
    save_registry(&r).map_err(CommandResponse::err)?;
    CommandResponse::ok(())
}

pub fn touch_project_impl(id: String) -> CommandResponse<()> {
    let mut r = load_registry();
    let now = chrono_like_now();
    if let Some(e) = r.projects.iter_mut().find(|e| e.id == id) {
        e.last_used_at = now;
        save_registry(&r).map_err(CommandResponse::err)?;
        return CommandResponse::ok(());
    }
    CommandResponse::err(format!("PROJECT_NOT_FOUND: {id}"))
}

#[tauri::command] pub async fn list_projects() -> CommandResponse<Vec<ProjectEntry>> { list_projects_impl() }
#[tauri::command] pub async fn add_project(path: String) -> CommandResponse<ProjectEntry> { add_project_impl(path) }
#[tauri::command] pub async fn remove_project(id: String) -> CommandResponse<()> { remove_project_impl(id) }
#[tauri::command] pub async fn touch_project(id: String) -> CommandResponse<()> { touch_project_impl(id) }
```

- [ ] **Step 5:`Cargo.toml` 加 `sha2` 依赖(若尚未有)**

Read `Cargo.toml` `[dependencies]` 段;若无 `sha2 = "1"` → 加。`tauri-plugin-dialog` 加在 T7 Step 6 一并做。

- [ ] **Step 6:`Cargo.toml` 加 `tauri-plugin-dialog`**

```toml
tauri-plugin-dialog = "2"
```

- [ ] **Step 7:`lib.rs` 注册 dialog plugin**

读 `src-tauri/src/lib.rs` 完整内容,找 `tauri::Builder::default()` 链。在 `.plugin(...)` 段加:

```rust
.plugin(tauri_plugin_dialog::init())
```

(具体 API 看 plugin 文档 2.x 版本;若用 `Builder::new().build()` 模式 → 调整)

- [ ] **Step 8:`capabilities/default.json` 加 dialog 权限**

```json
"permissions": [
    "core:default",
    "list_llm_profiles",
    "get_active_llm_profile_name",
    "save_llm_profile",
    "set_active_profile",
    "delete_llm_profile",
    "test_llm_connection",
    "dialog:default",
    "dialog:allow-open"
]
```

(具体 permission 标识以 plugin 文档为准)

- [ ] **Step 9:跑测试验证通过**

Run: `cd F:/soft/00selfmade/media-to-doc-ui/src-tauri && cargo test --lib t7_2_proj -v`
Expected: 5 个 PASS

- [ ] **Step 10:跑全部 Rust 测试**

Run: `cd F:/soft/00selfmade/media-to-doc-ui/src-tauri && cargo test --lib -q`
Expected: 113 passed(108 + 5 new)

- [ ] **Step 11:`cargo tauri build` 验证编译**

Run: `cd F:/soft/00selfmade/media-to-doc-ui/src-tauri && cargo tauri build 2>&1 | tail -30`
Expected: Finished + Bundling ...;warnings 5 → 仍 5

- [ ] **Step 12:Save state**

追加到 `task.md` §进度:
```
| T7 | Tauri project registry 4 commands + dialog plugin | ✅ | 5/5 | 同上 |
```
**不 commit**。

---

## Task 8: 前端 New Run tab 动态 dropdowns + task textarea + 选目录按钮

**Files:**
- Modify: `F:/soft/00selfmade/media-to-doc-ui/src/index.html`(`__mountNewRunTab__` + `buildNewRunForm` 大改 + 新增 `__projectTree__.refresh()`)
- Read: `__mountNewRunTab__` 当前实现(行 958-1042)

**Interfaces:**
- Consumes:`list_llm_profiles` / `list_projects` / `add_project` Tauri commands(T6 + T7 产出)
- Produces:UI 行为
  - mount 时拉 `list_llm_profiles` 填主 LLM `<select>` + Image Agent 策划 LLM `<select>`
  - 新增 task `<textarea>`
  - 新增「选目录」按钮 → `tauri-plugin-dialog` `open({directory:true})` → `add_project` → 刷新左侧树 + 自动选中
  - form 提交时新增 3 字段透传到 `run_pipeline`

- [ ] **Step 1:读 `buildNewRunForm` + `__mountNewRunTab__` 当前实现**

Run: `Read("F:/soft/00selfmade/media-to-doc-ui/src/index.html", offset=956, limit=90)`
Expected: 看到 `buildNewRunForm` + `__mountNewRunTab__`

- [ ] **Step 2:写「手动验收清单」checklist**(UI 测试靠 eyeball,不算 RED→GREEN 但要列)

```
- [ ] 打开 New Run tab → 主 LLM 下拉自动填 profile names
- [ ] Image Agent 折叠面板展开 → 策划 LLM 下拉也是 profile names
- [ ] 输入 task text 提交 → state.json 含 task_text
- [ ] 点「选目录」 → 弹原生文件选择器 → 选完左侧立即出现
- [ ] 选过同路径 → 不重复,合并 sessions
```

- [ ] **Step 3:`buildNewRunForm` 改造**

完整新 form(替换行 958-1008 整个 `buildNewRunForm` 函数体):

```javascript
function buildNewRunForm(coursePath) {
  const d = document.createElement('div');
  d.innerHTML = `
    <div class="card">
      <h2>New Run</h2>
      <div class="kv">
        <dt>Course</dt><dd>${escapeHtml(coursePath || '(请先选课程)')}</dd>
        <dt>工作目录</dt>
        <dd>
          <span id="new-run-workdir-display">${escapeHtml(coursePath || '(未选)')}</span>
          <button type="button" class="secondary" id="new-run-pick-dir-btn" style="margin-left: 8px;">选目录</button>
        </dd>
      </div>
      <form id="new-run-form" style="margin-top: 12px;">
        <label style="display: block; margin-bottom: 8px;">
          任务说明(可选):
          <textarea name="taskText" rows="4" placeholder="说说你想怎么处理这个视频,例如:&quot;突出第 2 节的客户案例&quot;" style="display: block; width: 100%; margin-top: 4px; font-family: inherit;"></textarea>
        </label>
        <div style="margin-top: 8px;">
          <label>主 LLM:
            <select name="llm" id="new-run-llm-select">
              <option value="">(default / 走 CLI 默认)</option>
            </select>
          </label>
        </div>
        <details style="margin-top: 8px;">
          <summary>Image Agent(可选)</summary>
          <div style="padding: 8px 0;">
            <label>策划 LLM:
              <select name="llmProfileImageAgent" id="new-run-image-agent-select">
                <option value="">(关闭 Image Agent)</option>
              </select>
            </label>
            <label style="margin-left: 12px;">出图 provider:
              <select name="imagegen">
                <option value="">(default)</option>
                <option value="skip">skip(只生成策划,不出图)</option>
                <option value="local_sdxl">local_sdxl(占位实现)</option>
              </select>
            </label>
          </div>
        </details>
        <div style="margin-top: 8px;">
          <label>Stop after:
            <select name="stopAfter" id="new-run-stop-after-select">
              <option value="">(none) 完整运行到 verify</option>
              <option value="audio">audio — 提取声音</option>
              <option value="asr">asr — 转写文字</option>
              <option value="frames">frames — 关键画面</option>
              <option value="ocr">ocr — 画面文字识别</option>
              <option value="asr_correct">asr_correct — 校正转写</option>
              <option value="chapters">chapters — 章节结构</option>
              <option value="draft">draft — 分章草稿</option>
              <option value="imagegen">imagegen — AI 配图</option>
              <option value="render">render — 生成 md/html</option>
              <option value="longdoc">longdoc — 深度净化</option>
              <option value="verify">verify — 质量检查(完整终点)</option>
            </select>
          </label>
        </div>
        <div style="margin-top: 8px;">
          <label style="margin-left: 12px;"><input type="checkbox" name="noLongdoc"> no-longdoc</label>
          <label style="margin-left: 12px;"><input type="checkbox" name="force"> force</label>
        </div>
        <div style="margin-top: 12px;">
          <button type="submit" id="new-run-submit-btn">▶ Run pipeline</button>
          <button type="button" class="secondary" id="new-run-cancel-btn" style="margin-left: 8px;">取消</button>
        </div>
      </form>
    </div>
  `;
  return d;
}
```

- [ ] **Step 4:`__mountNewRunTab__` 重写:拉 profiles + 装 dialog + 提交改**

```javascript
async function __mountNewRunTab__(container, tab) {
  // 1. 拉 profiles 填两个 select
  try {
    const r = await invoke('list_llm_profiles');
    if (r.ok) {
      const llmSel = container.querySelector('#new-run-llm-select');
      const agentSel = container.querySelector('#new-run-image-agent-select');
      // 保存现有 "(default)" option,清空再追加
      llmSel.innerHTML = '<option value="">(default / 走 CLI 默认)</option>';
      agentSel.innerHTML = '<option value="">(关闭 Image Agent)</option>';
      for (const p of r.data) {
        const opt1 = document.createElement('option');
        opt1.value = p.name;
        opt1.textContent = `${p.name} — ${p.provider}/${p.model}`;
        llmSel.appendChild(opt1);
        const opt2 = opt1.cloneNode(true);
        agentSel.appendChild(opt2);
      }
    } else {
      toast('读 profiles 失败: ' + r.error, 'error');
    }
  } catch (err) {
    toast('list_llm_profiles: ' + err, 'error');
  }

  // 2. 选目录按钮
  container.querySelector('#new-run-pick-dir-btn')?.addEventListener('click', async () => {
    try {
      // Tauri 2 + plugin-dialog(W15-A T12 已开 withGlobalTauri=true)
      const dir = await window.__TAURI__.dialog.open({ directory: true, multiple: false });
      if (!dir) return;
      const r = await invoke('add_project', { path: dir });
      if (!r.ok) { toast('add_project: ' + r.error, 'error'); return; }
      tab.coursePath = r.data.path;
      tab.workDir = null;
      container.querySelector('#new-run-workdir-display').textContent = r.data.path;
      // 刷新侧栏
      if (window.__projectTree__?.refresh) {
        await window.__projectTree__.refresh();
        window.__projectTree__.selectProject?.(r.data.id);
      }
      toast('已注册项目: ' + r.data.display_name, 'success');
    } catch (err) {
      toast('选目录: ' + err, 'error');
    }
  });

  // 3. form submit
  const form = container.querySelector('#new-run-form');
  form?.addEventListener('submit', async (e) => {
    e.preventDefault();
    const fd = new FormData(form);
    const inboxDir = tab.coursePath || container.querySelector('#new-run-workdir-display').textContent;
    if (!inboxDir || inboxDir === '(未选)') { toast('请先选课程或目录', 'error'); return; }
    const opts = {
      inboxDir,
      workspaceRoot: null,
      llmProfileName: fd.get('llm') || null,
      imageAgentProfileName: fd.get('llmProfileImageAgent') || null,
      taskText: fd.get('taskText') || null,
      imagegen: fd.get('imagegen') || null,
      stopAfter: fd.get('stopAfter') || null,
      noLongdoc: !!fd.get('noLongdoc'),
      force: !!fd.get('force'),
    };
    try {
      const r = await invoke('run_pipeline', opts);
      if (!r.ok) { toast('run_pipeline: ' + r.error, 'error'); return; }
      toast('Started: ' + r.data.work_dir, 'success');
      const newWd = r.data.work_dir;
      window.__tabManager__.closeTab(tab.id);
      window.__tabManager__.openTab({ type: 'session', workDir: newWd });
    } catch (err) {
      toast('run_pipeline: ' + err, 'error');
    }
  });

  // 4. 取消
  container.querySelector('#new-run-cancel-btn')?.addEventListener('click', () => {
    window.__tabManager__.closeTab(tab.id);
  });
}
window.__mountNewRunTab__ = __mountNewRunTab__;
```

- [ ] **Step 5:暴露 `__projectTree__.refresh()` + `.selectProject(id)`**

在现有 project tree 渲染函数(T6 实装,grep `__projectTree__` 找位置)末尾 append:

```javascript
window.__projectTree__ = window.__projectTree__ || {};
window.__projectTree__.refresh = async () => {
  const r = await invoke('list_projects');
  if (!r.ok) { toast('list_projects: ' + r.error, 'error'); return; }
  // 调用现有 render 函数
  if (typeof renderProjectTree === 'function') renderProjectTree(r.data);
  else if (window.renderProjectTree) window.renderProjectTree(r.data);
};
window.__projectTree__.selectProject = (id) => {
  // 高亮 + 滚动到项目节点
  const el = document.querySelector(`[data-project-id="${id}"]`);
  if (el) { el.scrollIntoView({ block: 'nearest' }); el.classList.add('selected'); }
};
```

具体 `renderProjectTree` 函数名以现状为准(读 `Grep("renderProjectTree", path="src/index.html")`)。

- [ ] **Step 6:手动 verify(eyeball)**

- 启动 `cargo tauri dev`(本地 dev 模式),打开 New Run tab
- 验 dropdown 自动填 profile
- 验 Image Agent 折叠面板展开有 2 个 select
- 验 Stop after 中文 label
- 验「选目录」按钮弹原生 picker
- 验选完左侧立即刷新 + 选中

- [ ] **Step 7:Save state**

追加到 `task.md` §进度:
```
| T8 | 前端 New Run tab 动态 dropdowns + task textarea + 选目录 | ✅ | 手动 verify | 同上 |
```
**不 commit**。

---

## Task 9: long-doc snapshot bootstrap + sync/verify 脚本

**Files:**
- Create: `F:/soft/00selfmade/media-to-doc/src/media_to_doc/data/long_doc_skill/SKILL.md`(从 Skill 真身复制)
- Create: `F:/soft/00selfmade/media-to-doc/src/media_to_doc/data/long_doc_skill/references/*.md`(13 文件)
- Create: `F:/soft/00selfmade/media-to-doc/src/media_to_doc/data/long_doc_skill/scripts/*.py`(10 文件)
- Create: `F:/soft/00selfmade/media-to-doc/src/media_to_doc/data/long_doc_skill/MANIFEST.json`(sha256 manifest)
- Create: `F:/soft/00selfmade/media-to-doc/scripts/sync_long_doc_skill.py`
- Create: `F:/soft/00selfmade/media-to-doc/scripts/verify_long_doc_skill.py`
- Create: `F:/soft/00selfmade/media-to-doc/tests/test_sync_long_doc_skill.py`

**Interfaces:**
- Consumes:`C:/Users/Duanyi/.claude/skills/long-doc-processor/` Skill 真身
- Produces:
  ```python
  # sync_long_doc_skill.py
  def sync(source: Path, dest: Path) -> SyncReport:
      """读 source 白名单 → 复制 → 写 MANIFEST → 返回 diff。"""
  # verify_long_doc_skill.py
  def verify(snapshot: Path) -> VerifyResult:
      """读 MANIFEST → 重算 hash → 对比。"""
  ```

- [ ] **Step 1:创建 snapshot 目录骨架**

Run:
```bash
mkdir -p "F:/soft/00selfmade/media-to-doc/src/media_to_doc/data/long_doc_skill/references"
mkdir -p "F:/soft/00selfmade/media-to-doc/src/media_to_doc/data/long_doc_skill/scripts"
```
Expected: 2 个空目录创建成功

- [ ] **Step 2:复制 SKILL.md + references/ + scripts/**

```bash
cp "C:/Users/Duanyi/.claude/skills/long-doc-processor/SKILL.md" \
   "F:/soft/00selfmade/media-to-doc/src/media_to_doc/data/long_doc_skill/SKILL.md"

cp "C:/Users/Duanyi/.claude/skills/long-doc-processor/references/"*.md \
   "F:/soft/00selfmade/media-to-doc/src/media_to_doc/data/long_doc_skill/references/"

cp "C:/Users/Duanyi/.claude/skills/long-doc-processor/scripts/"*.py \
   "F:/soft/00selfmade/media-to-doc/src/media_to_doc/data/long_doc_skill/scripts/"

ls "F:/soft/00selfmade/media-to-doc/src/media_to_doc/data/long_doc_skill/references/" | wc -l
# expected: 13
ls "F:/soft/00selfmade/media-to-doc/src/media_to_doc/data/long_doc_skill/scripts/" | wc -l
# expected: 10
```
**注意**:evals/、lessons.md、SKILL.md.v3.8.bak 等不进 snapshot(白名单)。

- [ ] **Step 3:写 `scripts/sync_long_doc_skill.py`**

```python
"""W15-A T7.2:同步 ~/.claude/skills/long-doc-processor/ → vendored snapshot。

用法:
    python scripts/sync_long_doc_skill.py [--source PATH] [--dest PATH]

默认 source = $CLAUDE_SKILLS_PATH 或 ~/.claude/skills/long-doc-processor
默认 dest   = <repo>/src/media_to_doc/data/long_doc_skill

退出码:
    0 同步成功
    1 source 不存在或部分文件复制失败
    2 参数错
"""
import argparse
import hashlib
import json
import shutil
import sys
from datetime import datetime, timezone
from pathlib import Path

# 白名单
WHITELIST_TOP = ["SKILL.md"]
WHITELIST_REFS = [
    "content-rules.md", "gotchas.md", "image-pipeline.md", "image-style.md",
    "ooxml-numbering.md", "phase-0-input.md", "phase-1-purification.md",
    "phase-2-merge.md", "phase-3-render-html.md", "qa-gates.md",
    "runtime-compatibility.md", "maintenance.md",
]
WHITELIST_SCRIPTS = [
    "doc_to_md.py", "generate_image.py", "markdown_to_docx.py",
    "markdown_to_html.py", "ocr_images.py", "renumber_headings.py",
    "validate_skill.py", "verify_docx.py", "verify_html.py",
    "xmind_to_md.py",
]


def sha256_file(p: Path) -> str:
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def sync(source: Path, dest: Path) -> dict:
    report = {"added": [], "modified": [], "deleted": [], "unchanged": [], "errors": []}
    if not source.is_dir():
        return {"error": f"source 不存在: {source}", **report}
    dest.mkdir(parents=True, exist_ok=True)
    (dest / "references").mkdir(exist_ok=True)
    (dest / "scripts").mkdir(exist_ok=True)

    files_to_sync = (
        [(source / f, dest / f) for f in WHITELIST_TOP] +
        [(source / "references" / f, dest / "references" / f) for f in WHITELIST_REFS] +
        [(source / "scripts" / f, dest / "scripts" / f) for f in WHITELIST_SCRIPTS]
    )

    for src, dst in files_to_sync:
        rel = str(dst.relative_to(dest))
        if not src.is_file():
            report["errors"].append(f"source 缺文件: {src}")
            continue
        new_hash = sha256_file(src)
        old_hash = sha256_file(dst) if dst.is_file() else None
        if old_hash == new_hash:
            report["unchanged"].append(rel)
            continue
        try:
            shutil.copy2(src, dst)
        except OSError as e:
            report["errors"].append(f"copy {rel} 失败: {e}")
            continue
        if old_hash is None:
            report["added"].append(rel)
        else:
            report["modified"].append(rel)

    # 写 MANIFEST
    manifest_files = []
    for src, dst in files_to_sync:
        if dst.is_file():
            manifest_files.append({
                "path": str(dst.relative_to(dest)).replace("\\", "/"),
                "sha256": sha256_file(dst),
                "size_bytes": dst.stat().st_size,
            })
    manifest = {
        "version": "1.0.0",
        "synced_from": str(source),
        "synced_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "files": manifest_files,
    }
    (dest / "MANIFEST.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    return report


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", default=None)
    parser.add_argument("--dest", default=None)
    args = parser.parse_args()

    default_source = Path.home() / ".claude" / "skills" / "long-doc-processor"
    source = Path(args.source) if args.source else Path(
        __import__("os").environ.get("CLAUDE_SKILLS_PATH", str(default_source))
    )
    default_dest = Path(__file__).resolve().parent.parent / "src" / "media_to_doc" / "data" / "long_doc_skill"
    dest = Path(args.dest) if args.dest else default_dest

    if not source.is_dir():
        print(f"ERROR: source 不存在: {source}", file=sys.stderr)
        return 1
    report = sync(source, dest)
    if "error" in report:
        print(f"ERROR: {report['error']}", file=sys.stderr)
        return 1
    print(f"Sync 完成: +{len(report['added'])} ~{len(report['modified'])} -{len(report['deleted'])} ={len(report['unchanged'])}")
    for e in report["errors"]:
        print(f"  ERROR: {e}", file=sys.stderr)
    return 1 if report["errors"] else 0


if __name__ == "__main__":
    sys.exit(main())
```

- [ ] **Step 4:跑 sync 生成 MANIFEST**

Run: `cd F:/soft/00selfmade/media-to-doc && python scripts/sync_long_doc_skill.py`
Expected: `Sync 完成: +X ~Y -Z =N`(X=26,Y=0,Z=0,N=0,首次全 added)

- [ ] **Step 5:验证 MANIFEST 生成**

Run: `head -20 "F:/soft/00selfmade/media-to-doc/src/media_to_doc/data/long_doc_skill/MANIFEST.json"`
Expected: 看到 `"version": "1.0.0"` + `"files"` 数组含 26 文件

- [ ] **Step 6:写 `scripts/verify_long_doc_skill.py`**

```python
"""W15-A T7.2:校验 snapshot hash 是否与 MANIFEST 一致。"""
import argparse
import hashlib
import json
import sys
from pathlib import Path


def sha256_file(p: Path) -> str:
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def verify(snapshot: Path) -> int:
    manifest_p = snapshot / "MANIFEST.json"
    if not manifest_p.is_file():
        print(f"ERROR: MANIFEST.json 缺失: {manifest_p}", file=sys.stderr)
        return 1
    manifest = json.loads(manifest_p.read_text(encoding="utf-8"))
    errors = []
    for entry in manifest.get("files", []):
        p = snapshot / entry["path"]
        if not p.is_file():
            errors.append(f"缺文件: {entry['path']}")
            continue
        actual = sha256_file(p)
        if actual != entry["sha256"]:
            errors.append(f"hash 漂移: {entry['path']} 期望 {entry['sha256'][:12]}... 实际 {actual[:12]}...")
    if errors:
        for e in errors:
            print(f"  {e}", file=sys.stderr)
        return 1
    print(f"Verify OK: {len(manifest.get('files', []))} 文件 hash 一致")
    return 0


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--snapshot", default=None)
    args = parser.parse_args()
    default_snapshot = Path(__file__).resolve().parent.parent / "src" / "media_to_doc" / "data" / "long_doc_skill"
    snapshot = Path(args.snapshot) if args.snapshot else default_snapshot
    sys.exit(verify(snapshot))


if __name__ == "__main__":
    sys.exit(main())
```

- [ ] **Step 7:写失败测试 `tests/test_sync_long_doc_skill.py`**

```python
"""W15-A T7.2:sync/verify 脚本测试。"""
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SNAPSHOT = ROOT / "src" / "media_to_doc" / "data" / "long_doc_skill"


def test_sync_copies_whitelisted_files(tmp_path):
    # mock source
    src = tmp_path / "skill"
    (src / "references").mkdir(parents=True)
    (src / "scripts").mkdir()
    (src / "SKILL.md").write_text("# mock SKILL")
    (src / "references" / "content-rules.md").write_text("# mock")
    (src / "scripts" / "doc_to_md.py").write_text("# mock")
    # 跑 sync
    r = subprocess.run(
        [sys.executable, str(ROOT / "scripts" / "sync_long_doc_skill.py"),
         "--source", str(src), "--dest", str(SNAPSHOT)],
        capture_output=True, text=True, timeout=30,
    )
    assert r.returncode == 0, f"sync 失败: stderr={r.stderr}"
    assert (SNAPSHOT / "SKILL.md").exists()
    assert (SNAPSHOT / "references" / "content-rules.md").exists()
    assert (SNAPSHOT / "scripts" / "doc_to_md.py").exists()


def test_sync_writes_manifest_with_correct_sha256(tmp_path):
    src = tmp_path / "skill2"
    (src / "SKILL.md").write_text("# test content")
    subprocess.run(
        [sys.executable, str(ROOT / "scripts" / "sync_long_doc_skill.py"),
         "--source", str(src), "--dest", str(SNAPSHOT)],
        check=True, capture_output=True, timeout=30,
    )
    manifest = json.loads((SNAPSHOT / "MANIFEST.json").read_text(encoding="utf-8"))
    sk = next(f for f in manifest["files"] if f["path"] == "SKILL.md")
    assert len(sk["sha256"]) == 64


def test_sync_source_missing_exits_1(tmp_path):
    r = subprocess.run(
        [sys.executable, str(ROOT / "scripts" / "sync_long_doc_skill.py"),
         "--source", str(tmp_path / "no_such_skill"), "--dest", str(SNAPSHOT)],
        capture_output=True, text=True, timeout=30,
    )
    assert r.returncode == 1
    assert "source 不存在" in r.stderr


def test_verify_passes_after_sync():
    # 假设 sync 已跑过(MANIFEST 与文件一致)
    r = subprocess.run(
        [sys.executable, str(ROOT / "scripts" / "verify_long_doc_skill.py")],
        capture_output=True, text=True, timeout=30,
    )
    assert r.returncode == 0, f"verify 应 pass: stderr={r.stderr}\nstdout={r.stdout}"


def test_verify_fails_on_drift(tmp_path):
    # 改 snapshot 一个文件
    sk = SNAPSHOT / "SKILL.md"
    original = sk.read_text(encoding="utf-8")
    sk.write_text("tampered content", encoding="utf-8")
    try:
        r = subprocess.run(
            [sys.executable, str(ROOT / "scripts" / "verify_long_doc_skill.py")],
            capture_output=True, text=True, timeout=30,
        )
        assert r.returncode == 1, "verify 应失败"
        assert "hash 漂移" in r.stderr
    finally:
        sk.write_text(original, encoding="utf-8")
        # 重新 sync 恢复
        src = Path.home() / ".claude" / "skills" / "long-doc-processor"
        if src.is_dir():
            subprocess.run(
                [sys.executable, str(ROOT / "scripts" / "sync_long_doc_skill.py")],
                check=False, capture_output=True, timeout=30,
            )
```

- [ ] **Step 8:跑测试**

Run: `cd F:/soft/00selfmade/media-to-doc && uv run pytest tests/test_sync_long_doc_skill.py -v`
Expected: 5 个 PASS

- [ ] **Step 9:Save state**

追加到 `task.md` §进度:
```
| T9 | long-doc snapshot bootstrap + sync/verify 脚本 | ✅ | 5/5 | 同上 |
```
**不 commit**。

---

## Task 10: 主仓 longdoc.py 读 vendored snapshot + pyproject package-data

**Files:**
- Modify: `F:/soft/00selfmade/media-to-doc/src/media_to_doc/pipeline/longdoc.py`(用 `importlib.resources` 读 vendored,不读 `~/.claude`)
- Modify: `F:/soft/00selfmade/media-to-doc/pyproject.toml`(加 `[tool.setuptools.package-data]` 或 hatch `force-include`)
- Modify: `F:/soft/00selfmade/media-to-doc/tests/test_longdoc_integration.py`(新建)

**Interfaces:**
- Consumes:long-doc vendored snapshot(T9 产出)
- Produces:
  ```python
  # longdoc.py
  def _skill_root() -> Path:
      """返回 vendored snapshot 真身,不读 ~/.claude。"""
      root = importlib.resources.files("media_to_doc.data.long_doc_skill")
      return Path(str(root))
  ```

- [ ] **Step 1:读 `longdoc.py` 当前实现**

Run: `Read("F:/soft/00selfmade/media-to-doc/src/media_to_doc/pipeline/longdoc.py")`

- [ ] **Step 2:写失败测试(integration 测试)**

```python
# tests/test_longdoc_integration.py(新建)
"""W15-A T7.2:longdoc.py 读 vendored snapshot,不依赖 ~/.claude。"""
from media_to_doc.pipeline.longdoc import load_purification_prompt, _skill_root


def test_skill_root_points_to_vendored_snapshot():
    root = _skill_root()
    assert root.is_dir(), f"vendored snapshot 应存在: {root}"
    assert (root / "SKILL.md").exists(), "SKILL.md 应在 snapshot 内"


def test_load_purification_prompt_returns_non_empty():
    text = load_purification_prompt()
    assert isinstance(text, str)
    assert len(text) > 100, "phase-1-purification.md 应有内容"


def test_load_purification_prompt_does_not_read_claude_dir(monkeypatch, tmp_path):
    """即使 ~/.claude/skills/long-doc-processor/ 不存在或被污染,longdoc 仍走 vendored。"""
    fake = tmp_path / "fake_skill"
    fake.mkdir()
    (fake / "references").mkdir()
    (fake / "references" / "phase-1-purification.md").write_text("WRONG content", encoding="utf-8")
    monkeypatch.setenv("CLAUDE_SKILLS_PATH", str(fake))
    text = load_purification_prompt()
    assert "WRONG content" not in text, "longdoc 不应读 CLAUDE_SKILLS_PATH,只读 vendored"
```

- [ ] **Step 3:跑测试验证失败**

Run: `cd F:/soft/00selfmade/media-to-doc && uv run pytest tests/test_longdoc_integration.py -v`
Expected: FAIL `load_purification_prompt` not defined

- [ ] **Step 4:`longdoc.py` 改实现**

```python
"""W15-A T7.2:longdoc stage —— 读 vendored snapshot,不依赖 ~/.claude。"""
import importlib.resources
from pathlib import Path


def _skill_root() -> Path:
    """返回 vendored Skill snapshot 真身路径。
    
    永远读 <package>/data/long_doc_skill/,不读 ~/.claude/skills/long-doc-processor/。
    """
    root = importlib.resources.files("media_to_doc.data.long_doc_skill")
    return Path(str(root))


def load_purification_prompt() -> str:
    return (_skill_root() / "references" / "phase-1-purification.md").read_text(encoding="utf-8")


def load_content_rules() -> str:
    return (_skill_root() / "references" / "content-rules.md").read_text(encoding="utf-8")


def load_qa_gates() -> str:
    return (_skill_root() / "references" / "qa-gates.md").read_text(encoding="utf-8")


def list_skill_scripts() -> list[Path]:
    """返回 scripts/*.py 路径列表(供 generate_image / renumber_headings 等调用)。"""
    return sorted((_skill_root() / "scripts").glob("*.py"))


# ────────────────────────────────────────────────────────────
# 既有 run_longdoc / process_long_doc 逻辑保留,但其内部 prompt 来源
# 改用 load_* 函数,不再 hardcode 副本
# ────────────────────────────────────────────────────────────
def run_longdoc(work_dir: Path, *args, **kwargs):
    """原 longdoc 入口;prompt 现从 vendored 读。"""
    prompt = load_purification_prompt() + "\n\n" + load_content_rules()
    # ... 调用既有 longdoc 处理逻辑,prompt 替换原 hardcode 部分 ...
```

具体 `run_longdoc` / `process_long_doc` 既有逻辑保留并按需替换 hardcode prompt 段;读 Step 1 后调整。

- [ ] **Step 5:`pyproject.toml` 加 package-data**

读 `pyproject.toml` `[tool.setuptools]` 或 `[tool.hatch.build]` 段(看主仓用哪个):

若 setuptools:

```toml
[tool.setuptools.package-data]
media_to_doc = ["data/long_doc_skill/**/*.md", "data/long_doc_skill/**/*.py", "data/long_doc_skill/MANIFEST.json"]
```

若 hatch:

```toml
[tool.hatch.build.targets.wheel.force-include]
"src/media_to_doc/data/long_doc_skill" = "media_to_doc/data/long_doc_skill"
```

- [ ] **Step 6:跑测试验证通过**

Run: `cd F:/soft/00selfmade/media-to-doc && uv run pytest tests/test_longdoc_integration.py -v`
Expected: 3 个 PASS

- [ ] **Step 7:验证 wheel 包含 snapshot**

Run:
```bash
cd F:/soft/00selfmade/media-to-doc && uv build
python -m zipfile -l dist/media_to_doc-*.whl | grep long_doc_skill | head -5
```
Expected: 看到 `media_to_doc/data/long_doc_skill/SKILL.md` 等文件在 wheel 内

- [ ] **Step 8:跑主仓全测试**

Run: `cd F:/soft/00selfmade/media-to-doc && uv run pytest -q`
Expected: 既有 + ≥3 新增,无回归

- [ ] **Step 9:Save state**

追加到 `task.md` §进度:
```
| T10 | longdoc.py 读 vendored + pyproject package-data | ✅ | 3/3 | 同上 |
```
**不 commit**。

---

## Task 11: Tauri `bundle.resources` 含 snapshot + Claude hook settings.json

**Files:**
- Modify: `F:/soft/00selfmade/media-to-doc-ui/src-tauri/tauri.conf.json`(`bundle.resources` 加 snapshot 路径)
- Modify: `C:/Users/Duanyi/.claude/settings.json`(加 `PostToolUse(Edit|Write)` hook,用 `update-config` Skill 优先)
- Read: 当前 `tauri.conf.json` `bundle` 段

**Interfaces:**
- Consumes:long-doc vendored snapshot(T9 产出)
- Produces:NSIS 自包含 + Claude hook 自动同步

- [ ] **Step 1:读 `tauri.conf.json` 完整内容**

Run: `Read("F:/soft/00selfmade/media-to-doc-ui/src-tauri/tauri.conf.json")`

- [ ] **Step 2:写「手动验收」checklist**

```
- [ ] cargo tauri build 后 NSIS installer 内含 long_doc_skill/SKILL.md
- [ ] 安装并启动 app 后,Python 跑 longdoc 能读到 vendored(已由 T10 测试覆盖)
- [ ] Claude Code 内 Edit ~/.claude/skills/long-doc-processor/SKILL.md → sync 脚本自动跑 → snapshot 更新
```

- [ ] **Step 3:`tauri.conf.json` `bundle.resources` 加 snapshot**

找到 `"bundle"` 段,加 `resources` 数组(若已存在则 append):

```json
"bundle": {
  ...existing,
  "resources": [
    "../../src/media_to_doc/data/long_doc_skill/**/*"
  ]
}
```

(具体看现有 schema;`resources` 可能是 `bundle.resources` 也可能在 `bundle.windows` 等;以 Tauri 2.x schema 为准)

- [ ] **Step 4:`cargo tauri build` 验证打包**

Run: `cd F:/soft/00selfmade/media-to-doc-ui/src-tauri && cargo tauri build 2>&1 | tail -20`
Expected: Finished + Bundling + 产物路径含 snapshot(用 `7z l <nsis.exe> | grep long_doc_skill` 验证)

- [ ] **Step 5:NSIS 内含 snapshot 验证**

Run:
```bash
"C:/Program Files/7-Zip/7z.exe" l "F:/soft/00selfmade/media-to-doc-ui/src-tauri/target/release/bundle/nsis/media-to-doc_1.4.2_x64-setup.exe" 2>&1 | grep -i long_doc_skill | head -5
```
Expected: 看到 `long_doc_skill/SKILL.md` 等文件

- [ ] **Step 6:Claude hook 优先用 `update-config` Skill**

调 `Skill(skill="update-config")`,描述:"在 `C:/Users/Duanyi/.claude/settings.json` 加一个 `PostToolUse(Edit|Write)` hook,触发命令 `python F:/soft/00selfmade/media-to-doc/scripts/sync_long_doc_skill.py`,匹配条件 file_path 以 `C:/Users/Duanyi/.claude/skills/long-doc-processor/` 开头"。

- [ ] **Step 7:若 `update-config` 报 schema 错,手编最小 JSON 增量**

读 `C:/Users/Duanyi/.claude/settings.json`,**保留**已有 `hooks` 段,在 `PostToolUse` 数组追加(若不存在则新建):

```json
{
  "matcher": "Edit|Write",
  "hooks": [
    {
      "type": "command",
      "command": "python F:/soft/00selfmade/media-to-doc/scripts/sync_long_doc_skill.py"
    }
  ]
}
```

保存前先 `Read` 整个文件,然后用 `Edit` 加 1 段,**不删不重排**已有 hooks。

- [ ] **Step 8:触发验证(手动)**

```bash
# 备份真身
cp "C:/Users/Duanyi/.claude/skills/long-doc-processor/SKILL.md" /tmp/SKILL.md.bak

# 改一行
echo "" >> "C:/Users/Duanyi/.claude/skills/long-doc-processor/SKILL.md"

# 等 3 秒,看 snapshot 是否更新
sleep 3
diff "C:/Users/Duanyi/.claude/skills/long-doc-processor/SKILL.md" "F:/soft/00selfmade/media-to-doc/src/media_to_doc/data/long_doc_skill/SKILL.md"
# Expected: 无 diff(sync 已跑过)

# 恢复
cp /tmp/SKILL.md.bak "C:/Users/Duanyi/.claude/skills/long-doc-processor/SKILL.md"
```

- [ ] **Step 9:Save state**

追加到 `task.md` §进度:
```
| T11 | Tauri bundle.resources + Claude hook | ✅ | 手动 verify | 同上 |
```
**不 commit**。

---

## Task 12: 全面验证 + 写 handoff + prompt-next

**Files:**
- Modify: `F:/soft/00selfmade/media-to-doc-ui/handoff-w15-a-t7-2-product-feedback-complete-2026-07-25.md`(新建 handoff)
- Modify: `F:/soft/00selfmade/media-to-doc-ui/prompt-w15-a-t7-2-next.md`(新建 prompt)
- Modify: `F:/soft/00selfmade/media-to-doc-ui/task.md`(标记 12 task 全 ✅ + T8 blocked)

**Interfaces:**
- Consumes:全部 11 task 产出
- Produces:handoff 文档 + next prompt + 全面测试报告

- [ ] **Step 1:跑 Rust 全测试**

Run: `cd F:/soft/00selfmade/media-to-doc-ui/src-tauri && cargo test --lib -q`
Expected: ≥113 passed(98 既有 + ≥15 新增)

- [ ] **Step 2:跑主仓全测试**

Run: `cd F:/soft/00selfmade/media-to-doc && uv run pytest -q`
Expected: ≥618 passed(604 既有 + ≥14 新增)

- [ ] **Step 3:`cargo tauri build` 重出 NSIS**

Run: `cd F:/soft/00selfmade/media-to-doc-ui/src-tauri && cargo tauri build 2>&1 | tail -20`
Expected: Finished + Bundling;warnings 5 → 仍 5

- [ ] **Step 4:记录产物信息**

```bash
ls -la "F:/soft/00selfmade/media-to-doc-ui/src-tauri/target/release/bundle/nsis/"
sha256sum "F:/soft/00selfmade/media-to-doc-ui/src-tauri/target/release/bundle/nsis/media-to-doc_1.4.2_x64-setup.exe"
```
记下文件名 + size + SHA256(写 handoff 用)

- [ ] **Step 5:沙箱真机验收(用户执行,本 task 提供步骤)**

如果用户尚未跑过 7 项新验收,写装机步骤到 handoff §6(沿用 W14-B+2 模板:`& "$env:LOCALAPPDATA\com.duanyi.mediatodoc\unins000.exe"` → 清缓存 → 装新 NSIS)。

**不替用户跑沙箱验证**(`mtd-verify.ps1` 是 T8 release 用;T7.2 不必)。

- [ ] **Step 6:写 handoff**

`handoff-w15-a-t7-2-product-feedback-complete-2026-07-25.md`,参考既有 handoff 模板,含:

```markdown
# Handoff — W15-A T7.2 第二轮产品反馈收口 COMPLETE

**日期**: 2026-07-25
**项目**: F:/soft/00selfmade/media-to-doc-ui
**当前分支**: feat/w15a-llm-api-settings(无 commit,加快模式)
**承接**: handoff-w15-a-task12-build-verify-2026-07-25.md §0.6 / §6
**基线**: 073b05e

## 0. 完成清单

12 task 全部 ✅:
- T1 主仓 CLI 3 flag + WorkflowConfig 3 字段
- T2 LLMConfig.from_profile_name + keyring 集成
- T3 task_text 落 state.json + chapter/draft prompt 注入
- T4 imagegen 策划 LLM + LocalSdxlProvider 最小实现
- T5 Tauri runner.rs build_mtd_*_args 加 3 参数
- T6 Tauri commands.rs per-run profile 注入
- T7 Tauri project registry 4 commands + dialog plugin
- T8 前端 New Run tab 动态 dropdowns + task textarea + 选目录
- T9 long-doc snapshot bootstrap + sync/verify 脚本
- T10 longdoc.py 读 vendored + pyproject package-data
- T11 Tauri bundle.resources + Claude hook
- T12 全面验证 + handoff

## 1. Build 产物

- NSIS: src-tauri/target/release/bundle/nsis/media-to-doc_1.4.2_x64-setup.exe
- 大小: <N> bytes
- SHA256: <hash>

## 2. 测试统计

- Rust: <N> passed (98 + ≥15)
- 主仓 pytest: <N> passed (604 + ≥14)

## 3. 7 项新验收(用户执行)

1. New Run tab 打开 → LLM 下拉列 MiniMax 等 profile ✓
2. Image Agent 折叠面板 → 策划 LLM + 出图 provider 两独立下拉 ✓
3. Stop after 各阶段 tooltip 中文 ✓
4. task textarea 输入 → state.json.task_text 落 ✓
5. 「选目录」按钮 → 原生 picker → 左侧立即出现 ✓
6. 同路径 add 重复 → 合并 sessions,不重复 ✓
7. long-doc Skill 改一行 → sync 自动跑过 → snapshot 更新 ✓

## 4. P1 长文档整合

- snapshot 26 文件 + MANIFEST
- sync/verify 脚本 5 测试
- Tauri bundle.resources 含 snapshot(NSIS 自包含)
- Claude hook settings.json PostToolUse 触发 sync

## 5. 设计红线遵守

- ✅ API key 走 keyring,不进 HTML/log/CLI
- ✅ 项目 ID 用规范化路径,不是 display name
- ✅ NSIS 自包含,不依赖用户 ~/.claude
- ✅ Image Agent 两层独立,不把文本模型当图片模型
- ✅ Tasks 1-11 未提交工作区保留,加快模式

## 6. 下一步:T8 release(继续 blocked)

- feature commit + bump v1.5.0
- 强清装机 + sandbox-verify mtd-verify.ps1
- reviewer + 等用户拍板 merge/release
```

- [ ] **Step 7:写 prompt-next**

`prompt-w15-a-t7-2-next.md`(≤30 行):

```markdown
承接:`handoff-w15-a-t7-2-product-feedback-complete-2026-07-25.md`
分支:`feat/w15a-llm-api-settings`(加快模式)
任务:T8 release session 接力

必交付:
- feature commit + bump v1.5.0
- 强清装机 + sandbox-verify (mtd-verify.ps1)
- cargo test --lib ≥113 / uv run pytest ≥618
- reviewer 2 轮通过
- 等用户拍板 merge / GitHub release

禁止:
- 不 reset / checkout / restore 覆盖 T1-T11 工作区
- 不动主仓 / 子仓无关改动
- 不启用定时调度(W15-B+ 再做)

加速模式:W15-A 整体一次 commit,feature commit 在 T8 才做。
```

- [ ] **Step 8:`task.md` 标记 12 task 全 ✅ + T8 blocked**

追加到 §进度:
```
| T8 | v1.5.0 release | blocked: T7.2 + 新验收通过后才开始 | - | - |
```

T1-T12 状态全部 ✅。

- [ ] **Step 9:Save state**

**不 commit**。提示用户:"T7.2 全部完工,T8 release session 接力。详见 handoff + prompt-next。"

---

## Self-Review Checklist

(本节是写作时自查清单,不是 task;实际执行时按 checklist 跑一次)

- [ ] **Spec 覆盖**:spec §2(P0-A)→ T1+T2+T5+T6;spec §3(P0-B)→ T3+T7+T8;spec §4(P0-C)→ T4;spec §5(P1)→ T9+T10+T11;spec §6(Stop after)→ T8(集成在 form label);spec §7(测试矩阵)→ T12 Step 1-3;spec §8(风险)→ 各 task 设计已规避
- [ ] **Placeholder 扫描**:无 TBD / TODO / "add appropriate error handling";所有代码段都有实际内容
- [ ] **Type 一致性**:`build_mtd_run_args` 9 参数顺序在 T5 改、T6 调用、T8 form submit 一致;`inject_profile_env` 签名在 T6 定义、T6 Step 4 测试用;`add_project` / `list_projects` 在 T7 定义、T8 调用
- [ ] **2 轮 review 标记**:T6(commands.rs) + T7(commands.rs + capabilities) 已写明 2 轮 review 要求
- [ ] **加快模式遵守**:所有 task Step 末尾"Save state"均"不 commit"
- [ ] **不修改无关文件**:Tasks 1-11 工作区累积保留,每个 task 只追加 / 新建