# W14-B+2 Tauri UI 完整化 — Design Spec

**日期**:2026-07-22
**承接**:`handoff-pipeline-w14b-plus-tauri-8cmds-2026-07-22.md` + `feat/w14b-plus-8-commands` 分支
**目标**:在 30 min `cargo tauri dev` 启动 + 5 tab 手动验证基础上,补完 3 个 P0/P1 功能:

1. `read_log` 后端 command(替代占位)+ 前端 2s 轮询 + offset diff
2. `read_lecture` W12-D `output_final/` 优先 + W3-W11 fallback
3. 前端 read_lecture modal(marked.js 渲染 md + iframe srcdoc 隔离 html)

---

## 1. 背景与现状

### 1.1 现状(W14-B+ 收尾)

- `feat/w14b-plus-8-commands` 分支 6 commit / 30 unit test pass
- 8 commands 全实装:T2=4 只读 FS,T3=4 子进程,T4=2 Python API bridge
- `read_lecture_impl` 当前只读 `<work>/chapters/raw/<stem>/<file>`(W3-W11 布局)
- 前端 `tailLog(path)` 是占位:显示"暂未实装 — 走 Tauri fs plugin 或后端 read_file command"
- Cargo.toml 已有 `tokio = { features = ["process","io-util","sync","rt","fs","macros","time"] }`
- `RunningRun.log_path` 已在 `RunPipelineResult` 和 `RunningRun` 里
- `<inbox>/output/mtd.log` 是真实写入位置(runner.rs:62),由 stdout/stderr 合并

### 1.2 撞墙(W14-B+ 已知)

- `cargo tauri dev` 在公司 VPN + schannel 下仍未在本机真跑过;handoff 给出 default crates-io + `CARGO_NET_TLS_VERIFY=false` workaround
- W12-D 产物 `<inbox>/output_final/<stem>.md` 等在 Tauri UI 的 `read_lecture` 读不到(代码只查 W3-W11 路径)

### 1.3 决策记录

| 决策 | 选定 | 替代(否决) |
|---|---|---|
| dev 验证 | 仅 launch + 5 tab 手动点(30min) | 跑短 demo 视频(2-3h) / 跑 107min 视频(4-5h) |
| log tail 实现 | 后端 read_log 一次性读全文 + offset | tauri-plugin-fs / tokio tail task + event emit |
| read_lecture 源 | W12-D output_final 优先 + W3-W11 fallback | 仅 W12-D(破坏 W10-A 老产物) / 保持 W3-W11 only |
| modal 渲染 | marked.js + iframe srcdoc | 纯 `<pre>` / 后端转 HTML |

---

## 2. 架构

### 2.1 模块关系

```
                    ┌─ Tauri shell (media-to-doc-ui.exe) ─┐
   user click       │  5 tab SPA (index.html)              │
   ────────────────▶  • Inbox / Run / Output / Health / L  │
       │             │  • marked.js 12KB (CDN + SRI)        │
       │             │  • iframe srcdoc sandbox=...         │
       │             └────────┬────────────────────────────┘
       │                      │ invoke(...)
       │                      ▼
       │             ┌─ Tauri commands (Rust) ───────┐
       │             │  • list_courses / check_status │
       │             │  • list_outputs                │
       │             │  • read_lecture   ← W12-D 优先 │
       │             │  • read_log      ← NEW         │
       │             │  • run_pipeline / cancel_run /  │
       │             │    resume_pipeline / list_running│
       │             │  • get_run_metrics / list_runs  │
       │             └────┬──────────────┬────────────┘
       │                  │              │
       │            spawn uv run mtd  std::fs
       │                  │              │
       ▼                  ▼              ▼
   ┌──────────────────┐ ┌────────────┐ ┌──────────────┐
   │  user click read │ │  mtd 进程  │ │  mtd.log     │
   │  → modal +       │ │  (subproc) │ │  (File)      │
   │    marked /      │ │            │ │  ← read_log  │
   │    iframe        │ │            │ │     2s 轮询  │
   └──────────────────┘ └────────────┘ └──────────────┘
```

### 2.2 数据流

**log tail(R1)**:
```
T=0s:run_pipeline → RunningRun.log_path = <work>/mtd.log
T=2s:前端 tailLog() → invoke('read_log', {path, offset: 0, maxLines: 200})
     → 后端:open file + seek 0 + read + return {content, new_offset: N}
T=4s:前端 tailLog() → invoke('read_log', {path, offset: N, maxLines: 200})
     → 后端:open file + seek N + read new content only (typically 0 bytes)
T=6s:append happened → invoke → 后端返回 delta → 前端 append 到 pre
```

