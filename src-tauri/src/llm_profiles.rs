//! LLM 服务商模板与 base_url/model 校验。
//!
//! Task 2:9 个内置服务商、名称映射和输入校验。
//! Task 3:profile metadata 持久化(JSON IO + 原子写)和 env var 映射。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
      default_base_url: "https://api.minimaxi.com/v1",
      default_model: "MiniMax-M3",
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
    "Custom" => Some(Provider::Custom),
    _ => None,
  }
}

pub fn provider_name(provider: Provider) -> &'static str {
  match provider {
    Provider::Anthropic => "Anthropic",
    Provider::OpenAI => "OpenAI",
    Provider::Ollama => "Ollama",
    Provider::LmStudio => "LM Studio",
    Provider::DeepSeek => "DeepSeek",
    Provider::Zhipu => "Zhipu GLM",
    Provider::Kimi => "Kimi",
    Provider::MiniMax => "MiniMax",
    Provider::Custom => "Custom",
  }
}

/// 仅允许 HTTPS 远端地址，或带端口的 localhost / IPv4 loopback HTTP 地址。
pub fn validate_base_url(url: &str) -> Result<(), String> {
  let trimmed = url.trim();
  if trimmed.is_empty() {
    return Err("INVALID_BASE_URL: 不能为空".into());
  }

  if let Some(rest) = trimmed.strip_prefix("https://") {
    let authority_end = rest
      .find(|c: char| c == '/' || c == '?' || c == '#')
      .unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    if !authority.is_empty()
      && !authority.contains(char::is_whitespace)
      && !authority.contains('@')
    {
      return Ok(());
    }
  }

  for prefix in ["http://localhost:", "http://127.0.0.1:"] {
    if let Some(remainder) = trimmed.strip_prefix(prefix) {
      let port = remainder
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
      if !port.is_empty() && port.parse::<u16>().is_ok() {
        return Ok(());
      }
    }
  }

  Err(format!(
    "INVALID_BASE_URL: 仅支持 https:// 或 http://localhost:* / http://127.0.0.1:*, 实际: {trimmed}"
  ))
}

/// model 必须为非空字符串，trim 后长度不超过 200 个字符。
pub fn validate_model(model: &str) -> Result<(), String> {
  let trimmed = model.trim();
  if trimmed.is_empty() {
    return Err("INVALID_MODEL: 不能为空".into());
  }

  let length = trimmed.chars().count();
  if length > 200 {
    return Err(format!("INVALID_MODEL: 长度 {length} 超过 200"));
  }

  Ok(())
}

// ── Task 3: profile metadata + env var ─────────────────────────────────────

/// 单个 profile 元数据(API key **不**进 metadata,统一存 OS keyring)。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProfileMeta {
  pub name: String,
  /// display_name 字符串(JSON 友好,不是 Provider enum)
  pub provider: String,
  pub base_url: String,
  pub model: String,
  #[serde(default)]
  pub note: Option<String>,
  #[serde(default)]
  pub tool_search_enabled: bool,
  #[serde(default)]
  pub experimental_betas_disabled: bool,
  /// RFC3339-ish ISO8601 字符串(用 chrono / SystemTime 都行,这里简单 String)
  pub created_at: String,
}

/// 完整 metadata 文件内容:active 名字 + profiles 列表。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MetadataFile {
  #[serde(default)]
  pub active: Option<String>,
  #[serde(default)]
  pub profiles: Vec<ProfileMeta>,
}

impl Default for MetadataFile {
  fn default() -> Self {
    Self {
      active: None,
      profiles: Vec::new(),
    }
  }
}

