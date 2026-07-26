# 新会话 prompt — 继续 W15-A T3

> 复制本文件全文，粘贴到新会话。

承接：`F:/soft/00selfmade/media-to-doc-ui/handoff-w15-a-t2-complete-2026-07-24.md`

请继续 `media-to-doc-ui` 的 W15-A，不要重新设计，不要回滚现有未提交改动。

## 第一条回复的强制要求

在运行工具、修改代码或讲技术实现之前，先用**非技术语言**告诉我：

1. 本会话准备执行哪些任务；
2. 每项任务完成后，普通用户能获得什么功能；
3. 本会话预计验证哪些结果。

请避免在这段开场说明里直接使用 `struct`、`enum`、JSON schema、env var、atomic write 等术语。可以按下面意思表达：

- 保存多个 AI 服务商配置，关闭应用再打开也不会丢；
- 记住当前启用的服务商；
- 运行课程处理时自动准备当前服务商需要的连接信息；
- 密钥继续由系统安全存储，不进入普通配置文件；
- 用自动测试证明保存、读取、切换和运行参数生成正确。

完成这段非技术说明后，直接开始执行，不要再次询问已经在 handoff/spec/plan 中拍板的事项。

## 当前状态

- 项目：`F:/soft/00selfmade/media-to-doc-ui`
- 分支：`feat/w15a-llm-api-settings`
- 基线 commit：`db84639`
- T1 已完成：OS keyring 存取，5 tests。
- T2 已完成：9 个 Provider 模板、名称映射、URL/model 校验，17 tests。
- 当前全量测试：`65 passed / 0 failed`。
- T1/T2、spec/plan 和 handoff 均未 commit；必须保留。
- W15-A 要整体一次 commit，不要为 T3 单独 commit。

## 必读文件

按顺序完整阅读：

1. `F:/soft/00selfmade/media-to-doc-ui/handoff-w15-a-t2-complete-2026-07-24.md`
2. `F:/soft/00selfmade/media-to-doc-ui/handoff-w15-a-providers-decided-v142-icon-2026-07-24.md` §0.5、§1、§4
3. `F:/soft/00selfmade/media-to-doc-ui/docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md` §3–§6、§8
4. `F:/soft/00selfmade/media-to-doc-ui/docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md` Task 3
5. `src-tauri/src/llm_profiles.rs`
6. `src-tauri/src/keyring_store.rs`
7. `src-tauri/src/lib.rs`
8. `src-tauri/Cargo.toml`
9. 当前 `git status --short --branch`

## 本会话主任务

直接执行 T3：在现有 `src-tauri/src/llm_profiles.rs` 上增加 profile metadata 持久化和运行参数映射。

应完成：

- `ProfileMeta`
- `MetadataFile`
- `metadata_path()`
- `load_profiles()`
- `save_profiles()`
- `get_active_profile()`
- `to_env_vars()`

必须遵守：

- 当前只有 9 个 Provider。plan 中的 12-provider 代码片段已经过时。
- MiniMax 固定使用 `https://api.minimaxi.com/v1` + `MiniMax-M3`。
- API key 不得写进 metadata JSON、日志或测试快照。
- 文件不存在时返回空配置。
- active 缺失时返回 `ACTIVE_PROFILE_REQUIRED:`。
- 未知 provider 返回 `PROVIDER_NOT_FOUND:`。
- Anthropic、OpenAI-compatible、Ollama 按 handoff 中的规则生成各自运行参数。
- Ollama 不得注入 API key。
- 保存时避免产生半写配置文件。
- 不要修改主仓 `media-to-doc`。

## 执行方式

严格 TDD：

1. 先写 T3 测试。
2. 运行定向测试，记录预期 RED 证据。
3. 写最小实现。
4. 运行 `llm_profiles::` 定向测试。
5. 运行完整 `cargo test --lib`。
6. 做独立 spec compliance + code quality review。
7. reviewer 发现 Critical / Important 时修复并复审。

如果 T3 完成后会话预算充足，继续 T4；否则撰写下一份 handoff。不要提交、push、release 或删除历史残留文件，除非我明确要求。

## 会话预算

- 活跃时间小于 2 小时。
- 并行 subagent 不超过 2 个。
- 不要重复读取同一大文件。
- 连续工具失败或上下文接近上限时，立即写 handoff，不要强撑。
