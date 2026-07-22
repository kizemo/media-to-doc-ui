# W14-B+2 Tauri UI 完整化 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 `feat/w14b-plus-8-commands` 分支上,补完 3 个 P0/P1 功能:`read_log` 命令、log tail 2s 轮询、`read_lecture` W12-D 3 级 fallback、read_lecture modal(marked.js + iframe srcdoc),并通过 `cargo tauri dev` 30min 5 tab 手动验证。

**Architecture:**
- 后端(Rust):1 个新 command `read_log` + 改 `read_lecture` 3 级 fallback
- 前端(vanilla TS in `index.html`):1 个 modal + marked.js CDN(SRI 锁版本)+ 2s log poll + Output tab [read] 按钮
- 布局优先级:对齐 Python MCP `tool_read_lecture` 的 W12-D 真相(同源码 3 级 fallback 语义)
- 安全:iframe srcdoc sandbox=`allow-same-origin`(无 script),`path` 校验只 endswith("mtd.log")

**Tech Stack:** Tauri 2.11.4 + Rust 1.97.1 + tokio(fs feature 已有)+ once_cell 已有 + marked@12.0.0(unpkg SRI 锁版)+ vanilla TS(无新 framework)

## Global Constraints

- **会话健康**:`<2h` 活跃时间预算,撞墙立即写 handoff,不要 `--resume` 5MB+ jsonl
- **每个新 shell 必设**(Cargo SSL 撞墙破解):
  ```bash
  export PATH="/c/Users/Duanyi/.cargo/bin:$PATH"
  cd "F:/soft/00selfmade/media-to-doc-ui/src-tauri"
  CARGO_NET_TLS_VERIFY=false cargo build
  ```
- **包管理器**:Tauri 仓用 `cargo`;主仓 `media-to-doc` 不可改动(本轮 W14-B+2 范围仅 `media-to-doc-ui/`)
- **测试**:`cargo test` 30 baseline + 9 新增 = 39 passed / 0 failed
- **代码风格**:rustfmt 默认 + 2 空格缩进(沿用既有 commands.rs);TypeScript camelCase
- **Commit 规范**:Conventional Commits;feat(ui): / fix(ui): / docs:
- **分支**:`feat/w14b-plus-8-commands`(已 7 commit,新增 3-4 commit)
- **不修改 master**

---

## File Structure

| 文件 | 状态 | 责任 |
|---|---|---|
| `src-tauri/src/commands.rs` | Modify | + `read_log` + `read_log_impl` + 5 单测;改 `read_lecture_impl` 3 级 fallback + 4 单测 |
| `src-tauri/src/lib.rs` | Modify | invoke_handler 加 `read_log` |
| `src/index.html` | Modify | marked.js CDN + modal CSS/JS + log tail JS + Output tab [read] 按钮 |
| `docs/superpowers/specs/2026-07-22-w14b-plus-2-ui-features-design.md` | Created (1173ab8+66cff5c) | spec 文档(本计划源头) |

---

## Task 1: 后端 `read_log` command + 5 单测

**Files:**
- Modify: `src-tauri/src/commands.rs`(在 `helpers` section 之后、`run_pipeline` 之前插入新 section)
- Modify: `src-tauri/src/lib.rs:73-90`(invoke_handler 加 `read_log`)
- Test: `src-tauri/src/commands.rs` 末尾 `mod tests`(追加 5 用例)

**Interfaces:**
- Consumes:`tokio::fs::File`(已有依赖,来自 Cargo.toml)+ `std::io::Read + Seek + BufRead`
- Produces:
  ```rust
  pub struct ReadLogResult {
    pub content: String,
    pub new_offset: u64,
    pub total_bytes: u64,
    pub truncated: bool,
    pub truncated_to_lines: bool,
  }
  pub fn read_log_impl(path: String, offset: u64, max_lines: usize) -> CommandResponse<ReadLogResult>;
  #[tauri::command]
  pub async fn read_log(path: String, offset: u64, max_lines: usize) -> CommandResponse<ReadLogResult>;
  ```

- [ ] **Step 1.1: 写 5 个失败的单测**

在 `commands.rs` `#[cfg(test)] mod tests` 末尾追加(在 `runner_tests` 之前):

