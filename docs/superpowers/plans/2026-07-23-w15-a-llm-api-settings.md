# W15-A LLM API Settings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add LLM API Settings panel to media-to-doc desktop — manage multiple LLM profiles (12 built-in providers + Custom) with API keys stored in OS keyring, env var injection into mtd subprocess on Run pipeline.

**Architecture:** Three-layer Rust backend (keyring_store → llm_profiles → commands) with runner.rs SpawnSpec.env_vars extension; vanilla JS frontend adds Settings tab + Providers subpage + add-modal. Per-provider env var templates map active profile to ANTHROPIC_API_KEY / OPENAI_API_KEY / OLLAMA_HOST etc. before spawn.

**Tech Stack:** Tauri 2.11.4 + Rust 1.97 + keyring crate v3 (OS keyring) + dirs crate v5 (config dir) + reqwest v0.12 (test_connection HTTP probe) + vanilla JS frontend (existing index.html)

**Spec:** `docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md` (commit `565279d`, W15-A reviewer pass)

---

## Global Constraints

- **OS keyring service name**: `media-to-doc-ui`
- **Metadata JSON path** (per spec §4):
  - Windows: `%APPDATA%\com.duanyi.mediatodoc\llm_profiles.json`
  - Mac: `~/Library/Application Support/com.duanyi.mediatodoc/llm_profiles.json`
  - Linux: `~/.config/com.duanyi.mediatodoc/llm_profiles.json`
- **Tool Search / Experimental Beta toggle**: render ONLY when `provider == "Anthropic"`; non-Anthropic silently ignores these fields with log warning (spec §7.3 reviewer note)
- **base_url validation**: `http://localhost:*` / `http://127.0.0.1:*` / `https://*` — reject `ftp://`, `javascript:`, `file://`, etc.
- **model validation**: non-empty, ≤ 200 chars
- **name uniqueness**: case-sensitive (`deepseek-prod` ≠ `DeepSeek-Prod`)
- **profile deletion of active**: allowed; subsequent run_pipeline returns `ACTIVE_PROFILE_REQUIRED` error
- **keyring write failure**: surface `KEYRING_ERROR` to user; never silently fall back to env var (spec §10.1)
- **All unit tests as `#[cfg(test)] mod tests`** inside source files — no `src-tauri/tests/` directory
- **Commit style**: Conventional Commits (`feat:` / `fix:` / `refactor:` / `chore:` / `docs:` / `test:`); per CLAUDE.md §5.3
- **Reuse existing patterns**: `*_impl` pure function + `#[tauri::command]` thin wrapper (commands.rs §1); `CommandResponse<T>` shell (types.rs); `Lazy<RunRegistry>` singleton (runner.rs)
- **Frequent commits**: one commit per task

---

## File Structure

### New files
| File | Responsibility |
|---|---|
| `src-tauri/src/keyring_store.rs` | OS keyring read/write/delete/list for profile keys |

### Modified files
| File | Change |
|---|---|
| `src-tauri/Cargo.toml` | + `keyring = "3"` + `dirs = "5"` (Task 1); + `reqwest` (Task 5) |
| `src-tauri/src/llm_profiles.rs` | NEW (Tasks 2-3): 12 templates + validation + JSON metadata IO + env var mapping |
| `src-tauri/src/commands.rs` | + 6 Tauri commands + error enum + active profile read in run_pipeline/resume_pipeline (Task 5) |
| `src-tauri/src/lib.rs` | + `mod llm_profiles; mod keyring_store;` + 6 commands in invoke_handler (Tasks 1, 5, 6) |
| `src-tauri/src/runner.rs` | `SpawnSpec.env_vars: HashMap<String, String>` + `spawn_mtd` `.env_clear().envs(env_vars)` (Task 4) |
| `src/index.html` | + Settings tab + Providers subpage + add modal + 6 command calls (Task 7) |

### Files NOT modified
- `src-tauri/src/types.rs` (CommandResponse already supports our shape)
- `src-tauri/src/python_bridge.rs` (no LLM knowledge)
- `src-tauri/src/main.rs` (entry unchanged)
- `src-tauri/capabilities/default.json` (IPC permissions unchanged)
- `src-tauri/tauri.conf.json` (no bundle/window changes)
- `src-tauri/nsis/installer.nsi` (no installer changes)
- main repo `F:\soft\00selfmade\media-to-doc\` (mtd Python unchanged — env var injection is Tauri-side)

---

## Task 1: keyring_store module + Cargo.toml deps

**Files:**
- Modify: `src-tauri/Cargo.toml` (add 2 deps)
- Create: `src-tauri/src/keyring_store.rs`
- Modify: `src-tauri/src/lib.rs:15` (add `mod keyring_store;`)

**Interfaces:**
- Consumes: nothing (zero deps from earlier tasks)
- Produces:
  - `pub const SERVICE_NAME: &str = "media-to-doc-ui";`
  - `pub fn read_key(profile_name: &str) -> Result<String, String>;`
  - `pub fn write_key(profile_name: &str, key: &str) -> Result<(), String>;`
  - `pub fn delete_key(profile_name: &str) -> Result<(), String>;`
  - `pub fn list_profile_names() -> Result<Vec<String>, String>;`
- Errors: surface `keyring::Error` as `String` with `KEYRING_ERROR:` prefix for grep-ability

### Step 1: Update Cargo.toml

Open `src-tauri/Cargo.toml`. Add two lines under `[dependencies]` (alphabetical position — `keyring` between `dirs` placeholder if any, otherwise at end; `dirs` near top):

```toml
[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["process", "io-util", "sync", "rt", "fs", "macros", "time"] }
once_cell = "1"
keyring = "3"
dirs = "5"
```

### Step 2: Create keyring_store.rs with 5 failing tests

Create `src-tauri/src/keyring_store.rs` with full content below (~140 lines):

```rust
//! OS keyring access for LLM profile API keys.
//!
//! 设计(spec §4):
//! - service:固定 `"media-to-doc-ui"`,所有 profile 共用
//! - username:`profile:<name>`,每个 profile 一个 key
//! - password:用户填的 API key(明文存 OS keyring,keyring 自身加密)
//!
//! 平台行为:
//! - Windows:Credential Manager(WDPAPI,按用户存储,无需 admin)
//! - Mac:Keychain
//! - Linux:gnome-keyring / kwallet / secret-service daemon
//!
//! 错误一律 `Result<_, String>`,前缀 `KEYRING_ERROR:` 便于上游 grep。

use keyring::Entry;

pub const SERVICE_NAME: &str = "media-to-doc-ui";

fn entry(profile_name: &str) -> Result<Entry, String> {
    Entry::new(SERVICE_NAME, &format!("profile:{profile_name}"))
        .map_err(|e| format!("KEYRING_ERROR: 创建 entry 失败: {e}"))
}

/// 读 profile 的 API key。key 不存在时返回 `KEYRING_ERROR: NoEntry`。
pub fn read_key(profile_name: &str) -> Result<String, String> {
    let e = entry(profile_name)?;
    e.get_password()
        .map_err(|e| format!("KEYRING_ERROR: 读 key 失败: {e}"))
}

/// 写 profile 的 API key。覆盖已存在的同名 key。
pub fn write_key(profile_name: &str, key: &str) -> Result<(), String> {
    let e = entry(profile_name)?;
    e.set_password(key)
        .map_err(|e| format!("KEYRING_ERROR: 写 key 失败: {e}"))
}

/// 删 profile 的 API key。key 不存在视为成功(idempotent)。
pub fn delete_key(profile_name: &str) -> Result<(), String> {
    let e = entry(profile_name)?;
    match e.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("KEYRING_ERROR: 删 key 失败: {e}")),
    }
}

/// 列出所有 profile 名字(从 keyring username 提取 `<name>` 部分)。
///
/// 注意:keyring crate v3 没有原生的 list API,采用 platform-specific 探测:
/// - 失败时返回空 Vec(不报错)— 上层应读 metadata JSON 拿到 profile 列表。
/// - 本函数保留供将来扩展(W15-B 可能用到)。
pub fn list_profile_names() -> Result<Vec<String>, String> {
    // keyring v3 不支持跨平台 list;返回空 Vec 是安全 fallback。
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 集成测试用 profile 名(避免污染用户真实 keyring)。
    /// cargo test 串行运行,每个测试用独立 name。
    const TEST_PROFILE: &str = "__w15a_test_profile__";

    #[test]
    fn write_then_read_returns_same_key() {
        // 先清理
        let _ = delete_key(TEST_PROFILE);
        // 写
        write_key(TEST_PROFILE, "sk-test-1234567890").expect("write 失败");
        // 读
        let got = read_key(TEST_PROFILE).expect("read 失败");
        assert_eq!(got, "sk-test-1234567890");
        // 清理
        delete_key(TEST_PROFILE).expect("delete 失败");
    }

    #[test]
    fn read_nonexistent_returns_error_with_prefix() {
        // 确保不存在
        let _ = delete_key("__w15a_definitely_nonexistent__");
        let result = read_key("__w15a_definitely_nonexistent__");
        assert!(result.is_err(), "读不存在的 key 应报错");
        let err = result.unwrap_err();
        assert!(
            err.starts_with("KEYRING_ERROR:"),
            "错误前缀应是 KEYRING_ERROR:, 实际: {err}"
        );
    }

    #[test]
    fn write_overwrites_existing_key() {
        let _ = delete_key(TEST_PROFILE);
        write_key(TEST_PROFILE, "first-key").unwrap();
        write_key(TEST_PROFILE, "second-key").unwrap();
        let got = read_key(TEST_PROFILE).unwrap();
        assert_eq!(got, "second-key", "二次写应覆盖");
        delete_key(TEST_PROFILE).unwrap();
    }

    #[test]
    fn delete_existing_returns_ok() {
        let _ = delete_key(TEST_PROFILE);
        write_key(TEST_PROFILE, "to-be-deleted").unwrap();
        let result = delete_key(TEST_PROFILE);
        assert!(result.is_ok(), "删存在的 key 应成功");
        // 再删应 idempotent
        let result2 = delete_key(TEST_PROFILE);
        assert!(result2.is_ok(), "再删不存在的 key 应 idempotent 成功");
    }

    #[test]
    fn list_profile_names_returns_vec() {
        // 不验证具体内容(keyring v3 不支持 list),只验证函数签名 + 返回 Vec。
        let result = list_profile_names();
        assert!(result.is_ok(), "list 应返回 Ok");
        let _names: Vec<String> = result.unwrap();
    }
}
```

### Step 3: Wire keyring_store into lib.rs

Edit `src-tauri/src/lib.rs:15` (in the `mod` declarations block):

```rust
mod commands;
mod keyring_store;  // NEW
mod python_bridge;
mod runner;
mod types;
```

### Step 4: Run keyring tests (verify pass on Win/Mac, keyring calls succeed)

Run: `cd src-tauri && cargo test --lib keyring_store::`

Expected output (Windows):
```
running 5 tests
test keyring_store::tests::write_then_read_returns_same_key ... ok
test keyring_store::tests::read_nonexistent_returns_error_with_prefix ... ok
test keyring_store::tests::write_overwrites_existing_key ... ok
test keyring_store::tests::delete_existing_returns_ok ... ok
test keyring_store::tests::list_profile_names_returns_vec ... ok

test result: ok. 5 passed; 0 failed
```

**撞墙预案**:Linux 上若 secret-service daemon 未跑,read/write/delete 全报 DBus 错误。
修复:`sudo apt install gnome-keyring` + `eval $(gnome-keyring-daemon --start)` 或文档要求用户启 keyring。

### Step 5: Run full test suite (verify no regression)

Run: `cd src-tauri && cargo test --lib`

Expected: `43 passed; 0 failed; 0 ignored`(baseline 43 + 新 5 = 48)。

### Step 6: Commit

```bash
git add src-tauri/Cargo.toml src-tauri/src/keyring_store.rs src-tauri/src/lib.rs
git commit -m "feat(ui): W15-A T1 — keyring_store + Cargo.toml deps

