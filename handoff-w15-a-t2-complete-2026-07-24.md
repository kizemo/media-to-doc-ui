# Handoff — W15-A T1+T2 完成，下一会话进入 T3

**日期**：2026-07-24  
**项目**：`F:/soft/00selfmade/media-to-doc-ui`  
**当前分支**：`feat/w15a-llm-api-settings`  
**基线 commit**：`db84639 build(ui): v1.4.2 — replace icon set with project logo`

---

## 0. 下一会话先做什么（非技术说明）

开始执行前，必须先用普通用户能理解的语言告知以下任务清单，不要直接讲 Rust struct、JSON schema 或环境变量：

1. **保存服务商配置**：让用户添加的 AI 服务商名称、接口地址、模型和备注在关闭应用后仍然保留。
2. **记住当前使用项**：允许多个配置共存，并记住当前启用的是哪一个。
3. **安全准备运行参数**：运行课程处理时，根据当前服务商自动准备所需连接信息；密钥仍由系统安全存储，不写进普通配置文件。
4. **防止配置损坏**：配置不存在时安全返回空列表，配置写入时避免留下半写文件，并对无效服务商给出清楚错误。
5. **用测试证明行为**：先写会失败的测试，再实现功能，最后确认本模块和全部已有测试通过。

面向用户的最终效果：用户以后只需在桌面端选一次 AI 服务商并保存，应用重启后仍能记住，后续运行课程处理时可以自动使用当前配置。

---

## 1. 本会话完成情况

### T1：系统密钥存储

已完成但未 commit：

- `src-tauri/src/keyring_store.rs`
  - `read_key`
  - `write_key`
  - `delete_key`
  - `list_profile_names`
  - 5 个单元测试
- `src-tauri/Cargo.toml`
  - `keyring = { version = "4", features = ["v1"] }`
  - `dirs = "5"`
- `src-tauri/src/lib.rs`
  - `mod keyring_store;`

关键事实：`keyring` v3 在 Windows 同进程多 `Entry::new` 场景存在读写 race；已改用 v4 + `v1` feature。T1 基线验证为 **48 passed / 0 failed**。

### T2：9 个服务商模板与输入校验

已完成但未 commit：

- 新建 `src-tauri/src/llm_profiles.rs`
- `src-tauri/src/lib.rs` 增加 `mod llm_profiles;`

实现内容：

- `Provider` enum，恰好 9 个：
  - Anthropic
  - OpenAI
  - Ollama
  - LM Studio
  - DeepSeek
  - Zhipu GLM
  - Kimi
  - MiniMax
  - Custom
- `Protocol` enum：`AnthropicSdk` / `OpenAiCompat` / `OllamaNative`
- `ProviderTemplate`
- `all_templates()` 9 套模板
- `provider_from_name()` / `provider_name()` 双向映射
- `validate_base_url()`
- `validate_model()`

MiniMax 已按用户拍板值实现：

- base URL：`https://api.minimaxi.com/v1`
- 默认 model：`MiniMax-M3`

明确删除旧 plan 中三个占位服务商：`ApitwoD`、`Shengsuanyun`、`TeamoRouter`。

---

## 2. T2 的 TDD 与审核证据

### RED 1

先建立 17 个测试和接口 stub：

- `running 17 tests`
- `1 passed / 16 failed`
- 失败原因是模板、名称映射和校验行为尚未实现，不是编译错误。

### Reviewer 首轮发现

Reviewer 发现 HTTPS 校验只检查 `https://` 前缀，会错误接受：

- `https://?query`
- `https://#fragment`
- `https:///path`

### RED 2

将上述回归场景加入原有测试，保持测试总数恰好 17：

- `16 passed / 1 failed`
- 失败来自 malformed HTTPS 被错误接受。

### GREEN

修复 authority 校验后：

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib llm_profiles::
17 passed / 0 failed

cargo test --manifest-path src-tauri/Cargo.toml --lib
65 passed / 0 failed

git diff --check
通过；只有既有 LF→CRLF warning
```

独立复审结论：

- Spec compliance：**通过**
- Code quality：**批准**
- Critical / Important：**无**

已知 Minor：spec §8 仍保留旧的“12 模板 + env mapping”测试口径，应在 W15-A 总文档同步时改成当前 9-provider 口径，不阻塞 T3。

---

## 3. 当前工作区状态

当前所有 W15-A 改动都未 commit，符合用户拍板的“整个 W15-A feature 一次性 commit”规则。

```text
## feat/w15a-llm-api-settings
 M docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md
 M docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md
 M src-tauri/Cargo.toml
 M src-tauri/src/lib.rs