/// metadata JSON 文件路径。匹配 `tauri.conf.json` identifier = `com.duanyi.mediatodoc`。
///
/// 平台行为:
/// - Windows:`%APPDATA%\com.duanyi.mediatodoc\llm_profiles.json`
/// - macOS / Linux:`dirs::config_dir()` 提供的 XDG-aware 路径
///   - macOS:`~/Library/Application Support/com.duanyi.mediatodoc/llm_profiles.json`
///   - Linux:`$XDG_CONFIG_HOME/com.duanyi.mediatodoc/llm_profiles.json` 或 `~/.config/com.duanyi.mediatodoc/llm_profiles.json`
/// - fallback:当前目录(测试用)
pub fn metadata_path() -> PathBuf {
  #[cfg(windows)]
  {
    if let Ok(appdata) = std::env::var("APPDATA") {
      return PathBuf::from(appdata)
        .join("com.duanyi.mediatodoc")
        .join("llm_profiles.json");
    }
  }
  #[cfg(any(target_os = "macos", target_os = "linux"))]
  {
    if let Some(config) = dirs::config_dir() {
      return config
        .join("com.duanyi.mediatodoc")
        .join("llm_profiles.json");
    }
  }
  PathBuf::from("llm_profiles.json")
}

/// 读 metadata JSON。文件不存在时返回默认空 MetadataFile,不报错。
pub fn load_profiles() -> Result<MetadataFile, String> {
  load_profiles_from(&metadata_path())
}

/// 读 metadata JSON(从指定路径)。文件不存在时返回默认空 MetadataFile,不报错。
///
/// 暴露 path 参数便于测试使用 tmpdir,避免污染用户真实配置目录。
pub fn load_profiles_from(path: &Path) -> Result<MetadataFile, String> {
  if !path.exists() {
    return Ok(MetadataFile::default());
  }
  let s = std::fs::read_to_string(path)
    .map_err(|e| format!("读 metadata 失败: {e}"))?;
  serde_json::from_str(&s).map_err(|e| format!("解析 metadata 失败: {e}"))
}

/// 写 metadata JSON。原子写(写临时文件 + rename)。
pub fn save_profiles(m: &MetadataFile) -> Result<(), String> {
  save_profiles_to(&metadata_path(), m)
}

/// 写 metadata JSON(到指定路径)。原子写:先写 `.json.tmp`,再 rename 覆盖。
///
/// 暴露 path 参数便于测试使用 tmpdir。
pub fn save_profiles_to(path: &Path, m: &MetadataFile) -> Result<(), String> {
  if let Some(parent) = path.parent() {
    std::fs::create_dir_all(parent)
      .map_err(|e| format!("建 metadata 目录失败: {e}"))?;
  }
  let tmp = path.with_extension("json.tmp");
  let json =
    serde_json::to_string_pretty(m).map_err(|e| format!("序列化 metadata 失败: {e}"))?;
  std::fs::write(&tmp, json).map_err(|e| format!("写 metadata tmp 失败: {e}"))?;
  if let Err(e) = std::fs::rename(&tmp, path) {
    // rename 失败时清理残留 tmp,避免用户看到脏文件
    let _ = std::fs::remove_file(&tmp);
    return Err(format!("rename metadata 失败: {e}"));
  }
  Ok(())
}

/// 取 active profile。`active` 为 None 或名字找不到时返回 `ACTIVE_PROFILE_REQUIRED:`。
pub fn get_active_profile() -> Result<ProfileMeta, String> {
  get_active_profile_in(&load_profiles()?)
}

/// 取 active profile(从已有 metadata)。`active` 为 None 或名字找不到时返回 `ACTIVE_PROFILE_REQUIRED:`。
///
/// 暴露 `&MetadataFile` 参数便于测试,避免依赖全局文件状态。
pub fn get_active_profile_in(m: &MetadataFile) -> Result<ProfileMeta, String> {
  let name = m.active.as_ref().ok_or_else(|| {
    "ACTIVE_PROFILE_REQUIRED: 无 active profile,请先在 Settings > Providers 设置一个".to_string()
  })?;
  m
    .profiles
    .iter()
    .find(|p| p.name == *name)
    .cloned()
    .ok_or_else(|| {
      format!(
        "ACTIVE_PROFILE_REQUIRED: active profile '{name}' 在 metadata 中不存在"
      )
    })
}

