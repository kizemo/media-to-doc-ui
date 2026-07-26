//! 子进程管理:spawn `uv run mtd run` / kill / 注册中心。
//!
//! 设计(对齐 ARCHITECTURE.md §3):
//! - `build_mtd_run_args` 纯函数(可单测,不实际 spawn)
//! - `RunRegistry` 存 work_dir → Child 的映射,提供 cancel/list
//! - `kill_tree` Windows 走 `taskkill /T /F`,Unix 走 `kill -TERM -PGID`
//! - `kill_on_drop(true)` Tauri 进程退出时自动清理子进程
//! - W14-C:max_concurrent=3(env override) + completed LRU 100 + cancel 2s 超时

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
  /// W15-A (T5):注入子进程的 env vars(由 active LLM profile + keyring key 生成)。
  /// `spawn_mtd` 会先 `.env_clear()` 防父进程 HTTP_PROXY 污染(W14-D 思路),
  /// 再 `.envs(&spec.env_vars)` 注入这一份。默认空 HashMap。
  #[serde(default)]
  pub env_vars: HashMap<String, String>,
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
///
/// W15-A T7.2:三个新参数 `llm_profile_name` / `image_agent_profile_name` /
/// `task_text` —— Tauri UI 是 profile 单一真相源,主仓不会自己查 profile
/// (无 registry);这 3 个 flag 仅作 logging / debug(让 spawn cmd line 显示
/// 用户选了哪个 profile / 任务文本)。真实认证仍走 `spec.env_vars`(`inject_
/// profile_env`)。
pub fn build_mtd_run_args(
  project_root: &Path,
  inbox: &Path,
  llm: Option<&str>,
  imagegen: Option<&str>,
  stop_after: Option<&str>,
  no_longdoc: bool,
  force: bool,
  // W15-A T7.2:per-run profile + task_text 透传(仅 logging)
  llm_profile_name: Option<&str>,
  image_agent_profile_name: Option<&str>,
  task_text: Option<&str>,
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
  // W15-A T7.2:per-run profile + task_text 透传(主仓仅 logging)
  if let Some(n) = llm_profile_name {
    args.extend(["--llm-profile-name".to_string(), n.to_string()]);
  }
  if let Some(n) = image_agent_profile_name {
    args.extend(["--image-agent-profile-name".to_string(), n.to_string()]);
  }
  if let Some(t) = task_text {
    args.extend(["--task-text".to_string(), t.to_string()]);
  }
  SpawnSpec {
    program,
    args,
    work_dir: work_dir.to_string_lossy().into_owned(),
    log_path: log_path.to_string_lossy().into_owned(),
    env_vars: HashMap::new(),
  }
}

/// 拼装 `uv run mtd resume <work_dir> [...]` 的参数(续跑用)。
///
/// W15-A T7.2:同步支持 3 个新 flag(同 `build_mtd_run_args` 语义)。
pub fn build_mtd_resume_args(
  project_root: &Path,
  work_dir: &Path,
  force: bool,
  stop_after: Option<&str>,
  // W15-A T7.2
  llm_profile_name: Option<&str>,
  image_agent_profile_name: Option<&str>,
  task_text: Option<&str>,
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
  // W15-A T7.2:per-run profile + task_text 透传(主仓仅 logging)
  if let Some(n) = llm_profile_name {
    args.extend(["--llm-profile-name".to_string(), n.to_string()]);
  }
  if let Some(n) = image_agent_profile_name {
    args.extend(["--image-agent-profile-name".to_string(), n.to_string()]);
  }
  if let Some(t) = task_text {
    args.extend(["--task-text".to_string(), t.to_string()]);
  }
  SpawnSpec {
    program,
    args,
    work_dir: work_dir.to_string_lossy().into_owned(),
    log_path: log_path.to_string_lossy().into_owned(),
    env_vars: HashMap::new(),
  }
}

// ─────────────────────────────────────────────────────────────
// RunRegistry —— 共享 state(W14-C:max_concurrent + LRU)
// ─────────────────────────────────────────────────────────────

const DEFAULT_MAX_CONCURRENT: usize = 3;
const COMPLETED_LRU_CAP: usize = 100;

