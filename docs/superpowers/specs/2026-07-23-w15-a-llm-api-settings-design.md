# W15-A — LLM API 设置(Desktop Settings > Providers)

**日期**:2026-07-23
**承接**:`handoff-w14g-e-msi-2026-07-23.md` §10(W14-G+ 收尾后)
**会话**:W15 第一子项目
**承接 handoff 一句话**:W14-G+ 收尾(NSIS D 盘默认 + 43/43 测试过),W15 是大工程第一子项目
**用户决策**(brainstorming 已拍板):
- 顺序:A → B → C
- Key 存储:系统 keyring
- 服务商:12 个内置 + Custom(全采纳)
- Profile:多存档 + 1 active
- Key 注入:env var 注入到 mtd 子进程
- UI 位置:新加 Settings tab
- 架构:方法 3(Rust keyring + 4 Tauri commands 抽象)

---

## 1. 目标

为 media-to-doc 桌面端添加 LLM API 设置面板,用户:
1. 在 Settings > Providers 子页管理多个 LLM profile(添加 / 编辑 / 删除 / 切换 active)
2. 选择 12 个内置服务商 + Custom,选完自动填 base_url + model 默认值
3. API key 加密存在 OS keyring,重启不丢
4. mtd run 子进程启动时,Tauri 自动从 keyring 读 active profile 的 key,env var 注入

**业务动机**:当前 mtd pipeline 跑通 LLM 需用户手工写 `~/.media-to-doc/config.yaml` + 设环境变量。W15-A 把这步零摩擦化,后续 B(会话 UI)用同一 active profile 做实时对话。

---

## 2. 架构(方法 3 — Rust keyring + Tauri commands 抽象)

```
┌──────────────────────────────────────────────────────────┐
│ Tauri Rust 后端                                           │
│ ┌────────────────┐  ┌────────────────┐                  │
│ │ keyring_store  │  │ llm_profiles   │                  │
│ │  ┌──────────┐  │  │ 12 服务商模板   │                  │
│ │  │keyring   │  │  │ Custom 校验    │                  │
│ │  │crate     │  │  │ env var 映射   │                  │
│ │  └──────────┘  │  └────────────────┘                  │
│ │   read/write    │         │                            │
│ │   OS keyring    │         │                            │
│ └────────────────┘         │                            │
│        │                    │                            │
│        └────────┬───────────┘                            │
│                 ▼                                        │
│  ┌──────────────────────────────────────────┐           │
│  │ commands.rs (4 + 1 个 Tauri command)     │           │
│  │  list_llm_profiles()                     │           │
│  │  get_active_llm_profile()                │           │
│  │  save_llm_profile(name, provider, ...)   │           │
│  │  set_active_profile(name)                │           │
│  │  delete_llm_profile(name)                │           │
│  │  test_llm_connection(profile)            │           │
│  └──────────────────────────────────────────┘           │
│                 │                                        │
│  ┌──────────────▼──────────────────────────┐             │
│  │ run_pipeline / resume_pipeline 修改     │             │
│  │ spawn mtd 前:读 active profile → env    │             │
│  │ 注入 ANTHROPIC_API_KEY / OPENAI_API_KEY │             │
│  │ / OLLAMA_HOST 等                        │             │
│  └─────────────────────────────────────────┘             │
└──────────────────────────────────────────────────────────┘
                 │
                 │ invoke('cmd', args)
                 ▼
┌──────────────────────────────────────────────────────────┐
│ Tauri 前端 (src/index.html)                              │
│  + Settings tab(6 tab 化)                                │
│  + Settings > Providers 子页:                            │
│    - profile 列表(可滚动)                                 │
│    - active 标星                                          │
│    - [+ 添加服务商] → modal:                            │
│        预设下拉(12 + Custom)                              │
│        → 选中自动填 base_url + model 默认值              │
│        名称 / 备注 / 接口地址 / API 密钥 / Tool Search  │
│        [测试连接] + [保存]                                │
│    - profile 行:[激活] [编辑] [删除]                       │
└──────────────────────────────────────────────────────────┘
```

---

