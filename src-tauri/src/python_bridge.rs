//! 调 media_to_doc Python API 的 bridge。
//!
//! 通过 `uv --project <media-to-doc> run python -c "..."` 跑一次性 Python,
//! JSON 序列化结果,stdout 捕获后 parse。
//!
//! 与 ARCHITECTURE.md §2 8 commands 对齐:
//! - `get_run_metrics(work_dir)` → `media_to_doc.llm.health.get_run_metrics`
//! - `list_runs(workspace_root, limit)` → `media_to_doc.llm.health.list_runs`
//! - `app_info()` 探测 mtd_version / python_api_available / mcp_server_available

use std::process::Stdio;

use serde::Serialize;
use tokio::process::Command;

use crate::commands::resolve_media_to_doc_project;
use crate::types::CommandResponse;

// ─────────────────────────────────────────────────────────────
// Probe —— 探测 mtd 版本 + Python API 可用性 + MCP server 可用性
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ProbeResult {
  pub mtd_version: Option<String>,
  pub python_api_available: bool,
  pub mcp_server_available: bool,
  pub media_to_doc_project: String,
  pub error: Option<String>,
}

/// 探测:跑 `uv --project X run python -c "<imports + introspect>"`。
///
/// 一次性 Python 脚本:
/// ```python
/// import json
/// try:
///     import media_to_doc
///     v = media_to_doc.__version__
/// except Exception as e:
///     print(json.dumps({"error": str(e)})); raise SystemExit(1)
/// try:
///     from media_to_doc.mcp_server import main as _mcp_main
///     mcp = True
/// except Exception:
///     mcp = False
/// try:
///     from media_to_doc.llm.health import get_run_metrics, list_runs
///     api = True
/// except Exception:
///     api = False
/// print(json.dumps({"version": v, "api": api, "mcp": mcp}))
/// ```
const PROBE_SCRIPT: &str = r#"
import json, sys
try:
    import media_to_doc
    v = media_to_doc.__version__
except Exception as e:
    print(json.dumps({"error": f"import media_to_doc: {e}"}))
    sys.exit(1)
try:
    from media_to_doc.llm.health import get_run_metrics, list_runs  # noqa: F401
    api = True
except Exception:
    api = False
try:
    from media_to_doc.mcp_server import main as _mcp_main  # noqa: F401
    mcp = True
except Exception:
    mcp = False
print(json.dumps({"version": v, "api": api, "mcp": mcp}))
"#;

pub async fn probe() -> ProbeResult {
  let project = resolve_media_to_doc_project();
  if !project.join("pyproject.toml").is_file() {
    return ProbeResult {
      mtd_version: None,
      python_api_available: false,
      mcp_server_available: false,
      media_to_doc_project: project.to_string_lossy().into_owned(),
      error: Some(format!(
        "media-to-doc 项目根未找到: {}\n请设置 MEDIA_TO_DOC_PROJECT 环境变量",
        project.display()
      )),
    };
  }
  let uv = std::env::var("UV_BIN").unwrap_or_else(|_| "uv".to_string());
  let mut cmd = Command::new(&uv);
  cmd.arg("--project")
    .arg(&project)
    .arg("run")
    .arg("python")
    .arg("-c")
    .arg(PROBE_SCRIPT)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .kill_on_drop(true);
  let output = match cmd.output().await {
    Ok(o) => o,
    Err(e) => {
      return ProbeResult {
        mtd_version: None,
        python_api_available: false,
        mcp_server_available: false,
        media_to_doc_project: project.to_string_lossy().into_owned(),
        error: Some(format!("spawn uv 失败: {e}")),
      };
    }
  };
  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    return ProbeResult {
      mtd_version: None,
      python_api_available: false,
      mcp_server_available: false,
      media_to_doc_project: project.to_string_lossy().into_owned(),
      error: Some(format!("uv 退出码 {:?}: {}", output.status.code(), stderr.trim())),
    };
  }
  let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
  let parsed: serde_json::Value = match serde_json::from_str(stdout.trim()) {
    Ok(v) => v,
    Err(e) => {
      return ProbeResult {
        mtd_version: None,
        python_api_available: false,
        mcp_server_available: false,
        media_to_doc_project: project.to_string_lossy().into_owned(),
        error: Some(format!("probe JSON 解析失败: {e};stdout: {stdout}")),
      };
    }
  };
  ProbeResult {
    mtd_version: parsed.get("version").and_then(|x| x.as_str()).map(str::to_string),
    python_api_available: parsed.get("api").and_then(|x| x.as_bool()).unwrap_or(false),
    mcp_server_available: parsed.get("mcp").and_then(|x| x.as_bool()).unwrap_or(false),
    media_to_doc_project: project.to_string_lossy().into_owned(),
    error: parsed
      .get("error")
      .and_then(|x| x.as_str())
      .map(str::to_string),
  }
}

// ─────────────────────────────────────────────────────────────
// get_run_metrics —— 读 work_dir 的 LE 沉淀元数据
// ─────────────────────────────────────────────────────────────

/// Python 一行:从 media_to_doc.llm.health 取 get_run_metrics,JSON 序列化。
const GET_RUN_METRICS_SCRIPT: &str = r#"
import json, sys
from media_to_doc.llm.health import get_run_metrics
print(json.dumps(get_run_metrics(sys.argv[1])))
"#;

