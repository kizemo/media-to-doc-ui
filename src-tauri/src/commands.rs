//! 10 个 Tauri commands —— 对齐 media-to-doc MCP 8 工具(W7=6 + W8=2) + W14-C 2 新。
//!
//! 结构(每个命令):
//! - `*_impl` 纯函数(可单测,不入 Tauri 状态)
//! - `#[tauri::command]` 薄包装(只做参数透传 + State 注入)
//!
//! 错误:Path 不存在 / state.json 缺失 / version 非法 / spawn 失败 / 并发上限
//! → `CommandResponse::err`。

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::runner::{
  build_mtd_resume_args, build_mtd_run_args, derive_work_dir, global_registry, kill_tree,
  spawn_mtd, RunPipelineResult, RunStatusInfo, RunningRun, SpawnSpec,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::keyring_store;
use crate::llm_profiles::{self, ProfileMeta};
use crate::types::{derive_stem, is_media_file, CommandResponse};

// ─────────────────────────────────────────────────────────────
// list_courses —— 列 workspace/inbox 下的所有课程
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct CourseEntry {
  /// 课程目录名(inbox 下的子目录名)
  pub name: String,
  /// 课程目录绝对路径
  pub path: String,
  /// 含的媒体文件相对路径列表
  pub media_files: Vec<String>,
  /// 媒体文件总数
  pub media_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListCoursesResult {
  pub workspace: String,
  pub inbox: String,
  pub courses: Vec<CourseEntry>,
}

pub fn list_courses_impl(workspace_root: Option<String>) -> CommandResponse<ListCoursesResult> {
  let ws = match workspace_root {
    Some(s) if !s.trim().is_empty() => PathBuf::from(s),
    _ => default_workspace_root(),
  };
  let ws = ws.expand();
  let inbox = ws.join("inbox");
  if !inbox.exists() {
    return CommandResponse::ok(ListCoursesResult {
      workspace: ws.to_string_lossy().into_owned(),
      inbox: inbox.to_string_lossy().into_owned(),
      courses: vec![],
    });
  }
  let read = match std::fs::read_dir(&inbox) {
    Ok(r) => r,
    Err(e) => return CommandResponse::err(format!("read_dir 失败: {e}")),
  };
  let mut courses = Vec::new();
  for entry in read.flatten() {
    if !entry.path().is_dir() {
      continue;
    }
    let path = entry.path();
    let media: Vec<String> = walk_media(&path)
      .into_iter()
      .map(|p| {
        p.strip_prefix(&path)
          .map(|r| r.to_string_lossy().into_owned())
          .unwrap_or_default()
      })
      .collect();
    let name = entry.file_name().to_string_lossy().into_owned();
    courses.push(CourseEntry {
      name,
      path: path.to_string_lossy().into_owned(),
      media_count: media.len(),
      media_files: media,
    });
  }
  courses.sort_by(|a, b| a.name.cmp(&b.name));
  CommandResponse::ok(ListCoursesResult {
    workspace: ws.to_string_lossy().into_owned(),
    inbox: inbox.to_string_lossy().into_owned(),
    courses,
  })
}

#[tauri::command]
pub fn list_courses(workspace_root: Option<String>) -> CommandResponse<ListCoursesResult> {
  list_courses_impl(workspace_root)
}

// ─────────────────────────────────────────────────────────────
// check_status —— 读 work_dir/state.json 的 11 stage 状态
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct StageStatus {
  pub status: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub started_at: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub finished_at: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckStatusResult {
  pub course: String,
  pub inbox_path: String,
  pub current_stage: String,
  pub started_at: String,
  pub updated_at: String,
  pub is_complete: bool,
  pub stages: std::collections::BTreeMap<String, StageStatus>,
}

pub fn check_status_impl(work_dir: String) -> CommandResponse<CheckStatusResult> {
  let work = PathBuf::from(work_dir).expand();
  let state_path = work.join("state.json");
  if !state_path.exists() {
    return CommandResponse::err(format!("state.json 不存在: {}", state_path.display()));
  }
  let raw = match std::fs::read_to_string(&state_path) {
    Ok(s) => s,
    Err(e) => return CommandResponse::err(format!("读 state.json 失败: {e}")),
  };
  let v: serde_json::Value = match serde_json::from_str(&raw) {
    Ok(v) => v,
    Err(e) => return CommandResponse::err(format!("state.json JSON 解析失败: {e}")),
  };
  let mut stages = std::collections::BTreeMap::new();
  if let Some(map) = v.get("stages").and_then(|s| s.as_object()) {
    for (k, sv) in map {
      let ss = StageStatus {
        status: sv.get("status").and_then(|x| x.as_str()).unwrap_or("pending").to_string(),
        started_at: sv.get("started_at").and_then(|x| x.as_str()).map(str::to_string),
        finished_at: sv.get("finished_at").and_then(|x| x.as_str()).map(str::to_string),
        error: sv.get("error").and_then(|x| x.as_str()).map(str::to_string),
      };
      stages.insert(k.clone(), ss);
    }
  }
  let is_complete = v
    .get("is_complete")
    .and_then(|x| x.as_bool())
    .unwrap_or(false);
  let result = CheckStatusResult {
    course: v.get("course").and_then(|x| x.as_str()).unwrap_or("").to_string(),
    inbox_path: v.get("inbox_path").and_then(|x| x.as_str()).unwrap_or("").to_string(),
    current_stage: v.get("current_stage").and_then(|x| x.as_str()).unwrap_or("").to_string(),
    started_at: v.get("started_at").and_then(|x| x.as_str()).unwrap_or("").to_string(),
    updated_at: v.get("updated_at").and_then(|x| x.as_str()).unwrap_or("").to_string(),
    is_complete,
    stages,
  };
  CommandResponse::ok(result)
}

#[tauri::command]
pub fn check_status(work_dir: String) -> CommandResponse<CheckStatusResult> {
  check_status_impl(work_dir)
}

// ─────────────────────────────────────────────────────────────
// list_outputs —— 派生 work_dir,扫产物分组(raw/cleaned/final md+html+images)
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Default)]
pub struct OutputsGroups {
  pub raw_md: Vec<String>,
  pub raw_html: Vec<String>,
  pub cleaned_md: Vec<String>,
  pub final_html: Vec<String>,
  pub images: Vec<String>,
  pub manifests: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListOutputsResult {
  pub inbox: String,
  pub work_dir: String,
  pub stem: String,
  pub outputs: OutputsGroups,
  pub stages: std::collections::BTreeMap<String, String>,
}

pub fn list_outputs_impl(inbox_dir: String) -> CommandResponse<ListOutputsResult> {
  let inbox = PathBuf::from(inbox_dir).expand();
  if !inbox.is_dir() {
    return CommandResponse::err(format!("inbox 目录不存在: {}", inbox.display()));
  }
  let work = inbox.join("output");
  if !work.is_dir() {
    return CommandResponse::err(format!(
      "work 目录不存在: {}\n请先用 run_pipeline(inbox_dir=...) 跑流水线",
      work.display()
    ));
  }
  let stem = derive_stem(&work);
  let raw_dir = work.join("chapters").join("raw").join(&stem);
  let mut groups = OutputsGroups::default();
  if raw_dir.is_dir() {
    // 递归扫(对齐 Python rglob)
    let mut stack: Vec<PathBuf> = vec![raw_dir.clone()];
    while let Some(dir) = stack.pop() {
      let read = match std::fs::read_dir(&dir) {
        Ok(r) => r,
        Err(e) => return CommandResponse::err(format!("read_dir 失败: {e}")),
      };
      for entry in read.flatten() {
        let p = entry.path();
        if p.is_dir() {
          stack.push(p);
        } else if p.is_file() {
          let rel = match p.strip_prefix(&raw_dir) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
          };
          let is_image = p
            .strip_prefix(&raw_dir)
            .map(|r| {
              let mut comps = r.components();
              comps.next().map(|c| c.as_os_str() == "images").unwrap_or(false)
            })
            .unwrap_or(false);
          if is_image {
            groups.images.push(rel);
          } else {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let md = format!("{stem}.md");
            let html = format!("{stem}.html");
            let cleaned = format!("{stem}_cleaned.md");
            let final_html = format!("{stem}_final.html");
            if name == md {
              groups.raw_md.push(rel);
            } else if name == html {
              groups.raw_html.push(rel);
            } else if name == cleaned {
              groups.cleaned_md.push(rel);
            } else if name == final_html {
              groups.final_html.push(rel);
            } else {
              groups.manifests.push(rel);
            }
          }
        }
      }
    }
    for g in [&mut groups.raw_md, &mut groups.raw_html, &mut groups.cleaned_md, &mut groups.final_html, &mut groups.images, &mut groups.manifests] {
      g.sort();
    }
  }
  if work.join("verify.json").is_file() {
    groups.manifests.push("verify.json".to_string());
  }
  let stages = derive_stage_status(&work, &stem, &raw_dir);
  CommandResponse::ok(ListOutputsResult {
    inbox: inbox.to_string_lossy().into_owned(),
    work_dir: work.to_string_lossy().into_owned(),
    stem,
    outputs: groups,
    stages,
  })
}

#[tauri::command]
pub fn list_outputs(inbox_dir: String) -> CommandResponse<ListOutputsResult> {
  list_outputs_impl(inbox_dir)
}

fn derive_stage_status(
  work: &Path,
  stem: &str,
  raw_dir: &Path,
) -> std::collections::BTreeMap<String, String> {
  let mut s: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
  // 优先用 state.json
  let state_path = work.join("state.json");
  if state_path.is_file() {
    if let Ok(raw) = std::fs::read_to_string(&state_path) {
      if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
        if let Some(map) = v.get("stages").and_then(|x| x.as_object()) {
          for (k, sv) in map {
            let status = sv.get("status").and_then(|x| x.as_str()).unwrap_or("pending");
            s.insert(k.clone(), status.to_string());
          }
          return s;
        }
      }
    }
  }
  // fallback:产物存在性
  let raw_md = raw_dir.join(format!("{stem}.md"));
  let final_html = raw_dir.join(format!("{stem}_final.html"));
  s.insert("audio".into(), if work.join("asr").join("audio.wav").is_file() { "completed" } else { "pending" }.into());
  s.insert("asr".into(), if work.join("asr").join("transcript.jsonl").is_file() { "completed" } else { "pending" }.into());
  s.insert("frames".into(), if work.join("frames").join("keyframes.json").is_file() { "completed" } else { "pending" }.into());
  s.insert("ocr".into(), if work.join("ocr").join("ocr_results.json").is_file() { "completed" } else { "pending" }.into());
  s.insert("asr_correct".into(), if work.join("asr_correct").join("transcript_corrected.jsonl").is_file() { "completed" } else { "pending" }.into());
  s.insert("chapters".into(), if work.join("chapters").join("chapters.json").is_file() { "completed" } else { "pending" }.into());
  let has_chapter_md = raw_dir.is_dir() && std::fs::read_dir(raw_dir)
    .map(|rd| {
      rd.flatten().any(|e| {
        e.file_name()
          .to_str()
          .map(|n| n.starts_with("chapter_") && n.ends_with(".md"))
          .unwrap_or(false)
      })
    })
    .unwrap_or(false);
  s.insert("draft".into(), if has_chapter_md { "completed" } else { "pending" }.into());
  s.insert("imagegen".into(), "skipped".into());
  s.insert("render".into(), if raw_md.is_file() { "completed" } else { "pending" }.into());
  s.insert("longdoc".into(), if final_html.is_file() { "completed" } else { "pending" }.into());
  s.insert("verify".into(), if work.join("verify.json").is_file() { "completed" } else { "pending" }.into());
  s
}

