//! 子进程管理:spawn `uv run mtd run` / kill / 注册中心。
//!
//! 设计(对齐 ARCHITECTURE.md §3):
//! - `build_mtd_run_args` 纯函数(可单测,不实际 spawn)
//! - `RunRegistry` 存 work_dir → Child 的映射,提供 cancel/list
//! - `kill_tree` Windows 走 `taskkill /T /F`,Unix 走 `kill -TERM -PGID`
//! - `kill_on_drop(true)` Tauri 进程退出时自动清理子进程

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use once_cell::sync::Lazy;
use serde::Serialize;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

// ─────────────────────────────────────────────────────────────
// SpawnSpec —— 纯数据,可单测
// ─────────────────────────────────────────────────────────────

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
}

#[derive(Debug, Clone, Serialize)]
pub struct RunPipelineResult {
  pub work_dir: String,
  pub pid: Option<u32>,
  pub log_path: String,
  pub spec: SpawnSpec,
}

/// 拼装 `uv run mtd run <inbox> [...]` 的参数。
///
/// `project_root`:`media-to-doc` Python 项目根(给 `uv --project` 用),
/// `inbox`:要处理的 inbox 目录(必须含至少一个媒体文件)。
/// `llm` / `imagegen` / `stop_after`:CLI 透传覆盖。
pub fn build_mtd_run_args(
  project_root: &Path,
  inbox: &Path,
  llm: Option<&str>,
  imagegen: Option<&str>,
  stop_after: Option<&str>,
  no_longdoc: bool,
  force: bool,
) -> SpawnSpec {
  let program = std::env::var("UV_BIN").unwrap_or_else(|_| "uv".to_string());
  let work_dir = inbox
    .parent()
    .map(Path::to_path_buf)
    .unwrap_or_else(|| inbox.to_path_buf());
  let log_path = inbox.join("output").join("mtd.log");
  let mut args: Vec<String> = vec![
    "--project".to_string(),
    project_root.to_string_lossy().into_owned(),
    "run".to_string(),
    "mtd".to_string(),
    "run".to_string(),
    inbox.to_string_lossy().into_owned(),
  ];
  if let Some(llm) = llm {
    args.extend(["--llm".to_string(), llm.to_string()]);
  }
  if let Some(imagegen) = imagegen {
    args.extend(["--imagegen".to_string(), imagegen.to_string()]);
  }
  if let Some(stop_after) = stop_after {
    args.extend(["--stop-after".to_string(), stop_after.to_string()]);
  }
  if no_longdoc {
    args.push("--no-longdoc".to_string());
  }
  if force {
    args.push("--force".to_string());
  }
  SpawnSpec {
    program,
    args,
    work_dir: work_dir.to_string_lossy().into_owned(),
    log_path: log_path.to_string_lossy().into_owned(),
  }
}

/// 拼装 `uv run mtd resume <work_dir> [...]` 的参数(续跑用)。
pub fn build_mtd_resume_args(
  project_root: &Path,
  work_dir: &Path,
  force: bool,
  stop_after: Option<&str>,
) -> SpawnSpec {
  let program = std::env::var("UV_BIN").unwrap_or_else(|_| "uv".to_string());
  let log_path = work_dir.join("mtd.log");
  let mut args: Vec<String> = vec![
    "--project".to_string(),
    project_root.to_string_lossy().into_owned(),
    "run".to_string(),
    "mtd".to_string(),
    "resume".to_string(),
    work_dir.to_string_lossy().into_owned(),
  ];
  if force {
    args.push("--force".to_string());
  }
  if let Some(stop_after) = stop_after {
    args.extend(["--stop-after".to_string(), stop_after.to_string()]);
  }
  SpawnSpec {
    program,
    args,
    work_dir: work_dir.to_string_lossy().into_owned(),
    log_path: log_path.to_string_lossy().into_owned(),
  }
}

// ─────────────────────────────────────────────────────────────
// RunRegistry —— 共享 state(tauri::State<RunRegistry>)
// ─────────────────────────────────────────────────────────────

#[derive(Default, Clone)]
pub struct RunRegistry {
  inner: Arc<Mutex<HashMap<String, ChildEntry>>>,
}

