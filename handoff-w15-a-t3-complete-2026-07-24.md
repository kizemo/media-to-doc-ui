# Handoff — W15-A T3 完成,下一会话进入 T4

**日期**:2026-07-24
**项目**:`F:/soft/00selfmade/media-to-doc-ui`
**当前分支**:`feat/w15a-llm-api-settings`
**基线 commit**:`db84639 build(ui): v1.4.2 — replace icon set with project logo`
**承接 handoff**:`handoff-w15-a-t2-complete-2026-07-24.md`

---

## 0. 下一会话先做什么(非技术说明)

开始执行前,必须先用普通用户能理解的语言告知以下任务清单,不要直接讲 Rust struct、JSON schema 或环境变量:

1. **添加 6 个桌面按钮**:为桌面前端的"AI 服务商配置"页面绑定 6 个操作(列出所有服务商 / 读取当前使用项 / 添加或修改 / 切换当前使用 / 删除 / 测试连接),前端点了立刻有响应。
2. **把配置改动落到磁盘**:用户的添加、编辑、删除、切换操作要能安全写入本地配置文件,不会留半写文件。
3. **错误信息能看懂**:用户填错或操作冲突时,前端能拿到明确的错误提示(比如"该名字已存在"、"没有当前使用的服务商"),而不是崩溃或弹通用错误。
4. **不破坏已有功能**:之前的所有按钮和自动化测试必须仍然正常工作。
5. **用测试证明行为正确**:先写会失败的测试,再实现功能,最后验证本模块和全部已有测试通过。

面向用户的最终效果:用户在桌面前端的所有"AI 服务商配置"操作都能正确读写本地文件并切换生效,出现错误时给出明确提示。

---

## 1. 本会话完成情况

### T3:profile metadata 持久化 + env var 映射 ✅

已完成但未 commit(遵守加快模式"W15-A feature 整体一次 commit"规则):

- `src-tauri/src/llm_profiles.rs` — 在 T2 文件上扩展

实现内容:

| 接口 | 行为 |
|---|---|
| `ProfileMeta` | 单个 profile 元数据;**不含 API key 字段**(密钥走 keyring) |
| `MetadataFile` | `{ active: Option<String>, profiles: Vec<ProfileMeta> }`;`Default` 返回空配置 |
| `metadata_path()` | Windows `%APPDATA%`,Mac/Linux `dirs::config_dir()`(XDG-aware),fallback cwd |
| `load_profiles()` / `load_profiles_from(&Path)` | 文件缺失返回 `MetadataFile::default()`(不报错);坏 JSON 返回 `"解析 metadata 失败: ..."` |
| `save_profiles()` / `save_profiles_to(&Path)` | 原子写:写 `.json.tmp` → `rename`,失败清理 tmp |
| `get_active_profile()` / `get_active_profile_in(&MetadataFile)` | active 缺失或名字找不到 → `"ACTIVE_PROFILE_REQUIRED: ..."` |
| `to_env_vars(meta, key) -> Result<HashMap, String>` | 未知 provider → `"PROVIDER_NOT_FOUND: ..."`;其余按 provider 派发 |

**env var 派发规则**(spec §5 拍板):

| Provider | 注入 |
|---|---|
| Anthropic | `ANTHROPIC_API_KEY=<key>`;`base_url` 非空且 ≠ `https://api.anthropic.com` 时加 `ANTHROPIC_BASE_URL` |
| OpenAI Compat(OpenAI / LM Studio / DeepSeek / Zhipu / Kimi / MiniMax / Custom) | `OPENAI_API_KEY=<key>`;非空 `OPENAI_BASE_URL` + 非空 `OPENAI_MODEL` |
| Ollama | `OLLAMA_HOST=<base_url>`(总设);非空 `OLLAMA_MODEL`;**不注入任何 key** |

**MiniMax**(用户 2026-07-24 拍板):走 OpenAI Compat 分支,base_url = `https://api.minimaxi.com/v1`,默认 model = `MiniMax-M3`。

**测试性助手**(`*_from(&Path)` / `*_in(&MetadataFile)`):测试用 tmpdir 或构造对象,避免触碰用户真实 APPDATA。这是 reviewer 第 1 项 Important 的修复。

---

## 2. T3 的 TDD 与审核证据

### RED

T3 测试补入后,定向测试编译失败,**28 个 E0425 / E0433 错误**(全部"未找到函数/类型"):
- `ProfileMeta` / `MetadataFile` 未定义
- `metadata_path` / `load_profiles` / `save_profiles_to` / `get_active_profile_in` / `to_env_vars` 未定义