fn max_concurrent_from_env() -> usize {
  std::env::var("MEDIA_TO_DOC_MAX_CONCURRENT")
    .ok()
    .and_then(|v| v.parse::<usize>().ok())
    .unwrap_or(DEFAULT_MAX_CONCURRENT)
}

#[derive(Clone)]
pub struct RunRegistry {
  inner: Arc<Mutex<HashMap<String, ChildEntry>>>,
  max_concurrent: usize,
  completed: Arc<Mutex<Vec<CompletedRun>>>,
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

/// 已完成(cancelled/failed/completed)的 run 记录(LRU 100)。
#[derive(Debug, Clone, Serialize)]
pub struct CompletedRun {
  pub work_dir: String,
  pub pid: Option<u32>,
  pub started_at: String,
  pub finished_at: String,
  pub inbox: String,
  pub log_path: String,
  /// "completed" | "cancelled" | "failed"
  pub status: String,
}

/// 前端用的统一 run 信息(含 running + completed)。
#[derive(Debug, Clone, Serialize)]
pub struct RunStatusInfo {
  pub work_dir: String,
  pub pid: Option<u32>,
  pub started_at: String,
  pub finished_at: Option<String>,
  pub inbox: String,
  pub log_path: String,
  /// "running" | "completed" | "cancelled" | "failed"
  pub status: String,
}

struct ChildEntry {
  child: Child,
  started_at: String,
  inbox: String,
  log_path: String,
}

impl RunRegistry {
  pub fn new() -> Self {
    Self {
      inner: Arc::new(Mutex::new(HashMap::new())),
      max_concurrent: max_concurrent_from_env(),
      completed: Arc::new(Mutex::new(Vec::new())),
    }
  }

  /// 返回当前最大并发数。
  pub fn max_concurrent(&self) -> usize {
    self.max_concurrent
  }

  /// 返回当前正在运行的 run 数量。
  pub async fn running_count(&self) -> usize {
    self.inner.lock().await.len()
  }

  /// 插入新 run;若已达并发上限则返回错误。
  /// 成功返回 Ok(()) 并自动 spawn 后台监控(子进程退出 → reap)。
  pub async fn insert(
    &self,
    work_dir: String,
    mut child: Child,
    inbox: String,
    log_path: String,
  ) -> Result<(), String> {
    let started_at = chrono_like_now();
    let pid = child.id();
    {
      let mut g = self.inner.lock().await;
      if g.len() >= self.max_concurrent {
        // 拒绝前先清理刚 spawn 的进程
        let _ = child.kill().await;
        return Err(format!(
          "并发上限已达(max={}),当前 {} 个任务运行中。请等待或 cancel 后再试。",
          self.max_concurrent,
          g.len()
        ));
      }
      g.insert(
        work_dir.clone(),
        ChildEntry {
          child,
          started_at: started_at.clone(),
          inbox: inbox.clone(),
          log_path: log_path.clone(),
        },
      );
    }
    // 后台监控:子进程退出 → 自动 reap
    let wd = work_dir.clone();
    tokio::spawn(async move {
      // 等子进程退出(cancel 也会触发退出,此时 wait() 立刻返回)
      let exit_status = {
        let mut g = REGISTRY.inner.lock().await;
        if let Some(mut entry) = g.remove(&wd) {
          match entry.child.wait().await {
            Ok(s) if s.success() => "completed".to_string(),
            Ok(_) => "failed".to_string(),
            Err(_) => "failed".to_string(),
          }
        } else {
          // 已被 cancel 拿走,不需要 reap
          return;
        }
      };
      REGISTRY.push_completed(CompletedRun {
        work_dir: wd.clone(),
        pid,
        started_at,
        finished_at: chrono_like_now(),
        inbox,
        log_path,
        status: exit_status,
      }).await;
    });
    Ok(())
  }

  /// 列出当前运行中的所有 run。
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