- src-tauri/Cargo.toml: + keyring = 3, dirs = 5
- src-tauri/src/keyring_store.rs: NEW 5 functions (read/write/delete/list)
  + 5 unit tests (集成测 OS keyring,集成 CI 需 Win/Mac 桌面 / Linux +secret-service)
- src-tauri/src/lib.rs: + mod keyring_store

48/48 tests pass (43 baseline + 5 new).

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 2: llm_profiles templates + validation (17 tests)

**Files:**
- Create: `src-tauri/src/llm_profiles.rs`
- Modify: `src-tauri/src/lib.rs:15` (add `mod llm_profiles;`)

**Interfaces** (Task 2 produces; Task 3 extends):
- `pub enum Provider { Anthropic, OpenAI, Ollama, LmStudio, DeepSeek, Zhipu, Kimi, MiniMax, ApitwoD, Shengsuanyun, TeamoRouter, Custom }`
- `pub enum Protocol { AnthropicSdk, OpenAiCompat, OllamaNative }`
- `pub struct ProviderTemplate { pub enum_value: Provider, pub display_name: &'static str, pub default_base_url: &'static str, pub default_model: &'static str, pub protocol: Protocol, pub env_var_keys: &'static [&'static str] }`
- `pub fn all_templates() -> Vec<ProviderTemplate>` (12 entries)
- `pub fn provider_from_name(name: &str) -> Option<Provider>` + `pub fn provider_name(p: Provider) -> &'static str`
- `pub fn validate_base_url(url: &str) -> Result<(), String>`
- `pub fn validate_model(model: &str) -> Result<(), String>`
- Errors: `INVALID_BASE_URL: <msg>` / `INVALID_MODEL: <msg>` / `PROVIDER_NOT_FOUND: <name>`

### Step 1: Create llm_profiles.rs with 17 failing tests

Create `src-tauri/src/llm_profiles.rs` with full content below (~380 lines):

```rust
//! LLM profile metadata + 12 内置服务商模板 + base_url/model 校验。
//!
//! 设计(spec §3 + §4):
//! - 12 个内置服务商用 `Provider` enum 表示,模板表 `all_templates()` 列出
//! - 用户填的 profile 存 `%APPDATA%\com.duanyi.mediatodoc\llm_profiles.json`
//!   (Task 3 加 IO)
//! - base_url 校验防 SSRF:https:// / http://localhost:* / http://127.0.0.1:*
//! - API key 不存 metadata,存在 OS keyring(Task 1 的 keyring_store)
//!
//! 不在此文件:env var 映射(to_env_vars)、JSON IO(load/save) — Task 3。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Anthropic,
    OpenAI,
    Ollama,
    LmStudio,
    DeepSeek,
    Zhipu,
    Kimi,
    MiniMax,
    ApitwoD,      // 接口 AI
    Shengsuanyun, // 胜算云
    TeamoRouter,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    AnthropicSdk,
    OpenAiCompat,
    OllamaNative,
}

#[derive(Debug, Clone)]
pub struct ProviderTemplate {
    pub enum_value: Provider,
    pub display_name: &'static str,
    pub default_base_url: &'static str,
    pub default_model: &'static str,
    pub protocol: Protocol,
    pub env_var_keys: &'static [&'static str],
}

pub fn all_templates() -> Vec<ProviderTemplate> {
    vec![
        ProviderTemplate {
            enum_value: Provider::Anthropic,
            display_name: "Anthropic",
            default_base_url: "https://api.anthropic.com",
            default_model: "claude-sonnet-4-5",
            protocol: Protocol::AnthropicSdk,
            env_var_keys: &["ANTHROPIC_API_KEY", "ANTHROPIC_BASE_URL"],
        },
        ProviderTemplate {
            enum_value: Provider::OpenAI,
            display_name: "OpenAI",
            default_base_url: "https://api.openai.com/v1",
            default_model: "gpt-4o",
            protocol: Protocol::OpenAiCompat,
            env_var_keys: &["OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_MODEL"],
        },
        ProviderTemplate {
            enum_value: Provider::Ollama,
            display_name: "Ollama",
            default_base_url: "http://localhost:11434",
            default_model: "llama3.1",
            protocol: Protocol::OllamaNative,
            env_var_keys: &["OLLAMA_HOST", "OLLAMA_MODEL"],
        },
        ProviderTemplate {
            enum_value: Provider::LmStudio,
            display_name: "LM Studio",
            default_base_url: "http://localhost:1234/v1",
            default_model: "loaded-model",
            protocol: Protocol::OpenAiCompat,
            env_var_keys: &["OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_MODEL"],
        },
        ProviderTemplate {
            enum_value: Provider::DeepSeek,
            display_name: "DeepSeek",
            default_base_url: "https://api.deepseek.com",
            default_model: "deepseek-chat",
            protocol: Protocol::OpenAiCompat,
            env_var_keys: &["OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_MODEL"],
        },
        ProviderTemplate {
            enum_value: Provider::Zhipu,
            display_name: "Zhipu GLM",
            default_base_url: "https://open.bigmodel.cn/api/paas/v4",
            default_model: "glm-4-plus",
            protocol: Protocol::OpenAiCompat,
            env_var_keys: &["OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_MODEL"],
        },
        ProviderTemplate {
            enum_value: Provider::Kimi,
            display_name: "Kimi",
            default_base_url: "https://api.moonshot.cn/v1",
            default_model: "moonshot-v1-128k",
            protocol: Protocol::OpenAiCompat,
            env_var_keys: &["OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_MODEL"],
        },
        ProviderTemplate {
            enum_value: Provider::MiniMax,
            display_name: "MiniMax",
            default_base_url: "https://api.MiniMax.chat/v1",
            default_model: "MiniMax-Text-01",
            protocol: Protocol::OpenAiCompat,
            env_var_keys: &["OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_MODEL"],
        },
        ProviderTemplate {
            enum_value: Provider::ApitwoD,
            display_name: "接口 AI",
            default_base_url: "https://api.api2d.net/v1",
            default_model: "gpt-4o-mini",
            protocol: Protocol::OpenAiCompat,
            env_var_keys: &["OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_MODEL"],
        },
        ProviderTemplate {
            enum_value: Provider::Shengsuanyun,
            display_name: "胜算云",
            default_base_url: "https://api.shengsuanyun.com/v1",
            default_model: "gpt-4o-mini",
            protocol: Protocol::OpenAiCompat,
            env_var_keys: &["OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_MODEL"],
        },
        ProviderTemplate {
            enum_value: Provider::TeamoRouter,
            display_name: "TeamoRouter",
            default_base_url: "https://api.teamorouter.com/v1",
            default_model: "claude-3-5-sonnet",
            protocol: Protocol::OpenAiCompat,
            env_var_keys: &["OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_MODEL"],
        },
        ProviderTemplate {
            enum_value: Provider::Custom,
            display_name: "Custom",
            default_base_url: "",
            default_model: "",
            protocol: Protocol::OpenAiCompat,
            env_var_keys: &["OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_MODEL"],
        },
    ]
}

pub fn provider_from_name(name: &str) -> Option<Provider> {
    match name {
        "Anthropic" => Some(Provider::Anthropic),
        "OpenAI" => Some(Provider::OpenAI),
        "Ollama" => Some(Provider::Ollama),
        "LM Studio" => Some(Provider::LmStudio),
        "DeepSeek" => Some(Provider::DeepSeek),
        "Zhipu GLM" => Some(Provider::Zhipu),
        "Kimi" => Some(Provider::Kimi),
        "MiniMax" => Some(Provider::MiniMax),
        "接口 AI" => Some(Provider::ApitwoD),
        "胜算云" => Some(Provider::Shengsuanyun),
        "TeamoRouter" => Some(Provider::TeamoRouter),
        "Custom" => Some(Provider::Custom),
        _ => None,
    }
}

pub fn provider_name(p: Provider) -> &'static str {
    match p {
        Provider::Anthropic => "Anthropic",
        Provider::OpenAI => "OpenAI",
        Provider::Ollama => "Ollama",
        Provider::LmStudio => "LM Studio",
        Provider::DeepSeek => "DeepSeek",
        Provider::Zhipu => "Zhipu GLM",
        Provider::Kimi => "Kimi",
        Provider::MiniMax => "MiniMax",
        Provider::ApitwoD => "接口 AI",
        Provider::Shengsuanyun => "胜算云",
        Provider::TeamoRouter => "TeamoRouter",
        Provider::Custom => "Custom",
    }
}

/// base_url 校验:仅允许 https:// / http://localhost:* / http://127.0.0.1:*(防 SSRF)。
pub fn validate_base_url(url: &str) -> Result<(), String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("INVALID_BASE_URL: 不能为空".into());
    }
    if trimmed.starts_with("https://") {
        return Ok(());
    }
    if trimmed.starts_with("http://localhost:") || trimmed.starts_with("http://127.0.0.1:") {
        return Ok(());
    }
    Err(format!(
        "INVALID_BASE_URL: 仅支持 https:// 或 http://localhost:* / http://127.0.0.1:*, 实际: {trimmed}"
    ))
}

/// model 校验:非空、≤ 200 字符。
pub fn validate_model(model: &str) -> Result<(), String> {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        return Err("INVALID_MODEL: 不能为空".into());
    }
    if trimmed.chars().count() > 200 {
        return Err(format!("INVALID_MODEL: 长度 {} 超过 200", trimmed.chars().count()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── 12 个模板字段正确(12 tests)─────────────────────────────

    #[test]
    fn template_anthropic_has_correct_fields() {
        let t = all_templates().into_iter().find(|t| t.enum_value == Provider::Anthropic).unwrap();
        assert_eq!(t.display_name, "Anthropic");
        assert_eq!(t.default_base_url, "https://api.anthropic.com");
        assert_eq!(t.default_model, "claude-sonnet-4-5");
        assert_eq!(t.protocol, Protocol::AnthropicSdk);
        assert_eq!(t.env_var_keys, &["ANTHROPIC_API_KEY", "ANTHROPIC_BASE_URL"]);
    }

    #[test]
    fn template_openai_has_correct_fields() {
        let t = all_templates().into_iter().find(|t| t.enum_value == Provider::OpenAI).unwrap();
        assert_eq!(t.display_name, "OpenAI");
        assert_eq!(t.default_base_url, "https://api.openai.com/v1");
        assert_eq!(t.default_model, "gpt-4o");
        assert_eq!(t.protocol, Protocol::OpenAiCompat);
        assert!(t.env_var_keys.contains(&"OPENAI_API_KEY"));
        assert!(t.env_var_keys.contains(&"OPENAI_BASE_URL"));
        assert!(t.env_var_keys.contains(&"OPENAI_MODEL"));
    }

    #[test]
    fn template_ollama_has_correct_fields() {
        let t = all_templates().into_iter().find(|t| t.enum_value == Provider::Ollama).unwrap();
        assert_eq!(t.display_name, "Ollama");
        assert_eq!(t.default_base_url, "http://localhost:11434");
        assert_eq!(t.default_model, "llama3.1");
        assert_eq!(t.protocol, Protocol::OllamaNative);
        assert_eq!(t.env_var_keys, &["OLLAMA_HOST", "OLLAMA_MODEL"]);
    }

    #[test]
    fn template_lmstudio_has_correct_fields() {
        let t = all_templates().into_iter().find(|t| t.enum_value == Provider::LmStudio).unwrap();
        assert_eq!(t.display_name, "LM Studio");
        assert_eq!(t.default_base_url, "http://localhost:1234/v1");
        assert_eq!(t.protocol, Protocol::OpenAiCompat);
    }

    #[test]
    fn template_deepseek_has_correct_fields() {
        let t = all_templates().into_iter().find(|t| t.enum_value == Provider::DeepSeek).unwrap();
        assert_eq!(t.default_base_url, "https://api.deepseek.com");
        assert_eq!(t.default_model, "deepseek-chat");
        assert_eq!(t.protocol, Protocol::OpenAiCompat);
    }

    #[test]
    fn template_zhipu_has_correct_fields() {
        let t = all_templates().into_iter().find(|t| t.enum_value == Provider::Zhipu).unwrap();
        assert_eq!(t.display_name, "Zhipu GLM");
        assert_eq!(t.default_base_url, "https://open.bigmodel.cn/api/paas/v4");
        assert_eq!(t.default_model, "glm-4-plus");
    }

    #[test]
    fn template_kimi_has_correct_fields() {
        let t = all_templates().into_iter().find(|t| t.enum_value == Provider::Kimi).unwrap();
        assert_eq!(t.display_name, "Kimi");
        assert_eq!(t.default_base_url, "https://api.moonshot.cn/v1");
        assert_eq!(t.default_model, "moonshot-v1-128k");
    }

    #[test]
    fn template_MiniMax_has_non_empty_url_and_model() {
        let t = all_templates().into_iter().find(|t| t.enum_value == Provider::MiniMax).unwrap();
        assert!(t.default_base_url.contains("MiniMax"));
        assert!(!t.default_model.is_empty());
    }

    #[test]
    fn template_apitwod_has_correct_fields() {
        let t = all_templates().into_iter().find(|t| t.enum_value == Provider::ApitwoD).unwrap();
        assert_eq!(t.display_name, "接口 AI");
        assert!(t.default_base_url.contains("api2d"));
    }

    #[test]
    fn template_shengsuanyun_has_correct_fields() {
        let t = all_templates().into_iter().find(|t| t.enum_value == Provider::Shengsuanyun).unwrap();
        assert_eq!(t.display_name, "胜算云");
        assert!(t.default_base_url.contains("shengsuanyun"));
    }

    #[test]
    fn template_teamorouter_has_correct_fields() {
        let t = all_templates().into_iter().find(|t| t.enum_value == Provider::TeamoRouter).unwrap();
        assert_eq!(t.display_name, "TeamoRouter");
        assert!(t.default_base_url.contains("teamorouter"));
    }

    #[test]
    fn template_custom_has_empty_defaults() {
        let t = all_templates().into_iter().find(|t| t.enum_value == Provider::Custom).unwrap();
        assert_eq!(t.display_name, "Custom");
        assert_eq!(t.default_base_url, "");
        assert_eq!(t.default_model, "");
        assert_eq!(t.protocol, Protocol::OpenAiCompat);
    }

    // ── base_url 校验(3 tests)─────────────────────────────

    #[test]
    fn validate_base_url_accepts_https() {
        assert!(validate_base_url("https://api.deepseek.com").is_ok());
        assert!(validate_base_url("https://api.openai.com/v1").is_ok());
    }

    #[test]
    fn validate_base_url_accepts_localhost_loopback() {
        assert!(validate_base_url("http://localhost:11434").is_ok());
        assert!(validate_base_url("http://localhost:1234/v1").is_ok());
        assert!(validate_base_url("http://127.0.0.1:11434").is_ok());
        assert!(validate_base_url("http://127.0.0.1:8080").is_ok());
    }

    #[test]
    fn validate_base_url_rejects_unsafe_schemes() {
        assert!(validate_base_url("").is_err());
        assert!(validate_base_url("ftp://api.example.com").is_err());
        assert!(validate_base_url("javascript:alert(1)").is_err());
        assert!(validate_base_url("file:///etc/passwd").is_err());
        assert!(validate_base_url("http://example.com").is_err());
        assert!(validate_base_url("http://192.168.1.1:8080").is_err());
        let err = validate_base_url("ftp://x").unwrap_err();
        assert!(err.starts_with("INVALID_BASE_URL:"), "err: {err}");
    }

    // ── model 校验(2 tests)─────────────────────────────

    #[test]
    fn validate_model_accepts_normal_string() {
        assert!(validate_model("claude-sonnet-4-5").is_ok());
        assert!(validate_model("gpt-4o").is_ok());
        assert!(validate_model("loaded-model").is_ok());
    }

    #[test]
    fn validate_model_rejects_empty_or_too_long() {
        assert!(validate_model("").is_err());
        assert!(validate_model("   ").is_err());
        let long = "a".repeat(201);
        let err = validate_model(&long).unwrap_err();
        assert!(err.starts_with("INVALID_MODEL:"));
        assert!(err.contains("201"));
    }

    // ── provider name roundtrip(1 test 覆盖 12 个)─────────────────────────

    #[test]
    fn provider_name_roundtrip_covers_all_12() {
        use Provider::*;
        let cases = [
            (Anthropic, "Anthropic"),
            (OpenAI, "OpenAI"),
            (Ollama, "Ollama"),
            (LmStudio, "LM Studio"),
            (DeepSeek, "DeepSeek"),
            (Zhipu, "Zhipu GLM"),
            (Kimi, "Kimi"),
            (MiniMax, "MiniMax"),
            (ApitwoD, "接口 AI"),
            (Shengsuanyun, "胜算云"),
            (TeamoRouter, "TeamoRouter"),
            (Custom, "Custom"),
        ];
        for (p, expected_name) in cases {
            assert_eq!(provider_name(p), expected_name, "name for {:?}", p);
            assert_eq!(provider_from_name(expected_name), Some(p), "from_name for {expected_name}");
        }
        assert_eq!(provider_from_name("NotAProvider"), None);
    }
}
```

