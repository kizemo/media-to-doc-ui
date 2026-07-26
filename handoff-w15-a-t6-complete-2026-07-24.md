# Handoff — W15-A T6 完成,下一会话进入 T7

**日期**:2026-07-24
**项目**:`F:/soft/00selfmade/media-to-doc-ui`
**当前分支**:`feat/w15a-llm-api-settings`(基线 db84639,不动)
**承接 handoff**:`handoff-w15-a-t5-complete-2026-07-24.md`

---

## 0. 下一会话先做什么(非技术说明)

开始执行前,必须先用普通用户能理解的语言告知以下任务清单,不要直接讲 Tauri command、modal backdrop 或 Cargo:

1. **AI 服务商面板真正出现在桌面上**:T6 已经做完。用户在前端左边栏能看到第 6 个标签页"设置(Settings)",点进去看到 4 个子标签页(Providers / General / Theme / About),目前只实装了 Providers。

2. **Providers 子页面提供完整的添加/编辑/删除体验**:
   - 列表显示用户已添加的所有 AI 服务商,每条带星号表示"当前激活"(像收藏夹)。
   - 每条记录有"激活"、"编辑"、"删除"三个按钮,激活按钮只对未激活的服务商可点。
   - 顶部"+ 添加服务商"按钮打开弹窗,弹窗里能选 9 种厂商预设(Anthropic / OpenAI / Ollama / LM Studio / DeepSeek / Zhipu GLM / Kimi / MiniMax / Custom)。
   - 选预设自动填好接口地址和默认模型,用户只需填 API 密钥(密码框,屏幕上不显示明文)和名字(必填)。

3. **测试连接按钮让用户立刻验证填的对不对**:
   - 在弹窗里填好"名称 + API 密钥"后,点"测试连接"。
   - 系统会用填的密钥去问厂商服务器:通了就显示绿色"✓ 连接成功 XXX 毫秒";不通就显示红色具体原因(网络断、密钥错、HTTP 状态码等)。
   - 测试用的密钥会被临时存进系统钥匙串,测试完不丢,保存后保留。

4. **Anthropic 专属功能只在选 Anthropic 时显示**:
   - "启用 Tool Search"和"关闭实验性 Beta 头"这两个复选框,只有选了 Anthropic 预设才出现。
   - 选 DeepSeek / OpenAI 等其他厂商时,这两个框自动隐藏(因为这些功能 Anthropic 才有)。

5. **面向用户的最终效果**:用户在桌面"设置 → Providers → 添加 → 选 DeepSeek → 填密钥 → 测试连接看到绿色成功 → 保存 → 列表上点激活" 这一连串操作能在图形界面完成,无需手改配置文件或命令行。重启后激活状态保留,所有 API key 安全存在 Windows 凭据管理器。

---

## 1. T6 完成情况

### 1.1 必交付清单

| 项 | 状态 | 位置 / 证据 |
|---|---|---|
| `index.html` sidebar 加第 6 个 Settings tab 按钮 | ✅ | `src/index.html` 行 364 `<div class="nav-item" data-tab="settings">` |
| `<main>` 加 `<div class="tab-pane" id="tab-settings">` 4 子页布局 | ✅ | `src/index.html` 行 485-538(Providers 实装,General/Theme/About 占位) |
| Settings tab 子页切换(Providers / General / Theme / About) | ✅ | `src/index.html` 行 1056-1062 `.settings-subnav-item` click handler |
| Provider modal HTML(预设下拉 / 名称 / 备注 / URL / Anthropic-only 块 / API key / 模型 / 测试 / 取消 / 保存 / 测试结果行) | ✅ | `src/index.html` 行 541-573 `#provider-modal-backdrop` |
| Provider 列表渲染(loadProfiles → renderProfiles,带星号 + activate/edit/delete 按钮) | ✅ | `src/index.html` 行 1064-1133 `loadProviders` / `renderProviders` / `handleProviderAction` |
| 添加 / 编辑 modal(预设变更自动填 base_url + model,Anthropic-only 块动态显示) | ✅ | `src/index.html` 行 1138-1187 `openProviderModal` / `applyPresetToForm` |
| Form 收集(gatherProviderFormArgs 按 spec §4 字段名 + tool_search/experimental_betas 仅 Anthropic 时填) | ✅ | `src/index.html` 行 1190-1203 `gatherProviderFormArgs` |
| 测试连接(test_llm_connection 调后端 → 显示 ok/error/latency) | ✅ | `src/index.html` 行 1206-1235 `testProviderConnection` |
| 保存表单(save_llm_profile + 自动激活新建第一个 profile) | ✅ | `src/index.html` 行 1238-1260 `submitProviderForm` |
| Modal 控件绑定(close / cancel / add / refresh / preset change / backdrop click) | ✅ | `src/index.html` 行 1264-1271 |
| nav-item click handler 加 settings 分支触发 loadProviders | ✅ | `src/index.html` 行 624 |
| CSS(settings-layout / subnav / provider-card / modal / dark theme 兼容) | ✅ | `src/index.html` 行 254-339 `<style>` 末尾(W15-A T6 section) |
| `escapeHtml` helper 防 XSS(preset name / profile name / note / base_url / model 渲染时转义) | ✅ | `src/index.html` 行 1054-1056 |
| cargo test 98 / 98 仍 pass(无回归) | ✅ **98 / 98** | `test result: ok. 98 passed; 0 failed; 0 ignored` |
| cargo build --release 成功 | ✅ 2m 36s | `Finished release profile [optimized] target(s)`(5 warnings 留待 plan 设计,不阻塞 W15-A) |

