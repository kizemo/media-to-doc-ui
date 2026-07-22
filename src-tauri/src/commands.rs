//! 8 个 Tauri commands —— 对齐 media-to-doc MCP 8 工具(W7=6 + W8=2)。
//!
//! 结构(每个命令):
//! - `*_impl` 纯函数(可单测,不入 Tauri 状态)
//! - `#[tauri::command]` 薄包装(只做参数透传 + State 注入)
//!
//! 错误:Path 不存在 / state.json 缺失 / version 非法 / spawn 失败
//! → `CommandResponse::err`。

use std::path::{Path, PathBuf};

use crate::runner::{
  build_mtd_resume_args, build_mtd_run_args, derive_work_dir, global_registry, kill_tree,
  spawn_mtd, RunPipelineResult, RunningRun,
};

use serde::Serialize;

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
  #[serde(skip_serializing_if = "Option::is_none")]
  pub note: Option<String>,
}

const VERSION_TPL: &[(&str, &str, &str)] = &[
  ("raw", "md", "{stem}.md"),
  ("raw", "html", "{stem}.html"),
  ("cleaned", "md", "{stem}_cleaned.md"),
  ("cleaned", "html", "{stem}_cleaned.html"),
  ("final", "md", "{stem}_final.md"),
  ("final", "html", "{stem}_final.html"),
];

pub fn read_lecture_impl(
  inbox_dir: String,
  version: String,
  fmt: Option<String>,
) -> CommandResponse<ReadLectureResult> {
  let fmt = fmt.unwrap_or_else(|| "md".to_string());
  let tpl = VERSION_TPL
    .iter()
    .find(|(v, f, _)| *v == version && *f == fmt)
    .map(|(_, _, t)| *t);
  let tpl = match tpl {
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
  let stem = derive_stem(&work);
  let rel = tpl.replace("{stem}", &stem);
  let target = work.join("chapters").join("raw").join(&stem).join(&rel);
  // html fallback 到 md 内容(对齐 Python 版)
  if !target.is_file() && fmt == "html" {
    let alt_rel = match version.as_str() {
      "raw" => format!("{stem}.md"),
      "cleaned" => format!("{stem}_cleaned.md"),
      "final" => format!("{stem}_final.md"),
      _ => unreachable!(),
    };
    let alt = work.join("chapters").join("raw").join(&stem).join(&alt_rel);
    if alt.is_file() {
      let content = match std::fs::read_to_string(&alt) {
        Ok(s) => s,
        Err(e) => return CommandResponse::err(format!("读 {} 失败: {e}", alt.display())),
      };
      return CommandResponse::ok(ReadLectureResult {
        version: version.clone(),
        fmt,
        path: alt.to_string_lossy().into_owned(),
        content: format!(
          "# {stem} ({version} · html)\n\n(html 版本不存在,以下为 md 版本内容)\n\n{content}"
        ),
        size_bytes: content.len(),
        note: Some("html 版本未生成,fallback 到 md".to_string()),
      });
    }
    return CommandResponse::err(format!("讲义文件不存在: {}\n可能是 longdoc 阶段未跑或 render 未生成", target.display()));
  }
  if !target.is_file() {
    return CommandResponse::err(format!("讲义文件不存在: {}\n请先用 run_pipeline 跑流水线", target.display()));
  }
  let content = match std::fs::read_to_string(&target) {
    Ok(s) => s,
    Err(e) => return CommandResponse::err(format!("读 {} 失败: {e}", target.display())),
  };
  CommandResponse::ok(ReadLectureResult {
    version,
    fmt,
    path: target.to_string_lossy().into_owned(),
    size_bytes: content.len(),
    content,
    note: None,
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

fn default_workspace_root() -> PathBuf {
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
    let inbox = tmpdir("read_lecture_fallback");
    let work = inbox.join("output");
    let raw = work.join("chapters").join("raw").join("demo");
    fs::create_dir_all(&raw).unwrap();
    fs::write(raw.join("demo_cleaned.md"), b"# cleaned md").unwrap();
    // no html
    let r = read_lecture_impl(
      inbox.to_string_lossy().into_owned(),
      "cleaned".into(),
      Some("html".into()),
    );
    assert!(r.ok);
    let data = r.data.unwrap();
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
}

// ─────────────────────────────────────────────────────────────
// run_pipeline / resume_pipeline / cancel_run / list_running
// (T3:子进程管理,对齐 MCP run_pipeline / resume_pipeline + 自有 cancel/list)
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
  let spec = build_mtd_run_args(
    &project,
    &inbox,
    llm.as_deref(),
    imagegen.as_deref(),
    stop_after.as_deref(),
    no_longdoc.unwrap_or(false),
    force.unwrap_or(false),
  );
  let work_dir = derive_work_dir(&inbox);
  let work_dir_str = work_dir.to_string_lossy().into_owned();
  if registry.is_running(&work_dir_str).await {
    return CommandResponse::err(format!(
      "该 work_dir 已在运行: {work_dir_str}\n请先 cancel_run 或 list_running 检查"
    ));
  }
  let child = match spawn_mtd(&spec).await {
    Ok(c) => c,
    Err(e) => return CommandResponse::err(e),
  };
  let pid = child.id();
  let log_path = spec.log_path.clone();
  registry
    .insert(work_dir_str.clone(), child, inbox.to_string_lossy().into_owned(), log_path.clone())
    .await;
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
  let spec = build_mtd_resume_args(
    &project,
    &work,
    force.unwrap_or(false),
    stop_after.as_deref(),
  );
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
  registry
    .insert(work_dir_str.clone(), child, inbox_for_registry, log_path.clone())
    .await;
  CommandResponse::ok(RunPipelineResult {
    work_dir: work_dir_str,
    pid,
    log_path,
    spec,
  })
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

#[tauri::command]
pub async fn list_running() -> CommandResponse<ListRunningResult> {
  let running = global_registry().list().await;
  CommandResponse::ok(ListRunningResult { running })
}

// ─────────────────────────────────────────────────────────────
// T3 unit tests
// ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod runner_tests {
  use super::*;

  fn project() -> PathBuf {
    PathBuf::from("F:/soft/00selfmade/media-to-doc")
  }

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
}
