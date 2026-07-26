# Handoff — W15-A 加快模式启动(v1.4.2 图标后,MiniMax-M3 已拍板)

**日期**:2026-07-24
**承接 handoff**:`handoff-w15-a-v141-release-2026-07-24.md` §9
**会话定位**:W15-A 大工程启动前的最终交接(加快模式)

---

## 0. 用户最近反馈(2026-07-24 本会话末)

| 反馈 | 决策 |
|---|---|
| MiniMax 默认大模型 | **MiniMax-M3**(用户明确指定,不是 MiniMax-Text-01) |
| MiniMax API 配置 | 不猜测,实装时由新会话直接核对 MiniMax 官方文档 |
| 桌面手动验 | **跳过 v1.4.2**,也跳过 W15-A 完成后的桌面验(除非用户主动要求) |
| 进度 | **太慢,要加快** |
| W15-A | **本项目下一阶段就是 W15-A 大工程开做** |

---

## 0.5 本会话进展(2026-07-24 第二段,加快模式)

| Step | 内容 | 状态 |
|---|---|---|
| 1 | 用户追问"MiniMax 默认大模型 = MiniMax-M3"(不是 MiniMax-Text-01)+ 不要猜测,核对官方文档 | ✅ MiniMax-M3 采纳 |
| 2 | 用户跳过 v1.4.2 桌面手动验(节省时间) | ✅ 接受 |
| 3 | 用户"进度太慢,要加快" | ✅ 加快模式规则确定(§1) |
| 4 | 用户授权开干 W15-A T1(本会话剩预算) | ✅ 执行 |
| 5 | W15-A T1:`keyring_store.rs` 4 函数 + 5 tests + Cargo.toml `keyring="3"` `dirs="5"` + lib.rs `mod keyring_store` | ✅ **48/48 tests passed**(43 baseline + 5 new) |
| 6 | 撞墙:keyring v3 + Windows 同进程多 `Entry::new` race(2 tests fail) | ✅ **升级到 keyring v4 + v1 feature 修复** |
| 7 | `cargo build --release` 验证 | ✅ 3m07s,6 warnings(list_profile_names unused spec 保留,W15-B 用) |

**T1 已就绪,文档改动 + 代码改动未 commit**(加快模式,等 W15-A feature 完整 commit)。新会话直接进 T2。

---

## 1. 加快模式规则(用户 2026-07-24 拍板)

| 项 | 旧模式(慢) | 新模式(快) |
|---|---|---|
| Commit 粒度 | 每 W 一 commit | **W15-A 整个 feature 一 commit**(`feat(ui): W15-A — LLM API Settings panel + 9 providers`) |
| Handoff 节奏 | 每 session 一 handoff | **W15-A 完成时一总 handoff**(中间不写) |
| Release 节奏 | 每 W 一 release(v1.4.x 频繁) | **W15-A 完成时一次性 v1.5.0 release** |
| 桌面手动验 | 每次 release 都验 | **默认跳过**,只在主用户主动要求时验 |
| Sandbox-verify | 每次都尝试 | **fallback(static + cargo test)**,环境不好就跳过 |
| Spec/plan review | 多次小改 | **已就绪,新会话直接读 spec/plan 开干** |
| 决策路径 | AskUserQuestion 多轮 | **已决策的全做,新会话只问未知项** |

**为什么这样**:用户明示"进度太慢",前几次会话每 W 一 release + 每 session 一 handoff 节奏过细。W15-A 是大工程(spec + plan 已就绪),直接开干不啰嗦。

---

## 2. 已完成(本会话 + 之前)

### 2.1 v1.4.2 release ✅

| 项 | 数据 |
|---|---|
| 图标源 | `docs/media-to-doc.png`(128×128 RGBA) |
| 处理 | Pillow LANCZOS → 1024×1024 + `cargo tauri icon` 38 个图标 |
| NSIS | 1,573,985 bytes,SHA256 `ce9d152e845c2dcd042755c371369adeb24534b32e2890ae848f4be8515c0707` |
| cargo test | 43/43 passed |
| cargo build | 2m28s |
| Commit / Tag / Release | `db84639` / `v1.4.2` / https://github.com/kizemo/media-to-doc-ui/releases/tag/v1.4.2 |
| **桌面手动验** | ⚠️ **用户 2026-07-24 决定跳过**(节省时间) |