**read_lecture(R2)**:
```
1. 前端 invoke('read_lecture', {inboxDir, version: 'cleaned', fmt: 'md'})
2. 后端 3 级 fallback 链:
   a) <inbox>/output_final/<stem>.<ext>             ← W12-D 真相(默认)
   b) <inbox>/output_final/<stem>_cleaned.<ext>     ← W12-D cleaned
      <inbox>/output_final/<stem>_final.<ext>       ← W12-D final html
   c) <inbox>/output/chapters/raw/<stem>/<file>    ← W3-W11 旧布局 fallback
3. 返回 {content, path, source: "output_final" | "legacy", size_bytes}
4. 前端 modal:
   - fmt=='md' → marked.parse(content) → innerHTML(modal-body)
   - fmt=='html' → iframe.srcdoc = content, sandbox="allow-same-origin"
```

### 2.3 错误处理

| 场景 | 处理 |
|---|---|
| `read_log` 文件不存在 | 返回 `CommandResponse::err` 含 path;前端 modal 显示"(日志尚未生成)" |
| `read_log` 文件被 truncate(罕见) | `total_bytes < offset` 时重置 offset = 0,返回全文件 + 标注 reset |
| `read_lecture` 全部 3 级都不存在 | 返回 err 含"请先跑流水线";modal 显示同样消息 |
| `read_lecture` html 不存在但 md 存在 | 自动 fallback 到 md,内容前加 `# <stem> (<version> · html)\n\n(html 版本不存在,以下为 md 版本内容)`,note="html fallback" |
| marked CDN 加载失败 | 降级到 `<pre>{content}</pre>` 纯文本(modal 加 banner "marked.js 加载失败,显示原始内容") |
| iframe srcdoc 内 CDN 失败 | 不处理(用户责任;iframe sandbox 关掉 script 避免 XSS 复发) |

---

## 3. API 设计

### 3.1 新增 Tauri command: `read_log`

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ReadLogResult {
    /// 增量内容(从 offset 到文件末尾)
    pub content: String,
    /// 新 offset(下次 read_log 的起点)
    pub new_offset: u64,
    /// 当前文件总字节数
    pub total_bytes: u64,
    /// 文件被 truncate 时为 true(offset 重置)
    pub truncated: bool,
    /// 截断到 max_lines(只返回前 N 行,余下在下次读)
    pub truncated_to_lines: bool,
}

#[tauri::command]
pub async fn read_log(
    path: String,
    offset: u64,
    max_lines: usize,
) -> CommandResponse<ReadLogResult>
```

**约束**:
- `path` 必须在 `<workspace_root>/<inbox>/output/mtd.log` 或 `/resume` 的 `<work>/mtd.log` 派生路径上;不实现严格白名单(本机用,信任用户),仅校验 `path.ends_with("mtd.log")`
- `offset` 单调递增由前端维护,无需文件锁
- `max_lines` 默认 200,硬上限 2000(防 DoS)
- `truncated` 当 `total_bytes < offset` 为 true,前端下次从 0 开始读

**测试**:
- `read_log_returns_empty_when_offset_equals_size`
- `read_log_returns_content_from_offset`
- `read_log_resets_on_truncate`
- `read_log_errors_on_missing_file`
- `read_log_caps_max_lines`

### 3.2 改 `read_lecture`:3 级 fallback

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ReadLectureResult {
    pub version: String,
    pub fmt: String,
    pub path: String,
    pub content: String,
    pub size_bytes: usize,
    /// "output_final" | "legacy" | "fallback_md"
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}
```

**3 级 fallback 链**(在 `read_lecture_impl` 内):
```rust
let candidates = match (version.as_str(), fmt.as_str()) {
    ("raw", "md")   => vec!["<stem>.md"],
    ("raw", "html") => vec!["<stem>.html", "<stem>.md"],   // html 缺 → md
    ("cleaned", "md")   => vec!["<stem>_cleaned.md"],
    ("cleaned", "html") => vec!["<stem>_cleaned.html", "<stem>_cleaned.md"],
    ("final", "md")     => vec!["<stem>_final.md"],
    ("final", "html")   => vec!["<stem>_final.html", "<stem>_final.md"],
    _ => return err,
};

for rel in &candidates {
    let p1 = output_final.join(rel);
    if p1.is_file() { return ok(p1, "output_final"); }
}
for rel in &candidates {
    let p2 = work.join("chapters/raw/<stem>").join(rel);
    if p2.is_file() { return ok(p2, "legacy"); }
}
return err;
```