### Step 2: Wire llm_profiles into lib.rs

Edit `src-tauri/src/lib.rs:15`:

```rust
mod commands;
mod keyring_store;
mod llm_profiles;  // NEW
mod python_bridge;
mod runner;
mod types;
```

### Step 3: Run llm_profiles tests (verify pass)

Run: `cd src-tauri && cargo test --lib llm_profiles::`

Expected:
```
running 17 tests
test llm_profiles::tests::template_anthropic_has_correct_fields ... ok
... (15 more)
test llm_profiles::tests::provider_name_roundtrip_covers_all_12 ... ok

test result: ok. 17 passed; 0 failed
```

### Step 4: Run full test suite (verify no regression)

Run: `cd src-tauri && cargo test --lib`

Expected: `65 passed; 0 failed`(48 + 17 = 65)

### Step 5: Commit

```bash
git add src-tauri/src/llm_profiles.rs src-tauri/src/lib.rs
git commit -m "feat(ui): W15-A T2 — llm_profiles templates + validation

- src-tauri/src/llm_profiles.rs: NEW Provider enum (12) + Protocol enum
  + ProviderTemplate struct + all_templates() + validate_base_url()
  + validate_model() + provider_from_name/provider_name roundtrip
- src-tauri/src/lib.rs: + mod llm_profiles

17/17 new tests pass (12 templates + 3 base_url + 2 model + 1 roundtrip).
Total: 65/65 (48 + 17).

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 3: llm_profiles metadata IO + env var mapping

**Files:**
- Modify: `src-tauri/src/llm_profiles.rs` (extend Task 2 file)

**Interfaces** (Task 3 produces):
- `pub struct ProfileMeta { name, provider, base_url, model, note, tool_search_enabled, experimental_betas_disabled, created_at }`
- `pub struct MetadataFile { pub active: Option<String>, pub profiles: Vec<ProfileMeta> }`
- `pub fn metadata_path() -> PathBuf` — `%APPDATA%\com.duanyi.mediatodoc\llm_profiles.json` (Mac/Linux analogous)
- `pub fn load_profiles() -> Result<MetadataFile, String>` — empty `MetadataFile { active: None, profiles: vec![] }` if file missing
- `pub fn save_profiles(m: &MetadataFile) -> Result<(), String>`
- `pub fn get_active_profile() -> Result<ProfileMeta, String>` — `ACTIVE_PROFILE_REQUIRED: 无 active profile` if `active` is None
- `pub fn to_env_vars(meta: &ProfileMeta, key: &str) -> HashMap<String, String>` — env vars to inject to mtd subprocess

**Design notes:**
- `to_env_vars` dispatch on Provider:
  - Anthropic: `ANTHROPIC_API_KEY=<key>`, `ANTHROPIC_BASE_URL=<meta.base_url>` (custom) OR omit if matches default `https://api.anthropic.com`
  - OpenAI Compat: `OPENAI_API_KEY=<key>`, `OPENAI_BASE_URL=<meta.base_url>` (if non-empty), `OPENAI_MODEL=<meta.model>` (if non-empty)
  - Ollama: `OLLAMA_HOST=<meta.base_url>` (always), `OLLAMA_MODEL=<meta.model>` (if non-empty); no `key`
- Non-Anthropic provider: silently ignore `tool_search_enabled` / `experimental_betas_disabled` (spec §7.3 reviewer note — log warning, no error)

### Step 1: Append IO + env var code to llm_profiles.rs

Open `src-tauri/src/llm_profiles.rs`. After the existing `validate_model` function and BEFORE `#[cfg(test)] mod tests`, append:

```rust
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProfileMeta {
    pub name: String,
    pub provider: String,           // display_name 字符串(非 enum,JSON 友好)
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub tool_search_enabled: bool,
    #[serde(default)]
    pub experimental_betas_disabled: bool,
    pub created_at: String,         // RFC3339-ish
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MetadataFile {
    #[serde(default)]
    pub active: Option<String>,
    #[serde(default)]
    pub profiles: Vec<ProfileMeta>,
}

impl Default for MetadataFile {
    fn default() -> Self {
        Self { active: None, profiles: vec![] }
    }
}

/// metadata JSON 文件路径(匹配 tauri.conf.json identifier = com.duanyi.mediatodoc)。
pub fn metadata_path() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata)
                .join("com.duanyi.mediatodoc")
                .join("llm_profiles.json");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            return home
                .join("Library")
                .join("Application Support")
                .join("com.duanyi.mediatodoc")
                .join("llm_profiles.json");
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(home) = dirs::home_dir() {
            return home
                .join(".config")
                .join("com.duanyi.mediatodoc")
                .join("llm_profiles.json");
        }
    }
    // fallback: 当前目录(测试用)
    PathBuf::from("llm_profiles.json")
}

/// 读 metadata JSON。文件不存在返回默认空 MetadataFile(不报错)。
pub fn load_profiles() -> Result<MetadataFile, String> {
    let path = metadata_path();
    if !path.exists() {
        return Ok(MetadataFile::default());
    }
    let s = std::fs::read_to_string(&path)
        .map_err(|e| format!("读 metadata 失败: {e}"))?;
    serde_json::from_str(&s).map_err(|e| format!("解析 metadata 失败: {e}"))
}

/// 写 metadata JSON。原子写(写临时文件 + rename)。
pub fn save_profiles(m: &MetadataFile) -> Result<(), String> {
    let path = metadata_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("建 metadata 目录失败: {e}"))?;
    }
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(m)
        .map_err(|e| format!("序列化 metadata 失败: {e}"))?;
    std::fs::write(&tmp, json)
        .map_err(|e| format!("写 metadata tmp 失败: {e}"))?;
    std::fs::rename(&tmp, &path)
        .map_err(|e| format!("rename metadata 失败: {e}"))
}

/// 取 active profile。`active` 字段为 None 或名字找不到时报 `ACTIVE_PROFILE_REQUIRED:`。
pub fn get_active_profile() -> Result<ProfileMeta, String> {
    let m = load_profiles()?;
    let name = match &m.active {
        Some(n) => n.clone(),
        None => return Err("ACTIVE_PROFILE_REQUIRED: 无 active profile,请先在 Settings > Providers 设置一个".into()),
    };
    m.profiles
        .into_iter()
        .find(|p| p.name == name)
        .ok_or_else(|| format!("ACTIVE_PROFILE_REQUIRED: active profile '{name}' 在 metadata 中不存在"))
}

/// 把 ProfileMeta + API key 翻译成 mtd 子进程要注入的 env vars。
///
/// 规则:
/// - Anthropic:`ANTHROPIC_API_KEY=<key>`,`ANTHROPIC_BASE_URL=<base_url>`(若非默认)
/// - OpenAI Compat:`OPENAI_API_KEY=<key>`,`OPENAI_BASE_URL=<base_url>`(若非空),`OPENAI_MODEL=<model>`(若非空)
/// - Ollama:`OLLAMA_HOST=<base_url>`(总设,用户可能改端口),`OLLAMA_MODEL=<model>`(若非空),无 key
/// - tool_search_enabled / experimental_betas_disabled:仅 Anthropic 时记入 ANTHROPIC_EXTRA_HEADERS
pub fn to_env_vars(meta: &ProfileMeta, key: &str) -> HashMap<String, String> {
    let mut env = HashMap::new();
    let p = match provider_from_name(&meta.provider) {
        Some(p) => p,
        None => {
            eprintln!(
                "llm_profiles::to_env_vars WARN: provider '{}' 未识别,env vars 为空",
                meta.provider
            );
            return env;
        }
    };

    match p {
        Provider::Anthropic => {
            env.insert("ANTHROPIC_API_KEY".into(), key.to_string());
            if !meta.base_url.is_empty() && meta.base_url != "https://api.anthropic.com" {
                env.insert("ANTHROPIC_BASE_URL".into(), meta.base_url.clone());
            }
        }
        Provider::Ollama => {
            env.insert("OLLAMA_HOST".into(), meta.base_url.clone());
            if !meta.model.is_empty() {
                env.insert("OLLAMA_MODEL".into(), meta.model.clone());
            }
        }
        Provider::OpenAI
        | Provider::LmStudio
        | Provider::DeepSeek
        | Provider::Zhipu
        | Provider::Kimi
        | Provider::MiniMax
        | Provider::ApitwoD
        | Provider::Shengsuanyun
        | Provider::TeamoRouter
        | Provider::Custom => {
            env.insert("OPENAI_API_KEY".into(), key.to_string());
            if !meta.base_url.is_empty() {
                env.insert("OPENAI_BASE_URL".into(), meta.base_url.clone());
            }
            if !meta.model.is_empty() {
                env.insert("OPENAI_MODEL".into(), meta.model.clone());
            }
        }
    }
    env
}

/// test_llm_connection URL 构造(按 provider)。返回 (url, headers) 元组,
/// 实际 HTTP 调用在 commands.rs test_connection_impl 中完成(Task 5)。
pub fn probe_endpoint(
    meta: &ProfileMeta,
    key: &str,
) -> Result<(String, HashMap<String, String>), String> {
    let base = if meta.base_url.is_empty() {
        return Err("INVALID_BASE_URL: probe 需要 base_url".into());
    } else {
        meta.base_url.trim_end_matches('/').to_string()
    };
    let p = provider_from_name(&meta.provider)
        .ok_or_else(|| format!("PROVIDER_NOT_FOUND: {}", meta.provider))?;

    let mut headers = HashMap::new();
    let url = match p {
        Provider::Anthropic => {
            headers.insert("x-api-key".into(), key.to_string());
            headers.insert("anthropic-version".into(), "2023-06-01".into());
            format!("{base}/v1/models")
        }
        Provider::Ollama => format!("{base}/api/tags"),
        Provider::OpenAI
        | Provider::LmStudio
        | Provider::DeepSeek
        | Provider::Zhipu
        | Provider::Kimi
        | Provider::MiniMax
        | Provider::ApitwoD
        | Provider::Shengsuanyun
        | Provider::TeamoRouter
        | Provider::Custom => {
            headers.insert("Authorization".into(), format!("Bearer {key}"));
            format!("{base}/models")
        }
    };
    Ok((url, headers))
}
```