### 2.2 W15-A 服务商清单决策 ✅

| 决策 | 内容 |
|---|---|
| MiniMax | **保留 + 真实支持**,默认 model = **`MiniMax-M3`**(用户指定) |
| 接口 AI / 胜算云 / TeamoRouter | **删除**(占位,无核实标准) |
| 总服务商数 | **12 → 9**(Anthropic / OpenAI / Ollama / LM Studio / DeepSeek / Zhipu / Kimi / MiniMax / Custom) |

### 2.3 MiniMax 真实 API 框架(已采纳)

| 项 | 值 | 来源 |
|---|---|---|
| base_url | `https://api.minimaxi.com/v1` | 已知,MiniMax 公开文档框架 |
| 默认 model | `MiniMax-M3` | **用户 2026-07-24 指定** |
| 协议 | OpenAI ChatCompletion 兼容 | 已知 |
| 认证 | Bearer Token | 已知 |
| Key 申请 | https://platform.minimaxi.com → 用户中心 → 接口密钥 | 已知 |
| 官方文档 | https://platform.minimaxi.com/document/ChatCompletion%20v2 | 已知 |

**未确认项**(实装时由新会话核对官方文档):
- `MiniMax-M3` 是否是当前 model 名(可能用户文档里有 `abab-M3` / `minimaxi-M3` 等别名)— 实装时先 list models 探测
- 模型 temperature / max_tokens 等参数默认值
- 是否有 `tool_use` / 多模态 endpoint 差异

---

## 3. W15-A 启动准备状态

### 3.1 spec ✅

`docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md` 已更新:
- §3:9 个服务商表格(MiniMax 真实 + 3 placeholder 删除)
- §6:6 个 Tauri commands API 契约
- §7:Settings tab + Providers 子页 + 添加 modal UI 设计
- §8:13 步验收清单 + 30+ 单测目标
- §10:风险章节更新(MiniMax 占位已部分解决)

### 3.2 plan ✅

`docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md` 顶部加 ⚠️ 实装决策段:
- 9 个服务商表格
- MiniMax 真实 API 信息(`MiniMax-M3` + `api.minimaxi.com/v1`)
- T1→T8 实装微调说明
- plan 内参考代码片段是 12 个版本,实装时按决策段 + spec 改 9 个版本

### 3.3 Cargo deps 已 T1 实装(plan §T1 已写,本会话 Cargo.toml 已改)

```toml
[dependencies]
keyring = { version = "4", features = ["v1"] }   # OS keyring (Win DPAPI / Mac Keychain / Linux Secret Service);v4 + v1 feature 修 v3 的 Windows race
dirs = "5"                                       # config dir 找 %APPDATA%/com.duanyi.mediatodoc
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }  # test_connection HTTP probe
```

**重要决策(本会话撞墙)**:keyring **v3 不行**,v3 + Windows 在同进程多次 `Entry::new()` 创建新对象后,set + 立即 get 会 fail("No matching entry found")。v4 + `features = ["v1"]` 修复。**实装必须用 v4**。

### 3.4 文档改动未 commit

| 文件 | 状态 |
|---|---|
| `docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md` | 修改未 commit |
| `docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md` | 修改未 commit |
| `handoff-w15-a-providers-decided-v142-icon-2026-07-24.md`(本文件) | 新建未 commit |

新会话第一次动作:把这些文档改动 + W15-A T1→T8 实现一次性 commit(`feat(ui): W15-A — LLM API Settings panel + 9 providers`)。

---

## 4. W15-A T1→T8 实施清单(加快模式直接开干)

