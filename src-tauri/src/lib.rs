//! Tauri 2 desktop shell for media-to-doc (W14-B 启动骨架)。
//!
//! 职责:
//! - 提供 desktop window + system tray
//! - 通过 Tauri commands 暴露 media_to_doc Python API 给前端
//! - 当前 W14-B 仅 hello world 骨架,W14-B+ 接入 mtd run / mcp 工具
//!
//! 参考:
//! - https://v2.tauri.app/start/
//! - CLAUDE.md §10 后续规划 v1.3 Phase 2

use serde::Serialize;

#[derive(Serialize)]
struct AppInfo {
  name: &'static str,
  version: &'static str,
  mtd_version: &'static str,  // 后端调 `uv run mtd --version` 取
  python_api_available: bool,
  mcp_server_available: bool,
}

/// 返回 app 元信息(给前端展示用)。
#[tauri::command]
fn app_info() -> AppInfo {
  AppInfo {
    name: "media-to-doc UI",
    version: env!("CARGO_PKG_VERSION"),
    // W14-B+ 实装:subprocess 调 `uv run mtd --version` 拿真实版本
    mtd_version: "(not yet wired — W14-B+)",
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
    .invoke_handler(tauri::generate_handler![app_info, ping])
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