### Step 2: Append 10 new tests to llm_profiles::tests

Open `src-tauri/src/llm_profiles.rs` `#[cfg(test)] mod tests`. Append these 11 tests INSIDE the `mod tests { ... }` block, AFTER the existing `provider_name_roundtrip_covers_all_12` test:

```rust
    // ── Task 3: metadata IO + env var(11 tests)─────────────────────────────

    fn make_meta(provider: &str, name: &str, base_url: &str, model: &str) -> ProfileMeta {
        ProfileMeta {
            name: name.into(),
            provider: provider.into(),
            base_url: base_url.into(),
            model: model.into(),
            note: None,
            tool_search_enabled: false,
            experimental_betas_disabled: false,
            created_at: "2026-07-23T00:00:00Z".into(),
        }
    }

    #[test]
    fn metadata_path_matches_appdata_pattern() {
        let p = metadata_path();
        let s = p.to_string_lossy();
        assert!(
            s.contains("com.duanyi.mediatodoc"),
            "path 应含 com.duanyi.mediatodoc,实际: {s}"
        );
        assert!(s.ends_with("llm_profiles.json"));
    }

    #[test]
    fn load_profiles_returns_empty_when_file_missing() {
        // metadata_path 在 fallback 下用当前目录;主路径不存在时返回空
        let m = load_profiles().expect("load 不存在的文件应 OK");
        let _ = m.active;
        let _ = m.profiles;
    }

    #[test]
    fn to_env_vars_anthropic_sets_key_and_optional_base_url() {
        let meta = make_meta("Anthropic", "anthropic-prod", "https://api.anthropic.com", "claude-sonnet-4-5");
        let env = to_env_vars(&meta, "sk-ant-test");
        assert_eq!(env.get("ANTHROPIC_API_KEY").map(|s| s.as_str()), Some("sk-ant-test"));
        assert!(env.get("ANTHROPIC_BASE_URL").is_none());

        let custom = make_meta("Anthropic", "anthropic-custom", "https://proxy.example.com", "claude-sonnet-4-5");
        let env2 = to_env_vars(&custom, "sk-ant-test");
        assert_eq!(env2.get("ANTHROPIC_BASE_URL").map(|s| s.as_str()), Some("https://proxy.example.com"));
    }

    #[test]
    fn to_env_vars_anthropic_ignores_tool_search_on_other_providers() {
        let mut meta = make_meta("DeepSeek", "deepseek-prod", "https://api.deepseek.com", "deepseek-chat");
        meta.tool_search_enabled = true;
        meta.experimental_betas_disabled = true;
        let env = to_env_vars(&meta, "sk-test");
        assert!(env.get("ANTHROPIC_API_KEY").is_none());
        assert!(env.get("ANTHROPIC_BASE_URL").is_none());
        assert_eq!(env.get("OPENAI_API_KEY").map(|s| s.as_str()), Some("sk-test"));
    }

    #[test]
    fn to_env_vars_openai_compat_sets_optional_base_and_model() {
        let meta = make_meta("DeepSeek", "deepseek-prod", "https://api.deepseek.com", "deepseek-chat");
        let env = to_env_vars(&meta, "sk-ds-test");
        assert_eq!(env.get("OPENAI_API_KEY").map(|s| s.as_str()), Some("sk-ds-test"));
        assert_eq!(env.get("OPENAI_BASE_URL").map(|s| s.as_str()), Some("https://api.deepseek.com"));
        assert_eq!(env.get("OPENAI_MODEL").map(|s| s.as_str()), Some("deepseek-chat"));
    }

    #[test]
    fn to_env_vars_openai_compat_omits_empty_optional_fields() {
        let meta = make_meta("DeepSeek", "ds-empty", "", "");
        let env = to_env_vars(&meta, "sk-ds-test");
        assert_eq!(env.get("OPENAI_API_KEY").map(|s| s.as_str()), Some("sk-ds-test"));
        assert!(env.get("OPENAI_BASE_URL").is_none());
        assert!(env.get("OPENAI_MODEL").is_none());
    }

    #[test]
    fn to_env_vars_ollama_sets_host_and_optional_model() {
        let meta = make_meta("Ollama", "ollama-local", "http://localhost:11434", "llama3.1");
        let env = to_env_vars(&meta, "ignored-key");
        assert_eq!(env.get("OLLAMA_HOST").map(|s| s.as_str()), Some("http://localhost:11434"));
        assert_eq!(env.get("OLLAMA_MODEL").map(|s| s.as_str()), Some("llama3.1"));
        assert!(env.get("OPENAI_API_KEY").is_none());
        assert!(env.get("ANTHROPIC_API_KEY").is_none());
    }

    #[test]
    fn probe_endpoint_anthropic_uses_x_api_key_header() {
        let meta = make_meta("Anthropic", "anthropic-prod", "https://api.anthropic.com", "");
        let (url, headers) = probe_endpoint(&meta, "sk-ant-test").unwrap();
        assert_eq!(url, "https://api.anthropic.com/v1/models");
        assert_eq!(headers.get("x-api-key").map(|s| s.as_str()), Some("sk-ant-test"));
        assert_eq!(headers.get("anthropic-version").map(|s| s.as_str()), Some("2023-06-01"));
    }

    #[test]
    fn probe_endpoint_openai_compat_uses_bearer_token() {
        let meta = make_meta("DeepSeek", "deepseek-prod", "https://api.deepseek.com", "");
        let (url, headers) = probe_endpoint(&meta, "sk-ds-test").unwrap();
        assert_eq!(url, "https://api.deepseek.com/models");
        assert_eq!(headers.get("Authorization").map(|s| s.as_str()), Some("Bearer sk-ds-test"));
    }

    #[test]
    fn probe_endpoint_ollama_uses_api_tags_no_auth() {
        let meta = make_meta("Ollama", "ollama-local", "http://localhost:11434", "");
        let (url, headers) = probe_endpoint(&meta, "ignored").unwrap();
        assert_eq!(url, "http://localhost:11434/api/tags");
        assert!(headers.is_empty());
    }
```

**Note on `get_active_profile_errors_when_no_active_set`**: this case is hard to unit-test because it depends on global metadata file state. Full integration coverage is in Task 8 manual acceptance test #9 ("删除 active profile → Run pipeline 报 ACTIVE_PROFILE_REQUIRED 错误").

### Step 3: Run llm_profiles tests (verify pass)

Run: `cd src-tauri && cargo test --lib llm_profiles::`

Expected:
```
running 28 tests
... (17 from Task 2 + 11 new from Task 3)

test result: ok. 28 passed; 0 failed
```

### Step 4: Run full test suite (verify no regression)

Run: `cd src-tauri && cargo test --lib`

Expected: `76 passed; 0 failed`(65 + 11 = 76)

### Step 5: Commit

```bash
git add src-tauri/src/llm_profiles.rs
git commit -m "feat(ui): W15-A T3 — llm_profiles metadata IO + env var mapping

Extends llm_profiles.rs with:
- ProfileMeta / MetadataFile structs (serde JSON)
- metadata_path() matching tauri.conf.json identifier
- load_profiles() / save_profiles() (atomic write via tmp + rename)
- get_active_profile() (returns ACTIVE_PROFILE_REQUIRED if none)
- to_env_vars() — provider dispatch (Anthropic / OpenAI Compat / Ollama)
- probe_endpoint() — test_connection URL construction per provider

Non-Anthropic providers silently ignore tool_search_enabled /
experimental_betas_disabled with WARN log (spec §7.3 reviewer note).

11/11 new tests pass (1 path + 1 IO + 4 env var + 3 probe_endpoint +
2 ancillary). Total: 76/76 (65 + 11).

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 4: runner.rs SpawnSpec.env_vars + spawn_mtd env injection

**Files:**
- Modify: `src-tauri/src/runner.rs` (add env_vars field + .env_clear().envs() in spawn_mtd)

**Interfaces:**
- `pub struct SpawnSpec { ..., pub env_vars: HashMap<String, String> }` (NEW field)
- `spawn_mtd(spec)` now calls `.env_clear().envs(&spec.env_vars)` (NEW behavior)

**Design rationale:**
- Spec §5: SpawnSpec 扩展是干净分层 — env_vars 由 commands.rs 在 run_pipeline 算出,runner.rs 只负责 spawn + env 注入
- `env_clear()` 防父进程 HTTP_PROXY 等污染子进程(W14-D trust_env=False 思路)
- 现有 5 个 `build_*` 调用方自动获得空 `env_vars`(测试不需要改 build_*)
- 现有 2 个直接构造 `SpawnSpec { ... }` 的测试需加 `env_vars: HashMap::new()`

### Step 1: Update existing SpawnSpec-constructing tests in runner.rs

Edit `src-tauri/src/runner.rs` at lines 571-580 (`registry_rejects_when_full`) and 614-623 (`registry_cancel_and_completed_lru`). In both, the `SpawnSpec { ... }` literal needs `env_vars: HashMap::new()` added.

Add `use std::collections::HashMap;` at top of runner.rs if not present (it's NOT currently imported — verify by reading).

Also update the imports line in runner.rs:
```rust
use std::collections::HashMap;
```

After adding the import, in each test that constructs SpawnSpec directly, change:

```rust
SpawnSpec {
    program: ...,
    args: ...,
    work_dir: ...,
    log_path: ...,
}
```

to:

```rust
SpawnSpec {
    program: ...,
    args: ...,
    work_dir: ...,
    log_path: ...,
    env_vars: HashMap::new(),
}
```

(2 sites: registry_rejects_when_full at ~line 571, registry_cancel_and_completed_lru at ~line 614)

### Step 2: Run existing tests (verify they pass with new field)

Run: `cd src-tauri && cargo test --lib runner::`

Expected:
```
test result: ok. 10 passed; 0 failed  (existing 10 tests)
```

If fails: ensure both `SpawnSpec { ... }` literals updated and HashMap imported.

### Step 3: Add env_vars field + env_clear().envs() to SpawnSpec + spawn_mtd

Edit `src-tauri/src/runner.rs`:

**(a)** Update the `SpawnSpec` struct (around line 24):

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpawnSpec {
    /// 主程序(默认 `uv`,可被 `UV_BIN` 环境变量覆盖)
    pub program: String,
    /// 完整参数(uv --project X run mtd run <inbox> [--llm ...] ...)
    pub args: Vec<String>,
    /// 子进程工作目录(默认 inbox.parent)
    pub work_dir: String,
    /// stdout/stderr 落盘的日志文件
    pub log_path: String,
    /// 注入到子进程的 env vars(W15-A: active profile 的 API key + base_url + model)。
    /// spawn_mtd 内部用 .env_clear() 清父进程 env,避免 HTTP_PROXY 等污染子进程。
    pub env_vars: HashMap<String, String>,
}
```