?? handoff-w15-a-providers-decided-v142-icon-2026-07-24.md
?? handoff-w15-a-v141-release-2026-07-24.md
?? prompt-next-session.md
?? src-tauri/src/keyring_store.rs
?? src-tauri/src/llm_profiles.rs
```

**禁止事项**：

- 不得 reset、checkout、restore 或覆盖上述未提交改动。
- 不得切回 `master` 直接开发。
- 不得提前 commit T1/T2；继续遵守 W15-A 整体一次 commit。
- 不得删除旧 handoff / prompt；删除需要用户二次确认。

---

## 4. 下一任务：T3

依据：`docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md` 的 Task 3，但任何 12-provider 示例都已过时，必须以当前 `llm_profiles.rs` 的 9-provider 实现为准。

T3 预计只扩展：

- `src-tauri/src/llm_profiles.rs`

目标接口：

- `ProfileMeta`
- `MetadataFile`
- `metadata_path()`
- `load_profiles()`
- `save_profiles()`
- `get_active_profile()`
- `to_env_vars()`

关键行为：

1. metadata 路径：`%APPDATA%/com.duanyi.mediatodoc/llm_profiles.json`，其他平台使用对应 config 目录。
2. 文件不存在时返回空 metadata，不报错。
3. 保存应创建父目录并使用安全写入方式，避免半写文件。
4. active 为空或找不到时返回 `ACTIVE_PROFILE_REQUIRED:`。
5. Provider 名称无法映射时返回 `PROVIDER_NOT_FOUND:`。
6. Anthropic 注入 `ANTHROPIC_API_KEY`，自定义 endpoint 时加 `ANTHROPIC_BASE_URL`。
7. OpenAI-compatible 注入 `OPENAI_API_KEY`、非空 `OPENAI_BASE_URL`、非空 `OPENAI_MODEL`。
8. Ollama 只注入 `OLLAMA_HOST` 和非空 `OLLAMA_MODEL`，不得注入 API key。
9. API key 绝不能写入 metadata JSON、日志或测试快照。
10. 非 Anthropic profile 的两个 Anthropic 专属开关应忽略，不报错。

### TDD 顺序

1. 先补 T3 测试。
2. 运行定向测试，确认因缺少 T3 行为而失败。
3. 最小实现 T3。
4. 运行 `llm_profiles::` 定向测试。
5. 运行完整 `cargo test --lib`。
6. 做独立 spec + code quality review。

如果 T3 完成且预算允许，再进入 T4；否则更新本 handoff 或写后继 handoff。

---

## 5. 项目进度定位

- 当前发布版本：子仓 v1.4.2。
- W15-A 目标版本：v1.5.0。
- W15-A 总任务：T1–T8。
- 已完成：T1、T2。
- 下一步：T3 metadata IO + env var mapping。
- 后续：T4 六个 Tauri commands；T5 runner env 注入；T6/T7 Settings UI；T8 全量测试、构建和验收。
- 用户要求加快：不要再做小版本 release；W15-A 完成后统一 feature commit 和 v1.5.0 release。

---

## 6. 下一会话必读顺序

1. 本文件：`handoff-w15-a-t2-complete-2026-07-24.md`
2. `handoff-w15-a-providers-decided-v142-icon-2026-07-24.md` §0.5、§1、§4
3. `docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md` §3–§6、§8
4. `docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md` Task 3
5. `src-tauri/src/llm_profiles.rs`
6. `src-tauri/src/keyring_store.rs`
7. `src-tauri/src/lib.rs`
8. `src-tauri/Cargo.toml`
9. `git status --short --branch`

---

## 7. 事实面状态

| 事实面 | 状态 | 证据 |
|---|---|---|
| 代码 | changed-and-verified | T1 + T2 已实现，T2 17/17，全量 65/65 |
| 运行态 | pending | W15-A 尚未接入 commands / runner / UI |
| 文档 | pending | spec §8 旧 12-provider 测试口径待总同步 |
| 规则 | verified-current | feature 分支、TDD、一次性 commit 规则已遵守 |
| 记忆 | out-of-scope | 本次仅做会话交接，不新增长期记忆 |
| 工作区 | pending | W15-A 未提交改动与历史残留文件均保留，不做破坏性清理 |