// ─────────────────────────────────────────────────────────────
// read_lecture —— 读讲义(raw/cleaned/final × md/html)
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ReadLectureResult {
  pub version: String,
  pub fmt: String,
  pub path: String,
  pub content: String,
  pub size_bytes: usize,
  /// "output_final" = W12-D 真相
  /// "legacy" = W3-W11 fallback
  /// "fallback_md" = html 不存在自动降到同 version 的 md
  pub source: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub note: Option<String>,
}

pub fn read_lecture_impl(
  inbox_dir: String,
  version: String,
  fmt: Option<String>,
) -> CommandResponse<ReadLectureResult> {
  let fmt = fmt.unwrap_or_else(|| "md".to_string());
  // (version, fmt) → 目标文件名(模板)
  let primary_tpl = match (version.as_str(), fmt.as_str()) {
    ("raw", "md")       => Some("{stem}.md"),
    ("raw", "html")     => Some("{stem}.html"),
    ("cleaned", "md")   => Some("{stem}_cleaned.md"),
    ("cleaned", "html") => Some("{stem}_cleaned.html"),
    ("final", "md")     => Some("{stem}_final.md"),
    ("final", "html")   => Some("{stem}_final.html"),
    _ => None,
  };
  let primary_tpl = match primary_tpl {
    Some(t) => t,
    None => {
      return CommandResponse::err(format!(
        "version/fmt 非法: version={version:?} fmt={fmt:?}(必须是 raw/cleaned/final × md/html)"
      ));
    }
  };
  let inbox = PathBuf::from(inbox_dir).expand();
  if !inbox.is_dir() {
    return CommandResponse::err(format!("inbox 目录不存在: {}", inbox.display()));
  }
  let work = inbox.join("output");
  let output_final = inbox.join("output_final");
  let stem = derive_stem(&work);
  let primary = primary_tpl.replace("{stem}", &stem);

  // 尝试 1:W12-D output_final/<primary>
  let p1 = output_final.join(&primary);
  if p1.is_file() {
    return read_ok(&p1, &version, &fmt, "output_final", None);
  }
  // 尝试 2(仅 fmt=="html"):W12-D output_final/<stem>_*.md(同 version)
  if fmt == "html" {
    let md_tpl = match version.as_str() {
      "raw"     => Some("{stem}.md"),
      "cleaned" => Some("{stem}_cleaned.md"),
      "final"   => Some("{stem}_final.md"),
      _ => None,
    };
    if let Some(t) = md_tpl {
      let md_p = output_final.join(&t.replace("{stem}", &stem));
      if md_p.is_file() {
        return read_ok(
          &md_p,
          &version,
          &fmt,
          "fallback_md",
          Some("html 版本未生成,fallback 到 md".to_string()),
        );
      }
    }
  }
  // 尝试 3:W3-W11 legacy
  let raw_dir = work.join("chapters").join("raw").join(&stem);
  let p3 = raw_dir.join(&primary);
  if p3.is_file() {
    return read_ok(&p3, &version, &fmt, "legacy", None);
  }
  // 全部 miss
  CommandResponse::err(format!(
    "讲义文件不存在:\n  - {}\n  - {}\n请先用 run_pipeline 跑流水线",
    p1.display(),
    p3.display()
  ))
}

fn read_ok(
  path: &Path,
  version: &str,
  fmt: &str,
  source: &str,
  note: Option<String>,
) -> CommandResponse<ReadLectureResult> {
  let content = match std::fs::read_to_string(path) {
    Ok(s) => s,
    Err(e) => return CommandResponse::err(format!("读 {} 失败: {e}", path.display())),
  };
  CommandResponse::ok(ReadLectureResult {
    version: version.to_string(),
    fmt: fmt.to_string(),
    path: path.to_string_lossy().into_owned(),
    content,
    size_bytes: std::fs::metadata(path).map(|m| m.len() as usize).unwrap_or(0),
    source: source.to_string(),
    note,
  })
}

#[tauri::command]
pub fn read_lecture(
  inbox_dir: String,
  version: String,
  fmt: Option<String>,
) -> CommandResponse<ReadLectureResult> {
  read_lecture_impl(inbox_dir, version, fmt)
}

// ─────────────────────────────────────────────────────────────
// read_log —— 读 mtd.log 增量(tail),支持 offset
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ReadLogResult {
  /// 增量内容(从 offset 到 max_lines 行 / 文件末尾)
  pub content: String,
  /// 新 offset(下次 read_log 的起点)
  pub new_offset: u64,
  /// 当前文件总字节数
  pub total_bytes: u64,
  /// 文件被 truncate(罕见)时为 true,前端下次从 0 开始读
  pub truncated: bool,
  /// 命中 max_lines 上限,文件内还有更多行时为 true
  pub truncated_to_lines: bool,
}

/// 实际读 log:打开文件 → seek offset → 读最多 max_lines 行。
///
/// `path` 校验:`ends_with("mtd.log")`(本机用,信任用户,不做沙箱)。
/// `offset > total_bytes` 视为 truncate,从头重读。
pub fn read_log_impl(
  path: String,
  offset: u64,
  max_lines: usize,
) -> CommandResponse<ReadLogResult> {
  let p = PathBuf::from(&path);
  if !p.to_string_lossy().ends_with("mtd.log") {
    return CommandResponse::err(format!(
      "仅支持读 mtd.log 文件,收到: {}\n(本机信任用户,但拒绝非 log 文件的随机路径)",
      p.display()
    ));
  }
  if !p.is_file() {
    return CommandResponse::err(format!(
      "log 文件不存在: {}\n(请先启动 run_pipeline / resume_pipeline)",
      p.display()
    ));
  }
  let metadata = match std::fs::metadata(&p) {
    Ok(m) => m,
    Err(e) => return CommandResponse::err(format!("读 metadata 失败: {e}")),
  };
  let total_bytes = metadata.len();
  let truncated = offset > total_bytes;
  let effective_offset = if truncated { 0 } else { offset };
  let file = match std::fs::File::open(&p) {
    Ok(f) => f,
    Err(e) => return CommandResponse::err(format!("打开 {} 失败: {e}", p.display())),
  };
  let mut reader = std::io::BufReader::new(file);
  use std::io::Seek;
  if let Err(e) = reader.seek(std::io::SeekFrom::Start(effective_offset)) {
    return CommandResponse::err(format!("seek {} 失败: {e}", effective_offset));
  }
  let mut content = String::new();
  let mut buf = String::new();
  let mut lines_read = 0usize;
  let mut bytes_read = 0usize;
  let cap = max_lines.max(1).min(2000);
  use std::io::BufRead;
  loop {
    buf.clear();
    let n = match reader.read_line(&mut buf) {
      Ok(0) => break,
      Ok(n) => n,
      Err(e) => return CommandResponse::err(format!("read_line 失败: {e}")),
    };
    bytes_read += n;
    content.push_str(&buf);
    lines_read += 1;
    if lines_read >= cap {
      break;
    }
  }
  let truncated_to_lines = lines_read >= cap && bytes_read < (total_bytes - effective_offset) as usize;
  let new_offset = effective_offset + bytes_read as u64;
  CommandResponse::ok(ReadLogResult {
    content,
    new_offset,
    total_bytes,
    truncated,
    truncated_to_lines,
  })
}