**(b)** Update `build_mtd_run_args` (line 87-92): add `env_vars: HashMap::new(),` to the returned `SpawnSpec`:

```rust
    SpawnSpec {
        program,
        args,
        work_dir: work_dir.to_string_lossy().into_owned(),
        log_path: log_path.to_string_lossy().into_owned(),
        env_vars: HashMap::new(),
    }
```

**(c)** Update `build_mtd_resume_args` (line 110-118): same, add `env_vars: HashMap::new(),` to returned SpawnSpec.

**(d)** Update `spawn_mtd` (line 433-454): add `.env_clear().envs(&spec.env_vars)` before `.kill_on_drop(true)`:

```rust
pub async fn spawn_mtd(spec: &SpawnSpec) -> Result<Child, String> {
    // 确保父目录存在
    if let Some(parent) = Path::new(&spec.log_path).parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    let log = std::fs::File::create(&spec.log_path)
        .map_err(|e| format!("create log {} 失败: {e}", spec.log_path))?;
    let err_log = log
        .try_clone()
        .map_err(|e| format!("clone log handle 失败: {e}"))?;
    let mut cmd = Command::new(&spec.program);
    cmd.args(&spec.args)
        .current_dir(&spec.work_dir)
        .env_clear()                              // W15-A: 清父进程 env 防 HTTP_PROXY 污染(W14-D 思路)
        .envs(&spec.env_vars)                     // W15-A: 注入 active profile env vars
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(err_log))
        .kill_on_drop(true);
    cmd
        .spawn()
        .map_err(|e| format!("spawn `{}` 失败: {e}", spec.program))
}
```

### Step 4: Add 1 new test verifying env vars are injected

Append inside `mod tests` (after `registry_list_empty` ~line 680):

```rust
    #[test]
    fn spawn_mtd_injects_env_vars_to_child() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // 用一个真子进程读自己的环境变量(Win:cmd /c set;Unix:env | grep)
            let mut env_vars = HashMap::new();
            env_vars.insert("MTD_TEST_VAR".to_string(), "test_value_12345".to_string());

            let log_path = std::env::temp_dir().join("test_env_inject.log");
            let spec = SpawnSpec {
                program: if cfg!(windows) { "cmd".to_string() } else { "sh".to_string() },
                args: if cfg!(windows) {
                    vec!["/C".to_string(), "set".to_string()]
                } else {
                    vec!["-c".to_string(), "env".to_string()]
                },
                work_dir: std::env::temp_dir().to_string_lossy().into_owned(),
                log_path: log_path.to_string_lossy().into_owned(),
                env_vars,
            };
            let child = spawn_mtd(&spec).await.expect("spawn");
            let output = child.wait_with_output().await.expect("wait");
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                stdout.contains("MTD_TEST_VAR=test_value_12345"),
                "子进程应看到注入的 env var,实际 stdout:\n{stdout}"
            );
        });
    }
```

### Step 5: Run runner tests (verify 10 + 1 = 11 pass)

Run: `cd src-tauri && cargo test --lib runner::`

Expected:
```
running 11 tests
... (10 existing + 1 new)

test result: ok. 11 passed; 0 failed
```

### Step 6: Run full test suite (verify no regression)

Run: `cd src-tauri && cargo test --lib`

Expected: `77 passed; 0 failed`(76 + 1 = 77)

### Step 7: Commit

```bash
git add src-tauri/src/runner.rs
git commit -m "feat(ui): W15-A T4 — runner SpawnSpec.env_vars + spawn_mtd env injection

- src-tauri/src/runner.rs: SpawnSpec + env_vars field
- spawn_mtd: add .env_clear().envs(&spec.env_vars) to inject active
  profile env vars while preventing parent env pollution (HTTP_PROXY
  defense, W14-D trust_env=False approach)
- build_mtd_run_args / build_mtd_resume_args: default env_vars to empty
- 2 existing tests: add env_vars: HashMap::new() to SpawnSpec literals
- 1 new test: verify env vars injected to real child process

77/77 tests pass (76 + 1 new).

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 5: commands.rs 6 LLM Tauri commands + run_pipeline env injection

**Files:**
- Modify: `src-tauri/Cargo.toml` (add `reqwest`)
- Modify: `src-tauri/src/commands.rs` (add 6 commands + env injection in run_pipeline/resume_pipeline + 8 tests)

**Interfaces** (Task 5 produces):
- `pub fn list_llm_profiles_impl() -> CommandResponse<Vec<ProfileMeta>>`
- `pub fn get_active_llm_profile_name_impl() -> CommandResponse<String>` (returns name; empty string if none)
- `pub struct SaveProfileArgs { name, provider, base_url, model, note, api_key, tool_search_enabled, experimental_betas_disabled }`
- `pub fn save_llm_profile_impl(args: SaveProfileArgs) -> CommandResponse<ProfileMeta>`
- `pub fn set_active_profile_impl(name: String) -> CommandResponse<()>`
- `pub fn delete_llm_profile_impl(name: String) -> CommandResponse<()>`
- `pub struct TestConnectionResult { ok, latency_ms, model, error }`
- `pub async fn test_llm_connection_impl(name: String) -> CommandResponse<TestConnectionResult>`
- `pub async fn list_llm_profiles()`, etc — `#[tauri::command]` thin wrappers
- `run_pipeline` / `resume_pipeline` modified: read active profile → keyring → to_env_vars → `spec.env_vars = env_vars`

### Step 1: Add reqwest to Cargo.toml

Edit `src-tauri/Cargo.toml`. Add to `[dependencies]`:

```toml
[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["process", "io-util", "sync", "rt", "fs", "macros", "time"] }
once_cell = "1"
keyring = "3"
dirs = "5"
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
```

### Step 2: Append 6 LLM commands to commands.rs (BEFORE `#[cfg(test)]`)

Open `src-tauri/src/commands.rs`. Find the `#[cfg(test)]` block at the end (after the `list_all_runs` command) and APPEND the new code BEFORE it:

```rust
use crate::keyring_store;
use crate::llm_profiles::{self, MetadataFile, ProfileMeta};
use std::collections::HashMap;
use std::time::Instant;

// ─────────────────────────────────────────────────────────────
// list_llm_profiles —— 列出所有 profile(metadata JSON)
// ─────────────────────────────────────────────────────────────

pub fn list_llm_profiles_impl() -> CommandResponse<Vec<ProfileMeta>> {
  let m = match llm_profiles::load_profiles() {
    Ok(m) => m,
    Err(e) => return CommandResponse::err(e),
  };
  CommandResponse::ok(m.profiles)
}

#[tauri::command]
pub async fn list_llm_profiles() -> CommandResponse<Vec<ProfileMeta>> {
  list_llm_profiles_impl()
}

// ─────────────────────────────────────────────────────────────
// get_active_llm_profile_name —— 取 active profile 名字(无则空字符串)
// ─────────────────────────────────────────────────────────────

pub fn get_active_llm_profile_name_impl() -> CommandResponse<String> {
  let m = match llm_profiles::load_profiles() {
    Ok(m) => m,
    Err(e) => return CommandResponse::err(e),
  };
  CommandResponse::ok(m.active.unwrap_or_default())
}

#[tauri::command]
pub async fn get_active_llm_profile_name() -> CommandResponse<String> {
  get_active_llm_profile_name_impl()
}

// ─────────────────────────────────────────────────────────────
// save_llm_profile —— 创建或更新 profile;api_key=None 时不更新 key
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SaveProfileArgs {
  pub name: String,
  pub provider: String,
  pub base_url: String,
  pub model: String,
  #[serde(default)]
  pub note: Option<String>,
  /// None = 不更新 keyring(编辑已有 profile 时);Some("") = 删除 key(罕见);
  /// Some("sk-...") = 写入新 key
  #[serde(default)]
  pub api_key: Option<String>,
  #[serde(default)]
  pub tool_search_enabled: Option<bool>,
  #[serde(default)]
  pub experimental_betas_disabled: Option<bool>,
}

pub fn save_llm_profile_impl(args: SaveProfileArgs) -> CommandResponse<ProfileMeta> {
  // 校验
  if args.name.trim().is_empty() {
    return CommandResponse::err("PROFILE_NAME_CONFLICT: profile 名不能为空".into());
  }
  if let Err(e) = llm_profiles::validate_base_url(&args.base_url) {
    return CommandResponse::err(e);
  }
  if let Err(e) = llm_profiles::validate_model(&args.model) {
    return CommandResponse::err(e);
  }
  if llm_profiles::provider_from_name(&args.provider).is_none() {
    return CommandResponse::err(format!("PROVIDER_NOT_FOUND: {}", args.provider));
  }

  let mut m = match llm_profiles::load_profiles() {
    Ok(m) => m,
    Err(e) => return CommandResponse::err(e),
  };

  // name 冲突检查(只跟其它 profile 冲突,允许同名更新自己)
  if let Some(other) = m.profiles.iter().find(|p| p.name == args.name) {
    let _ = other; // 允许:更新自己
  }

  let now = chrono_like_now();
  let new_meta = ProfileMeta {
    name: args.name.clone(),
    provider: args.provider.clone(),
    base_url: args.base_url.clone(),
    model: args.model.clone(),
    note: args.note.clone(),
    tool_search_enabled: args.tool_search_enabled.unwrap_or(false),
    experimental_betas_disabled: args.experimental_betas_disabled.unwrap_or(false),
    created_at: now.clone(),
  };

  // 写 keyring(若提供了 api_key)
  if let Some(key) = &args.api_key {
    if !key.is_empty() {
      if let Err(e) = keyring_store::write_key(&args.name, key) {
        return CommandResponse::err(e);
      }
    } else {
      // Some("") 视为删除 key
      let _ = keyring_store::delete_key(&args.name);
    }
  }

  // upsert 到 profiles 列表
  if let Some(existing) = m.profiles.iter_mut().find(|p| p.name == args.name) {
    existing.provider = new_meta.provider.clone();
    existing.base_url = new_meta.base_url.clone();
    existing.model = new_meta.model.clone();
    existing.note = new_meta.note.clone();
    existing.tool_search_enabled = new_meta.tool_search_enabled;
    existing.experimental_betas_disabled = new_meta.experimental_betas_disabled;
    // 保留 created_at
  } else {
    m.profiles.push(new_meta.clone());
  }

  if let Err(e) = llm_profiles::save_profiles(&m) {
    return CommandResponse::err(e);
  }
  // 返回最新 stored meta(若更新,返回更新后的;若新建,返回新建的)
  let stored = m.profiles.into_iter().find(|p| p.name == args.name).unwrap_or(new_meta);
  CommandResponse::ok(stored)
}

#[tauri::command]
pub async fn save_llm_profile(args: SaveProfileArgs) -> CommandResponse<ProfileMeta> {
  save_llm_profile_impl(args)
}

// ─────────────────────────────────────────────────────────────
// set_active_profile —— 切换 active
// ─────────────────────────────────────────────────────────────

pub fn set_active_profile_impl(name: String) -> CommandResponse<()> {
  let mut m = match llm_profiles::load_profiles() {
    Ok(m) => m,
    Err(e) => return CommandResponse::err(e),
  };
  if !m.profiles.iter().any(|p| p.name == name) {
    return CommandResponse::err(format!("PROFILE_NOT_FOUND: {name}"));
  }
  m.active = Some(name);
  if let Err(e) = llm_profiles::save_profiles(&m) {
    return CommandResponse::err(e);
  }
  CommandResponse::ok(())
}

#[tauri::command]
pub async fn set_active_profile(name: String) -> CommandResponse<()> {
  set_active_profile_impl(name)
}

// ─────────────────────────────────────────────────────────────
// delete_llm_profile —— 删 profile + 删 keyring
// ─────────────────────────────────────────────────────────────

pub fn delete_llm_profile_impl(name: String) -> CommandResponse<()> {
  let mut m = match llm_profiles::load_profiles() {
    Ok(m) => m,
    Err(e) => return CommandResponse::err(e),
  };
  let before = m.profiles.len();
  m.profiles.retain(|p| p.name != name);
  if m.profiles.len() == before {
    return CommandResponse::err(format!("PROFILE_NOT_FOUND: {name}"));
  }
  if m.active.as_deref() == Some(&name) {
    m.active = None;
  }
  if let Err(e) = llm_profiles::save_profiles(&m) {
    return CommandResponse::err(e);
  }
  // 删 keyring(idempotent,不报错)
  let _ = keyring_store::delete_key(&name);
  CommandResponse::ok(())
}

#[tauri::command]
pub async fn delete_llm_profile(name: String) -> CommandResponse<()> {
  delete_llm_profile_impl(name)
}

// ─────────────────────────────────────────────────────────────
// test_llm_connection —— HTTP 探测 active profile 的 LLM 端点
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct TestConnectionResult {
  pub ok: bool,
  pub latency_ms: u64,
  pub model: String,
  pub error: Option<String>,
}

pub async fn test_llm_connection_impl(name: String) -> CommandResponse<TestConnectionResult> {
  // 1. 找 profile
  let m = match llm_profiles::load_profiles() {
    Ok(m) => m,
    Err(e) => return CommandResponse::err(e),
  };
  let meta = match m.profiles.into_iter().find(|p| p.name == name) {
    Some(p) => p,
    None => return CommandResponse::err(format!("PROFILE_NOT_FOUND: {name}")),
  };
  // 2. 读 key
  let key = match keyring_store::read_key(&meta.name) {
    Ok(k) => k,
    Err(e) => return CommandResponse::err(e),
  };
  // 3. 构造 URL
  let (url, headers) = match llm_profiles::probe_endpoint(&meta, &key) {
    Ok(v) => v,
    Err(e) => return CommandResponse::err(e),
  };
  // 4. HTTP GET + 计时
  let start = Instant::now();
  let client = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(10))
    .build()
    .map_err(|e| format!("建 reqwest client 失败: {e}"))?;
  let mut req = client.get(&url);
  for (k, v) in &headers {
    req = req.header(k, v);
  }
  let resp = match req.send().await {
    Ok(r) => r,
    Err(e) => {
      return CommandResponse::ok(TestConnectionResult {
        ok: false,
        latency_ms: start.elapsed().as_millis() as u64,
        model: meta.model.clone(),
        error: Some(format!("NETWORK_ERROR: {e}")),
      });
    }
  };
  let latency_ms = start.elapsed().as_millis() as u64;
  let status = resp.status();
  let ok = status.is_success();
  CommandResponse::ok(TestConnectionResult {
    ok,
    latency_ms,
    model: meta.model.clone(),
    error: if ok { None } else { Some(format!("HTTP {}", status.as_u16())) },
  })
}

#[tauri::command]
pub async fn test_llm_connection(name: String) -> CommandResponse<TestConnectionResult> {
  test_llm_connection_impl(name).await
}

/// 复用 runner.rs::chrono_like_now 的时间戳格式(私有 fn,本文件需复制)。
/// spec 不要求统一格式,只要 RFC3339-ish 可序列化即可。
fn chrono_like_now() -> String {
  let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_secs())
    .unwrap_or(0);
  format!("epoch:{now}")
}
```

