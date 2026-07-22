//! Tauri 2 desktop shell for media-to-doc (W14-B+ 8 commands 实装)。
//!
//! 职责:
//! - 提供 desktop window + 后续 system tray
//! - 通过 8 个 Tauri commands 暴露 media_to_doc Python 能力给前端
//! - 4 个只读 FS commands + run/cancel 子进程 + get_run_metrics/list_runs(W14-B+)
//!
//! 参考:
//! - https://v2.tauri.app/start/
//! - CLAUDE.md §10 后续规划 v1.3 Phase 2
//! - ARCHITECTURE.md(本仓根目录)

use serde::Serialize;

mod commands;
mod python_bridge;
mod runner;
mod types;

pub use commands::{
  cancel_run, check_status, list_courses, list_outputs, list_running, read_lecture,
  resume_pipeline, run_pipeline, CancelResult, CheckStatusResult, CourseEntry,
  ListCoursesResult, ListOutputsResult, ListRunningResult, OutputsGroups, ReadLectureResult,
  StageStatus,
};
pub use python_bridge::{get_run_metrics, list_runs, probe, ProbeResult};
pub use runner::{RunPipelineResult, RunRegistry, RunningRun, SpawnSpec};
pub use types::{CommandResponse, SUPPORTED_EXTS};

#[derive(Debug, Clone, Serialize)]
pub struct AppInfo {
  name: &'static str,
  version: &'static str,
  /// media-to-doc Python 包版本(由 probe 探测)
  pub mtd_version: Option<String>,
  /// media_to_doc Python API 是否可 import(由 probe 探测)
  pub python_api_available: bool,
  /// mcp_server.main 是否可 import(由 probe 探测)
  pub mcp_server_available: bool,
  /// 探测的 media-to-doc 项目路径
  pub media_to_doc_project: String,
  /// 探测失败时的错误(成功时 None)
  pub probe_error: Option<String>,
}

/// 返回 app 元信息(给前端展示用)。
///
/// 内部 spawn uv run python 探测 mtd_version + Python API + MCP server。
#[tauri::command]
async fn app_info() -> AppInfo {
  let probe = python_bridge::probe().await;
  AppInfo {
    name: "media-to-doc UI",
    version: env!("CARGO_PKG_VERSION"),
    mtd_version: probe.mtd_version,
    python_api_available: probe.python_api_available,
    mcp_server_available: probe.mcp_server_available,
    media_to_doc_project: probe.media_to_doc_project,
    probe_error: probe.error,
  }
}

/// 当前阶段占位 ping。
#[tauri::command]
fn ping(message: String) -> String {
  format!("pong: {}", message)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  runner::init_registry();
  tauri::Builder::default()
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
    ])
    .run(tauri::generate_context!())
    .expect("error while running media-to-doc UI");
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn app_info_fields_are_sane() {
    let info = AppInfo {
      name: "media-to-doc UI",
      version: env!("CARGO_PKG_VERSION"),
      mtd_version: None,
      python_api_available: false,
      mcp_server_available: false,
      media_to_doc_project: String::new(),
      probe_error: None,
    };
    assert_eq!(info.name, "media-to-doc UI");
    assert!(!info.version.is_empty());
  }
}