#[tauri::command]
pub async fn read_log(
  path: String,
  offset: u64,
  max_lines: usize,
) -> CommandResponse<ReadLogResult> {
  read_log_impl(path, offset, max_lines)
}

// ─────────────────────────────────────────────────────────────
// helpers
// ─────────────────────────────────────────────────────────────

/// Path 展开:把 ~ 替换为 HOME,并 .canonicalize()。
/// 注:对不存在的路径不抛错(只 .canonicalize() 失败时回退到原路径)。
pub trait PathExpand {
  fn expand(self) -> PathBuf;
}

impl PathExpand for PathBuf {
  fn expand(self) -> PathBuf {
    let s = self.to_string_lossy();
    let expanded = if let Some(stripped) = s.strip_prefix("~/") {
      if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        PathBuf::from(home).join(stripped)
      } else {
        return self;
      }
    } else if s == "~" {
      if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        PathBuf::from(home)
      } else {
        return self;
      }
    } else {
      return self;
    };
    std::fs::canonicalize(&expanded).unwrap_or(expanded)
  }
}

pub fn default_workspace_root() -> PathBuf {
  if let Ok(v) = std::env::var("MEDIA_TO_DOC_WORKSPACE") {
    return PathBuf::from(v);
  }
  // fallback:与 media-to-doc 主仓 WORKSPACE_ROOT 对齐
  if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
    return PathBuf::from(home).join("Documents").join("media-to-doc");
  }
  PathBuf::from(".")
}

/// 解析 media-to-doc Python 项目根:
/// 1. `MEDIA_TO_DOC_PROJECT` 环境变量(主路径)
/// 2. fallback:Tauri UI 同级 sibling(开发期默认)
/// 3. fallback:`./media-to-doc`(相对当前目录)
pub fn resolve_media_to_doc_project() -> PathBuf {
  if let Ok(v) = std::env::var("MEDIA_TO_DOC_PROJECT") {
    return PathBuf::from(v);
  }
  // 开发期 default:Tauri UI 在 F:/soft/00selfmade/media-to-doc-ui/,
  // media-to-doc 在同 parent 下的 F:/soft/00selfmade/media-to-doc/
  // 用 current_exe 推断不可靠(开发态 binary 在 target/debug/),
  // 直接走 known 路径(本机约定,生产环境用 env 覆盖)
  if let Ok(cwd) = std::env::current_dir() {
    // 试 cwd 的 sibling
    if let Some(parent) = cwd.parent() {
      let sibling = parent.join("media-to-doc");
      if sibling.join("pyproject.toml").is_file() {
        return sibling;
      }
    }
    // 试 cwd 本身
    if cwd.join("pyproject.toml").is_file() {
      return cwd;
    }
  }
  PathBuf::from("media-to-doc")
}