  /// 列出全量 run(running + completed),按 started_at 降序。
  pub async fn list_all(&self) -> Vec<RunStatusInfo> {
    let mut out: Vec<RunStatusInfo> = Vec::new();
    // running
    {
      let g = self.inner.lock().await;
      for (k, v) in g.iter() {
        out.push(RunStatusInfo {
          work_dir: k.clone(),
          pid: v.child.id(),
          started_at: v.started_at.clone(),
          finished_at: None,
          inbox: v.inbox.clone(),
          log_path: v.log_path.clone(),
          status: "running".to_string(),
        });
      }
    }
    // completed
    {
      let completed = self.completed.lock().await;
      for c in completed.iter().rev() {
        out.push(RunStatusInfo {
          work_dir: c.work_dir.clone(),
          pid: c.pid,
          started_at: c.started_at.clone(),
          finished_at: Some(c.finished_at.clone()),
          inbox: c.inbox.clone(),
          log_path: c.log_path.clone(),
          status: c.status.clone(),
        });
      }
    }
    // 降序(最新的在前)
    out.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    out
  }

  /// 检查 work_dir 是否在注册中(且进程仍存活)。
  pub async fn is_running(&self, work_dir: &str) -> bool {
    let g = self.inner.lock().await;
    g.contains_key(work_dir)
  }

  /// 取消运行(work_dir 在注册中)→ 杀子进程 + 移除 + 记入 completed。
  /// 最多等待 2 秒让进程优雅退出;超时仍杀进程树 + 记 completed。
  /// 返回 `Some(pid)` 表示取消成功,`None` 表示未在运行。
  pub async fn cancel(&self, work_dir: &str) -> Option<u32> {
    let mut g = self.inner.lock().await;
    if let Some(mut entry) = g.remove(work_dir) {
      let pid = entry.child.id().unwrap_or(0);
      // 先尝试优雅 kill
      let _ = entry.child.kill().await;
      // 2 秒超时等待
      let waited = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        entry.child.wait(),
      )
      .await;
      // 无论是否超时,都杀进程树兜底
      if waited.is_err() {
        let _ = kill_tree(pid);
      }
      // 记入 completed LRU
      self.push_completed(CompletedRun {
        work_dir: work_dir.to_string(),
        pid: Some(pid),
        started_at: entry.started_at,
        finished_at: chrono_like_now(),
        inbox: entry.inbox,
        log_path: entry.log_path,
        status: "cancelled".to_string(),
      })
      .await;
      Some(pid)
    } else {
      None
    }
  }

  /// 由后台监控在 child 退出后自动调用,清理注册 + 记 completed。
  pub async fn reap(&self, work_dir: &str, exit_status: String) {
    let mut g = self.inner.lock().await;
    if let Some(entry) = g.remove(work_dir) {
      self.push_completed(CompletedRun {
        work_dir: work_dir.to_string(),
        pid: entry.child.id(),
        started_at: entry.started_at,
        finished_at: chrono_like_now(),
        inbox: entry.inbox,
        log_path: entry.log_path,
        status: exit_status,
      })
      .await;
    }
  }

  /// 写 completed 并维护 LRU 上限;如果 env 开启则持久化 runs.json。
  async fn push_completed(&self, entry: CompletedRun) {
    let mut completed = self.completed.lock().await;
    completed.push(entry);
    while completed.len() > COMPLETED_LRU_CAP {
      completed.remove(0);
    }
    // 持久化(opt-in)
    if std::env::var("MEDIA_TO_DOC_RUNS_PERSIST").as_deref() == Ok("true") {
      let _ = maybe_persist_runs_json(&completed);
    }
  }

  /// 返回 completed runs 的快照(只读)。
  pub async fn completed_snapshot(&self) -> Vec<CompletedRun> {
    self.completed.lock().await.clone()
  }
}

fn maybe_persist_runs_json(completed: &[CompletedRun]) -> Result<(), String> {
  let ws = crate::commands::default_workspace_root();
  if !ws.exists() {
    let _ = std::fs::create_dir_all(&ws);
  }
  let path = ws.join("runs.json");
  let json = serde_json::to_string_pretty(completed).map_err(|e| format!("序列化失败: {e}"))?;
  std::fs::write(&path, json).map_err(|e| format!("写 runs.json 失败: {e}"))
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
  let mut cmd = build_child_command(spec);
  cmd.stdout(Stdio::from(log))
    .stderr(Stdio::from(err_log));
  cmd
    .spawn()
    .map_err(|e| format!("spawn `{}` 失败: {e}", spec.program))
}