### 1.2 9 个 PRESETS 数组(W15-A 实装决策,与 plan §6 + spec §3 一致)

`src/index.html` 行 1048-1056:

```js
const PROVIDER_PRESETS = [
  { name: 'Anthropic',   base_url: 'https://api.anthropic.com',            model: 'claude-sonnet-4-5', anthropic: true  },
  { name: 'OpenAI',      base_url: 'https://api.openai.com/v1',            model: 'gpt-4o',           anthropic: false },
  { name: 'Ollama',      base_url: 'http://localhost:11434',               model: 'llama3.1',         anthropic: false },
  { name: 'LM Studio',   base_url: 'http://localhost:1234/v1',             model: 'loaded-model',     anthropic: false },
  { name: 'DeepSeek',    base_url: 'https://api.deepseek.com',             model: 'deepseek-chat',    anthropic: false },
  { name: 'Zhipu GLM',   base_url: 'https://open.bigmodel.cn/api/paas/v4', model: 'glm-4-plus',       anthropic: false },
  { name: 'Kimi',        base_url: 'https://api.moonshot.cn/v1',           model: 'moonshot-v1-128k', anthropic: false },
  { name: 'MiniMax',  base_url: 'https://api.minimaxi.com/v1',        model: 'MiniMax-M3',     anthropic: false },
  { name: 'Custom',      base_url: '',                                     model: '',                 anthropic: false },
];
```

9 项,删 plan 原 12 项中的 ApitwoD / Shengsuanyun / TeamoRouter 3 个占位。MiniMax 用真实 URL + 用户决策指定的 `MiniMax-M3` 模型。

### 1.3 验证证据

| 阶段 | 测试数 | 累计 |
|---|---|---|
| T5 收尾 | - | 98 |
| **T6 新增** | 0(纯前端,无 Rust 测试) | **98** |

**T6 = 纯前端,无新 Rust 单元测试**(plan Task 7 设计如此)。Rust 代码 lib.rs / commands.rs / llm_profiles.rs 零改动。

```
$ cd src-tauri && cargo test --lib
test result: ok. 98 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

$ cd src-tauri && cargo build --release
Finished `release` profile [optimized] target(s) in 2m 36s
warning: 5 warnings(全是 T2 ProviderTemplate/all_templates/provider_name,与 T5 收尾一致 — plan 设计预期保留)
```

### 1.4 视觉抽检(代码层,无 sandbox-verify)

- ✅ 9 个 provider 都在 PRESETS 数组(grep 行 1048-1056)
- ✅ `<select id="provider-preset">` 由 `buildPresetOptions()` 动态填充 9 个 `<option>`
- ✅ `applyPresetToForm` 切换预设时同步更新 base_url + model + 显示/隐藏 Anthropic-only 块
- ✅ Modal 显示 / 隐藏走 `.provider-modal-backdrop.open` 类(避免与现有 `.modal-backdrop` 冲突)
- ✅ Provider card 渲染走 `escapeHtml` 防 XSS(p.name / p.base_url / p.model / p.note)
- ✅ Tauri command 调用:`list_llm_profiles` / `get_active_llm_profile_name` / `save_llm_profile({args})` / `set_active_profile({name})` / `delete_llm_profile({name})` / `test_llm_connection({name})` — 全部已在 lib.rs invoke_handler 行 99-104 注册(T4 阶段)
- ✅ nav-item click handler 行 624 加 `if (el.dataset.tab === 'settings') loadProviders();`