符合 TDD RED 预期。

### GREEN

实现后定向测试:

```
running 35 tests
...
test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 48 filtered out
```

(T2 17 + T3 18 = 35)

### 全量验证

```
cargo test --lib
test result: ok. 83 passed; 0 failed; 0 ignored; 0 measured
```

| 阶段 | 测试数 | 累计 |
|---|---|---|
| Baseline | 43 | 43 |
| T1 (keyring) | 5 | 48 |
| T2 (templates + 校验) | 17 | 65 |
| **T3 (本次)** | **18** | **83** |

### Release build

```
cargo build --release → Finished in 2m 03s,24 warnings
```

24 warnings 全是 `pub` API "never used" — T4 (commands.rs) / T5 (runner.rs) 会消费,与 T1 handoff "list_profile_names unused spec 保留" 模式一致。

### Reviewer 首轮发现

独立 reviewer 给出 4 个 Important(无 Critical):

1. **`load_profiles_returns_empty_when_file_missing` 污染用户 APPDATA** → 改用 `load_profiles_from(&tmp_path("empty"))` 隔离 ✓
2. **Linux 不读 `XDG_CONFIG_HOME`** → 改用 `dirs::config_dir()`(Mac/Linux 共用,XDG-aware)✓
3. **rename 失败时 `.json.tmp` 残留** → 失败时先 `let _ = std::fs::remove_file(&tmp);` 再返回 Err ✓
4. **坏 JSON 路径无测试** → 加 `load_profiles_returns_error_on_corrupt_json` 测试 ✓

修复后再次定向测试 35/35、全量 83/83、release build OK。

### 复审结论

| 维度 | 结论 |
|---|---|
| Spec compliance | 通过(对照 spec §3-§6 §8,7 个必交付接口全覆盖) |
| Code quality | 批准(`_from` / `_in` 变体利于测试、错误前缀一致、安全边界守得住) |
| Critical / Important | **已全部修复,清零** |

---

## 3. 当前工作区状态

```
## feat/w15a-llm-api-settings
 M docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md
 M docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md
 M src-tauri/Cargo.toml                                              ← T1
 M src-tauri/src/lib.rs                                              ← T1 + T2
?? handoff-w15-a-providers-decided-v142-icon-2026-07-24.md          ← 本次未动
?? handoff-w15-a-t2-complete-2026-07-24.md                           ← 本次未动
?? handoff-w15-a-t3-complete-2026-07-24.md                           ← 本文件(新建)
?? handoff-w15-a-v141-release-2026-07-24.md                          ← 旧 handoff,superseded
?? prompt-next-session.md                                            ← 旧 prompt,superseded
?? src-tauri/src/keyring_store.rs                                    ← T1
?? src-tauri/src/llm_profiles.rs                                     ← T2 + T3
```

**禁止事项**:

- 不得 reset / checkout / restore / 覆盖上述未提交改动。
- 不得切回 `master` 直接开发。
- 不得提前 commit T1/T2/T3;继续遵守"W15-A feature 整体一次 commit"。
- 不得删除旧 handoff / prompt(删除需用户二次确认)。

**主仓状态**:`F:/soft/00selfmade/media-to-doc` 未动(仅 pre-existing untracked `docs/media-to-doc.png` + `docs/电商术语表.md`)。

---

## 4. 下一任务:T4

依据:`docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md` Task 4 与 spec §6(6 Tauri commands 契约)。

T4 必交付(在 `src-tauri/src/commands.rs`):

1. **6 个 Tauri command 函数**(`#[tauri::command]`):
   - `list_llm_profiles() -> CommandResponse<Vec<ProfileMeta>>`
   - `get_active_llm_profile_name() -> CommandResponse<String>`
   - `save_llm_profile(args: SaveProfileArgs) -> CommandResponse<ProfileMeta>`
   - `set_active_profile(name: String) -> CommandResponse<()>`
   - `delete_llm_profile(name: String) -> CommandResponse<()>`
   - `test_llm_connection(name: String) -> CommandResponse<TestConnectionResult>`

2. **`SaveProfileArgs` struct**(`#[derive(Deserialize)]`):
   `name` / `provider` / `base_url` / `model` / `note` / `api_key: Option<String>`(None = 不更新 key)/ `tool_search_enabled: Option<bool>` / `experimental_betas_disabled: Option<bool>`