| Task | 内容 | 预估 |
|---|---|---|
| T1 | `keyring_store.rs` 4 函数(read/write/delete/list_username)+ 5 tests | ✅ **本会话完成**(48/48 passed)|
| T2 | `llm_profiles.rs` Provider enum(9 个) + `all_templates()` + `validate_base_url` + 17 tests | 45min |
| T3 | `llm_profiles.rs` JSON metadata IO(`%APPDATA%/com.duanyi.mediatodoc/llm_profiles.json`)+ env var mapping(`to_env_vars()`) | 30min |
| T4 | `commands.rs` 6 Tauri commands + 错误码 enum + 8 tests | 45min |
| T5 | `runner.rs` SpawnSpec 加 `env_vars: HashMap<String, String>` 字段 + `spawn_mtd` `.env_clear().envs()` | 20min |
| T6 | `src/index.html` Settings tab(6th)+ Providers 子页 + profile 列表 + 激活/编辑/删除 | 45min |
| T7 | `src/index.html` 添加 modal + 9 预设下拉 + 表单 + 测试连接按钮 + 保存 | 45min |
| T8 | `cargo test --release`(43+30=73+ passed)+ `cargo tauri build` + 验收清单 13 步(本机跑一遍) | 30min |
| **总** | | **~5h**(需 2-3 session) |

### 4.1 单 session 内 T2→T4 或 T5→T8 二选一(T1 已本会话完成)

按单 session <2h 活跃时间预算:
- **Session 1(下次)**:T2 + T3 + T4(后端,150min)— Rust provider 模板 + commands + env var mapping
- **Session 2**:T5 + T6 + T7 + T8(集成,140min)— runner 集成 + 前端 UI + 验收

如果 session 1 超时,先把 T2→T3 commit(`feat(ui): W15-A — backend llm_profiles + 6 commands`),session 2 做 T4→T8。

### 4.2 9 个 Provider enum(实装直接用)

```rust
pub enum Provider {
    Anthropic, OpenAI, Ollama, LmStudio, DeepSeek, Zhipu, Kimi, MiniMax, Custom,
}
```

### 4.3 9 个 all_templates(实装直接用)

| enum_value | display_name | default_base_url | default_model |
|---|---|---|---|
| Provider::Anthropic | "Anthropic" | https://api.anthropic.com | claude-sonnet-4-5 |
| Provider::OpenAI | "OpenAI" | https://api.openai.com/v1 | gpt-4o |
| Provider::Ollama | "Ollama" | http://localhost:11434 | llama3.1 |
| Provider::LmStudio | "LM Studio" | http://localhost:1234/v1 | loaded-model |
| Provider::DeepSeek | "DeepSeek" | https://api.deepseek.com | deepseek-chat |
| Provider::Zhipu | "Zhipu GLM" | https://open.bigmodel.cn/api/paas/v4 | glm-4-plus |
| Provider::Kimi | "Kimi" | https://api.moonshot.cn/v1 | moonshot-v1-128k |
| Provider::MiniMax | "MiniMax" | https://api.minimaxi.com/v1 | **MiniMax-M3** |
| Provider::Custom | "Custom" | ""(用户填) | ""(用户填) |

### 4.4 关键 TDD 测试样例

```rust
#[test]
fn template_minimaxi_has_correct_fields() {
    let t = all_templates().into_iter()
        .find(|t| t.enum_value == Provider::MiniMax).unwrap();
    assert_eq!(t.display_name, "MiniMax");
    assert_eq!(t.default_base_url, "https://api.minimaxi.com/v1");
    assert_eq!(t.default_model, "MiniMax-M3");
}
```

---

## 5. 当前 git 状态(子仓)

```
db84639 (HEAD -> master, tag: v1.4.2, origin/master) build(ui): v1.4.2 — replace icon set with project logo
b59350c build(ui): v1.4.1 — version bump 1.4.0 → v1.4.1 + release notes
98166ce fix(ui): v0.1.0 badge regression in main window
... 之前 commits

untracked:
- handoff-w15-a-v141-release-2026-07-24.md(已 superseded,新会话可删)
- prompt-next-session.md(已 superseded,新会话可删)
- handoff-w15-a-providers-decided-v142-icon-2026-07-24.md(本文件,新会话需读)
- docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md(已修改未 commit)
- docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md(已修改未 commit)
```