```rust
  #[test]
  fn read_log_errors_on_missing_file() {
    let r = read_log_impl(
      std::env::temp_dir().join("definitely_not_here_xyz_mtd.log").to_string_lossy().into_owned(),
      0, 200,
    );
    assert!(!r.ok, "missing file should err");
    assert!(r.error.unwrap().contains("not found") || r.error.unwrap().contains("不存在"));
  }

  #[test]
  fn read_log_returns_empty_when_offset_equals_size() {
    let tmp = tmpdir("read_log_eq_size");
    let p = tmp.join("test.log");
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
    let p = tmp.join("test.log");
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
    let p = tmp.join("test.log");
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
    let p = tmp.join("test.log");
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
```

- [ ] **Step 1.2: 跑测试确认失败**

Run: `cd "F:/soft/00selfmade/media-to-doc-ui/src-tauri" && CARGO_NET_TLS_VERIFY=false cargo test read_log --no-run 2>&1 | tail -5`
Expected: 编译失败 `cannot find function read_log_impl`

- [ ] **Step 1.3: 实装 `read_log` + `read_log_impl`**

在 `commands.rs` `read_lecture` section 之后,新增 section:

```rust
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
```

- [ ] **Step 1.4: 在 `lib.rs` invoke_handler 加 `read_log`**

`src-tauri/src/lib.rs:73-90` `tauri::generate_handler![...]` 列表中,`list_runs` 之后追加 `read_log,`:

```rust
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
      // W14-B+2 read_log(后端 log tail)
      read_log,
    ])
```

- [ ] **Step 1.5: 跑 5 个 read_log 单测确认通过**

Run: `cd "F:/soft/00selfmade/media-to-doc-ui/src-tauri" && CARGO_NET_TLS_VERIFY=false cargo test --lib read_log 2>&1 | tail -10`
Expected: `5 passed; 0 failed`

- [ ] **Step 1.6: 跑全量 30 baseline 确认不破**

Run: `cd "F:/soft/00selfmade/media-to-doc-ui/src-tauri" && CARGO_NET_TLS_VERIFY=false cargo test --lib 2>&1 | tail -5`
Expected: `35 passed; 0 failed`(30 baseline + 5 new)

- [ ] **Step 1.7: Commit Task 1**

```bash
cd "F:/soft/00selfmade/media-to-doc-ui" && git add src-tauri/src/commands.rs src-tauri/src/lib.rs && git -c commit.gpgsign=false commit -m "feat(ui): W14-B+2 — read_log Tauri command (5 unit tests)"
```

---

## Task 2: `read_lecture` 3 级 fallback(W12-D 优先)+ 4 单测

**Files:**
- Modify: `src-tauri/src/commands.rs`(`read_lecture_impl` 函数 + 末尾 tests)

**Interfaces:**
- Produces(改 `ReadLectureResult` 加 `source` 字段,改 `read_lecture_impl` 签名不变):
  ```rust
  pub struct ReadLectureResult {
    pub version: String,
    pub fmt: String,
    pub path: String,
    pub content: String,
    pub size_bytes: usize,
    pub source: String,            // "output_final" | "legacy" | "fallback_md"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
  }
  ```

- [ ] **Step 2.1: 写 4 个失败单测**

在 `commands.rs` `mod tests` 末尾(在 Task 1 的 5 个 read_log 测试之后):

```rust
  #[test]
  fn read_lecture_prefers_output_final_over_legacy() {
    let tmp = tmpdir("read_lecture_w12d_prefer");
    let inbox = tmp.join("course");
    fs::create_dir_all(&inbox).unwrap();
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
    let inbox = tmp.join("course");
    fs::create_dir_all(&inbox).unwrap();
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
    let inbox = tmp.join("course");
    fs::create_dir_all(&inbox).unwrap();
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
    assert_eq!(d.source, "output_final");
    assert!(d.note.is_some());
    assert!(d.note.unwrap().contains("html") || d.content.contains("cleaned md body"));
    let _ = fs::remove_dir_all(&tmp);
  }

  #[test]
  fn read_lecture_errors_when_neither_layout_has_file() {
    let tmp = tmpdir("read_lecture_missing");
    let inbox = tmp.join("course");
    fs::create_dir_all(&inbox).unwrap();
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
```

- [ ] **Step 2.2: 跑测试确认失败(编译错)**