/// 构造 test_connection 用的探测 URL + headers。
///
/// 按 provider 派发:
/// - Anthropic:`<base_url>/v1/models`,headers = {x-api-key, anthropic-version}
/// - OpenAI Compat(OpenAI / LM Studio / DeepSeek / Zhipu / Kimi / MiniMax / Custom):
///   `<base_url>/models`,headers = {Authorization: Bearer <key>};空 base_url 报错
/// - Ollama:`<base_url>/api/tags`(无 key header);空 base_url 报错
///
/// 错误:未知 provider → `PROVIDER_NOT_FOUND:`;空 base_url(对 OpenAI Compat / Ollama)→ `INVALID_BASE_URL:`
pub fn probe_endpoint(
  meta: &ProfileMeta,
  key: &str,
) -> Result<(String, HashMap<String, String>), String> {
  let p = provider_from_name(&meta.provider).ok_or_else(|| {
    format!("PROVIDER_NOT_FOUND: {}", meta.provider)
  })?;

  let mut headers: HashMap<String, String> = HashMap::new();
  let base = meta.base_url.trim();

  let url = match p {
    Provider::Anthropic => {
      headers.insert("x-api-key".into(), key.to_string());
      headers.insert("anthropic-version".into(), "2023-06-01".into());
      let resolved = if base.is_empty() {
        "https://api.anthropic.com"
      } else {
        base
      };
      format!("{}/v1/models", resolved.trim_end_matches('/'))
    }
    Provider::Ollama => {
      if base.is_empty() {
        return Err("INVALID_BASE_URL: Ollama base_url 不能为空".into());
      }
      format!("{}/api/tags", base.trim_end_matches('/'))
    }
    Provider::OpenAI
    | Provider::LmStudio
    | Provider::DeepSeek
    | Provider::Zhipu
    | Provider::Kimi
    | Provider::MiniMax
    | Provider::Custom => {
      if base.is_empty() {
        return Err("INVALID_BASE_URL: OpenAI 兼容 base_url 不能为空".into());
      }
      if !key.is_empty() {
        headers.insert("Authorization".into(), format!("Bearer {key}"));
      }
      format!("{}/models", base.trim_end_matches('/'))
    }
  };
  Ok((url, headers))
}