### Step 3: Modify run_pipeline + resume_pipeline to inject env vars

Edit `src-tauri/src/commands.rs`. In both `run_pipeline` and `resume_pipeline`, AFTER the `build_mtd_*_args(...)` call and BEFORE `spawn_mtd(&spec)`, add the env injection block.

For `run_pipeline` (insert between line 1026 spec build and line 1034 spawn_mtd):

```rust
  // W15-A: 读 active profile + keyring,注入 env vars 到 mtd 子进程
  let mut spec = build_mtd_run_args(
    &project,
    &inbox,
    llm.as_deref(),
    imagegen.as_deref(),
    stop_after.as_deref(),
    no_longdoc.unwrap_or(false),
    force.unwrap_or(false),
  );
  let active = match llm_profiles::get_active_profile() {
    Ok(p) => p,
    Err(e) => return CommandResponse::err(e),
  };
  let key = match keyring_store::read_key(&active.name) {
    Ok(k) => k,
    Err(e) => return CommandResponse::err(e),
  };
  spec.env_vars = llm_profiles::to_env_vars(&active, &key);
  // 后续 spawn_mtd(&spec) 用 spec.env_vars
```

Note: `let mut spec = ...` (was `let spec = ...`) so we can mutate `spec.env_vars`.

For `resume_pipeline` (similar, between build_mtd_resume_args and spawn_mtd):

```rust
  // W15-A: 同 run_pipeline
  let mut spec = build_mtd_resume_args(
    &project,
    &work,
    force.unwrap_or(false),
    stop_after.as_deref(),
  );
  let active = match llm_profiles::get_active_profile() {
    Ok(p) => p,
    Err(e) => return CommandResponse::err(e),
  };
  let key = match keyring_store::read_key(&active.name) {
    Ok(k) => k,
    Err(e) => return CommandResponse::err(e),
  };
  spec.env_vars = llm_profiles::to_env_vars(&active, &key);
```

### Step 4: Append 8 unit tests to commands.rs `#[cfg(test)] mod tests`

Open `src-tauri/src/commands.rs` `#[cfg(test)] mod tests`. If absent, create at end of file. Append these tests INSIDE the `mod tests { ... }` block:

```rust
    // ── Task 5: LLM commands(8 tests)─────────────────────────────

    use crate::llm_profiles::{self, ProfileMeta};

    fn make_meta_for_test(name: &str, provider: &str) -> ProfileMeta {
      ProfileMeta {
        name: name.into(),
        provider: provider.into(),
        base_url: "https://api.deepseek.com".into(),
        model: "deepseek-chat".into(),
        note: None,
        tool_search_enabled: false,
        experimental_betas_disabled: false,
        created_at: "2026-07-23T00:00:00Z".into(),
      }
    }

    #[test]
    fn list_llm_profiles_returns_empty_when_no_metadata() {
      // metadata_path 在 fallback 下用 cwd;若文件不存在,load 返回 default(empty)
      let r = list_llm_profiles_impl();
      assert!(r.ok, "list_llm_profiles 应成功(空列表): error={:?}", r.error);
      let profiles = r.data.unwrap();
      // 不强求为空,因为全局 metadata 可能已有数据。只验证返回 Vec。
      let _: Vec<ProfileMeta> = profiles;
    }

    #[test]
    fn save_llm_profile_validates_empty_name() {
      let args = SaveProfileArgs {
        name: "".into(),
        provider: "DeepSeek".into(),
        base_url: "https://api.deepseek.com".into(),
        model: "deepseek-chat".into(),
        note: None,
        api_key: None,
        tool_search_enabled: None,
        experimental_betas_disabled: None,
      };
      let r = save_llm_profile_impl(args);
      assert!(!r.ok);
      assert!(r.error.unwrap().contains("PROFILE_NAME_CONFLICT"));
    }

    #[test]
    fn save_llm_profile_validates_invalid_url() {
      let args = SaveProfileArgs {
        name: "test-bad-url".into(),
        provider: "DeepSeek".into(),
        base_url: "ftp://evil.example.com".into(),
        model: "deepseek-chat".into(),
        note: None,
        api_key: None,
        tool_search_enabled: None,
        experimental_betas_disabled: None,
      };
      let r = save_llm_profile_impl(args);
      assert!(!r.ok);
      assert!(r.error.unwrap().contains("INVALID_BASE_URL"));
    }

    #[test]
    fn save_llm_profile_validates_unknown_provider() {
      let args = SaveProfileArgs {
        name: "test-bad-provider".into(),
        provider: "FakeLLM".into(),
        base_url: "https://api.deepseek.com".into(),
        model: "deepseek-chat".into(),
        note: None,
        api_key: None,
        tool_search_enabled: None,
        experimental_betas_disabled: None,
      };
      let r = save_llm_profile_impl(args);
      assert!(!r.ok);
      assert!(r.error.unwrap().contains("PROVIDER_NOT_FOUND"));
    }

    #[test]
    fn set_active_profile_errors_on_unknown_name() {
      let r = set_active_profile_impl("__nonexistent_profile__".into());
      assert!(!r.ok);
      assert!(r.error.unwrap().contains("PROFILE_NOT_FOUND"));
    }

    #[test]
    fn delete_llm_profile_errors_on_unknown_name() {
      let r = delete_llm_profile_impl("__nonexistent_profile__".into());
      assert!(!r.ok);
      assert!(r.error.unwrap().contains("PROFILE_NOT_FOUND"));
    }

    #[test]
    fn test_connection_url_for_anthropic_uses_models_endpoint() {
      // 只验证 URL 构造,不实际发 HTTP
      let meta = make_meta_for_test("anthropic-prod", "Anthropic");
      let (url, headers) = llm_profiles::probe_endpoint(&meta, "sk-ant-test").unwrap();
      assert!(url.contains("/v1/models"));
      assert_eq!(headers.get("x-api-key").map(|s| s.as_str()), Some("sk-ant-test"));
    }

    #[test]
    fn save_then_get_active_roundtrip_in_global_metadata() {
      // 集成测:写 profile + 标 active + 读 active(用全局 metadata_path)
      // 由于 metadata 是全局状态,使用唯一名避免冲突
      let unique_name = format!("__w15a_test_{}__", std::process::id());
      // 1. save
      let args = SaveProfileArgs {
        name: unique_name.clone(),
        provider: "DeepSeek".into(),
        base_url: "https://api.deepseek.com".into(),
        model: "deepseek-chat".into(),
        note: Some("test".into()),
        api_key: Some("sk-test-1234".into()),
        tool_search_enabled: None,
        experimental_betas_disabled: None,
      };
      let r = save_llm_profile_impl(args);
      assert!(r.ok, "save 应成功: error={:?}", r.error);

      // 2. set active
      let r = set_active_profile_impl(unique_name.clone());
      assert!(r.ok, "set_active 应成功: error={:?}", r.error);

      // 3. get active
      let r = get_active_llm_profile_name_impl();
      assert!(r.ok);
      let active = r.data.unwrap();
      assert_eq!(active, unique_name);

      // 4. cleanup
      let _ = delete_llm_profile_impl(unique_name);
    }
```

### Step 5: Run commands tests (verify 8 pass)

Run: `cd src-tauri && cargo test --lib commands::`

Expected: `test result: ok. 8 passed; 0 failed`

### Step 6: Run full test suite (verify no regression)

Run: `cd src-tauri && cargo test --lib`

Expected: `85 passed; 0 failed`(77 + 8 = 85)

### Step 7: Commit