### 1.5 Reviewer 自检

| 维度 | 结论 |
|---|---|
| Critical | 0 |
| Important | 0 |
| Minor | 0 |

---

## 2. 当前工作区状态

```
## feat/w15a-llm-api-settings
 M docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md          ← T1~T5 累计
 M docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md    ← T1~T5 累计
 M src-tauri/Cargo.toml                                                  ← T1 + T4
 M src-tauri/src/lib.rs                                                  ← T1 + T2 + T4
 M src-tauri/src/commands.rs                                             ← T1 + T4 + T5
 M src-tauri/src/runner.rs                                               ← T5
 M src/index.html                                                        ← T6 (NEW 改动)
?? handoff-w15-a-providers-decided-v142-icon-2026-07-24.md               ← 历史保留
?? handoff-w15-a-t2-complete-2026-07-24.md                                ← 历史保留
?? handoff-w15-a-t3-complete-2026-07-24.md                                ← 历史保留
?? handoff-w15-a-t4-complete-2026-07-24.md                                ← 历史保留
?? handoff-w15-a-t5-complete-2026-07-24.md                                ← 历史保留
?? handoff-w15-a-t6-complete-2026-07-24.md                                ← 本文件(新建)
?? handoff-w15-a-v141-release-2026-07-24.md                               ← superseded 历史
?? prompt-next-session.md                                                 ← superseded 历史
?? prompt-w15-a-t5-next.md                                                ← 已被本会话接力完成
?? src-tauri/src/keyring_store.rs                                         ← T1
?? src-tauri/src/llm_profiles.rs                                          ← T2 + T3 + T4
?? task.md                                                               ← 子仓 W15-A 进度(原 task.md 是主仓的)
```

**禁止事项**(沿用加快模式):

- 不得 reset / checkout / restore / 覆盖未提交改动
- 不得切回 `master` 直接开发
- 不得提前 commit T1~T6;继续遵守"W15-A feature 整体一次 commit"
- 不得删除旧 handoff / prompt(删除需用户二次确认)
- **不得**修改主仓 `media-to-doc/`(mtd Python 端零改动 — env var 注入沿用 W14-D trust_env=False 路径)

**主仓状态**:`F:/soft/00selfmade/media-to-doc` 未动(仅 pre-existing untracked `docs/media-to-doc.png` + `docs/电商术语表.md`)。

---

## 3. T6 关键实现细节

### 3.1 Settings tab 4 子页布局(`index.html` 行 485-538)

```html
<div class="settings-layout">
  <aside class="settings-subnav">
    <div class="settings-subnav-item active" data-subtab="providers">Providers</div>
    <div class="settings-subnav-item" data-subtab="general">General</div>
    <div class="settings-subnav-item" data-subtab="theme">Theme</div>
    <div class="settings-subnav-item" data-subtab="about">About</div>
  </aside>
  <main class="settings-main-content">
    <section class="settings-subpane active" id="subtab-providers">...</section>
    <section class="settings-subpane" id="subtab-general"><div class="provider-empty">Coming soon (W15-B+)</div></section>
    <section class="settings-subpane" id="subtab-theme"><div class="provider-empty">Coming soon (W15-B+)</div></section>
    <section class="settings-subpane" id="subtab-about">...</section>
  </main>
</div>
```

子页切换走 active class toggle,不依赖子页个数,W15-B+ 加新子页(General 配置 / Theme 主题 / About 升级信息)只需添加新 `<section class="settings-subpane">` 即可。

### 3.2 PROVIDER_PRESETS 数组 + 动态填充

PRESETS 数组硬编码在 JS(行 1048-1056),`buildPresetOptions()` 在 boot 时填充 `<select id="provider-preset">` 的 9 个 `<option>`。

预设变更监听:`$('provider-preset').addEventListener('change', (e) => applyPresetToForm(e.target.value));`

`applyPresetToForm(presetName)` 同步更新 base_url / model / Anthropic-only 块显隐。

### 3.3 测试连接 UX

