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
mod types;

pub use commands::{
  check_status, list_courses, list_outputs, read_lecture, CheckStatusResult, CourseEntry,
  ListCoursesResult, ListOutputsResult, OutputsGroups, ReadLectureResult, StageStatus,
};
pub use types::{CommandResponse, SUPPORTED_EXTS};

#[derive(Serialize)]
struct AppInfo {
  name: &'static str,
  version: &'static str,
  mtd_version: &'static str,  // 后端调 `uv run mtd --version` 取 — T6 实装
  python_api_available: bool,
  mcp_server_available: bool,
}

/// 返回 app 元信息(给前端展示用)。
#[tauri::command]
fn app_info() -> AppInfo {
  AppInfo {
    name: "media-to-doc UI",
    version: env!("CARGO_PKG_VERSION"),
    // W14-B+ T6 实装:subprocess 调 `uv run mtd --version` 拿真实版本
    mtd_version: "(not yet wired — W14-B+ T6)",
    python_api_available: false,
    mcp_server_available: false,
  }
}

/// 当前阶段占位 ping。
#[tauri::command]
fn ping(message: String) -> String {
  format!("pong: {}", message)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
    ])
    .run(tauri::generate_context!())
    .expect("error while running media-to-doc UI");
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn app_info_fields_are_sane() {
    let info = app_info();
    assert_eq!(info.name, "media-to-doc UI");
    assert!(!info.version.is_empty());
  }
}