3. **`TestConnectionResult` struct**(`#[derive(Serialize)]`):
   `{ ok: bool, latency_ms: u64, model: String, error: Option<String> }`

4. **错误码 enum**(用字符串前缀保持与 T3 一致):
   - `KEYRING_ERROR`(T1 已用)
   - `PROFILE_NOT_FOUND` / `PROFILE_NAME_CONFLICT`
   - `INVALID_BASE_URL` / `INVALID_MODEL`(T2 已用)
   - `NETWORK_ERROR`(test_connection 失败)
   - `PROVIDER_NOT_FOUND`(T3 已用)
   - `ACTIVE_PROFILE_REQUIRED`(T3 已用)

5. **8 个单元测试**:
   - 6 command happy path(list / get_active / save / set_active / delete / test_connection)
   - 2 error path:`PROFILE_NAME_CONFLICT`(保存时 name 已存在)+ `ACTIVE_PROFILE_REQUIRED`(无 active 时 get_active 报错)

6. **`src-tauri/src/lib.rs`**:
   - `pub use commands::{...6 个新 command + SaveProfileArgs + TestConnectionResult...}`
   - `invoke_handler` 数组追加 6 个 command

**关键实现细节**(避坑):

- **profile JSON 路径隔离**:commands 的 IO 必须走 `load_profiles()` / `save_profiles()`(走 `metadata_path()`),**不要**走 `load_profiles_from(&Path)` / `save_profiles_to(&Path)`(那是测试辅助)。但测试可以用 `serde_json::from_value` 构造 `MetadataFile` + 调用纯函数逻辑,**不**触碰文件系统。
- **keyring 持久化副作用**:`save_llm_profile` 必须同步调 `keyring_store::write_key`;`delete_llm_profile` 必须同步调 `keyring_store::delete_key`(idempotent);`set_active_profile` 只改 metadata JSON,不碰 keyring。
- **`api_key: None` 编辑语义**:用户编辑 profile 但不改 key 时,`api_key = None` → 必须**保留**原 keyring key,**不**写空串覆盖(否则用户重启应用会掉 key)。
- **`test_llm_connection` HTTP**:用 `reqwest::Client`(Cargo.toml 已有 `reqwest` dep + `rustls-tls`),按 provider 构造 URL + headers,测 `model_name` 是否在 `/v1/models` 或 `/models` 列表里。
- **非 Anthropic provider**:`tool_search_enabled` / `experimental_betas_disabled` 字段写入 metadata 但不参与 env 映射(T3 已遵守)。

### TDD 顺序

1. 先写 8 个 T4 测试(spec §6 错误码 + happy path)。
2. 运行定向测试,记录 RED 证据。
3. 最小实现 6 command + SaveProfileArgs + TestConnectionResult + lib.rs 注册。
4. 跑 `commands::` 定向测试到全过。
5. 跑完整 `cargo test --lib`(目标 91+ passed / 0 failed)。
6. 独立 spec + code quality review。

如果 T4 完成且预算允许,继续 T5(runner.rs SpawnSpec.env_vars);否则更新本 handoff 或写后继 handoff。

---

## 5. 项目进度定位

- 当前发布版本:子仓 v1.4.2。
- W15-A 目标版本:v1.5.0。
- W15-A 总任务:T1-T8。
- 已完成:**T1、T2、T3**。
- 下一步:T4(6 commands + 错误码 + 8 tests)。
- 后续:T5 runner env 注入;T6/T7 Settings UI;T8 全量验收 + `cargo tauri build` + v1.5.0 release。
- 用户要求加快:不要再做小版本 release;W15-A 完成后统一 feature commit 和 v1.5.0 release。

---

## 6. 下一会话必读顺序

1. 本文件:`handoff-w15-a-t3-complete-2026-07-24.md`
2. `handoff-w15-a-providers-decided-v142-icon-2026-07-24.md` §0.5、§1(加快模式规则)
3. `handoff-w15-a-t2-complete-2026-07-24.md` §1(T2 9-provider 决策细节)
4. `docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md` §3-§6(9 provider + 6 commands API 契约)、§7-§8(UI + 验收)
5. `docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md` Task 4(命令清单 + 错误码 + 测试列表)
6. `src-tauri/src/commands.rs`(已有结构 — W14-B+ 实装了 list_courses / check_status / list_outputs / read_lecture 等 10 个 command,T4 在此之上加 6 个 LLM 相关 command)
7. `src-tauri/src/keyring_store.rs`(T1,4 函数 + 5 tests)
8. `src-tauri/src/llm_profiles.rs`(T3,本次新加 7 个接口)
9. `src-tauri/src/lib.rs`(invoke_handler 注册位置)
10. `git status --short --branch`

