# Handoff — W15-A T5 完成,下一会话进入 T6

**日期**:2026-07-24
**项目**:`F:/soft/00selfmade/media-to-doc-ui`
**当前分支**:`feat/w15a-llm-api-settings`(基线 db84639,不动)
**承接 handoff**:`handoff-w15-a-t4-complete-2026-07-24.md`

---

## 0. 下一会话先做什么(非技术说明)

开始执行前,必须先用普通用户能理解的语言告知以下任务清单,不要直接讲 Tauri command、env_clear 或 Rust struct:

1. **把激活的 AI 服务商信息真正接通到跑流水线**:T5 已经做完。用户在前端选了"DeepSeek"为激活服务商后,点击"Run pipeline"时,Tauri 后端会从系统钥匙串(Windows 凭据管理器 / macOS 钥匙串 / Linux Secret Service)读出 DeepSeek 的 API key,再注入到底层跑 mtd 的子进程。用户在桌面前端操作,API key 全程不写配置文件,重启不丢,安全存系统钥匙串。

2. **避免父进程环境变量污染子进程**:本机开发时如果设过 HTTP_PROXY 等代理变量,T5 已实现 `env_clear()` 清掉它们,只注入 LLM 相关的 key + base URL,不再撞 SSL / DNS 等奇怪问题。同时保留父进程 PATH(uv 等可执行文件需要 PATH 来查找,否则 Windows 上找不到 uv.exe)。

3. **不破坏已有功能**:T1~T4 累计 95 个测试 + T5 新增 3 个 = 98 个测试,全部通过(0 failed)。

4. **下一步做 T6 前端 Settings tab**:把 T1~T5 后端能力接通到 UI — 6 tab 化 + Settings > Providers 子页 + 添加/编辑/删除 profile 弹窗 + 测试连接按钮。

面向用户的最终效果:用户在桌面前端选好激活的 AI 服务商,点击"Run pipeline",底层程序马上能拿到对应的 API key + base URL,无需手工设环境变量;且不会带上无关的系统代理变量。

---

## 1. T5 完成情况

### 1.1 必交付清单

| 项 | 状态 | 位置 / 证据 |
|---|---|---|
| `SpawnSpec` 加 `env_vars: HashMap<String, String>` 字段 | ✅ | `src-tauri/src/runner.rs` 行 24-39,`#[serde(default)]` 兼容旧 JSON |
| `build_mtd_run_args` / `build_mtd_resume_args` 初始化 `env_vars: HashMap::new()` | ✅ | `runner.rs` 行 92-98 / 行 124-130 |
| `spawn_mtd` 抽 `build_child_command` 纯函数 + `.env_clear().env("PATH").envs(&spec.env_vars)` | ✅ | `runner.rs` 行 467-475 |
| `commands.rs` `inject_active_llm_env()` 协作函数(get_active_profile → read_key → to_env_vars) | ✅ | `commands.rs` 行 1122-1134 |
| `run_pipeline` 改用 `let mut spec` + `inject_active_llm_env(&mut spec)` | ✅ | `commands.rs` 行 998-1061 |
| `resume_pipeline` 改用 `let mut spec` + `inject_active_llm_env(&mut spec)` | ✅ | `commands.rs` 行 1063-1119 |
| ≥1 个新单元测试 | ✅ **3 个** | `runner::tests` 3 个新测试 |
| 3 个原 SpawnSpec 字面量补 `env_vars: HashMap::new()`(不挂) | ✅ | `runner.rs` 行 604 / 618 / 646 |
| 全量 cargo test 目标 96+ passed / 0 failed | ✅ **98/98** | 见 §1.3 |
| release build | ✅ 2m 33s | 5 warnings(T6 frontend 会消费) |

### 1.2 3 个新单元测试

| 测试名 | 验证内容 |
|---|---|
| `spawn_spec_env_vars_defaults_to_empty` | `build_mtd_run_args` / `build_mtd_resume_args` 返回的 spec 默认 env_vars 是空 HashMap |
| `spawn_mtd_clears_parent_env_and_injects_spec_env` | 真 spawn cmd/sh echo 短进程:父进程设 HTTP_PROXY=evil:8080,spec.env_vars 注入 OPENAI_API_KEY=secret;子进程 echo 应含 secret 但不含 evil |
| `build_child_command_inherits_parent_path` | 真 spawn cmd/sh echo:验证 `.env("PATH", parent_path)` 注入,子进程 PATH 不空(避免 uv 找不到) |

### 1.3 测试证据

| 阶段 | 测试数 | 累计 |
|---|---|---|
| T4 收尾 | - | 95 |
| **T5 新增** | 3 | **98** |

最终:**98 / 98 passed; 0 failed**(目标 96+,超出)