Run: `cd "F:/soft/00selfmade/media-to-doc-ui/src-tauri" && CARGO_NET_TLS_VERIFY=false cargo test read_lecture_prefers --no-run 2>&1 | tail -10`
Expected: 编译失败,因 `source` 字段未在 `ReadLectureResult` 上

- [ ] **Step 2.3: 改 `ReadLectureResult` + `read_lecture_impl`**

替换 `commands.rs` 中 `ReadLectureResult` 结构(当前 line 329-338):

```rust
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
```

替换 `commands.rs` 中整个 `read_lecture_impl` 函数(当前 line 349-417):

```rust
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
          "output_final",
          Some("html 版本未生成,fallback 到 md"),
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
```

注意:原先的 `read_lecture_impl` 里 html→md fallback 是同一个函数内部做的(在 `target.is_file() && fmt == "html"` 时,改用 alt path);新的实现把它拆成 2 个 try,逻辑等价但更清晰。

- [ ] **Step 2.4: 跑 4 个新测试确认通过**

Run: `cd "F:/soft/00selfmade/media-to-doc-ui/src-tauri" && CARGO_NET_TLS_VERIFY=false cargo test --lib read_lecture 2>&1 | tail -10`
Expected: 全部 read_lecture 测试 pass(旧的 2 + 新的 4 = 6)

- [ ] **Step 2.5: 跑全量确认 30 baseline + 5 read_log + 4 read_lecture = 39 全过**

Run: `cd "F:/soft/00selfmade/media-to-doc-ui/src-tauri" && CARGO_NET_TLS_VERIFY=false cargo test --lib 2>&1 | tail -5`
Expected: `39 passed; 0 failed`

- [ ] **Step 2.6: Commit Task 2**

```bash
cd "F:/soft/00selfmade/media-to-doc-ui" && git add src-tauri/src/commands.rs && git -c commit.gpgsign=false commit -m "feat(ui): W14-B+2 — read_lecture W12-D output_final priority + legacy fallback (4 unit tests)"
```

---

## Task 3: 前端 marked.js + modal CSS/JS + log tail 2s poll

**Files:**
- Modify: `src/index.html`

**Interfaces:**
- Consumes: Tauri IPC `invoke('read_log', {path, offset, maxLines})` + `invoke('read_lecture', {inboxDir, version, fmt})`
- Produces:DOM modal `<div class="modal-backdrop">` + 工具栏 + 内容区;log tail `<pre id="log-tail">` 2s 自动滚动;Output tab 文件条目 [read] 按钮

- [ ] **Step 3.1: 算 marked@12.0.0 真实 SRI hash**

Run:
```bash
curl -sL https://unpkg.com/marked@12.0.0/marked.min.js \
  | openssl dgst -sha384 -binary \
  | openssl base64 -A
```
Expected: 一个 base64 字符串,假设返回 `2LfWAqRonsfHs7a9e9Rt0M4g6aHJBzF5Y2K4e1u3W1X8D9jZ3Y2K1X0Vf7N6l5M4o`(占位,实际以你机器输出为准)。记下完整 `sha384-<base64>`。

- [ ] **Step 3.2: 在 `<head>` 加 marked.js 引入 + modal CSS**

在 `src/index.html` line 7 (`<style>` 开头)之前,加:

```html
  <script src="https://unpkg.com/marked@12.0.0/marked.min.js"
          integrity="sha384-<REPLACE_WITH_STEP_3.1_OUTPUT>"
          crossorigin="anonymous"></script>
```

把 `<REPLACE_WITH_STEP_3.1_OUTPUT>` 替换成 step 3.1 真实输出。

在 `</style>`(line 168 之前)追加 modal CSS:

```css
    .modal-backdrop {
      position: fixed; inset: 0;
      background: rgba(0,0,0,0.7);
      display: none; align-items: center; justify-content: center;
      z-index: 2000;
    }
    .modal-backdrop.open { display: flex; }
    .modal {
      width: 80vw; height: 80vh;
      background: var(--bg-card);
      border: 1px solid var(--border);
      border-radius: 8px;
      display: flex; flex-direction: column;
    }
    .modal-toolbar {
      padding: 12px 16px; border-bottom: 1px solid var(--border);
      display: flex; align-items: center; gap: 12px;
    }
    .modal-toolbar .source { color: var(--fg-muted); font-size: 11px; }
    .modal-toolbar .spacer { flex: 1; }
    .modal-body { flex: 1; overflow: auto; padding: 16px; }
    .modal-body iframe { width: 100%; height: 100%; border: 0; background: #fff; }
    .modal-body pre { white-space: pre-wrap; word-break: break-word; }
    .modal-body h1, .modal-body h2, .modal-body h3 {
      color: var(--fg); border-bottom: 1px solid var(--border);
      padding-bottom: 4px; margin-top: 16px;
    }
    .modal-body code { background: var(--bg); padding: 2px 4px; border-radius: 3px; }
    .modal-body table { border-collapse: collapse; margin: 8px 0; }
    .modal-body th, .modal-body td { border: 1px solid var(--border); padding: 4px 8px; }
```

- [ ] **Step 3.3: 在 `<main>` 末尾追加 modal DOM**

在 `src/index.html` line 314(`</main>` 之前)追加:

```html
  <div class="modal-backdrop" id="modal-backdrop">
    <div class="modal">
      <div class="modal-toolbar">
        <span id="modal-title">Lecture</span>
        <span class="source" id="modal-source"></span>
        <span class="spacer"></span>
        <button class="secondary" id="modal-close">×</button>
      </div>
      <div class="modal-body" id="modal-body"></div>
    </div>
  </div>
```

- [ ] **Step 3.4: 替换占位 `tailLog` + 加 modal JS + Output tab [read] 按钮**

在 `src/index.html` line 489-494 替换 `async function tailLog` 函数:

```js
    let lastLogOffset = 0;
    async function tailLog(path, n = 200) {
      if (!path) return;
      try {
        const r = await invoke('read_log', { path, offset: lastLogOffset, maxLines: n });
        if (!r.ok) { $('log-tail').textContent = '(read_log 失败: ' + r.error + ')'; return; }
        if (r.data.truncated) {
          lastLogOffset = 0;
          $('log-tail').textContent = '';   // truncate → 清空重读
        }
        if (r.data.content) {
          const pre = $('log-tail');
          pre.textContent += r.data.content;
          // 限制 pre 长度(避免无限增长)
          if (pre.textContent.length > 200000) {
            pre.textContent = pre.textContent.slice(-100000);
          }
          pre.scrollTop = pre.scrollHeight;
        }
        lastLogOffset = r.data.new_offset;
      } catch (e) {
        $('log-tail').textContent = '(read_log 异常: ' + e + ')';
      }
    }
```

在 line 552(`await refreshCourses();` boot 之前)追加 modal 函数:

```js
    // ───────── Modal ─────────
    function showModal() { $('modal-backdrop').classList.add('open'); }
    function hideModal() {
      $('modal-backdrop').classList.remove('open');
      $('modal-body').innerHTML = '';
    }
    $('modal-close').addEventListener('click', hideModal);
    $('modal-backdrop').addEventListener('click', (e) => {
      if (e.target.id === 'modal-backdrop') hideModal();
    });
    async function openReadLecture(inbox, version, fmt) {
      $('modal-title').textContent = `${version} · ${fmt}`;
      $('modal-source').textContent = '(loading…)';
      $('modal-body').innerHTML = '<div class="empty">loading…</div>';
      showModal();
      try {
        const r = await invoke('read_lecture', { inboxDir: inbox, version, fmt });
        const body = $('modal-body');
        if (!r.ok) {
          body.innerHTML = '<div class="empty">' + r.error + '</div>';
          $('modal-source').textContent = '(error)';
          return;
        }
        $('modal-source').textContent = `source: ${r.data.source} · ${(r.data.size_bytes/1024).toFixed(1)} KB`;
        if (r.data.note) $('modal-source').textContent += ` · ${r.data.note}`;
        if (fmt === 'md' && window.marked) {
          body.innerHTML = marked.parse(r.data.content);
        } else if (fmt === 'html') {
          const iframe = document.createElement('iframe');
          iframe.sandbox = 'allow-same-origin';
          iframe.srcdoc = r.data.content;
          body.innerHTML = '';
          body.appendChild(iframe);
        } else {
          body.innerHTML = '<pre>' + r.data.content.replace(/</g, '&lt;') + '</pre>';
        }
      } catch (e) {
        $('modal-body').innerHTML = '<div class="empty">invoke 异常: ' + e + '</div>';
      }
    }
```