## 3. 12 个内置服务商模板

| # | 名称 | base_url | 默认 model | 协议 | 认证变量 | env var |
|---|---|---|---|---|---|---|
| 1 | Anthropic | https://api.anthropic.com | claude-sonnet-4-5 | Anthropic SDK | `ANTHROPIC_AUTH_TOKEN` | `ANTHROPIC_API_KEY` + `ANTHROPIC_BASE_URL` |
| 2 | OpenAI | https://api.openai.com/v1 | gpt-4o | OpenAI Compat | Bearer Token | `OPENAI_API_KEY` + `OPENAI_BASE_URL` + `OPENAI_MODEL`(可选) |
| 3 | Ollama | http://localhost:11434 | llama3.1 | Ollama native | (no auth) | `OLLAMA_HOST` + `OLLAMA_MODEL`(无 key) |
| 4 | LM Studio | http://localhost:1234/v1 | loaded-model | OpenAI Compat | Bearer Token | `OPENAI_API_KEY` + `OPENAI_BASE_URL` + `OPENAI_MODEL` |
| 5 | DeepSeek | https://api.deepseek.com | deepseek-chat | OpenAI Compat | Bearer Token | `OPENAI_API_KEY` + `OPENAI_BASE_URL` + `OPENAI_MODEL`(可选) |
| 6 | Zhipu GLM | https://open.bigmodel.cn/api/paas/v4 | glm-4-plus | OpenAI Compat | Bearer Token | `OPENAI_API_KEY` + `OPENAI_BASE_URL` + `OPENAI_MODEL`(可选) |
| 7 | Kimi | https://api.moonshot.cn/v1 | moonshot-v1-128k | OpenAI Compat | Bearer Token | `OPENAI_API_KEY` + `OPENAI_BASE_URL` + `OPENAI_MODEL`(可选) |
| 8 ⚠️ | MiniMax | https://api.MiniMax.chat/v1 ⚠️ | MiniMax-Text-01 ⚠️ | OpenAI Compat | Bearer Token | `OPENAI_API_KEY` + `OPENAI_BASE_URL` + `OPENAI_MODEL`(可选) |
| 9 ⚠️ | 接口 AI | https://api.api2d.net/v1 ⚠️ | gpt-4o-mini ⚠️ | OpenAI Compat | Bearer Token | `OPENAI_API_KEY` + `OPENAI_BASE_URL` + `OPENAI_MODEL`(可选) |
| 10 ⚠️ | 胜算云 | https://api.shengsuanyun.com/v1 ⚠️ | gpt-4o-mini ⚠️ | OpenAI Compat | Bearer Token | `OPENAI_API_KEY` + `OPENAI_BASE_URL` + `OPENAI_MODEL`(可选) |
| 11 ⚠️ | TeamoRouter | https://api.teamorouter.com/v1 ⚠️ | claude-3-5-sonnet ⚠️ | OpenAI Compat | Bearer Token | `OPENAI_API_KEY` + `OPENAI_BASE_URL` + `OPENAI_MODEL`(可选) |
| 12 | Custom | (空,用户填) | (空,用户填) | OpenAI Compat | Bearer Token | `OPENAI_API_KEY` + `OPENAI_BASE_URL` + `OPENAI_MODEL`(可选) |

⚠️ **占位标记**:8/9/10/11 四行 base_url + model 是 brainstorming 阶段填的占位值,实装前用户须核实 / 修正。Anthropic / OpenAI / Ollama / DeepSeek / Zhipu / Kimi 公开 API 真实。

**校验**(Custom + 用户编辑时):
- base_url:`http://localhost:*` / `http://127.0.0.1:*` / `https://*`(防 SSRF,本地地址不限端口,IPv4 loopback 支持 IP 字面量)
- model:非空字符串,长度 ≤ 200

---

## 4. Keyring 存储结构

**OS keyring**:
- service:`media-to-doc-ui`
- username:`profile:<name>`(每个 profile 1 个 key)
- password:用户填的 API key(明文存 OS keyring,keyring 自身加密)