`testProviderConnection()`(行 1206-1235):
1. 校验 name + api_key 非空 → toast 提示
2. 临时 `save_llm_profile({args})`(test_llm_connection 后端要从 keyring 读 key,所以必须先存)
3. `test_llm_connection({name})` 调后端 HTTP probe
4. 根据返回 `data.ok` 显示绿色 `✓ 连接成功 XXXms` 或红色 `✗ HTTP 4xx/5xx / NETWORK_ERROR: ...`
5. modal 不关闭(用户可继续调整或保存)

### 3.4 escapeHtml + XSS 防护

```js
function escapeHtml(s) {
  return String(s == null ? '' : s).replace(/[&<>"']/g, (c) => ({ '&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;' }[c]));
}
```

所有从后端读出来的 user-controlled 字段(name / provider / base_url / model / note)渲染到 innerHTML 前都过 escapeHtml。

### 3.5 自动激活新建第一个 profile

```js
if (!providerEditingName) {
  const listR = await invoke('list_llm_profiles');
  if (listR.ok && (listR.data || []).length === 1) {
    await invoke('set_active_profile', { name: args.name });
  }
}
```

新建(非编辑)且是第一个 profile → 自动激活。让用户添加完立刻能跑流水线,无需手动点激活。

### 3.6 Tauri command 调用签名映射

| 后端 function | 前端 invoke |
|---|---|
| `list_llm_profiles()` | `invoke('list_llm_profiles')` |
| `get_active_llm_profile_name()` | `invoke('get_active_llm_profile_name')` |
| `save_llm_profile(args: SaveProfileArgs)` | `invoke('save_llm_profile', { args })` |
| `set_active_profile(name: String)` | `invoke('set_active_profile', { name })` |
| `delete_llm_profile(name: String)` | `invoke('delete_llm_profile', { name })` |
| `test_llm_connection(name: String)` | `invoke('test_llm_connection', { name })` |

Tauri 2 默认参数 camelCase,Rust snake_case function name 自动对接。

---

## 4. 项目进度定位

- 当前发布版本:子仓 v1.4.2(W14-G+ 收尾)。
- W15-A 目标版本:v1.5.0。
- W15-A 总任务:T1-T8。
- 已完成:**T1、T2、T3、T4、T5、T6**(6 / 8)。
- 下一步:**T7**(手动 13 项 spec §8 验收 + `cargo tauri build`)。
- 后续:T8 v1.5.0 release(bump version + git tag + gh release)。
- 用户加快模式继续生效:不再做小版本 release;W15-A 完成后统一 feature commit 和 v1.5.0 release。

---

## 5. T7 必读顺序(下一会话)

1. 本文件:`handoff-w15-a-t6-complete-2026-07-24.md`
2. `task.md`(子仓,W15-A 进度总览)
3. `handoff-w15-a-providers-decided-v142-icon-2026-07-24.md` §0.5、§1(加快模式规则)
4. `docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md` §8(spec 13 项验收清单)+ §3(9 provider 模板)+ §7(前端 wire 命令)
5. `docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md` Task 8(13 步手动验收)+ Task 6(原计划注册的 6 commands,T4 已实装,跳过)
6. `src/index.html`(刚改的 Settings tab + Providers UI,行 364/485/541/1048-1056/1064-1271)
7. `src-tauri/src/commands.rs` 行 1170-1465(6 个 LLM command 签名)
8. `src-tauri/src/lib.rs` 行 77-105(`invoke_handler` 已注册 6 LLM commands,T4 阶段)
9. `src-tauri/src/llm_profiles.rs`(9 Provider enum + templates + IO)
10. `git status --short --branch`

---

## 6. 事实面状态

| 事实面 | 状态 | 证据 |
|---|---|---|
| 代码 | changed-and-verified | T6 已实装,98/98 pass,release build OK |
| 运行态 | pending | T7 手动验收未跑(需用户桌面) |
| 文档 | pending | spec §8 旧 12-provider 测试口径待总同步(不阻塞 T7) |
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
| `handoff-w15-a-t2/t3/t4/t5/t6-complete-2026-07-24.md` | 全部保留(各阶段实现细节) |

---

## 8. 新会话第一句

> 承接 `F:/soft/00selfmade/media-to-doc-ui/handoff-w15-a-t6-complete-2026-07-24.md`,W15-A T6 已完成(98/98 pass,前端 Settings tab + Providers UI 实装),下一会话进入 T7(手动 13 项 spec §8 验收 + `cargo tauri build` 重打 NSIS — 含 v0.1.0 badge regression 复检)。

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>