- [ ] **Step 3.5: 改 `renderOutputs` 给每个文件加 [read] 按钮**

替换 `src/index.html` line 513-525(整段 `el.innerHTML = ...` 模板):

```js
      function readBtn(file, version, fmt) {
        return `<button class="secondary" style="margin:2px 4px 2px 0;padding:4px 8px;font-size:11px"
                        onclick="openReadLecture('${d.inbox.replace(/'/g, "\\'")}','${version}','${fmt}')">📄 ${file}</button>`;
      }
      el.innerHTML = `
        <div style="font-family: ui-monospace, monospace; font-size: 12px; color: var(--fg-muted); margin-bottom: 8px;">
          work_dir: ${d.work_dir} · stem: ${d.stem}
        </div>
        <div><strong>raw_md:</strong> ${(d.outputs.raw_md || []).map(f => readBtn(f, 'raw', 'md')).join('') || '(none)'}</div>
        <div><strong>raw_html:</strong> ${(d.outputs.raw_html || []).map(f => readBtn(f, 'raw', 'html')).join('') || '(none)'}</div>
        <div><strong>cleaned_md:</strong> ${(d.outputs.cleaned_md || []).map(f => readBtn(f, 'cleaned', 'md')).join('') || '(none)'}</div>
        <div><strong>final_html:</strong> ${(d.outputs.final_html || []).map(f => readBtn(f, 'final', 'html')).join('') || '(none)'}</div>
        <div><strong>images:</strong> ${(d.outputs.images || []).join(', ') || '(none)'}</div>
        <div><strong>manifests:</strong> ${(d.outputs.manifests || []).join(', ') || '(none)'}</div>
        <h3 style="margin-top: 16px; font-size: 13px; color: var(--fg-muted);">Stages</h3>
        <div>${Object.entries(d.stages || {}).map(([k, v]) => `<span class="pill ${v}">${k}: ${v}</span> `).join('')}</div>
      `;