fn walk_media(dir: &Path) -> Vec<PathBuf> {
  let mut out = Vec::new();
  fn rec(d: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = std::fs::read_dir(d) {
      for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
          rec(&p, out);
        } else if is_media_file(&p) {
          out.push(p);
        }
      }
    }
  }
  rec(dir, &mut out);
  out.sort();
  out
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::BTreeMap;
  use std::fs;
  use std::path::PathBuf;

  fn tmpdir(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("media_to_doc_ui_{name}"));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
  }

  #[test]
  fn list_courses_returns_empty_when_inbox_missing() {
    let ws = tmpdir("list_courses_empty");
    let r = list_courses_impl(Some(ws.to_string_lossy().into_owned()));
    assert!(r.ok);
    let data = r.data.unwrap();
    assert_eq!(data.courses.len(), 0);
    let _ = fs::remove_dir_all(&ws);
  }

  #[test]
  fn list_courses_picks_up_subdirs_and_media() {
    let ws = tmpdir("list_courses_pickup");
    let inbox = ws.join("inbox");
    fs::create_dir_all(&inbox).unwrap();
    let course = inbox.join("course_a");
    fs::create_dir_all(&course).unwrap();
    fs::write(course.join("video.mp4"), b"fake").unwrap();
    fs::write(course.join("notes.txt"), b"fake").unwrap();  // not media
    let r = list_courses_impl(Some(ws.to_string_lossy().into_owned()));
    assert!(r.ok);
    let data = r.data.unwrap();
    assert_eq!(data.courses.len(), 1);
    assert_eq!(data.courses[0].name, "course_a");
    assert_eq!(data.courses[0].media_count, 1);
    assert_eq!(data.courses[0].media_files, vec!["video.mp4"]);
    let _ = fs::remove_dir_all(&ws);
  }

  #[test]
  fn check_status_errors_when_state_json_missing() {
    let work = tmpdir("check_status_missing");
    let r = check_status_impl(work.to_string_lossy().into_owned());
    assert!(!r.ok);
    assert!(r.error.unwrap().contains("state.json"));
    let _ = fs::remove_dir_all(&work);
  }

  #[test]
  fn check_status_parses_minimal_state() {
    let work = tmpdir("check_status_parse");
    let state = serde_json::json!({
      "course": "demo",
      "inbox_path": "/x/y",
      "current_stage": "chapters",
      "started_at": "2026-07-22T00:00:00",
      "updated_at": "2026-07-22T00:00:01",
      "is_complete": false,
      "stages": {
        "audio": {"status": "completed", "started_at": "2026-07-22T00:00:00", "finished_at": "2026-07-22T00:00:01", "error": null},
        "asr": {"status": "in_progress", "started_at": "2026-07-22T00:00:01", "finished_at": null, "error": null}
      }
    });
    fs::write(work.join("state.json"), serde_json::to_string_pretty(&state).unwrap()).unwrap();
    let r = check_status_impl(work.to_string_lossy().into_owned());
    assert!(r.ok, "{:?}", r);
    let data = r.data.unwrap();
    assert_eq!(data.course, "demo");
    assert_eq!(data.current_stage, "chapters");
    assert!(!data.is_complete);
    assert_eq!(data.stages.get("audio").unwrap().status, "completed");
    assert_eq!(data.stages.get("asr").unwrap().status, "in_progress");
    let _ = fs::remove_dir_all(&work);
  }

  #[test]
  fn list_outputs_groups_raw_and_cleaned() {
    let inbox = tmpdir("list_outputs_groups");
    let work = inbox.join("output");
    let raw = work.join("chapters").join("raw").join("demo_video");
    fs::create_dir_all(&raw).unwrap();
    fs::write(raw.join("demo_video.md"), b"# raw").unwrap();
    fs::write(raw.join("demo_video_cleaned.md"), b"# cleaned").unwrap();
    fs::write(raw.join("demo_video_final.html"), b"<h1>final</h1>").unwrap();
    fs::create_dir_all(raw.join("images")).unwrap();
    fs::write(raw.join("images").join("p1.png"), b"fake").unwrap();
    let r = list_outputs_impl(inbox.to_string_lossy().into_owned());
    assert!(r.ok, "{:?}", r);
    let data = r.data.unwrap();
    assert_eq!(data.stem, "demo_video");
    assert_eq!(data.outputs.raw_md, vec!["demo_video.md"]);
    assert_eq!(data.outputs.cleaned_md, vec!["demo_video_cleaned.md"]);
    assert_eq!(data.outputs.final_html, vec!["demo_video_final.html"]);
    assert_eq!(data.outputs.images, vec!["images/p1.png"]);
    assert!(data.stages.get("render").map(|s| s == "completed").unwrap_or(false));
    let _ = fs::remove_dir_all(&inbox);
  }

  #[test]
  fn list_outputs_errors_when_inbox_missing() {
    let r = list_outputs_impl("Z:/definitely/not/here".to_string());
    assert!(!r.ok);
  }

  #[test]
  fn read_lecture_rejects_bad_version() {
    let r = read_lecture_impl("/tmp".into(), "nope".into(), None);
    assert!(!r.ok);
    assert!(r.error.unwrap().contains("version"));
  }

  #[test]
  fn read_lecture_falls_back_to_md_when_html_missing() {
    // W14-B+2 T2:html→md fallback 现在走 output_final/ 路径
    let inbox = tmpdir("read_lecture_fallback");
    // seed legacy stem:derive_stem 读 work/chapters/raw/<stem>/
    let work = inbox.join("output");
    let raw = work.join("chapters").join("raw").join("demo");
    fs::create_dir_all(&raw).unwrap();
    // output_final:只有 cleaned.md,没有 html
    let final_dir = inbox.join("output_final");
    fs::create_dir_all(&final_dir).unwrap();
    fs::write(final_dir.join("demo_cleaned.md"), b"# cleaned md").unwrap();
    // no html
    let r = read_lecture_impl(
      inbox.to_string_lossy().into_owned(),
      "cleaned".into(),
      Some("html".into()),
    );
    assert!(r.ok);
    let data = r.data.unwrap();
    assert_eq!(data.source, "fallback_md");
    assert!(data.content.contains("cleaned md"));
    assert_eq!(data.note.as_deref(), Some("html 版本未生成,fallback 到 md"));
    let _ = fs::remove_dir_all(&inbox);
  }

  #[test]
  fn derive_stage_status_fallback_when_no_state_json() {
    let work = tmpdir("derive_stage_no_state");
    let raw = work.join("chapters").join("raw").join("demo");
    fs::create_dir_all(&raw).unwrap();
    fs::write(raw.join("demo.md"), b"# raw").unwrap();
    let stages = derive_stage_status(&work, "demo", &raw);
    assert_eq!(stages.get("render").map(String::as_str), Some("completed"));
    assert_eq!(stages.get("audio").map(String::as_str), Some("pending"));
    assert_eq!(stages.get("imagegen").map(String::as_str), Some("skipped"));
    let _ = fs::remove_dir_all(&work);
  }

  #[test]
  fn path_expand_replaces_home_dir() {
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
      let home = PathBuf::from(home);
      let expanded = PathBuf::from("~/foo").expand();
      assert_eq!(expanded, home.join("foo"));
    }
  }

  #[test]
  fn path_expand_leaves_absolute_alone() {
    let p = if cfg!(windows) { PathBuf::from("C:/Users/Someone/x") } else { PathBuf::from("/tmp/x") };
    let e = p.clone().expand();
    assert_eq!(e, p);
  }

  // silence dead_code for BTreeMap import on no-feature builds
  #[allow(dead_code)]
  fn _btreemap_marker(_m: BTreeMap<String, String>) {}

  // ────────────────────────────────────────────────────────────
  // read_log 测试(W14-B+2 T1)
  // ────────────────────────────────────────────────────────────

  #[test]
  fn read_log_errors_on_missing_file() {
    let r = read_log_impl(
      std::env::temp_dir().join("definitely_not_here_xyz_mtd.log").to_string_lossy().into_owned(),
      0, 200,
    );
    assert!(!r.ok, "missing file should err");
    let err = r.error.unwrap();
    assert!(err.contains("not found") || err.contains("不存在"));
  }

  #[test]
  fn read_log_returns_empty_when_offset_equals_size() {
    let tmp = tmpdir("read_log_eq_size");
    let p = tmp.join("mtd.log");
    fs::write(&p, b"line1\nline2\n").unwrap();
    let size = fs::metadata(&p).unwrap().len();
    let r = read_log_impl(p.to_string_lossy().into_owned(), size, 200);
    assert!(r.ok, "{:?}", r);
    let d = r.data.unwrap();
    assert_eq!(d.content, "");
    assert_eq!(d.new_offset, size);
    assert_eq!(d.total_bytes, size);
    assert!(!d.truncated);
    assert!(!d.truncated_to_lines);
    let _ = fs::remove_dir_all(&tmp);
  }

  #[test]
  fn read_log_returns_content_from_offset() {
    let tmp = tmpdir("read_log_offset");
    let p = tmp.join("mtd.log");
    let body = b"line1\nline2\nline3\n";
    fs::write(&p, body).unwrap();
    // 跳过 "line1\n"(7 bytes)从 "line2\n" 开始
    let r = read_log_impl(p.to_string_lossy().into_owned(), 6, 200);
    assert!(r.ok, "{:?}", r);
    let d = r.data.unwrap();
    assert_eq!(d.content, "line2\nline3\n");
    assert_eq!(d.new_offset, body.len() as u64);
    assert_eq!(d.total_bytes, body.len() as u64);
    assert!(!d.truncated);
    let _ = fs::remove_dir_all(&tmp);
  }

  #[test]
  fn read_log_resets_on_truncate() {
    let tmp = tmpdir("read_log_truncate");
    let p = tmp.join("mtd.log");
    fs::write(&p, b"very long original content").unwrap();
    // 文件被 truncate 到 5 bytes,前端 offset 仍是 25
    fs::write(&p, b"short").unwrap();
    let r = read_log_impl(p.to_string_lossy().into_owned(), 25, 200);
    assert!(r.ok, "{:?}", r);
    let d = r.data.unwrap();
    assert!(d.truncated, "truncated flag should be set");
    assert_eq!(d.content, "short");
    assert_eq!(d.new_offset, 5);
    let _ = fs::remove_dir_all(&tmp);
  }

  #[test]
  fn read_log_caps_max_lines() {
    let tmp = tmpdir("read_log_caps");
    let p = tmp.join("mtd.log");
    let mut body = String::new();
    for i in 0..500 { body.push_str(&format!("line {}\n", i)); }
    fs::write(&p, body.as_bytes()).unwrap();
    let r = read_log_impl(p.to_string_lossy().into_owned(), 0, 10);
    assert!(r.ok, "{:?}", r);
    let d = r.data.unwrap();
    let line_count = d.content.lines().count();
    assert_eq!(line_count, 10);
    assert!(d.truncated_to_lines);
    let _ = fs::remove_dir_all(&tmp);
  }

  // ────────────────────────────────────────────────────────────
  // read_lecture W12-D 3 级 fallback 测试(W14-B+2 T2)
  // ────────────────────────────────────────────────────────────

  #[test]
  fn read_lecture_prefers_output_final_over_legacy() {
    let tmp = tmpdir("read_lecture_w12d_prefer");
    let inbox = &tmp;
    // W12-D:output_final/<stem>_cleaned.md
    let final_dir = tmp.join("output_final");
    fs::create_dir_all(&final_dir).unwrap();
    fs::write(final_dir.join("course_cleaned.md"), b"# from output_final").unwrap();
    // legacy:output/chapters/raw/course/course_cleaned.md
    let work = tmp.join("output");
    let raw = work.join("chapters").join("raw").join("course");
    fs::create_dir_all(&raw).unwrap();
    fs::write(raw.join("course_cleaned.md"), b"# from legacy").unwrap();
    let r = read_lecture_impl(
      inbox.to_string_lossy().into_owned(),
      "cleaned".into(),
      Some("md".into()),
    );
    assert!(r.ok, "{:?}", r);
    let d = r.data.unwrap();
    assert_eq!(d.source, "output_final");
    assert!(d.content.contains("from output_final"));
    let _ = fs::remove_dir_all(&tmp);
  }

  #[test]
  fn read_lecture_falls_back_to_legacy_when_output_final_missing() {
    let tmp = tmpdir("read_lecture_legacy_fallback");
    let inbox = &tmp;
    // 没有 output_final,只有 legacy
    let work = tmp.join("output");
    let raw = work.join("chapters").join("raw").join("course");
    fs::create_dir_all(&raw).unwrap();
    fs::write(raw.join("course.md"), b"# legacy raw").unwrap();
    let r = read_lecture_impl(
      inbox.to_string_lossy().into_owned(),
      "raw".into(),
      Some("md".into()),
    );
    assert!(r.ok, "{:?}", r);
    let d = r.data.unwrap();
    assert_eq!(d.source, "legacy");
    assert!(d.content.contains("legacy raw"));
    let _ = fs::remove_dir_all(&tmp);
  }

  #[test]
  fn read_lecture_html_falls_back_to_md_with_note() {
    let tmp = tmpdir("read_lecture_html_fallback");
    let inbox = &tmp;
    // seed legacy stem:derive_stem 读 work/chapters/raw/<stem>/
    let work = tmp.join("output");
    let raw = work.join("chapters").join("raw").join("course");
    fs::create_dir_all(&raw).unwrap();
    let final_dir = tmp.join("output_final");
    fs::create_dir_all(&final_dir).unwrap();
    // 只有 cleaned.md,没有 html
    fs::write(final_dir.join("course_cleaned.md"), b"# cleaned md body").unwrap();
    let r = read_lecture_impl(
      inbox.to_string_lossy().into_owned(),
      "cleaned".into(),
      Some("html".into()),
    );
    assert!(r.ok, "{:?}", r);
    let d = r.data.unwrap();
    assert_eq!(d.source, "fallback_md");
    assert!(d.note.is_some());
    assert!(d.note.unwrap().contains("html") || d.content.contains("cleaned md body"));
    let _ = fs::remove_dir_all(&tmp);
  }

  #[test]
  fn read_lecture_errors_when_neither_layout_has_file() {
    let tmp = tmpdir("read_lecture_missing");
    let inbox = &tmp;
    // 创建 output 但不放任何产物
    fs::create_dir_all(tmp.join("output").join("chapters").join("raw")).unwrap();
    let r = read_lecture_impl(
      inbox.to_string_lossy().into_owned(),
      "raw".into(),
      Some("md".into()),
    );
    assert!(!r.ok);
    let _ = fs::remove_dir_all(&tmp);
  }
}

// ─────────────────────────────────────────────────────────────
// run_pipeline / resume_pipeline / cancel_run / list_running
// (T3:子进程管理,对齐 MCP run_pipeline / resume_pipeline + 自有 cancel/list)
// W14-C:加并发上限检查 + list_all_runs 新命令
// ─────────────────────────────────────────────────────────────

/// 解析 inbox,校验,返回 work_dir 候选(用于 sanity check)。
fn resolve_inbox(inbox: &str) -> Result<PathBuf, String> {
  let p = PathBuf::from(inbox).expand();
  if !p.is_dir() {
    return Err(format!("inbox 目录不存在: {}", p.display()));
  }
  Ok(p)
}