---

## 6. 当前发布版本

### 子仓 Tauri UI(`kizemo/media-to-doc-ui`)

| 版本 | 主要内容 |
|---|---|
| v1.3.0 | Tauri 2 桌面壳 + 8 commands + NSIS + portable |
| v1.4.0 | NSIS only + D 盘默认 |
| v1.4.1 | v0.1.0 badge regression fix |
| **v1.4.2** | **本会话:图标替换(38 个)+ version bump(桌面手动验跳过)** |
| **v1.5.0(待 W15-A 完成)** | **LLM API Settings panel + 9 providers + keyring + env 注入** |

### 主仓 `media-to-doc`(无本会话改动)

| 版本 | 类型 |
|---|---|
| v1.3.0 | trust_env 全 provider + Tauri UI v1.3.0 协同(W14-E) |

---

## 7. 已知风险 / 边界(继承)

- Win11 Insider Build 26200 沙箱 broken(继续 fallback)
- 公司 VPN 用户构建需 `CARGO_NET_TLS_VERIFY=false`
- Rust toolchain 1.97+
- macOS / Linux 编译需用户自查
- 9 个服务商 model 默认值每版本手工更新
- Linux keyring 需 gnome-keyring / kwallet / secret-service daemon
- env 注入只对 mtd 子进程生效;父 Tauri 进程瞬时持有 key ≤ 100ms
- **W15-A 实装前** MiniMax `MiniMax-M3` model 名由新会话核对 MiniMax 官方文档确认(可能别名 `abab-M3` / `minimaxi-M3` 等)

---

## 8. 上游主仓状态

主仓 `media-to-doc` v1.3.0 仍是最新(W15-A 是子仓 UI 改造,主仓无对应改动):
- PyPI: https://pypi.org/project/media-to-doc/
- GitHub: https://github.com/kizemo/media-to-doc/releases/tag/v1.3.0

---

## 9. 新会话 prompt 模板

**用法**:复制 §9.1 整段 → 粘贴到新会话 user message 开头。

### 9.1 新会话 prompt(可复制)

```text
承接 F:\soft\00selfmade\media-to-doc-ui\handoff-w15-a-providers-decided-v142-icon-2026-07-24.md

【用户 2026-07-24 最新反馈 - 必须遵守】
1. MiniMax 默认大模型 = MiniMax-M3(不是 MiniMax-Text-01);不要猜测,实装前核对 MiniMax 官方文档 https://platform.minimaxi.com/document/ChatCompletion%20v2
2. v1.4.2 桌面手动验已跳过(用户拍板);后续版本默认也不做桌面手动验,除非用户主动要求
3. 加快模式:W15-A 整个 feature 一 commit,中间不写 handoff,T1→T8 一气呵成(详见 Handoff §1 加快模式规则)
4. 当前项目目标:尽快推进 W15-A 大工程;不要再做小 W + 小 release

【W15-A 启动条件】
- spec + plan 已就绪(9 个服务商 + MiniMax-M3 + 6 commands + Settings UI + env 注入)
- 文档改动未 commit:docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md + plans/ 同名文件
- 加快模式要求:W15-A feature + 文档改动一次性 commit(feat(ui): W15-A — LLM API Settings panel + 9 providers)

【你(新会话)的第一件事】
读完 Handoff + spec + plan,然后:
1. 用 brainstorming skill 或 AskUserQuestion 只问 W15-A 实装中未知的细节(预计 <3 个问题,spec/plan 已覆盖大部分)
2. 实施 T1→T4(本 session,~150min),目标是后端 100% 通(cargo test 73+ passed)
3. 如果 session 超时,先 commit T1→T4,下次 session 接 T5→T8
4. 完成后给总 handoff:W15-A + v1.5.0 release

【预算】
- <2h 活跃时间,到点 /exit 或新开会话
- bash 调用 >100 拆任务
- 单回合 diff >500 行拆回合

【绝不要做】
- 不要再做小步 handoff
- 不要桌面手动验(除非用户主动要求)
- 不要猜测 MiniMax model 名(查文档)
- 不要硬编码 main 分支(沿用 master + 子仓独立 repo)
```