/// 从 SpawnSpec 构造 tokio::process::Command(纯函数,可单测)。
///
/// W15-A (T5) 关键行为:
/// - `.env_clear()` — 清空父进程 env vars(W14-D trust_env=False 思路,
///   防公司 VPN HTTP_PROXY 等 8 个 proxy vars 污染子进程撞 SSL/DNS)
/// - `.env("PATH", ...)` — 重新注入父进程 PATH,让 uv / sh 等可执行文件
///   在子进程里仍可被找到(env_clear 后 PATH 也被清了,uv 不在 System32,
///   Windows CreateProcess 会报"系统找不到指定的文件")
/// - `.envs(&spec.env_vars)` — 注入 active LLM profile 生成的 env vars
///   (ANTHROPIC_API_KEY / OPENAI_API_KEY / OLLAMA_HOST 等)。
///   spec.env_vars 不会注入 PATH,即使同名也仅覆盖,无副作用。
pub fn build_child_command(spec: &SpawnSpec) -> Command {
  let mut cmd = Command::new(&spec.program);
  cmd.args(&spec.args)
    .current_dir(&spec.work_dir)
    .env_clear()
    .env("PATH", std::env::var("PATH").unwrap_or_default())
    .envs(&spec.env_vars)
    .kill_on_drop(true);
  cmd
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
    let spec = build_mtd_run_args(&project(), &inbox(), None, None, None, false, false, None, None, None);
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
      None, None, None,
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
    let spec = build_mtd_resume_args(&project(), &work, false, None, None, None, None);
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
  fn registry_max_concurrent_default() {
    let reg = RunRegistry::new();
    assert_eq!(reg.max_concurrent(), DEFAULT_MAX_CONCURRENT);
  }

  #[test]
  fn registry_rejects_when_full() {
    // 设定 max_concurrent=1,验证 insert 被拒
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
      let reg = RunRegistry {
        inner: Arc::new(Mutex::new(HashMap::new())),
        max_concurrent: 1,
        completed: Arc::new(Mutex::new(Vec::new())),
      };
      // spawn a real long-running process
      let child = spawn_mtd(&SpawnSpec {
        program: if cfg!(windows) { "ping".to_string() } else { "sleep".to_string() },
        args: if cfg!(windows) {
          vec!["127.0.0.1".to_string(), "-n".to_string(), "30".to_string()]
        } else {
          vec!["30".to_string()]
        },
        work_dir: std::env::temp_dir().to_string_lossy().into_owned(),
        log_path: std::env::temp_dir().join("test_max1.log").to_string_lossy().into_owned(),
        env_vars: HashMap::new(),
      }).await.expect("spawn 1");
      let r1 = reg.insert("wd1".into(), child, "/inbox/a".into(), "/log/a".into()).await;
      assert!(r1.is_ok());
      assert_eq!(reg.running_count().await, 1);
      // 尝试插入第二个(不需要真实 child)
      let child2 = spawn_mtd(&SpawnSpec {
        program: if cfg!(windows) { "ping".to_string() } else { "sleep".to_string() },
        args: if cfg!(windows) {
          vec!["127.0.0.1".to_string(), "-n".to_string(), "5".to_string()]
        } else {
          vec!["5".to_string()]
        },
        work_dir: std::env::temp_dir().to_string_lossy().into_owned(),
        log_path: std::env::temp_dir().join("test_max2.log").to_string_lossy().into_owned(),
        env_vars: HashMap::new(),
      }).await.expect("spawn 2");
      let r2 = reg.insert("wd2".into(), child2, "/inbox/b".into(), "/log/b".into()).await;
      assert!(r2.is_err());
      assert!(r2.unwrap_err().contains("并发上限"));
      // cleanup
      reg.cancel("wd1").await;
    });
  }

  #[test]
  fn registry_cancel_and_completed_lru() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
      let reg = RunRegistry {
        inner: Arc::new(Mutex::new(HashMap::new())),
        max_concurrent: 3,
        completed: Arc::new(Mutex::new(Vec::new())),
      };
      // 插入 fake child
      let child = spawn_mtd(&SpawnSpec {
        program: if cfg!(windows) { "ping".to_string() } else { "sleep".to_string() },
        args: if cfg!(windows) {
          vec!["127.0.0.1".to_string(), "-n".to_string(), "5".to_string()]
        } else {
          vec!["5".to_string()]
        },
        work_dir: std::env::temp_dir().to_string_lossy().into_owned(),
        log_path: std::env::temp_dir().join("test_ccl.log").to_string_lossy().into_owned(),
        env_vars: HashMap::new(),
      }).await.expect("spawn");
      reg.insert("wd_c".into(), child, "/inbox/c".into(), "/log/c".into()).await.unwrap();
      // cancel
      let pid = reg.cancel("wd_c").await;
      assert!(pid.is_some());
      // 检查 completed
      let completed = reg.completed_snapshot().await;
      assert_eq!(completed.len(), 1);
      assert_eq!(completed[0].work_dir, "wd_c");
      assert_eq!(completed[0].status, "cancelled");
      // 检查不再 running
      assert!(!reg.is_running("wd_c").await);
    });
  }

  #[test]
  fn registry_cancel_nonexistent_returns_none() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
      let reg = RunRegistry::new();
      assert_eq!(reg.cancel("nonexistent").await, None);
    });
  }

  #[test]
  fn registry_list_all_includes_completed() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
      let reg = RunRegistry {
        inner: Arc::new(Mutex::new(HashMap::new())),
        max_concurrent: 3,
        completed: Arc::new(Mutex::new(vec![CompletedRun {
          work_dir: "old".into(),
          pid: Some(12345),
          started_at: "epoch:100".into(),
          finished_at: "epoch:200".into(),
          inbox: "/inbox/old".into(),
          log_path: "/log/old".into(),
          status: "completed".into(),
        }])),
      };
      let all = reg.list_all().await;
      // completed 应该出现
      assert!(all.iter().any(|r| r.work_dir == "old" && r.status == "completed"));
    });
  }

  #[test]
  fn registry_list_empty() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
      let reg = RunRegistry::new();
      let running = reg.list().await;
      assert_eq!(running.len(), 0);
      let all = reg.list_all().await;
      assert_eq!(all.len(), 0);
    });
  }

  // ── T5 (W15-A): env var 注入到 spawn_mtd 子进程 ─────────────────

  /// T5 测试 1:SpawnSpec.env_vars 字段存在且默认空 HashMap。
  /// 这是 W15-A Task 4 / spec §5 的字段存在性保证。
  #[test]
  fn spawn_spec_env_vars_defaults_to_empty() {
    let run_spec = build_mtd_run_args(
      &project(),
      &inbox(),
      None, None, None, false, false,
      None, None, None,
    );
    assert!(
      run_spec.env_vars.is_empty(),
      "build_mtd_run_args 默认 env_vars 应为空, 实际: {:?}",
      run_spec.env_vars
    );

    let resume_spec = build_mtd_resume_args(
      &project(),
      &inbox().join("output"),
      false, None,
      None, None, None,
    );
    assert!(
      resume_spec.env_vars.is_empty(),
      "build_mtd_resume_args 默认 env_vars 应为空, 实际: {:?}",
      resume_spec.env_vars
    );
  }

  /// T5 测试 2:`spawn_mtd` 真的 `.env_clear()` + `.envs(spec.env_vars)`。
  ///
  /// 实跑一个 cmd / sh echo 短进程:
  /// - 父进程设 HTTP_PROXY=evil:8080(模拟公司 VPN proxy 污染)
  /// - spec.env_vars 注入 OPENAI_API_KEY=secret
  /// - 子进程打印两个变量
  ///
  /// 期望子进程 stdout:
  /// - 含 `OPENAI_API_KEY=secret`(注入生效)
  /// - 不含 `evil:8080`(env_clear 生效)
  ///
  /// 用 cmd /c echo(W)或 sh -c echo(Unix)毫秒级进程,不真起 mtd(plan §89 允许)。
  /// 注:不能用 spawn_mtd 直跑,因为它把 stdout 重定向到 log 文件,
  /// `wait_with_output` 抓不到。改用 build_child_command 纯函数 + override stdout。
  #[test]
  fn spawn_mtd_clears_parent_env_and_injects_spec_env() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    // 父进程污染:HTTP_PROXY (公司 VPN 典型场景)
    std::env::set_var("HTTP_PROXY", "http://evil:8080");

    rt.block_on(async {
      let mut env_vars = HashMap::new();
      env_vars.insert("OPENAI_API_KEY".to_string(), "sk-w15a-secret".to_string());

      let spec = SpawnSpec {
        program: if cfg!(windows) { "cmd".to_string() } else { "sh".to_string() },
        args: if cfg!(windows) {
          vec![
            "/c".to_string(),
            "echo OPENAI_API_KEY=%OPENAI_API_KEY%&echo HTTP_PROXY=%HTTP_PROXY%".to_string(),
          ]
        } else {
          vec![
            "-c".to_string(),
            "echo OPENAI_API_KEY=$OPENAI_API_KEY; echo HTTP_PROXY=$HTTP_PROXY".to_string(),
          ]
        },
        work_dir: std::env::temp_dir().to_string_lossy().into_owned(),
        log_path: std::env::temp_dir().join("w15a-t5-env-test.log").to_string_lossy().into_owned(),
        env_vars,
      };

      let mut cmd = build_child_command(&spec);
      // override stdout 为 piped,才能 wait_with_output 抓到
      cmd.stdout(Stdio::piped());
      let child = cmd.spawn().expect("spawn 应成功");
      let output = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        child.wait_with_output(),
      )
      .await
      .expect("子进程 10s 内应退出")
      .expect("wait_with_output 应 OK");
      let stdout = String::from_utf8_lossy(&output.stdout);

      assert!(
        stdout.contains("sk-w15a-secret"),
        "spec.env_vars OPENAI_API_KEY 应注入子进程,stdout: {stdout}"
      );
      assert!(
        !stdout.contains("evil:8080"),
        "父进程 HTTP_PROXY 应被 env_clear 清掉,stdout: {stdout}"
      );
    });

    // 清理
    std::env::remove_var("HTTP_PROXY");
  }

  /// T5 测试 3:`build_child_command` 在 `.env_clear()` 后仍保留父进程 PATH。
  ///
  /// 必要性:env_clear 后 PATH 也是空,uv 等可执行文件在 Windows 上不在
  /// System32,CreateProcess 找不到会报"系统找不到指定的文件"。
  /// 因此 build_child_command 必须 `.env("PATH", std::env::var("PATH"))`。
  /// 父进程 PATH 应非空(测试机器上都有),子进程 echo PATH 验证保留成功。
  #[test]
  fn build_child_command_inherits_parent_path() {
    // 前置:父进程 PATH 应非空
    let parent_path = std::env::var("PATH").unwrap_or_default();
    assert!(
      !parent_path.is_empty(),
      "父进程 PATH 应非空,实际: {parent_path:?}"
    );

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
      let spec = SpawnSpec {
        program: if cfg!(windows) { "cmd".to_string() } else { "sh".to_string() },
        args: if cfg!(windows) {
          // Windows cmd: if "%PATH%"=="" (echo EMPTY) else (echo NONEMPTY)
          vec![
            "/c".to_string(),
            "if \"%PATH%\"==\"\" (echo EMPTY) else (echo NONEMPTY)".to_string(),
          ]
        } else {
          // Unix sh: if [ -z "$PATH" ]; then echo EMPTY; else echo NONEMPTY; fi
          vec![
            "-c".to_string(),
            "if [ -z \"$PATH\" ]; then echo EMPTY; else echo NONEMPTY; fi".to_string(),
          ]
        },
        work_dir: std::env::temp_dir().to_string_lossy().into_owned(),
        log_path: std::env::temp_dir().join("w15a-t5-path.log").to_string_lossy().into_owned(),
        env_vars: HashMap::new(),
      };

      let mut cmd = build_child_command(&spec);
      cmd.stdout(Stdio::piped());
      let child = cmd.spawn().expect("spawn 应成功");
      let output = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        child.wait_with_output(),
      )
      .await
      .expect("5s 内应退出")
      .expect("wait_with_output 应 OK");
      let stdout = String::from_utf8_lossy(&output.stdout);

      assert!(
        stdout.contains("NONEMPTY"),
        "build_child_command 应 .env(\"PATH\", parent_path),子进程 PATH 应非空,stdout: {stdout}"
      );
    });
  }

  // ── T7.2 (W15-A): runner args 透传 profile_name + task_text ─────────

  /// T7.2 测试 1:`build_mtd_run_args` 显式传 `llm_profile_name` → 拼接
  /// `--llm-profile-name <name>`。
  #[test]
  fn t7_2_build_run_args_with_profile_name_adds_flag() {
    let spec = build_mtd_run_args(
      &project(),
      &inbox(),
      None, None, None, false, false,
      Some("minimax-prod"),
      None,
      None,
    );
    assert!(
      spec.args.contains(&"--llm-profile-name".to_string()),
      "spec.args 应含 --llm-profile-name, 实际: {:?}",
      spec.args
    );
    let idx = spec
      .args
      .iter()
      .position(|a| a == "--llm-profile-name")
      .expect("flag 应存在");
    assert_eq!(spec.args[idx + 1], "minimax-prod");
  }

  /// T7.2 测试 2:`image_agent_profile_name` 拼接 `--image-agent-profile-name`。
  #[test]
  fn t7_2_build_run_args_with_image_agent_profile_name_adds_flag() {
    let spec = build_mtd_run_args(
      &project(),
      &inbox(),
      None, None, None, false, false,
      None,
      Some("deepseek-prod"),
      None,
    );
    assert!(spec.args.contains(&"--image-agent-profile-name".to_string()));
    let idx = spec
      .args
      .iter()
      .position(|a| a == "--image-agent-profile-name")
      .expect("flag 应存在");
    assert_eq!(spec.args[idx + 1], "deepseek-prod");
  }

  /// T7.2 测试 3:`task_text` 拼接 `--task-text`。
  #[test]
  fn t7_2_build_run_args_with_task_text_adds_flag() {
    let spec = build_mtd_run_args(
      &project(),
      &inbox(),
      None, None, None, false, false,
      None,
      None,
      Some("突出客户案例"),
    );
    assert!(spec.args.contains(&"--task-text".to_string()));
    let idx = spec
      .args
      .iter()
      .position(|a| a == "--task-text")
      .expect("flag 应存在");
    assert_eq!(spec.args[idx + 1], "突出客户案例");
  }

  /// T7.2 测试 4:三个新参数都不传 → 三 flag 都不拼接(零回归)。
  #[test]
  fn t7_2_build_run_args_without_new_params_no_flag() {
    let spec = build_mtd_run_args(
      &project(),
      &inbox(),
      Some("ollama"),
      Some("skip"),
      None,
      false,
      false,
      None, None, None,
    );
    assert!(!spec.args.contains(&"--llm-profile-name".to_string()));
    assert!(!spec.args.contains(&"--image-agent-profile-name".to_string()));
    assert!(!spec.args.contains(&"--task-text".to_string()));
  }

  /// T7.2 测试 5:`build_mtd_resume_args` 同步支持 3 个新 flag。
  #[test]
  fn t7_2_build_resume_args_with_all_three_new_flags() {
    let work = inbox().join("output");
    let spec = build_mtd_resume_args(
      &project(),
      &work,
      false,
      None,
      Some("minimax-prod"),
      Some("deepseek-prod"),
      Some("task"),
    );
    assert!(spec.args.contains(&"--llm-profile-name".to_string()));
    assert!(spec.args.contains(&"--image-agent-profile-name".to_string()));
    assert!(spec.args.contains(&"--task-text".to_string()));
    // resume 仍保留 work_dir 透传 + 不含 --force(零回归)
    assert!(spec.args.contains(&work.to_string_lossy().into_owned()));
    assert!(!spec.args.contains(&"--force".to_string()));
  }
}