```
$ cargo test --lib
test result: ok. 98 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### 1.4 Release build

```
$ cargo build --release
Finished `release` profile [optimized] target(s) in 2m 33s
warning: 5 warnings(全是 T2 ProviderTemplate/all_templates/provider_name,
                    T6 frontend 会消费)
```

(对比 T4 收尾时 8 warnings,降 3 个,因为 T5 runner 消费了 `to_env_vars` / `get_active_profile` / `read_key`)

### 1.5 Reviewer 复审结论

| 维度 | 结论 |
|---|---|
| Critical | 0(初稿 1 个:`env_clear()` 后 PATH 为空 uv 找不到 → 已修,加 `.env("PATH", parent_path)` + test 3 覆盖) |
| Important | 0(3 错误前缀 ACTIVE_PROFILE_REQUIRED/KEYRING_ERROR/PROVIDER_NOT_FOUND 通过 `?` 透传) |
| Minor | 0(5 warnings 留 T6 消费,不阻塞 W15-A) |

---

## 2. 当前工作区状态

```
## feat/w15a-llm-api-settings
 M docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md          ← T1~T4 累计
 M docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md    ← T1~T4 累计
 M src-tauri/Cargo.toml                                                  ← T1 + T4
 M src-tauri/src/lib.rs                                                  ← T1 + T2 + T4
 M src-tauri/src/commands.rs                                             ← T1 + T4 + T5
 M src-tauri/src/runner.rs                                               ← T5 (NEW 改动)