### 9.2 新会话应回答的问题(可选,AskUserQuestion)

如果新会话读完后还有真不清楚的细节,可问用户 1-3 个(避免阻塞):

| 问题 | 选项 |
|---|---|
| MiniMax 模型 list 探测 | A 启动时 list models 探测 / B 用户填默认 + 编辑可改 / C 不探测,信任用户填 |
| keyring 跨 session 持久化 | A 默认行为(Win DPAPI 跨 session 持久)/ B session-only 测试模式 |
| v1.5.0 release 时机 | A W15-A 完成后立即 / B 等用户桌面验过 / C 等主仓同步后 |
| 主仓 v1.4.x 同步 | A 本次 W15-A 后立即同步 / B 留未来 / C 不同步 |

---

## 10. 交付物清单(本会话)

| 文件 | 类型 | 状态 |
|---|---|---|
| `docs/RELEASE_NOTES_v1.4.2.md` | 新建 | ✅ committed `db84639` |
| `src-tauri/Cargo.toml` | 修改 1.4.1 → 1.4.2 | ✅ committed |
| `src-tauri/tauri.conf.json` | 修改 1.4.1 → 1.4.2 | ✅ committed |
| `src-tauri/nsis/installer.nsi` | 修改 1.4.1 → 1.4.2 | ✅ committed |
| `src-tauri/icons/source-icon.png` | 新建 | ✅ committed |
| `src-tauri/icons/*.png`(38 个) | 重生成 | ✅ committed |
| `docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md` | 修改 MiniMax-M3 + 9 服务商 | ⚠️ **修改未 commit**(新会话合入 W15-A feature commit) |
| `docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md` | 修改 MiniMax-M3 + 实装决策段 | ⚠️ **修改未 commit**(新会话合入 W15-A feature commit) |
| `handoff-w15-a-providers-decided-v142-icon-2026-07-24.md`(本文件) | 新建 | ⚠️ **新会话合入 W15-A 总 handoff**(本文件留作新会话 prompt 引用,新会话 commit 时如果觉得重复可以删) |

---

## 11. 历史 superseded handoff(新会话可删)

- `handoff-w15-a-v141-release-2026-07-24.md` — 已被本文件 supersede
- `prompt-next-session.md` — 上次会话遗留,新会话可删

新会话开始时建议:
```bash
cd F:/soft/00selfmade/media-to-doc-ui
rm handoff-w15-a-v141-release-2026-07-24.md prompt-next-session.md
git add docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md \
        docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md \
        handoff-w15-a-providers-decided-v142-icon-2026-07-24.md
```

(然后 T1→T4 的代码改动 add 进来一次性 commit)

---

## 12. 完成 W15-A 的最终验收清单

(用户拍板标准,不是机械验收)

| # | 验收 | 通过条件 |
|---|---|---|
| 1 | 9 个服务商下拉都能选 | Settings > Providers > 添加 modal 9 个按钮都在 |
| 2 | MiniMax 真实可用 | 选 MiniMax → 自动填 `api.minimaxi.com/v1` + `MiniMax-M3` → 填 key → 测试连接 → 成功 |
| 3 | keyring 持久化 | 重启 Tauri,profile 还在 + key 不用重新填 |
| 4 | env 注入有效 | Run pipeline → mtd 子进程收到 `OPENAI_API_KEY` + `OPENAI_BASE_URL` + `OPENAI_MODEL` |
| 5 | cargo test 全过 | 73+ passed / 0 failed |
| 6 | cargo tauri build NSIS 出包 | exit 0,生成 `media-to-doc_1.5.0_x64-setup.exe` |
| 7 | (可选,跳过)桌面手动验 | 用户主动要求时才做 |

完成 = v1.5.0 release(commit + tag + push + gh release)。