**Metadata JSON**(本机 `%APPDATA%/com.duanyi.mediatodoc/llm_profiles.json`):
```json
{
  "active": "deepseek-prod",
  "profiles": [
    {
      "name": "deepseek-prod",
      "provider": "DeepSeek",
      "base_url": "https://api.deepseek.com",
      "model": "deepseek-chat",
      "note": "主力 / 个人 key",
      "tool_search_enabled": false,
      "experimental_betas_disabled": false,
      "created_at": "2026-07-23T15:00:00Z"
    },
    { "name": "ollama-local", ... }
  ]
}
```

**为什么 metadata 不在 keyring**:keyring 适合存单个 secret;profile 列表元数据存 JSON 文件更易 list/edit/delete。

---

## 5. env var 注入(关键路径)

**触发点**:`run_pipeline` / `resume_pipeline` Tauri command 在调 `spawn_mtd` 前。

**逻辑**(三层协作):
```rust
// 1. llm_profiles + keyring_store:读 active profile + key,生成 env_vars
async fn run_pipeline(inbox: String, opts: ...) -> CommandResponse<...> {
    let active = llm_profiles::get_active_profile()?;        // metadata JSON
    let key = keyring_store::read_key(&active.name)?;        // OS keyring
    let env_vars = llm_profiles::to_env_vars(&active, &key)?; // 模板映射

    // 2. runner.rs:env_vars 透传到 SpawnSpec,spawn_mtd 内 .env_clear().envs(env_vars)
    let spec = build_mtd_run_args(...);
    let spec_with_env = SpawnSpec { env_vars, ..spec };
    let child = spawn_mtd(&spec_with_env).await?;            // .env_clear() + .envs(env_vars)

    // 3. RunRegistry: 注册 + 后台监控(沿用 W14-C 多课程并发)
    registry.insert(work_dir, child, inbox, log_path).await?;
    Ok(work_dir)
}
```

**SpawnSpec 扩展**(`runner.rs` 改动,与 §9 一致):
```rust
pub struct SpawnSpec {
    pub program: String,
    pub args: Vec<String>,
    pub work_dir: String,
    pub log_path: String,
    pub env_vars: HashMap<String, String>,  // NEW: env vars to inject to child
}

pub async fn spawn_mtd(spec: &SpawnSpec) -> Result<Child, String> {
    ...
    let mut cmd = Command::new(&spec.program);
    cmd.args(&spec.args)
        .current_dir(&spec.work_dir)
        .env_clear()                                  // 清父进程 env(W14-D trust_env=False 思路,防 HTTP_PROXY 污染)
        .envs(&spec.env_vars)                         // 注入 active profile env vars
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(err_log))
        .kill_on_drop(true);
    cmd.spawn().map_err(...)
}
```

**env var 名映射**(沿用 W14-D 全 provider trust_env=False 路径):
- Anthropic: `ANTHROPIC_API_KEY` + `ANTHROPIC_BASE_URL`(可选)
- OpenAI / OpenAI Compat: `OPENAI_API_KEY` + `OPENAI_BASE_URL`(可选) + `OPENAI_MODEL`(可选)
- LM Studio:同上,`OPENAI_MODEL` 必填(因 LM Studio 不暴露模型 list)
- Ollama: `OLLAMA_HOST` + `OLLAMA_MODEL`(无 key)

**安全**:
- env 注入只对 mtd 子进程生效。父 Tauri 进程 spawn 前瞬时持有 key 用于 env 注入(read_keyring → to_env_vars → spawn ≤ 100ms 窗口),**不在 log / UI / registry / 持久化文件中留痕**
- keyring 由 OS 加密(Win DPAPI / Mac Keychain / Linux Secret Service),明文不出 OS 边界
- spawn_mtd 用 `env_clear()` 清父进程环境再 `envs(env_vars)`,避免父进程 HTTP_PROXY 等污染子进程(W14-D trust_env=False 思路)

---

## 6. 6 个 Tauri commands(API 契约)

