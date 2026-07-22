//! Tauri commands 的统一返回壳(对齐 media-to-doc MCP 8 工具的 error 语义)。
//!
//! 形状:`{"ok": bool, "data": T?, "error": String?}`。
//! 成功时 error=None;失败时 data=None,error 携带可读消息。
//! Tauri 2 通过 serde 序列化直接返回给 WebView 端。

use serde::Serialize;

/// 统一响应壳。所有 `#[tauri::command]` 都返回这个类型(或带具体 data 的版本)。
#[derive(Debug, Clone, Serialize)]
pub struct CommandResponse<T: Serialize> {
  /// 是否成功
  pub ok: bool,
  /// 成功时的数据载荷
  pub data: Option<T>,
  /// 失败时的错误描述
  pub error: Option<String>,
}

impl<T: Serialize> CommandResponse<T> {
  /// 构造成功响应。
  pub fn ok(data: T) -> Self {
    Self {
      ok: true,
      data: Some(data),
      error: None,
    }
  }

  /// 构造失败响应(快捷方式)。
  pub fn err(msg: impl Into<String>) -> Self {
    Self {
      ok: false,
      data: None,
      error: Some(msg.into()),
    }
  }
}

/// 单视频 stem 派生默认(fallback,跟 Python 版 `tool_list_outputs` 一致)。
pub fn derive_stem(work_dir: &std::path::Path) -> String {
  let chapters_raw = work_dir.join("chapters").join("raw");
  if chapters_raw.is_dir() {
    if let Some(entry) = std::fs::read_dir(&chapters_raw)
      .ok()
      .and_then(|rd| rd.flatten().find(|e| e.path().is_dir()))
    {
      if let Some(name) = entry.file_name().to_str() {
        return name.to_string();
      }
    }
  }
  "output".to_string()
}

/// media-to-doc 支持的音视频后缀(跟 `pipeline.audio.SUPPORTED_EXTS` 对齐)。
pub const SUPPORTED_EXTS: &[&str] = &[
  "mp4", "mov", "mkv", "avi", "webm", "flv", "wmv", "m4v", "mpg", "mpeg",
  "mp3", "wav", "m4a", "flac", "aac", "ogg", "opus", "wma",
  "jpg", "jpeg", "png", "gif", "bmp", "webp",
];

/// 判断文件后缀是否在 SUPPORTED_EXTS 中(大小写不敏感)。
pub fn is_media_file(path: &std::path::Path) -> bool {
  path
    .extension()
    .and_then(|e| e.to_str())
    .map(|e| SUPPORTED_EXTS.iter().any(|s| s.eq_ignore_ascii_case(e)))
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::PathBuf;

  #[test]
  fn response_ok_shape() {
    let r: CommandResponse<i32> = CommandResponse::ok(42);
    let j = serde_json::to_value(&r).unwrap();
    assert_eq!(j["ok"], true);
    assert_eq!(j["data"], 42);
    assert!(j["error"].is_null());
  }

  #[test]
  fn response_err_shape() {
    let r: CommandResponse<i32> = CommandResponse::err("boom");
    let j = serde_json::to_value(&r).unwrap();
    assert_eq!(j["ok"], false);
    assert!(j["data"].is_null());
    assert_eq!(j["error"], "boom");
  }

  #[test]
  fn supported_exts_recognises_common_video() {
    assert!(is_media_file(&PathBuf::from("a.mp4")));
    assert!(is_media_file(&PathBuf::from("A.MP4")));
    assert!(is_media_file(&PathBuf::from("x.mov")));
    assert!(!is_media_file(&PathBuf::from("nope.txt")));
    assert!(!is_media_file(&PathBuf::from("noext")));
  }

  #[test]
  fn derive_stem_falls_back_to_output() {
    let tmp = tempdir_in_cwd("types_test");
    assert_eq!(derive_stem(&tmp), "output");
    let _ = std::fs::remove_dir_all(&tmp);
  }

  fn tempdir_in_cwd(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
  }
}