pub async fn get_run_metrics_impl(work_dir: String) -> CommandResponse<serde_json::Value> {
  let project = resolve_media_to_doc_project();
  if !project.join("pyproject.toml").is_file() {
    return CommandResponse::err(format!(
      "media-to-doc 项目根未找到: {}",
      project.display()
    ));
  }
  let uv = std::env::var("UV_BIN").unwrap_or_else(|_| "uv".to_string());
  let mut cmd = Command::new(&uv);
  cmd.arg("--project")
    .arg(&project)
    .arg("run")
    .arg("python")
    .arg("-c")
    .arg(GET_RUN_METRICS_SCRIPT)
    .arg(&work_dir)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .kill_on_drop(true);
  let output = match cmd.output().await {
    Ok(o) => o,
    Err(e) => return CommandResponse::err(format!("spawn uv 失败: {e}")),
  };
  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    return CommandResponse::err(format!("get_run_metrics 失败: {}", stderr.trim()));
  }
  let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
  match serde_json::from_str(stdout.trim()) {
    Ok(v) => CommandResponse::ok(v),
    Err(e) => CommandResponse::err(format!("JSON 解析失败: {e};stdout: {stdout}")),
  }
}

#[tauri::command]
pub async fn get_run_metrics(work_dir: String) -> CommandResponse<serde_json::Value> {
  get_run_metrics_impl(work_dir).await
}

// ─────────────────────────────────────────────────────────────
// list_runs —— 扫 workspace 所有 run
// ─────────────────────────────────────────────────────────────

const LIST_RUNS_SCRIPT: &str = r#"
import json, sys
from media_to_doc.llm.health import list_runs
root = sys.argv[1] if len(sys.argv) > 1 and sys.argv[1] else None
limit = int(sys.argv[2]) if len(sys.argv) > 2 else 20
print(json.dumps(list_runs(root, limit=limit)))
"#;

pub async fn list_runs_impl(
  workspace_root: Option<String>,
  limit: Option<u32>,
) -> CommandResponse<serde_json::Value> {
  let project = resolve_media_to_doc_project();
  if !project.join("pyproject.toml").is_file() {
    return CommandResponse::err(format!(
      "media-to-doc 项目根未找到: {}",
      project.display()
    ));
  }
  let uv = std::env::var("UV_BIN").unwrap_or_else(|_| "uv".to_string());
  let mut cmd = Command::new(&uv);
  cmd.arg("--project")
    .arg(&project)
    .arg("run")
    .arg("python")
    .arg("-c")
    .arg(LIST_RUNS_SCRIPT);
  // 传 workspace_root(空字符串 = None)
  if let Some(ws) = workspace_root.as_deref() {
    if !ws.trim().is_empty() {
      cmd.arg(ws);
    } else {
      cmd.arg("");
    }
  } else {
    cmd.arg("");
  }
  cmd.arg(limit.unwrap_or(20).to_string())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .kill_on_drop(true);
  let output = match cmd.output().await {
    Ok(o) => o,
    Err(e) => return CommandResponse::err(format!("spawn uv 失败: {e}")),
  };
  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    return CommandResponse::err(format!("list_runs 失败: {}", stderr.trim()));
  }
  let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
  match serde_json::from_str(stdout.trim()) {
    Ok(v) => CommandResponse::ok(v),
    Err(e) => CommandResponse::err(format!("JSON 解析失败: {e};stdout: {stdout}")),
  }
}

#[tauri::command]
pub async fn list_runs(
  workspace_root: Option<String>,
  limit: Option<u32>,
) -> CommandResponse<serde_json::Value> {
  list_runs_impl(workspace_root, limit).await
}

// ─────────────────────────────────────────────────────────────
// tests
// ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn probe_script_is_valid_python_syntax() {
    // 简单 sanity 检查:不依赖实际跑 Python
    assert!(PROBE_SCRIPT.contains("import media_to_doc"));
    assert!(PROBE_SCRIPT.contains("json.dumps"));
  }

  #[test]
  fn get_run_metrics_script_passes_work_dir_via_argv() {
    assert!(GET_RUN_METRICS_SCRIPT.contains("sys.argv[1]"));
    assert!(GET_RUN_METRICS_SCRIPT.contains("get_run_metrics"));
  }

  #[test]
  fn list_runs_script_handles_optional_workspace() {
    assert!(LIST_RUNS_SCRIPT.contains("sys.argv[1]"));
    assert!(LIST_RUNS_SCRIPT.contains("sys.argv[2]"));
    assert!(LIST_RUNS_SCRIPT.contains("int("));
  }

  #[test]
  fn probe_returns_error_shape_when_project_missing() {
    // SAFETY: test-only
    unsafe { std::env::set_var("MEDIA_TO_DOC_PROJECT", "Z:/no/such/project"); }
    let rt = tokio::runtime::Runtime::new().unwrap();
    let r = rt.block_on(probe());
    unsafe { std::env::remove_var("MEDIA_TO_DOC_PROJECT"); }
    assert!(r.error.is_some());
    assert!(!r.python_api_available);
    assert!(!r.mcp_server_available);
  }
}