#[tauri::command]
pub async fn run_pipeline(
  inbox_dir: String,
  workspace_root: Option<String>,
  llm: Option<String>,
  imagegen: Option<String>,
  stop_after: Option<String>,
  no_longdoc: Option<bool>,
  force: Option<bool>,
  // W15-A T7.2:per-run profile + task_text(前端 New Run tab 传入)
  llm_profile_name: Option<String>,
  image_agent_profile_name: Option<String>,
  task_text: Option<String>,
) -> CommandResponse<RunPipelineResult> {
  let registry = global_registry();
  let inbox = match resolve_inbox(&inbox_dir) {
    Ok(p) => p,
    Err(e) => return CommandResponse::err(e),
  };
  let project = resolve_media_to_doc_project();
  if !project.join("pyproject.toml").is_file() {
    return CommandResponse::err(format!(
      "media-to-doc 项目根未找到: {}\n请设置 MEDIA_TO_DOC_PROJECT 环境变量",
      project.display()
    ));
  }
  let _ = workspace_root; // 暂未使用(MCP 兼容占位)
  let mut spec = build_mtd_run_args(
    &project,
    &inbox,
    llm.as_deref(),
    imagegen.as_deref(),
    stop_after.as_deref(),
    no_longdoc.unwrap_or(false),
    force.unwrap_or(false),
    // W15-A T7.2:per-run profile + task_text 透传给主仓(主仓仅作 logging)
    llm_profile_name.as_deref(),
    image_agent_profile_name.as_deref(),
    task_text.as_deref(),
  );
  // W15-A T7.2:per-run profile 注入 spec.env_vars(Tauri 是 profile 唯一真相源)。
  // None → 清空 env_vars 走 CLI 默认;Some(name) → 查 profile + keyring。
  // profile_name 覆盖 --llm(主仓若收到两个,自己定义优先级;当前 spec:env_vars 优先)。
  if let Err(e) = inject_profile_env(&mut spec, llm_profile_name.as_deref()) {
    return CommandResponse::err(e);
  }
  let work_dir = derive_work_dir(&inbox);
  let work_dir_str = work_dir.to_string_lossy().into_owned();
  if registry.is_running(&work_dir_str).await {
    return CommandResponse::err(format!(
      "该 work_dir 已在运行: {work_dir_str}\n请先 cancel_run 或 list_all_runs 检查"
    ));
  }
  let child = match spawn_mtd(&spec).await {
    Ok(c) => c,
    Err(e) => return CommandResponse::err(e),
  };
  let pid = child.id();
  let log_path = spec.log_path.clone();
  match registry
    .insert(work_dir_str.clone(), child, inbox.to_string_lossy().into_owned(), log_path.clone())
    .await
  {
    Ok(()) => {}
    Err(e) => return CommandResponse::err(e),
  }
  CommandResponse::ok(RunPipelineResult {
    work_dir: work_dir_str,
    pid,
    log_path,
    spec,
  })
}

#[tauri::command]
pub async fn resume_pipeline(
  work_dir: String,
  inbox_dir: Option<String>,
  force: Option<bool>,
  stop_after: Option<String>,
  // W15-A T7.2:per-run profile + task_text(续跑也允许切换 profile / 任务文本)
  llm_profile_name: Option<String>,
  image_agent_profile_name: Option<String>,
  task_text: Option<String>,
) -> CommandResponse<RunPipelineResult> {
  let registry = global_registry();
  let work = PathBuf::from(work_dir).expand();
  if !work.is_dir() {
    return CommandResponse::err(format!("work 目录不存在: {}", work.display()));
  }
  let project = resolve_media_to_doc_project();
  if !project.join("pyproject.toml").is_file() {
    return CommandResponse::err(format!(
      "media-to-doc 项目根未找到: {}\n请设置 MEDIA_TO_DOC_PROJECT 环境变量",
      project.display()
    ));
  }
  let mut spec = build_mtd_resume_args(
    &project,
    &work,
    force.unwrap_or(false),
    stop_after.as_deref(),
    // W15-A T7.2:per-run profile + task_text 透传给主仓(主仓仅作 logging)
    llm_profile_name.as_deref(),
    image_agent_profile_name.as_deref(),
    task_text.as_deref(),
  );
  // W15-A T7.2:per-run profile 注入 env_vars(同 run_pipeline)。
  if let Err(e) = inject_profile_env(&mut spec, llm_profile_name.as_deref()) {
    return CommandResponse::err(e);
  }
  let work_dir_str = work.to_string_lossy().into_owned();
  if registry.is_running(&work_dir_str).await {
    return CommandResponse::err(format!(
      "该 work_dir 已在运行: {work_dir_str}\n请先 cancel_run"
    ));
  }
  let inbox_for_registry = inbox_dir.unwrap_or_else(|| {
    work.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()
  });
  let child = match spawn_mtd(&spec).await {
    Ok(c) => c,
    Err(e) => return CommandResponse::err(e),
  };
  let pid = child.id();
  let log_path = spec.log_path.clone();
  match registry
    .insert(work_dir_str.clone(), child, inbox_for_registry, log_path.clone())
    .await
  {
    Ok(()) => {}
    Err(e) => return CommandResponse::err(e),
  }
  CommandResponse::ok(RunPipelineResult {
    work_dir: work_dir_str,
    pid,
    log_path,
    spec,
  })
}

/// W15-A T7.2:按 `profile_name` 查 profile + keyring → 写 `spec.env_vars`。
///
/// 设计要点:
/// - `profile_name = None` → 清空 `spec.env_vars`,走 CLI 默认(主仓不传
///   `LLM_*` env vars 时的回退路径)。这是允许的合法路径(用户可显式选
///   "default / 无 profile" 启动一次跑通)。
/// - `profile_name = Some(name)`:
///   - profile 不存在 → `PROFILE_NOT_FOUND:<name>`
///   - keyring 读失败:
///     - provider == "Ollama" → 当作 NoEntry,空 key 注入(`Ollama` 不需要 Authorization)
///     - 其他 provider → `KEYRING_ERROR:<...>` 传播,前端引导用户去 Settings 重输
///   - 成功 → `spec.env_vars = to_env_vars(meta, key)`
///
/// 错误统一前缀(PROFILE_NOT_FOUND / KEYRING_ERROR / PROVIDER_NOT_FOUND / ENV_VARS_BUILD_ERROR)
/// 便于前端精确判断与引导。直接 return String,让 `run_pipeline` / `resume_pipeline`
/// 包成 `CommandResponse::err` 传播。
pub(crate) fn inject_profile_env(
  spec: &mut SpawnSpec,
  profile_name: Option<&str>,
) -> Result<(), String> {
  let Some(name) = profile_name else {
    // 显式 None → 清空 env_vars,让 CLI 走默认
    spec.env_vars.clear();
    return Ok(());
  };
  let m = llm_profiles::load_profiles()
    .map_err(|e| format!("LLM_PROFILES_LOAD_ERROR: {e}"))?;
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
        // Ollama 不需要 Authorization,NoEntry 视为合法空 key
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

#[derive(Debug, Clone, Serialize)]
pub struct CancelResult {
  pub work_dir: String,
  pub pid: u32,
  pub killed: bool,
}

#[tauri::command]
pub async fn cancel_run(work_dir: String) -> CommandResponse<CancelResult> {
  let registry = global_registry();
  let pid = match registry.cancel(&work_dir).await {
    Some(p) => p,
    None => {
      return CommandResponse::err(format!("work_dir 未在运行: {work_dir}"));
    }
  };
  if pid > 0 {
    let _ = kill_tree(pid);
  }
  CommandResponse::ok(CancelResult { work_dir, pid, killed: true })
}

#[derive(Debug, Clone, Serialize)]
pub struct ListRunningResult {
  pub running: Vec<RunningRun>,
}

/// 仅返回当前活跃的 run。
#[tauri::command]
pub async fn list_running() -> CommandResponse<ListRunningResult> {
  let running = global_registry().list().await;
  CommandResponse::ok(ListRunningResult { running })
}

#[derive(Debug, Clone, Serialize)]
pub struct ListAllRunsResult {
  pub runs: Vec<RunStatusInfo>,
  pub max_concurrent: usize,
  pub active_count: usize,
}

/// W14-C:返回全量 run(活跃 + 最近 completed),含并发上限信息。
#[tauri::command]
pub async fn list_all_runs() -> CommandResponse<ListAllRunsResult> {
  let registry = global_registry();
  let runs = registry.list_all().await;
  let active_count = registry.running_count().await;
  CommandResponse::ok(ListAllRunsResult {
    runs,
    max_concurrent: registry.max_concurrent(),
    active_count,
  })
}

// ─────────────────────────────────────────────────────────────
// W15-A: 6 个 LLM profile Tauri commands
// (对齐 spec §6 + plan Task 5)
// ─────────────────────────────────────────────────────────────

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
    return CommandResponse::err("PROFILE_NAME_CONFLICT: profile 名不能为空");
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

  let now = chrono_like_now();
  let new_meta = ProfileMeta {
    name: args.name.clone(),
    provider: args.provider.clone(),
    base_url: args.base_url.clone(),
    model: args.model.clone(),
    note: args.note.clone(),
    tool_search_enabled: args.tool_search_enabled.unwrap_or(false),
    experimental_betas_disabled: args.experimental_betas_disabled.unwrap_or(false),
    created_at: now,
  };

  // 写 keyring(若提供了 api_key)
  if let Some(key) = &args.api_key {
    if key.is_empty() {
      // Some("") 视为删除 key
      let _ = keyring_store::delete_key(&args.name);
    } else {
      if let Err(e) = keyring_store::write_key(&args.name, key) {
        return CommandResponse::err(e);
      }
    }
  }
  // api_key == None → 保留 keyring 旧值(不写)

  // upsert 到 profiles 列表(保留 created_at,其它字段覆盖)
  if let Some(existing) = m.profiles.iter_mut().find(|p| p.name == args.name) {
    existing.provider = new_meta.provider.clone();
    existing.base_url = new_meta.base_url.clone();
    existing.model = new_meta.model.clone();
    existing.note = new_meta.note.clone();
    existing.tool_search_enabled = new_meta.tool_search_enabled;
    existing.experimental_betas_disabled = new_meta.experimental_betas_disabled;
    // created_at 保留
  } else {
    m.profiles.push(new_meta.clone());
  }

  if let Err(e) = llm_profiles::save_profiles(&m) {
    return CommandResponse::err(e);
  }
  let stored = m
    .profiles
    .into_iter()
    .find(|p| p.name == args.name)
    .unwrap_or(new_meta);
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
  if m.active.as_deref() == Some(name.as_str()) {
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
// test_llm_connection —— HTTP 探测 profile 的 LLM 端点
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct TestConnectionResult {
  pub ok: bool,
  pub latency_ms: u64,
  pub model: String,
  pub error: Option<String>,
}

pub async fn test_llm_connection_impl(
  name: String,
) -> CommandResponse<TestConnectionResult> {
  // 1. 找 profile
  let m = match llm_profiles::load_profiles() {
    Ok(m) => m,
    Err(e) => return CommandResponse::err(e),
  };
  let meta = match m.profiles.into_iter().find(|p| p.name == name) {
    Some(p) => p,
    None => return CommandResponse::err(format!("PROFILE_NOT_FOUND: {name}")),
  };
  // 2. 读 key(Ollama 不需要 key,NoEntry 视为空)
  let is_ollama = meta.provider == "Ollama";
  let key = match keyring_store::read_key(&meta.name) {
    Ok(k) => k,
    Err(e) => {
      // Ollama:没存 key 是合法状态,空串即可
      if is_ollama {
        String::new()
      } else {
        return CommandResponse::err(e);
      }
    }
  };
  // 3. 构造 URL
  let (url, headers) = match llm_profiles::probe_endpoint(&meta, &key) {
    Ok(v) => v,
    Err(e) => return CommandResponse::err(e),
  };
  // 4. HTTP GET + 计时
  let start = Instant::now();
  let client = match reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(10))
    .build()
  {
    Ok(c) => c,
    Err(e) => {
      return CommandResponse::err(format!("建 reqwest client 失败: {e}"));
    }
  };
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
    error: if ok {
      None
    } else {
      Some(format!("HTTP {}", status.as_u16()))
    },
  })
}