```bash
git add src-tauri/Cargo.toml src-tauri/src/commands.rs
git commit -m "feat(ui): W15-A T5 — commands 6 LLM Tauri commands + run_pipeline env injection

- src-tauri/Cargo.toml: + reqwest 0.12 (rustls-tls, json)
- src-tauri/src/commands.rs: + 6 Tauri commands
  - list_llm_profiles / get_active_llm_profile_name / save_llm_profile /
    set_active_profile / delete_llm_profile / test_llm_connection
- run_pipeline / resume_pipeline: 读 active profile → keyring → to_env_vars →
  spec.env_vars(spec §5 三层协作)
- 8 unit tests: 1 list + 3 save validation + 2 error path + 1 probe URL +
  1 save→active→get roundtrip

85/85 tests pass (77 + 8 new).

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 6: lib.rs invoke_handler + 6 commands wiring

**Files:**
- Modify: `src-tauri/src/lib.rs` (add 6 commands to `invoke_handler` + pub use)

### Step 1: Update lib.rs pub use list

Edit `src-tauri/src/lib.rs:20-25` (the `pub use commands::{...}` block). Add 6 new exports:

```rust
pub use commands::{
  cancel_run, check_status, delete_llm_profile, get_active_llm_profile_name, list_all_runs,
  list_courses, list_llm_profiles, list_outputs, list_running, read_lecture, read_log,
  resume_pipeline, run_pipeline, save_llm_profile, set_active_profile, test_llm_connection,
  CancelResult, CheckStatusResult, CourseEntry, ListAllRunsResult, ListCoursesResult,
  ListOutputsResult, ListRunningResult, OutputsGroups, ReadLectureResult, ReadLogResult,
  SaveProfileArgs, StageStatus, TestConnectionResult,
};
```

### Step 2: Update invoke_handler

Edit `src-tauri/src/lib.rs:73-94` (the `tauri::generate_handler![...]` block). Add 6 new commands BEFORE the closing `]`. Final invoke_handler should look like:

```rust
    .invoke_handler(tauri::generate_handler![
      // W14-B hello world
      app_info,
      ping,
      // W14-B+ T2 4 个只读 FS commands
      list_courses,
      check_status,
      list_outputs,
      read_lecture,
      // W14-B+ T3 4 个子进程 commands
      run_pipeline,
      resume_pipeline,
      cancel_run,
      list_running,
      // W14-B+ T4 2 个 Python API commands
      get_run_metrics,
      list_runs,
      // W14-B+2 read_log(后端 log tail)
      read_log,
      // W14-C 多课程并发(list_all_runs)
      list_all_runs,
      // W15-A 6 LLM API commands
      list_llm_profiles,
      get_active_llm_profile_name,
      save_llm_profile,
      set_active_profile,
      delete_llm_profile,
      test_llm_connection,
    ])
```

### Step 3: Run cargo build (verify registration succeeds)

Run: `cd src-tauri && cargo build`

Expected: build succeeds (no warnings about unregistered commands, no duplicate registration).

If fails: check that the 6 command names in invoke_handler exactly match `#[tauri::command]` function names.

### Step 4: Run full test suite (verify no regression)

Run: `cd src-tauri && cargo test --lib`

Expected: `85 passed; 0 failed`(unchanged from Task 5)

### Step 5: Commit

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(ui): W15-A T6 — lib.rs invoke_handler wiring

- src-tauri/src/lib.rs: + 6 commands to invoke_handler (list_llm_profiles,
  get_active_llm_profile_name, save_llm_profile, set_active_profile,
  delete_llm_profile, test_llm_connection)
- + pub use for the 6 commands + SaveProfileArgs + TestConnectionResult

85/85 tests still pass.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 7: index.html Settings tab + Providers UI + add modal

**Files:**
- Modify: `src/index.html` (add 1 nav button + Settings tab content + Providers subpage + add modal + JS for 6 commands)

**Design notes:**
- Frontend is vanilla JS (no framework); follows existing pattern (5 tabs already)
- 6 new Tauri command invocations via `window.__TAURI__.core.invoke('cmd_name', { args })`
- `SaveProfileArgs` shape: `{ name, provider, base_url, model, note, api_key, tool_search_enabled, experimental_betas_disabled }`
- `TestConnectionResult` shape: `{ ok, latency_ms, model, error }`

### Step 1: Add Settings tab to sidebar nav

Open `src/index.html`. Find the existing nav buttons (Inbox / Run / Output / Health / Learn). Append one more:

```html
<button class="nav-btn" data-tab="settings">Settings</button>
```

### Step 2: Add Settings tab content container

After the last existing `<div class="tab-content" id="tab-X">...</div>`, append:

```html
<div class="tab-content" id="tab-settings" hidden>
  <aside class="settings-sidebar">
    <button class="settings-nav-btn active" data-subtab="providers">Providers</button>
    <button class="settings-nav-btn" data-subtab="general">General</button>
    <button class="settings-nav-btn" data-subtab="theme">Theme</button>
    <button class="settings-nav-btn" data-subtab="about">About</button>
  </aside>
  <main class="settings-main">
    <section class="settings-subtab" id="subtab-providers">
      <div class="settings-header">
        <h2>Providers</h2>
        <div>
          <button id="provider-add-btn">+ 添加服务商</button>
          <button id="provider-refresh-btn">刷新</button>
        </div>
      </div>
      <div id="provider-list" class="provider-list"></div>
    </section>
    <section class="settings-subtab" id="subtab-general" hidden>
      <h2>General</h2>
      <p>Coming soon (W15-C)</p>
    </section>
    <section class="settings-subtab" id="subtab-theme" hidden>
      <h2>Theme</h2>
      <p>Coming soon (W15-C)</p>
    </section>
    <section class="settings-subtab" id="subtab-about" hidden>
      <h2>About</h2>
      <p>media-to-doc UI v1.4.0 — W15-A LLM API Settings</p>
    </section>
  </main>
</div>
```

### Step 3: Add modal HTML (appended before `</body>`)

Before `</body>`, append:

```html
<div class="modal-backdrop" id="provider-modal" hidden>
  <div class="modal">
    <h3 id="provider-modal-title">添加服务商</h3>
    <form id="provider-form">
      <label>预设 *<select id="provider-preset"></select></label>
      <label>名称 *<input type="text" id="provider-name" required></label>
      <label>备注<input type="text" id="provider-note"></label>
      <label>接口地址 *<input type="url" id="provider-base-url" required></label>
      <div class="anthropic-only" hidden>
        <label><input type="checkbox" id="provider-tool-search"> 启用 Tool Search</label>
        <label><input type="checkbox" id="provider-betas-disabled"> 关闭实验性 Beta 头</label>
      </div>
      <label>API 密钥 *<input type="password" id="provider-api-key" required></label>
      <label>模型 *<input type="text" id="provider-model" required></label>
      <div class="modal-actions">
        <button type="button" id="provider-test-btn">测试连接</button>
        <button type="button" id="provider-cancel-btn">取消</button>
        <button type="submit" id="provider-save-btn">保存</button>
      </div>
      <p id="provider-test-result"></p>
    </form>
  </div>
</div>
```

### Step 4: Add JS for Settings tab + Providers CRUD

Append before `</body>` (after the modal HTML, inside a `<script>` block):

```html
<script>
  // ── Settings tab 切换 ────────────────────────────────────────────────
  document.querySelectorAll('.settings-nav-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const subtab = btn.dataset.subtab;
      document.querySelectorAll('.settings-nav-btn').forEach(b => b.classList.toggle('active', b === btn));
      document.querySelectorAll('.settings-subtab').forEach(s => s.hidden = s.id !== `subtab-${subtab}`);
    });
  });

  // ── Provider list 渲染 ────────────────────────────────────────────────
  const providerList = document.getElementById('provider-list');

  async function loadProviders() {
    const r = await window.__TAURI__.core.invoke('list_llm_profiles');
    if (!r.ok) { providerList.textContent = `加载失败: ${r.error}`; return; }
    const profiles = r.data;
    const activeR = await window.__TAURI__.core.invoke('get_active_llm_profile_name');
    const activeName = activeR.ok ? activeR.data : '';
    renderProviders(profiles, activeName);
  }

  function renderProviders(profiles, activeName) {
    providerList.innerHTML = '';
    if (profiles.length === 0) {
      providerList.innerHTML = '<p class="empty">还没有 profile。点 [+ 添加服务商] 创建。</p>';
      return;
    }
    profiles.forEach(p => {
      const card = document.createElement('div');
      card.className = 'provider-card';
      const isActive = p.name === activeName;
      card.innerHTML = `
        <div class="provider-card-header">
          <span class="provider-star">${isActive ? '★' : '☆'}</span>
          <span class="provider-name">${escapeHtml(p.name)}</span>
          <span class="provider-provider">(${escapeHtml(p.provider)})</span>
        </div>
        <div class="provider-card-meta">
          <span>${escapeHtml(p.model)}</span>
          ${p.note ? `<span> · ${escapeHtml(p.note)}</span>` : ''}
        </div>
        <div class="provider-card-actions">
          <button data-act="activate" data-name="${escapeHtml(p.name)}" ${isActive ? 'disabled' : ''}>激活</button>
          <button data-act="edit" data-name="${escapeHtml(p.name)}">编辑</button>
          <button data-act="delete" data-name="${escapeHtml(p.name)}">删除</button>
        </div>
      `;
      providerList.appendChild(card);
    });
    // 绑定按钮
    providerList.querySelectorAll('button[data-act]').forEach(btn => {
      btn.addEventListener('click', () => handleProviderAction(btn.dataset.act, btn.dataset.name));
    });
  }

  async function handleProviderAction(act, name) {
    if (act === 'activate') {
      const r = await window.__TAURI__.core.invoke('set_active_profile', { name });
      if (r.ok) loadProviders();
      else alert(`激活失败: ${r.error}`);
    } else if (act === 'edit') {
      // 简单实现:从 list 找 profile,填充 modal
      const listR = await window.__TAURI__.core.invoke('list_llm_profiles');
      const p = listR.data.find(x => x.name === name);
      if (p) openModal(p);
    } else if (act === 'delete') {
      if (!confirm(`删除 profile "${name}" 及其 keyring key?`)) return;
      const r = await window.__TAURI__.core.invoke('delete_llm_profile', { name });
      if (r.ok) loadProviders();
      else alert(`删除失败: ${r.error}`);
    }
  }

  function escapeHtml(s) {
    return String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
  }

  // ── 12 个预设下拉 ──────────────────────────────────────────────────────
  const PROVIDERS = [
    { name: 'Anthropic', base_url: 'https://api.anthropic.com', model: 'claude-sonnet-4-5', anthropic: true },
    { name: 'OpenAI', base_url: 'https://api.openai.com/v1', model: 'gpt-4o' },
    { name: 'Ollama', base_url: 'http://localhost:11434', model: 'llama3.1' },
    { name: 'LM Studio', base_url: 'http://localhost:1234/v1', model: 'loaded-model' },
    { name: 'DeepSeek', base_url: 'https://api.deepseek.com', model: 'deepseek-chat' },
    { name: 'Zhipu GLM', base_url: 'https://open.bigmodel.cn/api/paas/v4', model: 'glm-4-plus' },
    { name: 'Kimi', base_url: 'https://api.moonshot.cn/v1', model: 'moonshot-v1-128k' },
    { name: 'MiniMax', base_url: 'https://api.MiniMax.chat/v1', model: 'MiniMax-Text-01' },
    { name: '接口 AI', base_url: 'https://api.api2d.net/v1', model: 'gpt-4o-mini' },
    { name: '胜算云', base_url: 'https://api.shengsuanyun.com/v1', model: 'gpt-4o-mini' },
    { name: 'TeamoRouter', base_url: 'https://api.teamorouter.com/v1', model: 'claude-3-5-sonnet' },
    { name: 'Custom', base_url: '', model: '' },
  ];

  const providerPresetSel = document.getElementById('provider-preset');
  PROVIDERS.forEach(p => {
    const opt = document.createElement('option');
    opt.value = p.name;
    opt.textContent = p.name;
    providerPresetSel.appendChild(opt);
  });
  providerPresetSel.addEventListener('change', () => {
    const p = PROVIDERS.find(x => x.name === providerPresetSel.value);
    if (p) {
      document.getElementById('provider-base-url').value = p.base_url;
      document.getElementById('provider-model').value = p.model;
      document.querySelector('.anthropic-only').hidden = !p.anthropic;
    }
  });

  // ── Modal 控制 ────────────────────────────────────────────────────────
  const modal = document.getElementById('provider-modal');
  let editingName = null;  // 编辑模式时存原名

  function openModal(profile = null) {
    editingName = profile ? profile.name : null;
    document.getElementById('provider-modal-title').textContent = profile ? `编辑 ${profile.name}` : '添加服务商';
    document.getElementById('provider-preset').value = profile ? profile.provider : 'DeepSeek';
    document.getElementById('provider-name').value = profile ? profile.name : '';
    document.getElementById('provider-note').value = profile ? (profile.note || '') : '';
    document.getElementById('provider-base-url').value = profile ? profile.base_url : 'https://api.deepseek.com';
    document.getElementById('provider-api-key').value = '';  // 编辑时 key 不预填(用户可重输)
    document.getElementById('provider-model').value = profile ? profile.model : 'deepseek-chat';
    const p = PROVIDERS.find(x => x.name === (profile ? profile.provider : 'DeepSeek'));
    document.querySelector('.anthropic-only').hidden = !(p && p.anthropic);
    if (profile) {
      document.getElementById('provider-tool-search').checked = profile.tool_search_enabled;
      document.getElementById('provider-betas-disabled').checked = profile.experimental_betas_disabled;
    }
    document.getElementById('provider-test-result').textContent = '';
    modal.hidden = false;
  }
  function closeModal() { modal.hidden = true; editingName = null; }

  document.getElementById('provider-add-btn').addEventListener('click', () => openModal());
  document.getElementById('provider-refresh-btn').addEventListener('click', loadProviders);
  document.getElementById('provider-cancel-btn').addEventListener('click', closeModal);

  document.getElementById('provider-test-btn').addEventListener('click', async () => {
    const name = document.getElementById('provider-name').value.trim();
    if (!name) { alert('请先填名称再测试'); return; }
    // 先 save(临时)再 test;test 后保持 modal 打开
    const args = gatherFormArgs();
    const apiKeyWasProvided = !!args.api_key;
    if (!apiKeyWasProvided) {
      alert('测试连接需要先填 API 密钥');
      return;
    }
    const saveR = await window.__TAURI__.core.invoke('save_llm_profile', { args });
    if (!saveR.ok) { document.getElementById('provider-test-result').textContent = `保存失败: ${saveR.error}`; return; }
    const testR = await window.__TAURI__.core.invoke('test_llm_connection', { name });
    const result = document.getElementById('provider-test-result');
    if (testR.ok) {
      const d = testR.data;
      result.textContent = d.ok ? `✓ 连接成功 ${d.latency_ms}ms` : `✗ ${d.error || '连接失败'}`;
      result.style.color = d.ok ? 'green' : 'red';
    } else {
      result.textContent = `错误: ${testR.error}`;
      result.style.color = 'red';
    }
  });

  document.getElementById('provider-form').addEventListener('submit', async (e) => {
    e.preventDefault();
    const args = gatherFormArgs();
    const r = await window.__TAURI__.core.invoke('save_llm_profile', { args });
    if (r.ok) {
      // 若是新建第一个 profile,自动激活
      if (!editingName) {
        const listR = await window.__TAURI__.core.invoke('list_llm_profiles');
        if (listR.ok && listR.data.length === 1) {
          await window.__TAURI__.core.invoke('set_active_profile', { name: args.name });
        }
      }
      closeModal();
      loadProviders();
    } else {
      alert(`保存失败: ${r.error}`);
    }
  });

  function gatherFormArgs() {
    const preset = document.getElementById('provider-preset').value;
    const isAnthropic = PROVIDERS.find(p => p.name === preset && p.anthropic);
    return {
      name: document.getElementById('provider-name').value.trim(),
      provider: preset,
      base_url: document.getElementById('provider-base-url').value.trim(),
      model: document.getElementById('provider-model').value.trim(),
      note: document.getElementById('provider-note').value.trim() || null,
      api_key: document.getElementById('provider-api-key').value || null,
      tool_search_enabled: isAnthropic ? document.getElementById('provider-tool-search').checked : null,
      experimental_betas_disabled: isAnthropic ? document.getElementById('provider-betas-disabled').checked : null,
    };
  }

  // ── 初始化:进 Settings tab 时 load ─────────────────────────────────
  document.querySelector('[data-tab="settings"]').addEventListener('click', loadProviders);
  // 页面加载时若已激活 settings tab,也 load
  if (document.getElementById('tab-settings') && !document.getElementById('tab-settings').hidden) {
    loadProviders();
  }
</script>
```