?? handoff-w15-a-providers-decided-v142-icon-2026-07-24.md               ← 历史保留
?? handoff-w15-a-t2-complete-2026-07-24.md                                ← 历史保留
?? handoff-w15-a-t3-complete-2026-07-24.md                                ← 历史保留
?? handoff-w15-a-t4-complete-2026-07-24.md                                ← 历史保留
?? handoff-w15-a-t5-complete-2026-07-24.md                                ← 本文件(新建)
?? handoff-w15-a-v141-release-2026-07-24.md                               ← superseded 历史
?? prompt-next-session.md                                                 ← superseded 历史
?? prompt-w15-a-t5-next.md                                                ← 已被本会话接力
?? src-tauri/src/keyring_store.rs                                         ← T1
?? src-tauri/src/llm_profiles.rs                                          ← T2 + T3 + T4
?? task.md                                                               ← 本会话新建(W15-A 进度)
```

**禁止事项**(沿用加快模式):

- 不得 reset / checkout / restore / 覆盖未提交改动
- 不得切回 `master` 直接开发
- 不得提前 commit T1~T5;继续遵守"W15-A feature 整体一次 commit"
- 不得删除旧 handoff / prompt(删除需用户二次确认)
- **不得**修改主仓 `media-to-doc/`(mtd Python 端零改动 — env var 注入沿用 W14-D trust_env=False 路径)

**主仓状态**:`F:/soft/00selfmade/media-to-doc` 未动(仅 pre-existing untracked `docs/media-to-doc.png` + `docs/电商术语表.md`)。

---

## 3. T5 关键实现细节

### 3.1 `build_child_command` 纯函数(runner.rs 行 467-475)

```rust
pub fn build_child_command(spec: &SpawnSpec) -> Command {
  let mut cmd = Command::new(&spec.program);
  cmd.args(&spec.args)
    .current_dir(&spec.work_dir)
    .env_clear()                                                        // 防 HTTP_PROXY 污染
    .env("PATH", std::env::var("PATH").unwrap_or_default())             // ★ 保留 PATH 让 uv 可被找到
    .envs(&spec.env_vars)                                               // 注入 active LLM profile env
    .kill_on_drop(true);
  cmd
}
```

**关键避坑**:`.env_clear()` 后 PATH 也被清,Windows 上 uv.exe 不在 System32,CreateProcess 找不到会报"系统找不到指定的文件"。修复:env_clear 后手动 `.env("PATH", parent_path)` 重新注入父 PATH。spec.env_vars 不会注入 PATH(由 `llm_profiles::to_env_vars` 保证),所以无冲突。

**为什么不保留全部父 env**:W14-D 撞过的公司 VPN proxy(HTTP_PROXY/HTTPS_PROXY/NO_PROXY/all_proxy 等 8 个 proxy vars)会污染子进程,撞 SSL/DNS。env_clear 是子进程级隔离(类似 Python httpx trust_env=False 的思路)。PATH 必须保留是因为 uv 在 PATH 中查找。

### 3.2 `inject_active_llm_env` 协作函数(commands.rs 行 1128-1134)

```rust
fn inject_active_llm_env(spec: &mut crate::runner::SpawnSpec) -> Result<(), String> {
  let active = llm_profiles::get_active_profile()?;       // ACTIVE_PROFILE_REQUIRED:
  let key = keyring_store::read_key(&active.name)?;       // KEYRING_ERROR:
  let env_vars = llm_profiles::to_env_vars(&active, &key)?; // PROVIDER_NOT_FOUND:
  spec.env_vars = env_vars;
  Ok(())
}
```

**错误传播**:`?` 透传底层 String,前缀保留。`run_pipeline` / `resume_pipeline` 收到 Err 后直接 `CommandResponse::err(e)`,前端能 grep 前缀判断错误类型(例如引导用户去 Settings > Providers 设置 active profile)。

### 3.3 run_pipeline / resume_pipeline 改动

```rust
let mut spec = build_mtd_run_args(...);    // 原:let spec
if let Err(e) = inject_active_llm_env(&mut spec) {
  return CommandResponse::err(e);
}
let child = match spawn_mtd(&spec).await { ... };
```

`mut` 是必须的,因为 `inject_active_llm_env` 需要 mutate `spec.env_vars`。

---

## 4. 项目进度定位

- 当前发布版本:子仓 v1.4.2(W14-G+ 收尾)。
- W15-A 目标版本:v1.5.0。
- W15-A 总任务:T1-T8。
- 已完成:**T1、T2、T3、T4、T5**(5 / 8)。
- 下一步:T6(frontend Settings UI 接入,可与 T5 后端能力并行)。
- 后续:T7 全量验收 + `cargo tauri build`;T8 v1.5.0 release。
- 用户加快模式继续生效:不再做小版本 release;W15-A 完成后统一 feature commit 和 v1.5.0 release。

---

## 5. T6 必读顺序(下一会话)

1. 本文件:`handoff-w15-a-t5-complete-2026-07-24.md`
2. `task.md`(新建,W15-A 进度总览)
3. `handoff-w15-a-providers-decided-v142-icon-2026-07-24.md` §0.5、§1(加快模式规则)
4. `docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md` §6-§9(前端 UI 设计)+ §3(9-provider 模板)+ §7(前端 wire 命令)
5. `docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md` Task 5/6/7/8(plan 编号 5=前端 UI Task)
6. `src/index.html`(前端入口,找 Settings tab 插入点 + 已有 5 tab 结构作参考)
7. `src-tauri/src/lib.rs`(6 commands invoke_handler 注册位置)
8. `src-tauri/src/commands.rs` 行 ~1170-1530(6 个 LLM command 签名:`list_llm_profiles` / `get_active_llm_profile_name` / `save_llm_profile` / `set_active_profile` / `delete_llm_profile` / `test_llm_connection`)
9. `src-tauri/src/llm_profiles.rs`(9 provider 模板 + `ProfileMeta` + `SaveProfileArgs` + `TestConnectionResult` 结构)
10. `git status --short --branch`

---

## 6. 事实面状态

| 事实面 | 状态 | 证据 |
|---|---|---|
| 代码 | changed-and-verified | T5 已实现,98/98 pass,release build OK |
| 运行态 | pending | T6 前端未接,UI 看不到 LLM 设置面板 |
| 文档 | pending | spec §8 旧 12-provider 测试口径待总同步(不阻塞 T5) |
| 规则 | verified-current | feature 分支、TDD、一次性 commit 规则已遵守 |
| 记忆 | out-of-scope | 本次仅做会话交接,不新增长期记忆 |
| 工作区 | pending | W15-A 未提交改动与历史残留文件均保留,不做破坏性清理 |

---

## 7. 历史 superseded 文件(建议保留,等你二次确认是否清理)

按全局 CLAUDE.md "删除文件前先二次确认",本会话**不**自动删除:

| 文件 | 状态 |
|---|---|
| `handoff-w15-a-v141-release-2026-07-24.md` | 已被 `handoff-w15-a-providers-decided-v142-icon-2026-07-24.md` supersede |
| `prompt-next-session.md` | 已被本分支各会话 prompt supersede |
| `prompt-w15-a-t5-next.md` | 已被本会话接力完成,可保留作历史 |
| `handoff-w15-a-providers-decided-v142-icon-2026-07-24.md` | 保留(加快模式规则 + 9-provider 决策) |
| `handoff-w15-a-t2/t3/t4/t5-complete-2026-07-24.md` | 全部保留(各阶段实现细节) |

---

## 8. 新会话第一句

> 承接 `F:/soft/00selfmade/media-to-doc-ui/handoff-w15-a-t5-complete-2026-07-24.md`,W15-A T5 已完成(98/98 pass),下一会话进入 T6(frontend Settings UI 接入 — 把 T1~T5 后端能力接到 UI,6 tab 化 + Providers 子页 + 添加/编辑 modal)。