/// 把 ProfileMeta + API key 翻译成 mtd 子进程要注入的 env vars。
///
/// 规则:
/// - Anthropic:`ANTHROPIC_API_KEY=<key>`,`ANTHROPIC_BASE_URL=<base_url>`(若非默认)
/// - OpenAI Compat(OpenAI / DeepSeek / Zhipu / Kimi / MiniMax / LM Studio / Custom):
///   `OPENAI_API_KEY=<key>`,`OPENAI_BASE_URL=<base_url>`(若非空),`OPENAI_MODEL=<model>`(若非空)
/// - Ollama:`OLLAMA_HOST=<base_url>`(总设),`OLLAMA_MODEL=<model>`(若非空);**不注入 API key**
///
/// 错误:未知 provider 名时返回 `PROVIDER_NOT_FOUND:`。
pub fn to_env_vars(
  meta: &ProfileMeta,
  key: &str,
) -> Result<HashMap<String, String>, String> {
  let mut env = HashMap::new();
  let p = provider_from_name(&meta.provider).ok_or_else(|| {
    format!("PROVIDER_NOT_FOUND: {}", meta.provider)
  })?;

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

  Ok(env)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn template(provider: Provider) -> ProviderTemplate {
    all_templates()
      .into_iter()
      .find(|template| template.enum_value == provider)
      .expect("provider template should exist")
  }

  #[test]
  fn template_anthropic_has_correct_fields() {
    let template = template(Provider::Anthropic);
    assert_eq!(template.display_name, "Anthropic");
    assert_eq!(template.default_base_url, "https://api.anthropic.com");
    assert_eq!(template.default_model, "claude-sonnet-4-5");
    assert_eq!(template.protocol, Protocol::AnthropicSdk);
    assert_eq!(
      template.env_var_keys,
      &["ANTHROPIC_API_KEY", "ANTHROPIC_BASE_URL"]
    );
  }

  #[test]
  fn template_openai_has_correct_fields() {
    let template = template(Provider::OpenAI);
    assert_eq!(template.display_name, "OpenAI");
    assert_eq!(template.default_base_url, "https://api.openai.com/v1");
    assert_eq!(template.default_model, "gpt-4o");
    assert_eq!(template.protocol, Protocol::OpenAiCompat);
    assert_eq!(
      template.env_var_keys,
      &["OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_MODEL"]
    );
  }

  #[test]
  fn template_ollama_has_correct_fields() {
    let template = template(Provider::Ollama);
    assert_eq!(template.display_name, "Ollama");
    assert_eq!(template.default_base_url, "http://localhost:11434");
    assert_eq!(template.default_model, "llama3.1");
    assert_eq!(template.protocol, Protocol::OllamaNative);
    assert_eq!(template.env_var_keys, &["OLLAMA_HOST", "OLLAMA_MODEL"]);
  }

  #[test]
  fn template_lm_studio_has_correct_fields() {
    let template = template(Provider::LmStudio);
    assert_eq!(template.display_name, "LM Studio");
    assert_eq!(template.default_base_url, "http://localhost:1234/v1");
    assert_eq!(template.default_model, "loaded-model");
    assert_eq!(template.protocol, Protocol::OpenAiCompat);
    assert_eq!(
      template.env_var_keys,
      &["OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_MODEL"]
    );
  }

  #[test]
  fn template_deepseek_has_correct_fields() {
    let template = template(Provider::DeepSeek);
    assert_eq!(template.display_name, "DeepSeek");
    assert_eq!(template.default_base_url, "https://api.deepseek.com");
    assert_eq!(template.default_model, "deepseek-chat");
    assert_eq!(template.protocol, Protocol::OpenAiCompat);
    assert_eq!(
      template.env_var_keys,
      &["OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_MODEL"]
    );
  }

  #[test]
  fn template_zhipu_has_correct_fields() {
    let template = template(Provider::Zhipu);
    assert_eq!(template.display_name, "Zhipu GLM");
    assert_eq!(
      template.default_base_url,
      "https://open.bigmodel.cn/api/paas/v4"
    );
    assert_eq!(template.default_model, "glm-4-plus");
    assert_eq!(template.protocol, Protocol::OpenAiCompat);
    assert_eq!(
      template.env_var_keys,
      &["OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_MODEL"]
    );
  }

  #[test]
  fn template_kimi_has_correct_fields() {
    let template = template(Provider::Kimi);
    assert_eq!(template.display_name, "Kimi");
    assert_eq!(template.default_base_url, "https://api.moonshot.cn/v1");
    assert_eq!(template.default_model, "moonshot-v1-128k");
    assert_eq!(template.protocol, Protocol::OpenAiCompat);
    assert_eq!(
      template.env_var_keys,
      &["OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_MODEL"]
    );
  }

  #[test]
  fn template_minimax_has_correct_fields() {
    let template = template(Provider::MiniMax);
    assert_eq!(template.display_name, "MiniMax");
    assert_eq!(template.default_base_url, "https://api.minimaxi.com/v1");
    assert_eq!(template.default_model, "MiniMax-M3");
    assert_eq!(template.protocol, Protocol::OpenAiCompat);
    assert_eq!(
      template.env_var_keys,
      &["OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_MODEL"]
    );
  }

  #[test]
  fn template_custom_has_correct_fields() {
    let template = template(Provider::Custom);
    assert_eq!(template.display_name, "Custom");
    assert_eq!(template.default_base_url, "");
    assert_eq!(template.default_model, "");
    assert_eq!(template.protocol, Protocol::OpenAiCompat);
    assert_eq!(
      template.env_var_keys,
      &["OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_MODEL"]
    );
  }

  #[test]
  fn validate_base_url_accepts_https() {
    assert!(validate_base_url("https://api.deepseek.com").is_ok());
    assert!(validate_base_url("  https://api.openai.com/v1  ").is_ok());
  }

  #[test]
  fn validate_base_url_accepts_localhost_loopback() {
    assert!(validate_base_url("http://localhost:11434").is_ok());
    assert!(validate_base_url("http://localhost:1234/v1").is_ok());
    assert!(validate_base_url("http://127.0.0.1:8080").is_ok());
  }

  #[test]
  fn validate_base_url_rejects_unsafe_values() {
    for url in [
      "",
      "ftp://api.example.com",
      "javascript:alert(1)",
      "file:///etc/passwd",
      "http://example.com",
      "http://192.168.1.1:8080",
      "http://localhost.evil.example:11434",
      "https://?query",
      "https://#fragment",
      "https:///path",
    ] {
      let error = validate_base_url(url).expect_err("unsafe URL should be rejected");
      assert!(error.starts_with("INVALID_BASE_URL:"), "error: {error}");
    }
  }

  #[test]
  fn validate_model_accepts_non_empty_model_up_to_200_chars() {
    assert!(validate_model("claude-sonnet-4-5").is_ok());
    assert!(validate_model("  gpt-4o  ").is_ok());
    assert!(validate_model(&"m".repeat(200)).is_ok());
  }

  #[test]
  fn validate_model_rejects_empty_or_too_long() {
    for model in ["", "   "] {
      let error = validate_model(model).expect_err("blank model should be rejected");
      assert!(error.starts_with("INVALID_MODEL:"), "error: {error}");
    }

    let error = validate_model(&"m".repeat(201)).expect_err("long model should be rejected");
    assert!(error.starts_with("INVALID_MODEL:"), "error: {error}");
    assert!(error.contains("201"), "error: {error}");
  }

  #[test]
  fn provider_names_roundtrip_for_all_nine() {
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
      (Custom, "Custom"),
    ];

    for (provider, expected_name) in cases {
      assert_eq!(provider_name(provider), expected_name);
      assert_eq!(provider_from_name(expected_name), Some(provider));
    }
    assert_eq!(provider_from_name("NotAProvider"), None);
  }

  #[test]
  fn all_templates_contains_exactly_nine_entries() {
    assert_eq!(all_templates().len(), 9);
  }

  #[test]
  fn all_templates_has_unique_providers() {
    let mut providers = Vec::new();
    for template in all_templates() {
      assert!(
        !providers.contains(&template.enum_value),
        "duplicate provider: {:?}",
        template.enum_value
      );
      providers.push(template.enum_value);
    }
  }

  // ── T3: metadata IO + env var(16 tests)─────────────────────────────────

  use std::path::PathBuf;

  fn make_meta(
    provider: &str,
    name: &str,
    base_url: &str,
    model: &str,
  ) -> ProfileMeta {
    ProfileMeta {
      name: name.into(),
      provider: provider.into(),
      base_url: base_url.into(),
      model: model.into(),
      note: None,
      tool_search_enabled: false,
      experimental_betas_disabled: false,
      created_at: "2026-07-24T00:00:00Z".into(),
    }
  }

  fn tmp_path(tag: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .map(|d| d.as_nanos())
      .unwrap_or(0);
    p.push(format!(
      "w15a-t3-{tag}-{}-{nanos}.json",
      std::process::id()
    ));
    p
  }

  #[test]
  fn metadata_path_contains_app_identifier() {
    let p = metadata_path();
    let s = p.to_string_lossy();
    assert!(
      s.contains("com.duanyi.mediatodoc"),
      "metadata path 应含 com.duanyi.mediatodoc, 实际: {s}"
    );
    assert!(
      s.ends_with("llm_profiles.json"),
      "应以 llm_profiles.json 结尾, 实际: {s}"
    );
  }

  #[test]
  fn load_profiles_returns_empty_when_file_missing() {
    // 用一个保证不存在的 tmp 路径,避免污染用户真实 APPDATA 配置
    let path = tmp_path("empty");
    let m = load_profiles_from(&path).expect("文件不存在应返回默认配置");
    assert!(m.active.is_none());
    assert!(m.profiles.is_empty());
  }

  #[test]
  fn load_profiles_returns_error_on_corrupt_json() {
    let path = tmp_path("corrupt");
    std::fs::write(&path, "{ this is not valid json").expect("write 应成功");
    let err = load_profiles_from(&path).expect_err("坏 JSON 应报错");
    assert!(
      err.starts_with("解析 metadata 失败"),
      "错误前缀应是'解析 metadata 失败',实际: {err}"
    );
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn save_and_load_roundtrip_preserves_data() {
    let path = tmp_path("roundtrip");
    let mut m = MetadataFile::default();
    m.active = Some("ollama-local".into());
    m.profiles.push(make_meta(
      "DeepSeek",
      "deepseek-prod",
      "https://api.deepseek.com",
      "deepseek-chat",
    ));
    m.profiles.push(make_meta(
      "Ollama",
      "ollama-local",
      "http://localhost:11434",
      "llama3.1",
    ));

    save_profiles_to(&path, &m).expect("save 应成功");
    let loaded = load_profiles_from(&path).expect("load 应成功");

    assert_eq!(loaded.active.as_deref(), Some("ollama-local"));
    assert_eq!(loaded.profiles.len(), 2);
    assert_eq!(loaded.profiles[0].name, "deepseek-prod");
    assert_eq!(loaded.profiles[1].provider, "Ollama");
    assert_eq!(loaded.profiles[1].base_url, "http://localhost:11434");
    assert_eq!(loaded.profiles[1].model, "llama3.1");

    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn save_profiles_creates_missing_parent_dir() {
    let mut path = tmp_path("parent-create");
    path.pop();
    path.push("w15a-t3-nested");
    path.push("never-existed");
    path.push("llm_profiles.json");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());

    let mut m = MetadataFile::default();
    m.profiles.push(make_meta(
      "DeepSeek",
      "ds",
      "https://api.deepseek.com",
      "deepseek-chat",
    ));
    save_profiles_to(&path, &m).expect("save 应自动创建父目录");

    assert!(path.exists());
    let loaded = load_profiles_from(&path).expect("load 应成功");
    assert_eq!(loaded.profiles.len(), 1);

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
  }

  #[test]
  fn save_profiles_atomic_no_tmp_leftover() {
    let path = tmp_path("atomic");
    let mut m = MetadataFile::default();
    m.profiles.push(make_meta(
      "Ollama",
      "ollama-local",
      "http://localhost:11434",
      "llama3.1",
    ));
    save_profiles_to(&path, &m).expect("save 应成功");

    let tmp = path.with_extension("json.tmp");
    assert!(
      !tmp.exists(),
      "save 后不应残留 .tmp 文件, 但发现: {tmp:?}"
    );

    // 二次写入也应不残留 .tmp
    m.profiles.push(make_meta(
      "DeepSeek",
      "ds",
      "https://api.deepseek.com",
      "deepseek-chat",
    ));
    save_profiles_to(&path, &m).expect("二次 save 应成功");
    assert!(!tmp.exists(), "二次 save 后也不应残留 .tmp");

    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn get_active_profile_in_errors_when_no_active_set() {
    let m = MetadataFile::default();
    let err = get_active_profile_in(&m).expect_err("active 为 None 应报错");
    assert!(
      err.starts_with("ACTIVE_PROFILE_REQUIRED:"),
      "错误前缀应是 ACTIVE_PROFILE_REQUIRED:, 实际: {err}"
    );
  }

  #[test]
  fn get_active_profile_in_errors_when_active_name_not_in_profiles() {
    let mut m = MetadataFile::default();
    m.active = Some("ghost".into());
    m.profiles.push(make_meta(
      "DeepSeek",
      "ds",
      "https://api.deepseek.com",
      "deepseek-chat",
    ));
    let err = get_active_profile_in(&m).expect_err("active 名找不到应报错");
    assert!(
      err.starts_with("ACTIVE_PROFILE_REQUIRED:"),
      "错误前缀应是 ACTIVE_PROFILE_REQUIRED:, 实际: {err}"
    );
    assert!(err.contains("ghost"), "错误应指明找不到的名字: {err}");
  }

  #[test]
  fn get_active_profile_in_returns_matching_profile() {
    let mut m = MetadataFile::default();
    m.profiles.push(make_meta(
      "DeepSeek",
      "deepseek-prod",
      "https://api.deepseek.com",
      "deepseek-chat",
    ));
    m.profiles.push(make_meta(
      "Ollama",
      "ollama-local",
      "http://localhost:11434",
      "llama3.1",
    ));
    m.active = Some("ollama-local".into());

    let active = get_active_profile_in(&m).expect("应能找到 active profile");
    assert_eq!(active.provider, "Ollama");
    assert_eq!(active.model, "llama3.1");
    assert_eq!(active.base_url, "http://localhost:11434");
  }

  #[test]
  fn to_env_vars_anthropic_default_omits_base_url() {
    let meta = make_meta(
      "Anthropic",
      "anthropic-prod",
      "https://api.anthropic.com",
      "claude-sonnet-4-5",
    );
    let env = to_env_vars(&meta, "sk-ant-test").expect("Anthropic 已知 provider 应 OK");
    assert_eq!(
      env.get("ANTHROPIC_API_KEY").map(String::as_str),
      Some("sk-ant-test")
    );
    assert!(
      env.get("ANTHROPIC_BASE_URL").is_none(),
      "默认 endpoint 不应注入 ANTHROPIC_BASE_URL"
    );
    assert!(env.get("OPENAI_API_KEY").is_none());
    assert!(env.get("OLLAMA_HOST").is_none());
  }

  #[test]
  fn to_env_vars_anthropic_custom_base_url_sets_it() {
    let meta = make_meta(
      "Anthropic",
      "anthropic-custom",
      "https://proxy.example.com",
      "claude-sonnet-4-5",
    );
    let env = to_env_vars(&meta, "sk-ant-test").expect("Anthropic 自定义 endpoint 应 OK");
    assert_eq!(
      env.get("ANTHROPIC_API_KEY").map(String::as_str),
      Some("sk-ant-test")
    );
    assert_eq!(
      env.get("ANTHROPIC_BASE_URL").map(String::as_str),
      Some("https://proxy.example.com")
    );
  }

  #[test]
  fn to_env_vars_openai_compat_basic() {
    let meta = make_meta(
      "DeepSeek",
      "deepseek-prod",
      "https://api.deepseek.com",
      "deepseek-chat",
    );
    let env = to_env_vars(&meta, "sk-ds-test").expect("DeepSeek 已知 provider 应 OK");
    assert_eq!(
      env.get("OPENAI_API_KEY").map(String::as_str),
      Some("sk-ds-test")
    );
    assert_eq!(
      env.get("OPENAI_BASE_URL").map(String::as_str),
      Some("https://api.deepseek.com")
    );
    assert_eq!(
      env.get("OPENAI_MODEL").map(String::as_str),
      Some("deepseek-chat")
    );
    assert!(env.get("ANTHROPIC_API_KEY").is_none());
    assert!(env.get("OLLAMA_HOST").is_none());
  }

  #[test]
  fn to_env_vars_openai_compat_omits_empty_optional_fields() {
    let meta = make_meta("DeepSeek", "ds-empty", "", "");
    let env = to_env_vars(&meta, "sk-ds-test").expect("DeepSeek 空 base/model 应 OK");
    assert_eq!(
      env.get("OPENAI_API_KEY").map(String::as_str),
      Some("sk-ds-test")
    );
    assert!(
      env.get("OPENAI_BASE_URL").is_none(),
      "空 base_url 不应注入"
    );
    assert!(env.get("OPENAI_MODEL").is_none(), "空 model 不应注入");
  }

  #[test]
  fn to_env_vars_ollama_no_api_key() {
    let meta = make_meta(
      "Ollama",
      "ollama-local",
      "http://localhost:11434",
      "llama3.1",
    );
    let env = to_env_vars(&meta, "ignored-by-ollama").expect("Ollama 应 OK");
    assert_eq!(
      env.get("OLLAMA_HOST").map(String::as_str),
      Some("http://localhost:11434")
    );
    assert_eq!(
      env.get("OLLAMA_MODEL").map(String::as_str),
      Some("llama3.1")
    );
    assert!(
      env.get("OPENAI_API_KEY").is_none(),
      "Ollama 不得注入 OPENAI_API_KEY"
      );
    assert!(
      env.get("ANTHROPIC_API_KEY").is_none(),
      "Ollama 不得注入 ANTHROPIC_API_KEY"
    );
  }

  #[test]
  fn to_env_vars_ollama_omits_empty_model() {
    let meta = make_meta(
      "Ollama",
      "ollama-default-model",
      "http://localhost:11434",
      "",
    );
    let env = to_env_vars(&meta, "ignored").expect("Ollama 应 OK");
    assert_eq!(
      env.get("OLLAMA_HOST").map(String::as_str),
      Some("http://localhost:11434")
    );
    assert!(
      env.get("OLLAMA_MODEL").is_none(),
      "空 model 不应注入 OLLAMA_MODEL"
    );
  }

  #[test]
  fn to_env_vars_minimax_uses_openai_compat_env_vars() {
    let meta = make_meta(
      "MiniMax",
      "minimax-prod",
      "https://api.minimaxi.com/v1",
      "MiniMax-M3",
    );
    let env = to_env_vars(&meta, "sk-mm-test").expect("MiniMax 应 OK");
    assert_eq!(
      env.get("OPENAI_API_KEY").map(String::as_str),
      Some("sk-mm-test")
    );
    assert_eq!(
      env.get("OPENAI_BASE_URL").map(String::as_str),
      Some("https://api.minimaxi.com/v1")
    );
    assert_eq!(
      env.get("OPENAI_MODEL").map(String::as_str),
      Some("MiniMax-M3")
    );
  }

  #[test]
  fn to_env_vars_unknown_provider_returns_PROVIDER_NOT_FOUND() {
    let meta = make_meta("MysteryProvider", "m", "https://x.example.com", "m");
    let err = to_env_vars(&meta, "sk-test").expect_err("未知 provider 应报错");
    assert!(
      err.starts_with("PROVIDER_NOT_FOUND:"),
      "错误前缀应是 PROVIDER_NOT_FOUND:, 实际: {err}"
    );
  }

  #[test]
  fn api_key_never_appears_in_metadata_serialization() {
    // 反向保险:确保 metadata JSON 序列化结果不包含任何 key 字面量
    let mut m = MetadataFile::default();
    m.profiles.push(make_meta(
      "DeepSeek",
      "ds",
      "https://api.deepseek.com",
      "deepseek-chat",
    ));
    let json = serde_json::to_string(&m).expect("serialize 应成功");
    assert!(!json.contains("sk-"), "metadata JSON 不应包含 sk- 前缀");
    assert!(
      !json.contains("api_key"),
      "metadata JSON 不应包含 api_key 字段名"
    );
    assert!(
      !json.contains("password"),
      "metadata JSON 不应包含 password 字段名"
    );
  }
}
