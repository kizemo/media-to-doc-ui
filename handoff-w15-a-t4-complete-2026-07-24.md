# Handoff — W15-A T4 完成,下一会话进入 T5

**日期**:2026-07-24
**项目**:`F:/soft/00selfmade/media-to-doc-ui`
**当前分支**:`feat/w15a-llm-api-settings`(基线 db84639,不动)
**承接 handoff**:`handoff-w15-a-t3-complete-2026-07-24.md`

---

## 0. 下一会话先做什么(非技术说明)

开始执行前,必须先用普通用户能理解的语言告知以下任务清单,不要直接讲 Rust struct、JSON schema 或环境变量:

1. **把激活的 AI 服务商信息真正接通到跑流水线的子进程**:目前用户在桌面前端选了"DeepSeek"为激活服务商后,点击"Run pipeline"时,这个选择还没传给底层跑工作的程序。T5 任务就是把这一步接通,让底层程序能拿到 DeepSeek 的 API key 和 base URL。
2. **避免父进程环境变量污染子进程**:本机开发时如果设过 HTTP_PROXY 等代理变量,会把它们带进跑工作的程序,可能撞 SSL / DNS 等奇怪问题。T5 要保证子进程拿到的是干净的、只有 AI 相关的环境变量。
3. **不破坏已有功能**:之前 T1~T4 已经做好的 95 个测试还要全过。
4. **用测试证明行为正确**:先写会失败的测试,再实现功能,最后验证本模块和全部已有测试通过。

面向用户的最终效果:用户在桌面前端选好激活的 AI 服务商,点击"Run pipeline",底层程序马上能拿到对应的 API key + base URL,无需手工设环境变量;且不会带上无关的系统代理变量。

---

## 1. T4 完成情况

### T4:commands.rs 6 个 LLM Tauri command + SaveProfileArgs + TestConnectionResult ✅

已完成但未 commit(遵守加快模式"W15-A feature 整体一次 commit"规则)。

### 1.1 必交付清单

| 项 | 状态 | 位置 |
|---|---|---|
| 6 个 `#[tauri::command]` 函数 | ✅ | `src-tauri/src/commands.rs` 行 ~1170-1530 |
| `SaveProfileArgs` struct | ✅ | 同上 |
| `TestConnectionResult` struct | ✅ | 同上 |
| `llm_profiles::probe_endpoint()` 纯函数 | ✅ | `src-tauri/src/llm_profiles.rs` 行 ~338 |
| `lib.rs` pub use + invoke_handler 注册 6 command | ✅ | `src-tauri/src/lib.rs` |
| `reqwest` 依赖(Cargo.toml,rustls-tls,无 json feature) | ✅ | `src-tauri/Cargo.toml` |
| 8 个错误码前缀(KEYRING_ERROR / PROFILE_NOT_FOUND / PROFILE_NAME_CONFLICT / INVALID_BASE_URL / INVALID_MODEL / NETWORK_ERROR / PROVIDER_NOT_FOUND / ACTIVE_PROFILE_REQUIRED) | ✅ | 散落在 commands.rs / llm_profiles.rs |

### 1.2 6 个 command 签名

```rust
#[tauri::command] pub async fn list_llm_profiles() -> CommandResponse<Vec<ProfileMeta>>
#[tauri::command] pub async fn get_active_llm_profile_name() -> CommandResponse<String>
#[tauri::command] pub async fn save_llm_profile(args: SaveProfileArgs) -> CommandResponse<ProfileMeta>
#[tauri::command] pub async fn set_active_profile(name: String) -> CommandResponse<()>
#[tauri::command] pub async fn delete_llm_profile(name: String) -> CommandResponse<()>
#[tauri::command] pub async fn test_llm_connection(name: String) -> CommandResponse<TestConnectionResult>
```

每个 command 有 `*_impl` 纯函数(单测入口) + `#[tauri::command]` 薄包装(只做参数透传)。

### 1.3 测试证据

| 阶段 | 测试数 | 累计 |
|---|---|---|
| Baseline | 43 | 43 |
| T1 (keyring) | 5 | 48 |
| T2 (templates + 校验) | 17 | 65 |
| T3 (metadata IO + env) | 18 | 83 |
| **T4 原始 9 个** | 9 | 92 |
| **T4 reviewer fix +3 个** | 3 | **95** |