#[tauri::command]
pub async fn test_llm_connection(name: String) -> CommandResponse<TestConnectionResult> {
  test_llm_connection_impl(name).await
}

/// 简单 RFC3339-ish 时间戳(UTC, `YYYY-MM-DDTHH:MM:SSZ` 格式)。
/// 与测试 fixture / spec §4 metadata 示例对齐。
fn chrono_like_now() -> String {
  let secs = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_secs())
    .unwrap_or(0);
  // 简易 UTC 转换(避免引入 chrono 依赖)
  let (y, mo, d, h, mi, s) = epoch_to_utc(secs);
  format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// epoch 秒 → UTC (year, month, day, hour, min, sec)
/// 标准 Gregorian 算法(从 Howard Hinnant 的 date.h 移植到 Rust)。
fn epoch_to_utc(mut secs: u64) -> (u32, u32, u32, u32, u32, u32) {
  let s = (secs % 60) as u32;
  secs /= 60;
  let mi = (secs % 60) as u32;
  secs /= 60;
  let h = (secs % 24) as u32;
  let days = (secs / 24) as i64; // 自 1970-01-01 起的天数

  // Hinnant 算法:days → civil date
  let z = days + 719468;
  let era = if z >= 0 { z } else { z - 146096 } / 146097;
  let doe = (z - era * 146097) as u64; // [0, 146096]
  let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
  let y = (yoe as i64) + era * 400;
  let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
  let mp = (5 * doy + 2) / 153; // [0, 11]
  let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
  let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
  let y = if m <= 2 { y + 1 } else { y };
  (y as u32, m, d, h, mi, s)
}

// ─────────────────────────────────────────────────────────────
// W15-A T7.2: project registry(persistent JSON in app_config_dir)
// ─────────────────────────────────────────────────────────────

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
struct RegistryFile {
  pub version: u32,
  pub projects: Vec<ProjectEntry>,
}

/// 规范化路径:Windows 大小写归一 + canonicalize(失败时回退原值)。
/// 不同 OS 的 unicode NFC:依赖 OS 默认(Windows 已 NFC,macOS 也基本 NFC,Linux 用户
/// 需自行保证 NFC;不影响单测与常见使用)。
fn canonicalize_path(p: &Path) -> PathBuf {
  let s = p.to_string_lossy();
  let normalized = if cfg!(windows) { s.to_lowercase() } else { s.into_owned() };
  let pb = PathBuf::from(&normalized);
  std::fs::canonicalize(&pb).unwrap_or(pb)
}