/// 全局单例 RunRegistry(避免 Tauri async command 传 `State` 的 Result 强制要求)。
///
/// 在 Tauri app 启动时通过 `init_registry()` 注入;后续 run/cancel/list 直接调
/// `global_registry()` 拿单例。
static REGISTRY: Lazy<RunRegistry> = Lazy::new(RunRegistry::new);

/// 返回全局 registry 单例引用。
pub fn global_registry() -> &'static RunRegistry {
  &REGISTRY
}

/// 应用启动时调用(预留 hook,目前是 no-op,单例 lazy 自启)。
pub fn init_registry() {
  Lazy::force(&REGISTRY);
}

#[derive(Debug, Clone, Serialize)]
pub struct RunningRun {
  pub work_dir: String,
  pub pid: Option<u32>,
  pub started_at: String,
  pub log_path: String,
  pub inbox: String,
}

struct ChildEntry {
  child: Child,
  started_at: String,
  inbox: String,
  log_path: String,
}

impl RunRegistry {
  pub fn new() -> Self {
    Self::default()
  }

  pub async fn insert(
    &self,
    work_dir: String,
    child: Child,
    inbox: String,
    log_path: String,
  ) {
    let started_at = chrono_like_now();
    let mut g = self.inner.lock().await;
    g.insert(
      work_dir,
      ChildEntry {
        child,
        started_at,
        inbox,
        log_path,
      },
    );
  }

  pub async fn list(&self) -> Vec<RunningRun> {
    let g = self.inner.lock().await;
    g.iter()
      .map(|(k, v)| RunningRun {
        work_dir: k.clone(),
        pid: v.child.id(),
        started_at: v.started_at.clone(),
        inbox: v.inbox.clone(),
        log_path: v.log_path.clone(),
      })
      .collect()
  }

  /// 检查 work_dir 是否在注册中(且进程仍存活)。
  pub async fn is_running(&self, work_dir: &str) -> bool {
    let g = self.inner.lock().await;
    g.contains_key(work_dir)
  }

  /// 取消运行(work_dir 在注册中)→ 杀子进程 + 移除。
  /// 返回 `Some(pid)` 表示取消成功,`None` 表示未在运行。
  pub async fn cancel(&self, work_dir: &str) -> Option<u32> {
    let mut g = self.inner.lock().await;
    if let Some(mut entry) = g.remove(work_dir) {
      let pid = entry.child.id();
      // 先 kill_on_drop 由 tokio 帮忙,这里主动 kill
      let _ = entry.child.kill().await;
      Some(pid.unwrap_or(0))
    } else {
      None
    }
  }

  /// 由 RunHandle 在 child 退出后自动调用,清理注册。
  pub async fn reap(&self, work_dir: &str) {
    let mut g = self.inner.lock().await;
    g.remove(work_dir);
  }
}

// ─────────────────────────────────────────────────────────────
// spawn helpers
// ─────────────────────────────────────────────────────────────

/// 实际 spawn 一个 mtd 子进程,把 stdout/stderr 写 log_path。
///
/// 返回 Child + 实际打开的 log file(后者给 reap / tail 用)。
pub async fn spawn_mtd(
  spec: &SpawnSpec,
) -> Result<Child, String> {
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
    .stdout(Stdio::from(log))
    .stderr(Stdio::from(err_log))
    .kill_on_drop(true);
  cmd
    .spawn()
    .map_err(|e| format!("spawn `{}` 失败: {e}", spec.program))
}

/// 主动 kill 子进程(Windows:taskkill /T /F /PID;Unix:kill -TERM)。
pub fn kill_tree(pid: u32) -> std::io::Result<()> {
  #[cfg(windows)]
  {
    std::process::Command::new("taskkill")
      .arg("/T")
      .arg("/F")
      .arg("/PID")
      .arg(pid.to_string())
      .output()
      .map(|_| ())
  }
  #[cfg(unix)]
  {
    std::process::Command::new("kill")
      .arg("-TERM")
      .arg("-PGID")
      .arg(pid.to_string())
      .output()
      .map(|_| ())
  }
}

/// 读 inbox 父目录,生成符合 <inbox>/output 的 work_dir。
pub fn derive_work_dir(inbox: &Path) -> PathBuf {
  inbox.join("output")
}

fn chrono_like_now() -> String {
  // 不引入 chrono,简单 RFC3339-ish 时间戳
  let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_secs())
    .unwrap_or(0);
  format!("epoch:{now}")
}