**测试**(在 `commands.rs::tests` 追加):
- `read_lecture_prefers_output_final_over_legacy`
- `read_lecture_falls_back_to_legacy_when_output_final_missing`
- `read_lecture_html_falls_back_to_md_with_note`
- `read_lecture_returns_source_field`

### 3.3 前端 `index.html` 改动

**marked.js 引入**:
```html
<script src="https://unpkg.com/marked@12.0.0/marked.min.js"
        integrity="sha384-<SRI>"
        crossorigin="anonymous"></script>
```
SRI hash 需在写 spec 时用 https://www.srihash.org/ 算,落到 commit 时再贴。

**modal CSS**(追加):
```css
.modal-backdrop {
  position: fixed; inset: 0;
  background: rgba(0,0,0,0.7);
  display: flex; align-items: center; justify-content: center;
  z-index: 2000;
}
.modal {
  width: 80vw; height: 80vh;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
  display: flex; flex-direction: column;
}
.modal-toolbar { padding: 12px; border-bottom: 1px solid var(--border); }
.modal-body { flex: 1; overflow: auto; padding: 16px; }
.modal-body iframe { width: 100%; height: 100%; border: 0; }
```

**log tail JS**(替换占位 `tailLog`):
```js
let lastLogOffset = 0;
async function tailLog(path, n = 200) {
  if (!path) return;
  try {
    const r = await invoke('read_log', { path, offset: lastLogOffset, maxLines: n });
    if (!r.ok) { $('log-tail').textContent = '(read_log 失败: ' + r.error + ')'; return; }
    if (r.data.truncated) lastLogOffset = 0;       // 文件被 truncate → 重置
    const delta = r.data.content;
    if (delta) {
      const pre = $('log-tail');
      pre.textContent += delta;
      // 限制 pre 长度(避免无限增长)
      if (pre.textContent.length > 200_000) {
        pre.textContent = pre.textContent.slice(-100_000);
      }
      pre.scrollTop = pre.scrollHeight;
    }
    lastLogOffset = r.data.new_offset;
  } catch (e) {
    $('log-tail').textContent = '(read_log 异常: ' + e + ')';
  }
}
```

**modal JS**:
```js
async function openReadLecture(inbox, version, fmt) {
  const r = await invoke('read_lecture', { inboxDir: inbox, version, fmt });
  const body = $('modal-body');
  if (!r.ok) { body.innerHTML = '<div class="empty">' + r.error + '</div>'; showModal(); return; }
  if (fmt === 'md' && window.marked) {
    body.innerHTML = marked.parse(r.data.content);
  } else if (fmt === 'html') {
    const iframe = document.createElement('iframe');
    iframe.sandbox = 'allow-same-origin';
    iframe.srcdoc = r.data.content;
    body.innerHTML = '';
    body.appendChild(iframe);
  } else {
    body.innerHTML = '<pre>' + r.data.content + '</pre>';
  }
  $('modal-source').textContent = `source: ${r.data.source} · ${(r.data.size_bytes/1024).toFixed(1)} KB`;
  showModal();
}
```

**Output tab 文件条目加 [read] 按钮**:
```js
// 在 renderOutputs 内,把每个 file 加按钮
${d.outputs.raw_md.map(f => `<button onclick="openReadLecture('${d.inbox}','raw','md')">📄 ${f}</button>`).join('')}
```

---

## 4. 测试策略

### 4.1 单元测试(cargo test)

| 项 | 数量 | 说明 |
|---|---|---|
| `read_log` 单测 | 5 | 正常 offset / truncate 重置 / missing file / max_lines 上限 / 空文件 |
| `read_lecture` 3 级 fallback | 4 | output_final 优先 / legacy fallback / html→md fallback / source 字段 |
| 既有 8 commands | 不动 | 30 baseline 保持 |

**目标**:5 + 4 = 9 新增,30 → 39 passed / 0 failed。

### 4.2 手动测试(`cargo tauri dev` 30min)

| 步骤 | 期望 |
|---|---|
| 1. 启动 dev,看到 Inbox tab 5 tab 渲染 | status dot 绿 |
| 2. Inbox:输入 workspace + Refresh | list_courses 返空 / 课程列表 |
| 3. Run:选课 + LLM=ollama + Run pipeline | run_pipeline 返 pid + log_path;UI 显示 running |
| 4. Run:log tail 区域 2s 间隔出现新日志 | 看到 `[audio]` 等 stage 标记 |
| 5. Run:5s poll → stage-grid 11 dot 渐次亮 | check_status 返 stages map |
| 6. Run:Cancel → child killed,registry 清理 | cancel_run 返 killed=true |
| 7. Output:list_outputs 显示 raw/cleaned/final/images 分组 | list_outputs ok |
| 8. Output:点 `cleaned.md` [read] → modal → marked 渲染 | 看到 H1/H2/表格/TOC |
| 9. Output:点 `final.html` [read] → modal → iframe srcdoc | HTML 自带样式生效 |
| 10. Health:get_run_metrics / list_runs 返 JSON | 不 panic |
| 11. Learn:app_info 真实探针成功 | 看到 mtd_version / pyapi / mcp 状态 |