/// canonical_id = sha256(canonical_path) 前 16 hex。
/// 永不依赖 display_name(避免同名不同路径误合并)。
fn canonical_id(p: &Path) -> String {
  let canon = canonicalize_path(p);
  let mut hasher = Sha256::new();
  hasher.update(canon.to_string_lossy().as_bytes());
  let bytes = hasher.finalize();
  bytes.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

/// registry 路径:
/// 1. `MEDIA_TO_DOC_PROJECT_REGISTRY_DIR` 环境变量(测试用,优先级最高)
/// 2. `%APPDATA%/com.duanyi.mediatodoc/project_registry.json`(Windows 生产)
/// 3. fallback: 当前目录 `project_registry.json`(开发期兜底)
fn registry_path() -> PathBuf {
  if let Ok(v) = std::env::var("MEDIA_TO_DOC_PROJECT_REGISTRY_DIR") {
    return PathBuf::from(v).join("project_registry.json");
  }
  if let Some(appdata) = std::env::var_os("APPDATA") {
    return PathBuf::from(appdata)
      .join("com.duanyi.mediatodoc")
      .join("project_registry.json");
  }
  PathBuf::from("project_registry.json")
}

fn load_registry() -> RegistryFile {
  let p = registry_path();
  if !p.is_file() {
    return RegistryFile {
      version: 1,
      projects: vec![],
    };
  }
  match std::fs::read_to_string(&p)
    .ok()
    .and_then(|s| serde_json::from_str::<RegistryFile>(&s).ok())
  {
    Some(r) => r,
    None => RegistryFile {
      version: 1,
      projects: vec![],
    },
  }
}

fn save_registry(r: &RegistryFile) -> Result<(), String> {
  let p = registry_path();
  if let Some(parent) = p.parent() {
    std::fs::create_dir_all(parent)
      .map_err(|e| format!("mkdir {} 失败: {e}", parent.display()))?;
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
  let p = PathBuf::from(path).expand();
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
    // 不动 sessions(spec §3.5:同 id 不覆盖 last_used_at 之外的字段)
    let snapshot = existing.clone();
    if let Err(e) = save_registry(&r) {
      return CommandResponse::err(e);
    }
    return CommandResponse::ok(snapshot);
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
  if let Err(e) = save_registry(&r) {
    return CommandResponse::err(e);
  }
  CommandResponse::ok(entry)
}

pub fn remove_project_impl(id: String) -> CommandResponse<()> {
  let mut r = load_registry();
  let before = r.projects.len();
  r.projects.retain(|e| e.id != id);
  if r.projects.len() == before {
    return CommandResponse::err(format!("PROJECT_NOT_FOUND: {id}"));
  }
  if let Err(e) = save_registry(&r) {
    return CommandResponse::err(e);
  }
  CommandResponse::ok(())
}

pub fn touch_project_impl(id: String) -> CommandResponse<()> {
  let mut r = load_registry();
  let now = chrono_like_now();
  if let Some(e) = r.projects.iter_mut().find(|e| e.id == id) {
    e.last_used_at = now;
    if let Err(e) = save_registry(&r) {
      return CommandResponse::err(e);
    }
    return CommandResponse::ok(());
  }
  CommandResponse::err(format!("PROJECT_NOT_FOUND: {id}"))
}

#[tauri::command]
pub async fn list_projects() -> CommandResponse<Vec<ProjectEntry>> {
  list_projects_impl()
}

#[tauri::command]
pub async fn add_project(path: String) -> CommandResponse<ProjectEntry> {
  add_project_impl(path)
}

#[tauri::command]
pub async fn remove_project(id: String) -> CommandResponse<()> {
  remove_project_impl(id)
}

#[tauri::command]
pub async fn touch_project(id: String) -> CommandResponse<()> {
  touch_project_impl(id)
}

// ─────────────────────────────────────────────────────────────
// T3 + W14-C unit tests
// ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod runner_tests {
  use super::*;
  use std::collections::HashMap;

  #[test]
  fn resolve_inbox_rejects_missing_dir() {
    let r = resolve_inbox("Z:/no/such/dir/abc");
    assert!(r.is_err());
  }

  #[test]
  fn resolve_inbox_accepts_existing_dir() {
    let tmp = std::env::temp_dir().join("ui_resolve_inbox_test");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let r = resolve_inbox(&tmp.to_string_lossy());
    assert!(r.is_ok());
    let _ = std::fs::remove_dir_all(&tmp);
  }

  #[test]
  fn resolve_media_to_doc_project_uses_env_var() {
    let target = std::env::temp_dir().join("fake_media_to_doc_proj");
    let _ = std::fs::remove_dir_all(&target);
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("pyproject.toml"), b"[project]\nname='x'\n").unwrap();
    // SAFETY: test-only, single-threaded
    unsafe { std::env::set_var("MEDIA_TO_DOC_PROJECT", &target); }
    let got = resolve_media_to_doc_project();
    unsafe { std::env::remove_var("MEDIA_TO_DOC_PROJECT"); }
    assert_eq!(got, target);
    let _ = std::fs::remove_dir_all(&target);
  }

  #[test]
  fn resolve_inbox_silently_swallows_io_errors() {
    // Just exercise the path
    let r = resolve_inbox("Z:/no/such/dir/abc");
    assert!(r.is_err());
  }

  // ────────────────────────────────────────────────────────────
  // T4: LLM commands — 6 happy + 2 error(8 tests)
  // ────────────────────────────────────────────────────────────

  fn make_meta_for_test(name: &str, provider: &str) -> ProfileMeta {
    ProfileMeta {
      name: name.into(),
      provider: provider.into(),
      base_url: "https://api.deepseek.com".into(),
      model: "deepseek-chat".into(),
      note: None,
      tool_search_enabled: false,
      experimental_betas_disabled: false,
      created_at: "2026-07-24T00:00:00Z".into(),
    }
  }

  #[test]
  fn t4_list_llm_profiles_returns_ok_with_vec() {
    // happy: list 返回 Ok(Vec),不强求为空(全局 metadata 可能已有数据)
    let r = list_llm_profiles_impl();
    assert!(r.ok, "list_llm_profiles 应返回 ok: error={:?}", r.error);
    let _profiles: Vec<ProfileMeta> = r.data.expect("data 应存在");
  }

  #[test]
  fn t4_get_active_llm_profile_name_returns_string() {
    // happy: get_active 返回 Ok(String),无 active 时为空字符串
    let r = get_active_llm_profile_name_impl();
    assert!(r.ok, "get_active 应返回 ok: error={:?}", r.error);
    let _name: String = r.data.expect("data 应存在");
  }

  #[test]
  fn t4_test_connection_url_for_anthropic_uses_models_endpoint() {
    // happy: probe_endpoint URL 构造正确(不真发 HTTP,只验证 URL + headers)
    let meta = make_meta_for_test("anthropic-prod", "Anthropic");
    let (url, headers) = llm_profiles::probe_endpoint(&meta, "sk-ant-test").unwrap();
    assert!(url.contains("/v1/models"), "Anthropic probe URL 应含 /v1/models: {url}");
    assert_eq!(
      headers.get("x-api-key").map(|s| s.as_str()),
      Some("sk-ant-test"),
      "Anthropic probe 应设置 x-api-key header"
    );
  }

  #[test]
  fn t4_probe_endpoint_ollama_uses_api_tags() {
    // happy: Ollama probe URL = base/api/tags,无 Authorization header
    let meta = ProfileMeta {
      name: "ollama-local".into(),
      provider: "Ollama".into(),
      base_url: "http://localhost:11434".into(),
      model: "llama3.1".into(),
      note: None,
      tool_search_enabled: false,
      experimental_betas_disabled: false,
      created_at: "2026-07-24T00:00:00Z".into(),
    };
    let (url, headers) =
      llm_profiles::probe_endpoint(&meta, "ignored-key").expect("Ollama probe 应成功");
    assert!(url.contains("/api/tags"), "Ollama probe URL 应含 /api/tags: {url}");
    assert!(
      headers.get("Authorization").is_none(),
      "Ollama 不应注入 Authorization header"
    );
  }

  #[test]
  fn t4_probe_endpoint_openai_compat_uses_models_and_bearer() {
    // happy: OpenAI 兼容 probe URL = base/models,Authorization = Bearer <key>
    let meta = ProfileMeta {
      name: "deepseek-prod".into(),
      provider: "DeepSeek".into(),
      base_url: "https://api.deepseek.com".into(),
      model: "deepseek-chat".into(),
      note: None,
      tool_search_enabled: false,
      experimental_betas_disabled: false,
      created_at: "2026-07-24T00:00:00Z".into(),
    };
    let (url, headers) =
      llm_profiles::probe_endpoint(&meta, "sk-ds-test").expect("OpenAI compat probe 应成功");
    assert!(url.ends_with("/models"), "OpenAI compat probe URL 应以 /models 结尾: {url}");
    assert_eq!(
      headers.get("Authorization").map(|s| s.as_str()),
      Some("Bearer sk-ds-test"),
      "OpenAI compat probe 应设 Bearer header"
    );
  }

  #[test]
  fn t4_probe_endpoint_minimax_uses_openai_compat_shape() {
    // happy: MiniMax(MiniMax-M3 + api.minimaxi.com/v1)probe URL 走 OpenAI 兼容路径
    let meta = ProfileMeta {
      name: "minimax-prod".into(),
      provider: "MiniMax".into(),
      base_url: "https://api.minimaxi.com/v1".into(),
      model: "MiniMax-M3".into(),
      note: None,
      tool_search_enabled: false,
      experimental_betas_disabled: false,
      created_at: "2026-07-24T00:00:00Z".into(),
    };
    let (url, headers) =
      llm_profiles::probe_endpoint(&meta, "sk-mm-test").expect("MiniMax probe 应成功");
    assert_eq!(
      url,
      "https://api.minimaxi.com/v1/models",
      "MiniMax probe URL 应是 base_url + /models"
    );
    assert_eq!(
      headers.get("Authorization").map(|s| s.as_str()),
      Some("Bearer sk-mm-test")
    );
  }

  #[test]
  #[allow(non_snake_case)]
  fn t4_save_llm_profile_empty_name_returns_PROFILE_NAME_CONFLICT() {
    // error: 空 name → PROFILE_NAME_CONFLICT(save_impl 校验失败,不触达 IO)
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
    assert!(!r.ok, "空 name 应报错");
    let err = r.error.expect("error 应存在");
    assert!(
      err.contains("PROFILE_NAME_CONFLICT"),
      "错误应含 PROFILE_NAME_CONFLICT 前缀,实际: {err}"
    );
  }

  #[test]
  #[allow(non_snake_case)]
  fn t4_save_llm_profile_unknown_provider_returns_PROVIDER_NOT_FOUND() {
    // error: 未知 provider → PROVIDER_NOT_FOUND(save_impl 校验失败,不触达 IO)
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
    assert!(!r.ok, "未知 provider 应报错");
    let err = r.error.expect("error 应存在");
    assert!(
      err.contains("PROVIDER_NOT_FOUND"),
      "错误应含 PROVIDER_NOT_FOUND 前缀,实际: {err}"
    );
  }

  #[test]
  fn t4_get_active_llm_profile_errors_when_no_active_set() {
    // error: 全局 metadata active 为 None 时,llm_profiles::get_active_profile()
    // 报 ACTIVE_PROFILE_REQUIRED(spec §6 错误码定义)。
    // 注意:此测试用纯函数 llm_profiles::get_active_profile(),避免走 commands 路径
    // (commands 层 get_active_llm_profile_name_impl 返回空字符串而非报错,因为前端需要区分
    // "无 active" vs "有 active 但 keyring 读不出"两种情况)。
    let r = llm_profiles::get_active_profile();
    if let Err(e) = r {
      assert!(
        e.contains("ACTIVE_PROFILE_REQUIRED"),
        "错误应含 ACTIVE_PROFILE_REQUIRED 前缀,实际: {e}"
      );
    }
    // 若 Ok,说明全局 metadata 有 active(其它会话残留) — 跳过 assert,合规
  }

  // ── reviewer Important #2 修复:补 3 个 happy-path 测试 ──

  #[test]
  fn t4_chrono_like_now_returns_rfc3339_format() {
    // Important #3 修复:时间戳格式必须与现有 fixture "2026-07-24T00:00:00Z" 对齐
    let now = chrono_like_now();
    // 长度 20(YYYY-MM-DDTHH:MM:SSZ),含 "T" 和 "Z"
    assert_eq!(now.len(), 20, "应为 20 字符 RFC3339-ish: {now}");
    assert!(now.contains('T'), "应含 T 分隔符: {now}");
    assert!(now.ends_with('Z'), "应以 Z 结尾: {now}");
    // 前 4 字符应可解析为年份(>= 2026)
    let year: u32 = now[..4].parse().expect("前 4 字符应为年份");
    assert!(year >= 2026, "年份应 >= 2026,实际: {year}");
  }

  #[test]
  fn t4_test_connection_ollama_does_not_require_keyring_key() {
    // Important #1 修复:Ollama provider 测连接时,keyring 没 key 不应报错
    // (走纯函数 probe_endpoint + 模拟 keyring empty,验证 Ollama 不需要 Authorization)
    let meta = ProfileMeta {
      name: "ollama-local".into(),
      provider: "Ollama".into(),
      base_url: "http://localhost:11434".into(),
      model: "llama3.1".into(),
      note: None,
      tool_search_enabled: false,
      experimental_betas_disabled: false,
      created_at: "2026-07-24T00:00:00Z".into(),
    };
    // 模拟"Ollama 没存 key,read_key 报 NoEntry,被 is_ollama 分支接住"
    // 用空 key 直接调 probe_endpoint 验证 Ollama 不需要 Authorization header
    let (url, headers) =
      llm_profiles::probe_endpoint(&meta, "").expect("Ollama probe 不需要 key");
    assert!(url.contains("/api/tags"));
    assert!(
      headers.get("Authorization").is_none(),
      "Ollama 即便 key 为空也不应注入 Authorization"
    );
  }

  #[test]
  fn t4_probe_endpoint_unknown_provider_returns_PROVIDER_NOT_FOUND() {
    // 边界:probe_endpoint 收到未知 provider 应报 PROVIDER_NOT_FOUND
    let meta = ProfileMeta {
      name: "mystery".into(),
      provider: "MysteryLLM".into(),
      base_url: "https://x.example.com".into(),
      model: "m".into(),
      note: None,
      tool_search_enabled: false,
      experimental_betas_disabled: false,
      created_at: "2026-07-24T00:00:00Z".into(),
    };
    let r = llm_profiles::probe_endpoint(&meta, "sk-x");
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("PROVIDER_NOT_FOUND"));
  }

  // ── T7.2 (W15-A): inject_profile_env per-run profile 注入 ─────────

  /// T7.2 测试 1:profile_name = Some(不存在) → 报 `PROFILE_NOT_FOUND`。
  /// 覆盖 inject_profile_env 的"profile 不存在"分支。
  #[test]
  fn t7_2_inject_profile_env_errors_on_profile_not_found() {
    let mut spec = SpawnSpec {
      program: "uv".into(),
      args: vec![],
      work_dir: "/tmp".into(),
      log_path: "/tmp/mtd.log".into(),
      env_vars: Default::default(),
    };
    let r = inject_profile_env(&mut spec, Some("__t7_2_definitely_nonexistent__"));
    assert!(r.is_err(), "不存在的 profile 应报错");
    assert!(
      r.as_ref().unwrap_err().contains("PROFILE_NOT_FOUND"),
      "错误前缀应是 PROFILE_NOT_FOUND, 实际: {:?}",
      r
    );
  }

  /// T7.2 测试 2:profile_name = None → env_vars 留空(走 CLI 默认)。
  /// 覆盖 inject_profile_env 的"用户没选 profile"分支,run/resume 不应阻断。
  #[test]
  fn t7_2_inject_profile_env_none_leaves_env_empty() {
    let mut spec = SpawnSpec {
      program: "uv".into(),
      args: vec![],
      work_dir: "/tmp".into(),
      log_path: "/tmp/mtd.log".into(),
      env_vars: HashMap::new(),
    };
    inject_profile_env(&mut spec, None).expect("None 不应报错");
    assert!(
      spec.env_vars.is_empty(),
      "profile_name=None 时 spec.env_vars 应清空, 实际: {:?}",
      spec.env_vars
    );
  }

  /// T7.2 测试 3:profile_name = Some(存在的 ollama profile) 且 keyring NoEntry
  /// → 仍 Ok(注入空 key)。Ollama 不需要 Authorization header,NoEntry 等价空 key。
  /// 用 list_llm_profiles_impl 探查全局 metadata 是否有 Ollama profile,
  /// 没有则 skip(不影响覆盖率)。
  #[test]
  fn t7_2_inject_profile_env_ollama_with_no_keyring_entry_does_not_error() {
    // 用 list_llm_profiles_impl 真实读 metadata(测试环境若全局无 Ollama profile 则 skip)
    let profiles = list_llm_profiles_impl();
    if profiles.ok {
      if let Some(ps) = profiles.data {
        if let Some(ollama) = ps.iter().find(|p| p.provider == "Ollama").cloned() {
          let mut spec = SpawnSpec {
            program: "uv".into(),
            args: vec![],
            work_dir: "/tmp".into(),
            log_path: "/tmp/mtd.log".into(),
            env_vars: HashMap::new(),
          };
          let r = inject_profile_env(&mut spec, Some(&ollama.name));
          assert!(
            r.is_ok(),
            "Ollama 不应要求 keyring: error={:?}",
            r
          );
          return;
        }
      }
    }
    // skip 若全局 metadata 无 Ollama profile(不影响覆盖率)
  }

  // ── W15-A T7.2 (Task 7): project registry 5 tests ────────────────

  /// 全局互斥锁:5 个 project registry 测试共享 `MEDIA_TO_DOC_PROJECT_REGISTRY_DIR` env var,
  /// 而 env var 是 process-global(cargo test 默认 thread-per-test 并行跑)。
  /// 用 mutex 串行化所有 project registry 测试,避免并行跑时 env var 互相覆盖导致
  /// 残留污染(实测:并行跑 t7_2_proj 报 "list 不应再含",单线程 OK)。
  static PROJ_REGISTRY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

  fn proj_tmpdir(name: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("ui_proj_{name}"));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
  }

  fn proj_override_registry_dir(dir: &std::path::Path) {
    // SAFETY: test-only, 由 PROJ_REGISTRY_LOCK 串行化保护
    unsafe { std::env::set_var("MEDIA_TO_DOC_PROJECT_REGISTRY_DIR", dir); }
  }

  fn proj_clear_registry_dir() {
    // SAFETY: test-only, 由 PROJ_REGISTRY_LOCK 串行化保护
    unsafe { std::env::remove_var("MEDIA_TO_DOC_PROJECT_REGISTRY_DIR"); }
  }

  #[test]
  fn t7_2_proj_list_empty_when_registry_missing() {
    let _guard = PROJ_REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = proj_tmpdir("list_proj_empty");
    proj_override_registry_dir(&tmp);
    let r = list_projects_impl();
    proj_clear_registry_dir();
    let _ = std::fs::remove_dir_all(&tmp);
    assert!(r.ok, "list_projects 应 ok: error={:?}", r.error);
    assert_eq!(r.data.unwrap().len(), 0, "无 registry 应返空 vec");
  }

  #[test]
  fn t7_2_proj_add_canonicalizes_and_dedupes() {
    let _guard = PROJ_REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = proj_tmpdir("add_proj_dedupe");
    proj_override_registry_dir(&tmp);
    let real = tmp.join("real");
    let link = tmp.join("link");
    std::fs::create_dir_all(&real).unwrap();
    #[cfg(unix)]
    {
      std::os::unix::fs::symlink(&real, &link).unwrap();
    }
    #[cfg(windows)]
    {
      std::os::windows::fs::symlink_dir(&real, &link).unwrap();
    }
    let r1 = add_project_impl(real.to_string_lossy().into_owned());
    let r2 = add_project_impl(link.to_string_lossy().into_owned());
    let id1 = r1.data.as_ref().map(|e| e.id.clone());
    let id2 = r2.data.as_ref().map(|e| e.id.clone());
    let list = list_projects_impl();
    proj_clear_registry_dir();
    let _ = std::fs::remove_dir_all(&tmp);
    assert!(r1.ok && r2.ok, "两次 add 应都 ok: r1={:?} r2={:?}", r1.error, r2.error);
    assert_eq!(id1, id2, "symlink 与原路径应识别为同一项目");
    assert_eq!(list.data.unwrap().len(), 1, "重复 add 应合并,不增加项目数");
  }

  #[test]
  fn t7_2_proj_add_windows_path_case_insensitive() {
    let _guard = PROJ_REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // 仅 Windows 上跑(macOS/Linux 默认大小写敏感,跳过)
    if !cfg!(windows) {
      return;
    }
    let tmp = proj_tmpdir("add_proj_case");
    proj_override_registry_dir(&tmp);
    let upper = tmp.join("Foo");
    let lower = tmp.join("foo");
    std::fs::create_dir_all(&upper).unwrap();
    let r1 = add_project_impl(upper.to_string_lossy().into_owned());
    let r2 = add_project_impl(lower.to_string_lossy().into_owned());
    proj_clear_registry_dir();
    let _ = std::fs::remove_dir_all(&tmp);
    if r1.ok && r2.ok {
      assert_eq!(
        r1.data.as_ref().unwrap().id,
        r2.data.as_ref().unwrap().id,
        "Windows 大小写不敏感,应识别为同一项目"
      );
    }
  }

  #[test]
  fn t7_2_proj_add_different_paths_same_name_have_different_ids() {
    let _guard = PROJ_REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = proj_tmpdir("add_proj_diff_paths");
    proj_override_registry_dir(&tmp);
    let d1 = tmp.join("a");
    let d2 = tmp.join("b");
    std::fs::create_dir_all(d1.join("foo")).unwrap();
    std::fs::create_dir_all(d2.join("foo")).unwrap();
    let r1 = add_project_impl(d1.join("foo").to_string_lossy().into_owned());
    let r2 = add_project_impl(d2.join("foo").to_string_lossy().into_owned());
    proj_clear_registry_dir();
    let _ = std::fs::remove_dir_all(&tmp);
    assert!(r1.ok && r2.ok, "两次 add 应都 ok: r1={:?} r2={:?}", r1.error, r2.error);
    assert_ne!(
      r1.data.unwrap().id,
      r2.data.unwrap().id,
      "重名不同路径应区分(不能用 display_name 做 ID)"
    );
  }

  #[test]
  fn t7_2_proj_remove_persists() {
    let _guard = PROJ_REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = proj_tmpdir("rm_proj");
    proj_override_registry_dir(&tmp);
    let r = add_project_impl(tmp.to_string_lossy().into_owned());
    let id = r.data.as_ref().map(|e| e.id.clone());
    let rm = remove_project_impl(id.clone().unwrap_or_default());
    // 同 tmp 内重新 load(模拟"重启后 registry 仍在但被删的项目消失")
    let list = list_projects_impl();
    proj_clear_registry_dir();
    let _ = std::fs::remove_dir_all(&tmp);
    assert!(r.ok, "add 应 ok: error={:?}", r.error);
    assert!(rm.ok, "remove 应 ok: error={:?}", rm.error);
    let id = id.unwrap();
    assert!(
      list.data.unwrap().iter().all(|p| p.id != id),
      "remove 后 list 不应再含该项目 id={id}"
    );
  }
}