```rust
// 1. 列出所有 profile
pub async fn list_llm_profiles() -> CommandResponse<Vec<ProfileMeta>>;

// 2. 读 active profile + 名字
pub async fn get_active_llm_profile_name() -> CommandResponse<String>;

// 3. 保存(新建或更新)profile
pub struct SaveProfileArgs {
  pub name: String,
  pub provider: String,
  pub base_url: String,
  pub model: String,
  pub note: Option<String>,
  pub api_key: Option<String>,  // None = 不更新 key(编辑时)
  pub tool_search_enabled: Option<bool>,
  pub experimental_betas_disabled: Option<bool>,
}
pub async fn save_llm_profile(args: SaveProfileArgs) -> CommandResponse<ProfileMeta>;

// 4. 切换 active
pub async fn set_active_profile(name: String) -> CommandResponse<()>;

// 5. 删除
pub async fn delete_llm_profile(name: String) -> CommandResponse<()>;

// 6. 测试连接(健康度探测)
pub async fn test_llm_connection(name: String) -> CommandResponse<TestConnectionResult>;
// 返回 { ok: bool, latency_ms: u64, model: String, error: Option<String> }
```

**错误码**:
- `KEYRING_ERROR`:`keyring` crate 调用失败
- `PROFILE_NOT_FOUND`:profile 名不存在
- `PROFILE_NAME_CONFLICT`:保存时 name 冲突(强制 unique)
- `INVALID_BASE_URL`:Custom 或编辑时 URL 非法
- `INVALID_MODEL`:model 字段非法
- `NETWORK_ERROR`:test_connection 失败
- `PROVIDER_NOT_FOUND`:provider 名不在 12 个内置清单
- `ACTIVE_PROFILE_REQUIRED`:run_pipeline 时无 active profile

---

## 7. 前端 UI(附图 2 风格 + media-to-doc 风格)

### 7.1 Settings tab(新增 6th tab)

**侧边栏**(从 5 tab 变 6 tab):
```
Inbox
Run
Output
Health
Learn
Settings  ← NEW
```

**Settings 子菜单**(左侧抽屉 / tab 内子导航):
```
- Providers(LLM API 配置)
- General(预留 C)
- Theme(预留 C)
- About
```

W15-A 实装 Providers,其它子菜单显示但内容"Coming soon"(预留 B/C)。

### 7.2 Providers 子页

**Layout**:
```
┌──────────────────────────────────────────────────┐
│ Providers  [+ 添加服务商]              [刷新]    │
├──────────────────────────────────────────────────┤
│ ★ DeepSeek (active)                              │
│   deepseek-prod · 主力 / 个人 key                │
│   [激活]  [编辑]  [删除]                          │
│                                                  │
│ ★ Ollama Local                                   │
│   ollama-local · 本地开发                        │
│   [激活]  [编辑]  [删除]                          │
│                                                  │
│   Anthropic                                      │
│   anthropic-prod · 备用                          │
│   [激活]  [编辑]  [删除]                          │
└──────────────────────────────────────────────────┘
```

### 7.3 添加 modal(附图 2 风格)

```
┌── 添加服务商 ─────────────────────────────────┐
│  预设 *                                          │
│  [DeepSeek] [Zhipu GLM] [Kimi] [MiniMax]         │
│  [LM Studio] [Ollama] [Anthropic] [OpenAI]      │
│  [接口 AI] [胜算云] [TeamoRouter] [Custom]      │
│                                                  │
│  名称 *                                          │
│  [deepseek-prod________]                          │
│                                                  │
│  备注                                            │
│  [主力 / 个人 key____]                            │
│                                                  │
│  接口地址 *                                      │
│  [https://api.deepseek.com______]                 │
│                                                  │
│  认证变量                                        │
│  [Bearer Token (ANTHROPIC_AUTH_TOKEN) ▼]         │
│                                                  │
│  [✓] 启用 Tool Search(Anthropic only)             │
│  [ ] 关闭实验性 Beta 头                           │
│                                                  │
│  API 密钥 *                                      │
│  [sk-••••••••] 👁                                  │
│  🔑 获取 API Key ↗                                │
│                                                  │
│  模型                                            │
│  [deepseek-chat________]                          │
│                                                  │
│  [测试连接]  [取消]  [添加]                       │
└──────────────────────────────────────────────────┘
```