---

## 7. 事实面状态

| 事实面 | 状态 | 证据 |
|---|---|---|
| 代码 | changed-and-verified | T1 + T2 + T3 已实现,T3 18/18,全量 83/83 |
| 运行态 | pending | W15-A 尚未接入 commands / runner / UI |
| 文档 | pending | spec §8 旧 12-provider 测试口径待总同步(不阻塞 T4) |
| 规则 | verified-current | feature 分支、TDD、一次性 commit 规则已遵守 |
| 记忆 | out-of-scope | 本次仅做会话交接,不新增长期记忆 |
| 工作区 | pending | W15-A 未提交改动与历史残留文件均保留,不做破坏性清理 |

---

## 8. 新会话 prompt(可复制)

```
承接 F:/soft/00selfmade/media-to-doc-ui/handoff-w15-a-t3-complete-2026-07-24.md

【加快模式规则 — 必须遵守,来自 handoff-w15-a-providers-decided-v142-icon-2026-07-24.md §1】
1. W15-A 整个 feature 一 commit(feat(ui): W15-A — LLM API Settings panel + 9 providers)
2. 中间不写小 handoff,只在 W15-A 完成时一总 handoff
3. 桌面手动验默认跳过
4. sandbox-verify 默认 fallback(static + cargo test)
5. 已决策的全做,新会话只问未知项
6. spec §8 旧 12-provider 测试口径留 W15-A 总文档同步时改

【当前状态】
- 分支:feat/w15a-llm-api-settings(基线 db84639,不动)
- 已完成:T1(48/48)、T2(65/65)、T3(83/83,本次新加 18 tests)
- 必交付:T1+T2+T3 改动均未 commit,W15-A 整体一次 commit
- 历史 handoff/prompt 文件:不删除,等你二次确认

【本会话第一件事】
读完本 prompt + handoff-w15-a-t3-complete-2026-07-24.md + spec §3-§6 §7-§8 + plan Task 4,
然后用 brainstorming/AskUserQuestion 只问 T4 实装中未知的细节(预计 <3 个问题)。

【执行纪律】
1. 先写 8 个 T4 测试(spec §6 错误码 + happy path)
2. 跑定向测试,记录 RED
3. 最小实现 6 command + SaveProfileArgs + TestConnectionResult + lib.rs invoke_handler
4. 跑 commands:: 定向测试到全过
5. 跑完整 cargo test --lib(目标 91+ passed / 0 failed)
6. 独立 spec + code quality review;Critical / Important 必须修
7. 不要 reset / checkout / restore 未提交改动
8. 不要切回 master 开发

【绝不要做】
- 不要 commit T4 单 commit(等 W15-A 整体 commit)
- 不要 commit / push / release
- 不要删历史 handoff / prompt 文件
- 不要改主仓 media-to-doc
- 不要碰 T1/T2/T3 已交付的代码(keyring_store / llm_profiles / Cargo.toml / lib.rs 已有 mod)
- 不要猜测 MiniMax 模型名(沿用 T3 拍板的 MiniMax-M3 + api.minimaxi.com/v1)

【会话预算】
- <2h 活跃时间,到点 /exit 或新开会话
- 并行 subagent ≤2
- 单回合 diff >500 行拆回合
- bash >100 拆任务
- 撞墙征兆出现立即写 handoff
```

---

## 9. 历史 superseded 文件(建议保留,等你二次确认是否清理)

按全局 CLAUDE.md "删除文件前先二次确认",本会话**不**自动删除:

| 文件 | 状态 |
|---|---|
| `handoff-w15-a-v141-release-2026-07-24.md` | 已被 `handoff-w15-a-providers-decided-v142-icon-2026-07-24.md` supersede |
| `prompt-next-session.md` | 已被本文件 supersede |
| `handoff-w15-a-providers-decided-v142-icon-2026-07-24.md` | 保留(记录加快模式规则 + 9-provider 决策细节) |
| `handoff-w15-a-t2-complete-2026-07-24.md` | 保留(记录 T2 实现细节) |
| `handoff-w15-a-t3-complete-2026-07-24.md` | 本文件,保留 |