```

- [ ] **Step 3.6: 静态校验(无 Tauri runtime 时也要不报错)**

Run: `cd "F:/soft/00selfmade/media-to-doc-ui/src" && node -e "const fs = require('fs'); const html = fs.readFileSync('index.html', 'utf8'); console.log('size:', html.length, 'has marked:', html.includes('marked@12.0.0'), 'has modal:', html.includes('modal-backdrop'), 'has tailLog:', html.includes('lastLogOffset'));"`
Expected: `size: <N> has marked: true has modal: true has tailLog: true`

(若 node 不可用,改为 `grep -c marked@12.0.0 src/index.html` / `grep -c modal-backdrop src/index.html` / `grep -c lastLogOffset src/index.html` 都应返回 1。)

- [ ] **Step 3.7: Commit Task 3**

```bash
cd "F:/soft/00selfmade/media-to-doc-ui" && git add src/index.html && git -c commit.gpgsign=false commit -m "feat(ui): W14-B+2 — log tail 2s poll + read_lecture modal (marked.js + iframe srcdoc)"
```

---

## Task 4: `cargo tauri dev` 启动 + 5 tab 手动验证

**Files:** 不改任何文件,只跑 `cargo tauri dev` + 手动点 5 tab。

- [ ] **Step 4.1: 启动 dev shell,设环境变量**

Run:
```bash
export PATH="/c/Users/Duanyi/.cargo/bin:$PATH"
cd "F:/soft/00selfmade/media-to-doc-ui"
CARGO_NET_TLS_VERIFY=false cargo tauri dev 2>&1 | tee /tmp/w14bplus2-dev.log
```
Expected: 编译 12.5MB binary 启动成功(可能 30-60s),media-to-doc-ui.exe 窗口弹出,标题"media-to-doc",自动打开 DevTools。

- [ ] **Step 4.2: 验证 Inbox tab**

- status dot 绿,显示 `mtd <ver> ready`(来自 `app_info`)
- 输入 workspace 路径(留空用默认)+ Refresh
- 若 inbox 存在课程,看到课程列表;否则显示 inbox 目录路径空状态
- 期望:`list_courses` invoke 成功,`status-dot.green`

- [ ] **Step 4.3: 验证 Run tab(选课 + run_pipeline + log tail + cancel)**

- 从 Inbox 选一个课程(state.selectedInbox 设置)
- LLM=ollama, stop_after=chapters(快速验证,不跑整 11 stage)
- 点 Run pipeline
- 期望:toast "Pipeline started, pid=N",`cancel-btn` 可用,5s 后 status grid 11 dot 渐次亮,log tail 区域 2s 间隔出现 `[audio]` `[asr]` 等 stage 标记
- 点 Cancel
- 期望:toast "Cancelled pid=N",`cancel-btn` 变 disabled,log tail 停止增长

- [ ] **Step 4.4: 验证 Output tab(read_lecture modal)**

- 切到 Output tab,点 list_outputs 已成功(显示分组)
- 点 `cleaned.md` 的 [read] 按钮
- 期望:modal 弹出,marked 渲染 H1/H2/表格/TOC(若有),source 显示 `source: output_final · X.X KB`(或 `legacy`)
- 关 modal,点 `final.html` [read]
- 期望:modal 内 iframe srcdoc 渲染,html 自带样式生效

- [ ] **Step 4.5: 验证 Health + Learn tab**

- Health:get_run_metrics 返 JSON(LLM providers 字典),list_runs 返 runs 列表
- Learn:app_info 5 字段全填,mtd_version = "1.2.1" 或更新

- [ ] **Step 4.6: 任何 bug 立即修**

观察 dev tools console + 5 tab 行为;发现任一 crash / invoke 失败 / UI 不刷新 → 立即定位修。

- [ ] **Step 4.7: 关闭 dev 进程**

Run: 在另一个 shell 中 `pkill -f media-to-doc-ui` 或直接关窗口

- [ ] **Step 4.8: 不 commit(纯验证,改动已在 Task 1-3 commit)**

不需 commit。

---

## Task 5: 收尾(全量 cargo test + handoff + 主仓 CLAUDE.md 同步)

**Files:**
- Modify: 主仓 `media-to-doc/handoff-pipeline-w14b-plus-2-ui-features-2026-07-22.md`(新建)
- Modify: 主仓 `media-to-doc/CLAUDE.md` §10(Tauri UI 子项目段 + 后续规划)

- [ ] **Step 5.1: 全量 cargo test 确认 39 pass**

Run: `cd "F:/soft/00selfmade/media-to-doc-ui/src-tauri" && CARGO_NET_TLS_VERIFY=false cargo test --lib 2>&1 | tail -5`
Expected: `39 passed; 0 failed`

- [ ] **Step 5.2: 写 handoff**

`F:\soft\00selfmade\media-to-doc\handoff-pipeline-w14b-plus-2-ui-features-2026-07-22.md` 新建:

```markdown
# Handoff — W14-B+2:Tauri UI 完整化(read_log + read_lecture W12-D + modal)

**日期**:2026-07-22
**承接会话**:`media-to-doc-ui` / `feat/w14b-plus-8-commands` 分支
**本会话主目标**:
- 补完 3 个 P0/P1:`read_log` command / log tail 2s 轮询 / `read_lecture` W12-D 优先 / read_lecture modal
- `cargo tauri dev` 启动 + 5 tab 手动验证 8 commands

## 全部完成 ✅

| Task | 内容 | Commit | 测试 |
|---|---|---|---|
| T1 | 后端 `read_log` + 5 单测 | (Task 1 commit) | +5 |
| T2 | `read_lecture` 3 级 fallback + 4 单测 | (Task 2 commit) | +4 |
| T3 | 前端 marked.js + modal + log tail + [read] 按钮 | (Task 3 commit) | — (手测) |
| T4 | `cargo tauri dev` 启动 + 5 tab 手动验证 | (无 commit) | — |
| T5 | 全量测试 + 本 handoff | docs commit | 39 / 0 |

**总测试**:39 passed / 0 failed(baseline 30 + 9 新增)

## 关键设计

### read_log(后端)

```rust
pub struct ReadLogResult {
    pub content: String,
    pub new_offset: u64,
    pub total_bytes: u64,
    pub truncated: bool,         // 文件被 truncate(offset > total_bytes)
    pub truncated_to_lines: bool, // 命中 max_lines 上限
}
```

**offset 模式**:`path + offset` → 增量内容 + new_offset;前端 2s 轮询只传 delta;max_lines 默认 200 硬上限 2000。

### read_lecture 3 级 fallback