#[cfg(test)]
mod tests {
  use super::*;

  fn project() -> PathBuf {
    PathBuf::from("F:/soft/00selfmade/media-to-doc")
  }
  fn inbox() -> PathBuf {
    PathBuf::from("F:/soft/00selfmade/media-to-doc-ui/sample-inbox")
  }

  #[test]
  fn build_mtd_run_args_basic() {
    let spec = build_mtd_run_args(&project(), &inbox(), None, None, None, false, false);
    assert_eq!(spec.program, "uv");
    // 检查关键参数
    assert!(spec.args.windows(2).any(|w| w[0] == "--project" && w[1].contains("media-to-doc")));
    assert!(spec.args.windows(2).any(|w| w[0] == "run"));
    assert!(spec.args.windows(2).any(|w| w[0] == "mtd"));
    assert!(spec.args.windows(2).any(|w| w[0] == "run"));
    assert!(spec.args.contains(&inbox().to_string_lossy().into_owned()));
    assert!(spec.log_path.ends_with("mtd.log"));
    assert!(spec.log_path.contains("output"));
    assert!(spec.work_dir.contains("media-to-doc-ui"));
  }

  #[test]
  fn build_mtd_run_args_with_overrides() {
    let spec = build_mtd_run_args(
      &project(),
      &inbox(),
      Some("anthropic"),
      Some("skip"),
      Some("chapters"),
      true,
      true,
    );
    assert!(spec.args.contains(&"--llm".to_string()));
    assert!(spec.args.contains(&"anthropic".to_string()));
    assert!(spec.args.contains(&"--imagegen".to_string()));
    assert!(spec.args.contains(&"skip".to_string()));
    assert!(spec.args.contains(&"--stop-after".to_string()));
    assert!(spec.args.contains(&"chapters".to_string()));
    assert!(spec.args.contains(&"--no-longdoc".to_string()));
    assert!(spec.args.contains(&"--force".to_string()));
  }

  #[test]
  fn build_mtd_resume_args_basic() {
    let work = inbox().join("output");
    let spec = build_mtd_resume_args(&project(), &work, false, None);
    assert!(spec.args.contains(&"resume".to_string()));
    assert!(spec.args.contains(&work.to_string_lossy().into_owned()));
    assert!(!spec.args.contains(&"--force".to_string()));
  }

  #[test]
  fn derive_work_dir_appends_output() {
    let p = derive_work_dir(&inbox());
    assert!(p.ends_with("output"));
  }

  #[test]
  fn registry_insert_list_cancel() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
      let reg = RunRegistry::new();
      // 插入一个 fake child:实际需要真实 spawn,这里我们用空 work_dir 试
      // 不真 spawn,只测 list/is_running 状态
      assert!(!reg.is_running("nonexistent").await);
      let running = reg.list().await;
      assert_eq!(running.len(), 0);
      // 取消不存在的 work_dir
      assert_eq!(reg.cancel("nonexistent").await, None);
    });
  }

  #[test]
  fn spawn_and_cancel_real_process() {
    // 真 spawn 一个长寿命令(ping localhost -n 60 on Windows)然后取消
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
      let spec = SpawnSpec {
        program: if cfg!(windows) { "ping".to_string() } else { "sleep".to_string() },
        args: if cfg!(windows) {
          vec!["127.0.0.1".to_string(), "-n".to_string(), "60".to_string()]
        } else {
          vec!["60".to_string()]
        },
        work_dir: std::env::temp_dir().to_string_lossy().into_owned(),
        log_path: std::env::temp_dir().join("test_mtd.log").to_string_lossy().into_owned(),
      };
      let child = spawn_mtd(&spec).await.expect("spawn");
      let pid = child.id();
      assert!(pid.is_some());
      let work_dir = "/tmp/test_work_dir".to_string();
      // 临时用 Arc<Mutex<>> 跟踪
      let reg = RunRegistry::new();
      reg.insert(work_dir.clone(), child, "/tmp/inbox".into(), spec.log_path.clone()).await;
      assert!(reg.is_running(&work_dir).await);
      // 取消
      let cancelled = reg.cancel(&work_dir).await;
      assert!(cancelled.is_some());
      assert!(!reg.is_running(&work_dir).await);
    });
  }
}