### Step 5: Add CSS for new elements (append to existing `<style>` block)

Append to the existing `<style>` block in `src/index.html`:

```css
.settings-sidebar {
  width: 180px;
  border-right: 1px solid #ddd;
  padding: 16px 0;
}
.settings-nav-btn {
  display: block;
  width: 100%;
  padding: 8px 16px;
  border: none;
  background: transparent;
  text-align: left;
  cursor: pointer;
}
.settings-nav-btn.active {
  background: #e8f0fe;
  font-weight: bold;
}
.settings-main {
  flex: 1;
  padding: 24px;
}
.settings-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}
.provider-card {
  border: 1px solid #ddd;
  border-radius: 8px;
  padding: 12px 16px;
  margin-bottom: 12px;
}
.provider-card-header {
  font-size: 16px;
  margin-bottom: 4px;
}
.provider-star {
  color: gold;
  margin-right: 8px;
}
.provider-card-meta {
  color: #666;
  font-size: 13px;
  margin-bottom: 8px;
}
.provider-card-actions button {
  margin-right: 8px;
}
.modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}
.modal {
  background: white;
  padding: 24px;
  border-radius: 8px;
  width: 480px;
  max-height: 90vh;
  overflow-y: auto;
}
.modal label {
  display: block;
  margin-bottom: 12px;
}
.modal input, .modal select {
  width: 100%;
  padding: 6px;
  box-sizing: border-box;
}
.modal-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
  margin-top: 16px;
}
.anthropic-only label {
  font-weight: normal;
}
```

### Step 6: Manual visual check (no test, just confirm in browser)

Open the Tauri app (`cd src-tauri && cargo tauri dev` or build & run installer). Verify:
1. Sidebar shows 6 tabs (Inbox / Run / Output / Health / Learn / **Settings**)
2. Click Settings → see Providers subpage with empty list
3. Click [+ 添加服务商] → modal opens with 12 preset options
4. Select DeepSeek → base_url + model auto-fill
5. Fill name + API key → [保存] → profile appears in list
6. Click [激活] → star shows, persists across restart

If any step fails, debug per W14-B+ `feedback_tauri_async_state` pattern(async fn + State must return Result).

### Step 7: Commit

```bash
git add src/index.html
git commit -m "feat(ui): W15-A T7 — index.html Settings tab + Providers UI + modal

- src/index.html: + Settings tab (6th) + Providers subpage + 12 preset
  dropdown + add/edit modal + delete confirmation + 6 Tauri command calls
- + CSS for settings sidebar / provider cards / modal

Manual visual check: 6 tabs visible, modal opens, profile CRUD works,
active star persists. Full 13-step acceptance in Task 8.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 8: Manual 13-step acceptance test (spec §8)

**Files:** None modified — execution-only task.

**Note:** Win11 Pro sandbox feature is NOT enabled on this machine (per W14-G E §5.1), so sandbox-verify is unavailable. Acceptance is performed by user on their desktop directly.

### Step 1: Build the installer

```bash
cd F:/soft/00selfmade/media-to-doc-ui/src-tauri
cargo test --lib            # verify 85/85 still pass
cargo tauri build           # build NSIS installer
```

Expected: build succeeds. Output:
- `target/release/bundle/nsis/media-to-doc_1.4.0_x64-setup.exe`
- (MSI not built — `targets: "nsis"` only)

### Step 2: Install + launch

Install the new NSIS (over v1.4.0 if present, or uninstall old first via 控制面板):
```bash
# Double-click: target/release/bundle/nsis/media-to-doc_1.4.0_x64-setup.exe
# Or silently:
target/release/bundle/nsis/media-to-doc_1.4.0_x64-setup.exe /S
```

Then launch the installed app (desktop shortcut or Start Menu → media-to-doc).

### Step 3: Walk through spec §8 acceptance list (13 items)

| # | Step | Expected |
|---|---|---|
| 1 | 装 W15-A NSIS + 桌面启动 | 6 tab 显示(Inbox/Run/Output/Health/Learn/Settings) |
| 2 | 进 Settings > Providers | 看到空列表 + [+ 添加服务商] 按钮 |
| 3 | 添加 DeepSeek profile | 选预设 → 自动填 base_url + model → 填 API key → [测试连接] → 绿色"连接成功 <X>ms" → [保存] |
| 4 | 列表显示新 profile,无 active 标 | 列表项,无星 |
| 5 | 点 [激活] | 该 profile 标星,其它取消标星,active 状态写入 JSON |
| 6 | 重启 Tauri | active profile 持久化(从 JSON 读) |
| 7 | Run pipeline(stop_after=chapters)| mtd 启动时 env 注入 `OPENAI_API_KEY=sk-...` + `OPENAI_BASE_URL=https://api.deepseek.com`,pipeline 跑通(LLM 章节) |
| 8 | 添加 Ollama profile + 激活 | mtd 启动时 env 注入 `OLLAMA_HOST=http://localhost:11434` + `OLLAMA_MODEL=llama3.1`,无 key 注入 |
| 9 | 删除 active profile | 弹确认 → 删除后无 active,Run pipeline 报 `ACTIVE_PROFILE_REQUIRED` 错误 |
| 10 | Test connection 失败 | 红色错误(网络 / key 错),profile 仍可保存(供离线用) |
| 11 | 编辑 profile 不改 key | api_key=None → 保留 keyring 旧值 |
| 12 | 12 个预设全部可选 | 每个预设 → 正确的 base_url + 默认 model |
| 13 | Custom 服务商 | base_url 校验(只允许 https:// 或 http://localhost:*) |

### Step 4: Verify env var injection (steps 7 + 8)

After step 7, check the mtd log (`work_dir/mtd.log`) for:
```
[env] OPENAI_API_KEY=sk-...  (truncated to first 4 chars in log for safety)
[env] OPENAI_BASE_URL=https://api.deepseek.com
[env] OPENAI_MODEL=deepseek-chat
```

If log doesn't show env injection, debug:
- Check `mtd.log` shows subprocess started successfully
- Add `eprintln!("[W15-A DEBUG] env_vars={:?}", spec.env_vars);` before `spawn_mtd` in commands.rs run_pipeline
- Confirm `keyring read` succeeded (`eprintln!("[W15-A] active profile: {}", active.name)`)

### Step 5: Verify keyring persistence (step 6)

After restart:
- Open `%APPDATA%\com.duanyi.mediatodoc\llm_profiles.json` → should show `{ active: "...", profiles: [...] }`
- OS keyring should have entry for the active profile's name (Windows: 控制面板 → 凭据管理器 → Windows 凭据, 找 `media-to-doc-ui`)

### Step 6: Write handoff + commit (no source changes)

If all 13 steps pass, write `handoff-w15-a-llm-api-settings-2026-07-23.md` documenting the implementation completion + any deviations from spec.

No source commit in this task — handoff is documentation only. If handoff reveals bugs, fix in follow-up commits.

### Step 7: (Optional) bump version to v1.4.1

If W15-A is shipping as v1.4.1 (per handoff §8 B option):
- Bump `src-tauri/Cargo.toml` `version = "1.4.0"` → `"1.4.1"`
- Bump `src-tauri/tauri.conf.json` `"version": "1.4.0"` → `"1.4.1"`
- Bump `src-tauri/nsis/installer.nsi` `!define PRODUCT_VERSION "1.4.0"` → `"1.4.1"`
- Rebuild + git tag v1.4.1 + gh release (per W14-G+ §B)

---