```
1. <inbox>/output_final/<stem>.<ext>           W12-D 真相
2. <inbox>/output_final/<stem>_*.md (仅 fmt=html)  W12-D html→md 兜底
3. <inbox>/output/chapters/raw/<stem>/<file>    W3-W11 legacy fallback
```

新加 `source: "output_final" | "legacy"` 字段供前端显示;html 缺时加 `note: "html 版本未生成,fallback 到 md"`。

### 前端 modal(marked.js + iframe)

- `marked@12.0.0` from unpkg,**SRI 锁版本**(`<commit 时算的 hash>`)
- md 走 `marked.parse(content)`;html 走 `<iframe srcdoc sandbox="allow-same-origin">`(关 script 防 XSS)
- marked CDN 失败时降级到 `<pre>` 纯文本

## 撞墙 / 修正

(留空 — 写实际撞到的)

## 文件索引

| 文件 | 路径 | 改动 |
|---|---|---|
| Spec | `media-to-doc-ui/docs/superpowers/specs/2026-07-22-w14b-plus-2-ui-features-design.md` | 设计(已 commit `1173ab8`+`66cff5c`) |
| Plan | `media-to-doc-ui/docs/superpowers/plans/2026-07-22-w14b-plus-2-ui-features.md` | 实施计划(本会话) |
| Backend | `media-to-doc-ui/src-tauri/src/commands.rs` | +read_log +read_lecture 3 级 + 9 单测 |
| Lib | `media-to-doc-ui/src-tauri/src/lib.rs` | invoke_handler +read_log |
| Frontend | `media-to-doc-ui/src/index.html` | marked.js + modal + log tail + [read] 按钮 |

## 下次会话(W14-C 候选)

- A. Tauri UI 多课程并发 UI(后端 list_running 已支持)
- B. Tauri UI release build + NSIS 安装器(v1.4 Phase 3)
- C. 合并 `feat/w14b-plus-8-commands` → `master` + v1.3.0 release
- D. Anthropic / OpenAI Compat provider `trust_env=False` 加固
- E. 真实端到端 11 stage 流水线在 Tauri UI 内跑通(短 demo 视频)

## 下次会话第一句

> 承接 `handoff-pipeline-w14b-plus-2-ui-features-2026-07-22.md`,Tauri UI 3 个 P0/P1 已实装,39 测试 / 0 failed。准备做 W14-C 候选(参见 handoff §下次会话)。
```

- [ ] **Step 5.3: 同步主仓 CLAUDE.md §10**

在 `F:\soft\00selfmade\media-to-doc\CLAUDE.md` §10 "后续规划" 表格,找 `v1.3 Phase 2 — Tauri UI` 一行,改:

```
| **v1.3 Phase 2 — Tauri UI** | 3 次点击跑通 + 桌面壳 + log tail + modal | ✅ W14-B+ + W14-B+2(分支 `feat/w14b-plus-8-commands`,8 commit,39 unit test / 0 failed) |
```

并在 §10 末尾的"Tauri UI 子项目(独立 repo)"段,加 1 段:

```
**W14-B+2 收尾**(2026-07-22,~3h):
- 后端 `read_log` Tauri command(offset 模式 + 5 单测)
- `read_lecture` 改 W12-D output_final 优先 + W3-W11 legacy fallback(+ 4 单测)
- 前端 marked@12.0.0 CDN(SRI 锁版本)+ iframe srcdoc modal
- 前端 log tail 2s 轮询 + offset diff
- Output tab 文件 [read] 按钮
- 39 unit test / 0 failed(baseline 30 + 9 new)
- 详见 `handoff-pipeline-w14b-plus-2-ui-features-2026-07-22.md`
```

- [ ] **Step 5.4: Commit handoff + CLAUDE.md 同步**

```bash
cd "F:/soft/00selfmade/media-to-doc" && git add handoff-pipeline-w14b-plus-2-ui-features-2026-07-22.md CLAUDE.md && git -c commit.gpgsign=false commit -m "docs(release): W14-B+2 — Tauri UI log tail + read_lecture modal (39 tests, 0 failed)"
```

- [ ] **Step 5.5: 收工报告**

回复用户:W14-B+2 完成,3 commit(后端 2 + 前端 1)+ docs commit,39 unit test / 0 failed,`cargo tauri dev` 5 tab 手动验证通过。handoff 已写,主仓 CLAUDE.md §10 同步。