**交互**:
- 选预设 → base_url + 默认 model + 认证变量 自动填
- 编辑 base_url 改 Custom 时不阻塞
- API 密钥显示密码框(可切可见)
- [测试连接] 用当前表单值探测(无需先保存)
- [保存] 调 save_llm_profile
- 关闭 modal → 刷新列表

**Tool Search / Experimental Beta toggle 显示规则**:
- `tool_search_enabled` 与 `experimental_betas_disabled` 是 **Anthropic 专属** 字段
- 规则:当且仅当 `provider == "Anthropic"` 时,modal 显示这两个 checkbox;其他 provider 隐藏(not disabled,直接不渲染)
- 后端 `save_llm_profile`:非 Anthropic provider 收到 `tool_search_enabled=true` 时静默忽略(写 log,不报错),保持向后兼容

---

## 8. 验收清单

| # | 验证项 | 期望 |
|---|---|---|
| 1 | 装 W15-A NSIS + 桌面启动 | 6 tab 显示(Inbox/Run/Output/Health/Learn/Settings) |
| 2 | 进 Settings > Providers | 看到空列表 + [+ 添加服务商] 按钮 |
| 3 | 添加 DeepSeek profile | 选预设 → 自动填 base_url + model → 填 API key → [测试连接] → 绿色"连接成功 <X>ms" → [保存] |
| 4 | 列表显示新 profile,无 active 标 | 列表项,无星 |
| 5 | 点 [激活] | 该 profile 标星,其它取消标星,active 状态写入 JSON |
| 6 | 重启 Tauri | active profile 持久化(从 JSON 读) |
| 7 | Run pipeline(stop_after=chapters)| mtd 启动时 env 注入 `OPENAI_API_KEY=sk-...` + `OPENAI_BASE_URL=https://api.deepseek.com`,pipeline 跑通(LLM 章节) |
| 8 | 添加 Ollama profile + 激活 | mtd 启动时 env 注入 `OLLAMA_HOST=http://localhost:11434` + `OLLAMA_MODEL=llama3.1`,无 key 注入 |
| 9 | 删除 active profile | 弹确认 → 删除后无 active,Run pipeline 报 `ACTIVE_PROFILE_REQUIRED` 错误 |
| 10 | Test connection 失败 | 红色错误(网络 / key 错),profile 仍可保存(供离线用) |
| 11 | 编辑 profile 不改 key | api_key=None → 保留 keyring 旧值 |
| 12 | 12 个预设全部可选 | 每个预设 → 正确的 base_url + 默认 model |
| 13 | Custom 服务商 | base_url 校验(只允许 https:// 或 http://localhost:*)|

**单元测试**(目标 +30 cases,共 73+,**全部走 `#[cfg(test)] mod tests` 在源文件内**,沿用现有 43 测试模式,不新建 `src-tauri/tests/` 目录):
- `src-tauri/src/keyring_store.rs`:5 tests(read/write/delete happy + read nonexistent 报错 + list_username)
- `src-tauri/src/llm_profiles.rs`:17 tests(12 模板字段正确 + 5 env var 映射 + base_url 校验 / model 校验)
- `src-tauri/src/commands.rs`:8 tests(6 commands happy path + 2 error path:PROFILE_NAME_CONFLICT + ACTIVE_PROFILE_REQUIRED)
- **不引入** `src-tauri/tests/`(集成测试)— keyring crate 跨进程隔离麻烦,unit test 已覆盖核心逻辑

**集成测试**(手动,1 session):
- 13 步验收清单

---

## 9. 改动清单

| 文件 | 改动 |
|---|---|
| `src-tauri/Cargo.toml` | + `keyring = "3"` |
| `src-tauri/src/keyring_store.rs` | NEW — 4 函数:read / write / delete / list_username |
| `src-tauri/src/llm_profiles.rs` | NEW — 12 模板 + env var 映射 + base_url 校验 + JSON metadata IO |
| `src-tauri/src/commands.rs` | + 6 Tauri commands + 1 错误码 enum + run_pipeline / resume_pipeline 改读 active profile + 调 llm_profiles::to_env_vars |
| `src-tauri/src/lib.rs` | invoke_handler + 6 个 command |
| `src-tauri/src/runner.rs` | SpawnSpec 加 `env_vars: HashMap<String, String>` 字段;spawn_mtd 加 `.env_clear().envs(&spec.env_vars)` |
| `src/index.html` | + Settings tab + Settings > Providers 子页 + 添加 modal + 列表渲染 + 调用 6 commands |
| `src-tauri/capabilities/default.json` | 不改(IPC 不需新权限) |
| `src-tauri/src/keyring_store.rs`(内嵌) | + 5 unit tests |
| `src-tauri/src/llm_profiles.rs`(内嵌) | + 17 unit tests |
| `src-tauri/src/commands.rs`(内嵌) | + 8 unit tests |
| `docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md` | 本 spec |
| `docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md` | 实施 plan(writing-plans skill 写) |
| `handoff-w15-a-llm-api-settings-2026-07-XX.md` | handoff |

**不改**:
- ❌ 主仓 `media-to-doc/`(mtd 端零改动 — env var 注入沿用 W14-D trust_env=False 路径)
- ❌ `tauri.conf.json`(无新 bundle / icon / window 配置)
- ❌ `src-tauri/nsis/installer.nsi`(W14-G+ 收尾状态)

---

## 10. 风险 / 边界

### 10.1 风险

| 风险 | 缓解 |
|---|---|
| keyring crate 跨平台兼容 | 选 v3(主流支持 Win/Mac/Linux);Win 走 Windows Credential Manager(WDPAPI,按用户存储,无需 admin);Mac 走 Keychain;Linux 需 gnome-keyring / kwallet 或 secret-service daemon |
| Custom URL SSRF | 校验只允许 `https://` + `http://localhost:*` |
| 12 个服务商 model 默认值过期 | spec 标注"每版本手工更新",W15-A 启动后用 mtd 主仓 LLMConfig 同步 |
| **12 个服务商 base_url 是占位** | **brainstorming 阶段填的占位值(MiniMax/接口AI/胜算云/TeamoRouter 等),W15-A 实装时需用户核实 / 修正。Anthropic / OpenAI / Ollama / DeepSeek / Zhipu / Kimi 公开 API 是真实的,其它不一定** |
| keyring 读失败的 fallback | 报清晰错误,不静默降级到环境变量 |
| 切换 active 后已跑 run 不受影响 | 只影响新 spawn 的 mtd;已跑 run 用旧 env(进程独立) |

### 10.2 边界(不做)

- ❌ 不做"自动 key 轮换"(用户自己管理)
- ❌ 不做"用量统计"(留 W15-B 或后续)
- ❌ 不做"团队共享 profile"(留未来)
- ❌ 不做"profile 导入导出"(留未来)
- ❌ 不改主仓 mtd(env var 注入沿用)

---

## 11. 后续(W15-B / W15-C)

**W15-B(会话 UI)**:
- 用 active profile 做 LLM 实时对话
- 项目侧栏(按 inbox 目录分组)
- 多会话管理(列表 + 切换 + 新建 + 删除)
- 右下角浮动输入框(附图 1)
- Markdown 渲染 + 流式输出
- Token 用量统计
- 依赖:W15-A 完成

**W15-C(UI 强化)**:
- 主题(浅色 / 深色 / 跟随系统)
- 快捷键(发送 / 切换 tab / 搜索)
- 拖拽(文件 / 项目重排)
- 多语言(中 / 英)
- 动效(过渡 / 加载)
- 独立,可与 W15-B 并行

---

## 12. 估算

- 实装:1 个 session(~2h)
- 单元测试 + 集成验收:1 个 session(~1.5h)
- sandbox-verify 桌面手动验收:用户桌面(~30min)
- 跨 session:~2-3h,等用户桌面验收 + reviewer pass

---

## 13. 下次会话第一句

> 承接 `docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md`,W15-A LLM API 设置 spec 已写,等用户 review。review pass 后调 writing-plans skill 写实施 plan。