最终:**95 / 95 passed; 0 failed**(目标 91+,已超出)

```
$ cargo test --lib
test result: ok. 95 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### 1.4 Release build

```
$ cargo build --release
Finished `release` profile [optimized] target(s) in 2m 43s
warning: 8 warnings(全是 T1~T3 pub API "never used",T5 runner 会消费)
```

### 1.5 Reviewer 复审结论

| 维度 | 结论 |
|---|---|
| Critical | 0 |
| Important | 0(3 条已修: Ollama 无 key 不报错 / chrono 格式 / reqwest 瘦 feature) |
| Minor | 0(4 条全部接受;不阻塞 W15-A) |

---

## 2. 当前工作区状态

```
## feat/w15a-llm-api-settings
 M docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md
 M docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md
 M src-tauri/Cargo.toml                                              ← T1 + T4
 M src-tauri/src/lib.rs                                              ← T1 + T2 + T4
?? handoff-w15-a-providers-decided-v142-icon-2026-07-24.md          ← 历史保留
?? handoff-w15-a-t2-complete-2026-07-24.md                           ← 历史保留
?? handoff-w15-a-t3-complete-2026-07-24.md                           ← 历史保留
?? handoff-w15-a-t4-complete-2026-07-24.md                          ← 本文件(新建)
?? handoff-w15-a-v141-release-2026-07-24.md                          ← superseded
?? prompt-next-session.md                                            ← superseded
?? prompt-w15-a-t5-next.md                                           ← 本文件配套(新建)
?? src-tauri/src/keyring_store.rs                                    ← T1
?? src-tauri/src/llm_profiles.rs                                     ← T2 + T3 + T4(probe_endpoint)
```

**禁止事项**(沿用加快模式):

- 不得 reset / checkout / restore / 覆盖未提交改动
- 不得切回 `master` 直接开发
- 不得提前 commit T1~T4;继续遵守"W15-A feature 整体一次 commit"
- 不得删除旧 handoff / prompt(删除需用户二次确认)
- **不得**修改主仓 `media-to-doc/`(mtd Python 端零改动 — env var 注入沿用 W14-D trust_env=False 路径)

**主仓状态**:`F:/soft/00selfmade/media-to-doc` 未动(仅 pre-existing untracked `docs/media-to-doc.png` + `docs/电商术语表.md`)。

---

## 3. 下一任务:T5

依据:`docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md` **Task 4**(plan 编号与 handoff T 编号不同,plan Task 4 = handoff T5)+ spec §5。

### 3.1 T5 必交付

1. **`SpawnSpec` 加 `env_vars: HashMap<String, String>` 字段** — `src-tauri/src/runner.rs`
2. **`spawn_mtd()` 加 `.env_clear().envs(env_vars)`** — 同一文件,清父进程 env 后注入 spec.env_vars
3. **`run_pipeline` 改读 active profile** — `src-tauri/src/commands.rs` 行 ~995 区域
4. **`resume_pipeline` 改读 active profile** — 同一文件 行 ~1056 区域
5. **env 注入 3 步协作**:
   ```
   let active = llm_profiles::get_active_profile()?;
   let key = keyring_store::read_key(&active.name)?;
   spec.env_vars = llm_profiles::to_env_vars(&active, &key)?;
   ```
6. **≥ 1 个新单元测试** — 验证 spawn_mtd 注入的 env vars 不含父进程的 HTTP_PROXY 等污染
7. **`let mut spec = ...`** 修改 — 现有 `let spec = build_mtd_*_args(...)` 改成 `let mut` 才能 mutate `spec.env_vars`

### 3.2 关键避坑提示

- **`env_clear()` 防 HTTP_PROXY 污染**:W14-D 已验证,公司 VPN proxy 会带进子进程撞 SSL。env_clear() 后只 inject spec.env_vars,父进程 HTTP_PROXY/HTTPS_PROXY/NO_PROXY/all_proxy 等 8 个 proxy vars 全清掉。
- **ACTIV E_PROFILE_REQUIRED 错误传播**:`llm_profiles::get_active_profile()` 已在 T3 实现好,active=None 时报 `ACTIVE_PROFILE_REQUIRED:` 前缀。`run_pipeline` / `resume_pipeline` 直接 return CommandResponse::err(这条错误即可)。
- **测试隔离**:spawn_mtd 测试**不要**真起 mtd 子进程(耗时且污染),只需验证 builder 的 env 配置(用 `tokio::process::Command` 的 args / env 抓取,或重构 spawn_mtd 让 env 构造可单测)。
- **runner.rs 已有测试**:`registry_*` 系列 6 个测试在 `runner::tests` — 改 SpawnSpec 时要确保这些不挂;T3 handoff 提到 6 个原测试。

### 3.3 TDD 顺序

1. 先写 ≥ 1 个 T5 测试(验证 env_clear + envs 行为)
2. 跑定向测试,记录 RED
3. 最小实现:SpawnSpec 加字段 + spawn_mtd 改 .env_clear().envs() + run/resume_pipeline 加 3 行 env 注入
4. 跑 `runner::` 定向测试到全过
5. 跑完整 `cargo test --lib`(目标 96+ passed / 0 failed)
6. 独立 spec + code quality review;Critical / Important 必须修

如果 T5 完成且预算允许,继续 T6(runner.rs 改后,前端 UI Settings tab 接入);否则更新本 handoff 或写后继 handoff。

---

## 4. 项目进度定位

- 当前发布版本:子仓 v1.4.2。
- W15-A 目标版本:v1.5.0。
- W15-A 总任务:T1-T8。
- 已完成:**T1、T2、T3、T4**(4 / 8)。
- 下一步:T5(runner.rs SpawnSpec.env_vars + spawn_mtd env injection)。
- 后续:T6 frontend Settings UI(简版,可与 T5 并行);T7/T8 全量验收 + `cargo tauri build` + v1.5.0 release。
- 用户加快模式继续生效:不再做小版本 release;W15-A 完成后统一 feature commit 和 v1.5.0 release。

---

## 5. 下一会话必读顺序

1. 本文件:`handoff-w15-a-t4-complete-2026-07-24.md`
2. 简短 prompt:`prompt-w15-a-t5-next.md`(只看路径)
3. `handoff-w15-a-providers-decided-v142-icon-2026-07-24.md` §0.5、§1(加快模式规则)
4. `docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md` §5(env var 注入关键路径)+ §4 错误码
5. `docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md` **Task 4**(plan 编号的 runner.rs Task 4 = handoff T5)
6. `src-tauri/src/runner.rs`(已有 SpawnSpec + spawn_mtd + 6 个 runner_tests)
7. `src-tauri/src/commands.rs` 行 ~995-1110(run_pipeline + resume_pipeline 当前位置)
8. `src-tauri/src/llm_profiles.rs` 行 ~338-360(`to_env_vars` + `probe_endpoint` 纯函数)
9. `src-tauri/src/keyring_store.rs`(T1,4 函数 + 5 tests,read_key/write_key 给 T5 用)
10. `git status --short --branch`

---

## 6. 事实面状态

| 事实面 | 状态 | 证据 |
|---|---|---|
| 代码 | changed-and-verified | T1+T2+T3+T4 已实现,95/95 pass |
| 运行态 | pending | W15-A 尚未接通 runner / UI |
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
| `prompt-next-session.md` | 已被本会话的 `prompt-w15-a-t5-next.md` supersede |
| `handoff-w15-a-providers-decided-v142-icon-2026-07-24.md` | 保留(记录加快模式规则 + 9-provider 决策细节) |
| `handoff-w15-a-t2-complete-2026-07-24.md` | 保留(记录 T2 实现细节) |
| `handoff-w15-a-t3-complete-2026-07-24.md` | 保留(记录 T3 实现细节) |
| `handoff-w15-a-t4-complete-2026-07-24.md` | 本文件,保留 |

---

## 8. 新会话第一句

> 承接 `F:/soft/00selfmade/media-to-doc-ui/handoff-w15-a-t4-complete-2026-07-24.md`,W15-A T4 已完成(95/95 pass),下一会话进入 T5(runner.rs SpawnSpec.env_vars + spawn_mtd env 注入)。