任一失败 → 立即修;cargo test 重跑确认。

### 4.3 回归测试

- 30 baseline 仍 pass(不破坏既有 8 commands 行为)
- `cargo clippy -- -D warnings` 不新增 warning
- 前端无 console error(打开 dev tools 看)

---

## 5. 风险与缓解

| 风险 | 缓解 |
|---|---|
| `cargo tauri dev` 再次撞 Cargo SSL | handoff §T1 workaround 必用,提前 `CARGO_NET_TLS_VERIFY=false` + PATH |
| 公司 VPN proxy 污染 subprocess 启动 | runner 已有 MEDIA_TO_DOC_PROJECT env + `kill_on_drop`;`uv` 命令走 PATH,proxy 撞到 ollama SDK(W14-B 已修)不影响本会话 |
| marked.js CDN 不可达(SRI 锁版本) | UI fallback 到 `<pre>` 纯文本;spec 写明降级路径 |
| iframe srcdoc 引发 XSS | sandbox 关闭 script(allow-same-origin 不含 allow-scripts);与 W12-C mermaid 修复保持一致 |
| read_log 大文件 >10MB 内存 | max_lines=200 硬上限 + 前端 pre 截到 200KB;>10MB log 罕见(runner 用 `tee` 类似机制单 stage < 50MB) |
| W12-D 老产物(无 output_final)兼容 | fallback 链到 W3-W11,source="legacy" 标注 |
| 30 min 验证时长撞 session 健康上限(2h) | brainstorming 已设 <2h 预算,本会话总长不超过 4h(包含 spec + plan + 实装) |

---

## 6. 不在范围

- **真实端到端 11 stage 流水线**(W14-C)
- **多课程并发 UI**(list_running 后端已支持,UI 暂单 course)
- **系统托盘 + 系统通知**(W14-B+2 留 W14-C)
- **NSIS 安装器 / release build**(v1.4 Phase 3)
- **Tauri attach 模式(跨 session 跟踪)**(ARCHITECTURE.md §3 留)
- **anthropic / openai_compat provider `trust_env=False` 加固**(W14-B+ follow-up)

---

## 7. 验收 Checklist

- [ ] 3 commit 落地:`read_log` command / `read_lecture` W12-D 优先 / 前端 log tail + modal
- [ ] `cargo test` 30 → 39 passed / 0 failed
- [ ] `cargo tauri dev` 启动 OK(每新 shell `CARGO_NET_TLS_VERIFY=false` + PATH)
- [ ] 5 tab 手动验证 11 步全过(§4.2)
- [ ] 既有 8 commands 行为不变
- [ ] docs/RELEASE_NOTES_v1.3.0-alpha.md 写(可选,本轮 v1.3 不强求上 PyPI)
- [ ] handoff `handoff-pipeline-w14b-plus-2-ui-features-2026-07-22.md` 写,等 W14-C 接手

---

## 8. 交付物

| 路径 | 内容 |
|---|---|
| `media-to-doc-ui/src-tauri/src/commands.rs` | + `read_log` command + `read_lecture` 3 级 fallback |
| `media-to-doc-ui/src-tauri/src/lib.rs` | invoke_handler 加 `read_log` |
| `media-to-doc-ui/src/index.html` | marked.js + modal CSS/JS + log tail JS + Output tab [read] 按钮 |
| `media-to-doc-ui/docs/superpowers/specs/2026-07-22-w14b-plus-2-ui-features-design.md` | 本文件 |
| `media-to-doc/handoff-pipeline-w14b-plus-2-ui-features-2026-07-22.md` | 会话快照(W14-C 接手) |

---

**Why this design**:借 8 commands 既有 6 commit 的 impl/test 模式,3 个 P0/P1 项都加在后端 + 前端各 1 个独立小文件;marked.js 替代重后端转换,iframe srcdoc 复用 W12-C 的 sandbox 防 XSS 经验;W12-D 3 级 fallback 与 Python 端 MCP 8 工具语义保持一致,前端无须特殊处理。

**How to apply**:进入 writing-plans 阶段时按本 spec §3 的 API 切 5 个任务(2 后端 + 2 前端 + 1 收尾);cargo test 是真理,任何新代码改动先跑 `cargo test` 30 baseline + 新增 9 用例再合并。
