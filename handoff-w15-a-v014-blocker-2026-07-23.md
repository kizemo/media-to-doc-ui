# Handoff — W15-A + v1.4.0 桌面手动验撞墙 + v1.4.1 子仓发布阻塞

**日期**:2026-07-23
**承接 handoff**:`handoff-w14g-e-msi-2026-07-23.md` §10(下次会话第一句)
**承接会话承诺**:W14-G+ NSIS D 盘默认 release 发布 + 桌面手动验 4 步 → v1.4.1 子仓发布

---

## 1. 已完成 ✅

| Step | 内容 | 结论 |
|---|---|---|
| 1 | W14-G+ reviewer 二次 review commit `9e2b080`(改 installer.nsi + tauri.conf.json) | ✅ PASS,WIP 清除 |
| 2 | W15-A spec reviewer pass 修订 | ✅ commit `565279d`(6 项修复 + 4 占位标记) |
| 3 | W15-A 实施 plan(8 task TDD) | ✅ commit `f28156e`(2467 行) |
| 4 | release/ NSIS 文件就位 + SHA256 验证 | ✅ `d0aa450e...` 1,576,687 bytes |
| 5 | 用户桌面手动装 | ✅ 装到 `D:\Program Files\MediaToDoc\`,启动 |
| 6 | sandbox-verify | ❌ **未跑**(Win11 Pro sandbox feature off,见 §5) |

---

## 2. 用户桌面手动验反馈(2026-07-23)

| # | 步骤 | 用户回报 | 期望对比 |
|---|---|---|---|
| 1 | 双击 NSIS 装 | ✅ 可以安装 | OK |
| 2 | 启动主窗口 | ❌ **界面和需求不符** — 截图见 §6 | 应正确显示 v1.4.0 版本 badge + 5 tab |
| 3 | 验证 D 盘目录 | ✅ 验证通过 | OK |
| 4 | 卸载 | ✅ 验证通过 | OK |

**用户新增项目级 feedback**(已存 memory):
- `feedback_sandbox_verify_before_release.md`:发布 .exe 前**必跑** sandbox-verify 真机装验证(主仓 wheel 除外)
- `feedback_minimize_user_intervention.md`:装机/真机验证类重复步骤尽量自己跑,不要求用户手动执行

---

## 3. 发现的 UI Bug(v0.1.0 regression)

**症状**:v1.4.0 build 启动后,主窗口 title badge 显示 `v0.1.0`,与 `Cargo.toml` / `tauri.conf.json` 的 `1.4.0` 不符。

**根因**:`src/index.html:257` 硬编码了 W14-B 早期的占位值:

```html
<span class="badge" id="version-badge">v0.1.0</span>
```

JS 在 line 452 用 `app_info.version` 覆盖:

```javascript
$('version-badge').textContent = 'v' + info.version;
```

**但**:`loadAppInfo()` 是 async 调用,在它完成前(以及失败时)badge 仍显示硬编码 `v0.1.0`。截图时 app_info 还在加载中(`status-text: loading…`,`COURSES: Loading...`),badge 露馅。

**为什么之前没被发现**:
- cargo test 43/43 跑不到(纯 Rust 测试,不渲染 WebView)
- code review 看 spec/plan 不看 index.html 这类视觉细节
- 之前的 v1.3.0 → v1.4.0 bump 只改了 Cargo.toml + tauri.conf.json + installer.nsi,**没改 index.html**

**修复方案**(本会话未做,留给下次会话):
- 选项 A:把 `index.html:257` 改为 `''`(空)或 `'...'`(占位),让 JS 唯一来源
- 选项 B:在 `loadAppInfo()` 失败/超时(>2s)显示 `'unknown'`,给用户清晰信号
- 选项 C:在 badge 上加 `class="hidden"` 直到 JS 填值
- 推荐:A(最简单) + B(防 app_info hang 时不显示 v0.1.0)

**相关 grep**:`src/index.html` 内 `"v0\."` / `"v1\."` / `"version"` 共 7 处,但**只有 line 257 的硬编码 v0.1.0 是 bug**;line 379-380 是 info 面板的 placeholder(动态填),line 452-455 是 JS(正确)。

---

## 4. v1.4.1 子仓发布状态

**阻塞点**:
1. ❌ v0.1.0 UI bug 未修复 → 即使 SHA256 通过、reviewer pass,装出窗口仍有 regression
2. ❌ sandbox-verify 未跑 → 用户明确要求"发布 exe 前必跑沙箱验",不能绕过
3. ❌ Win11 Pro sandbox feature off → 沙箱验跑不动(需 admin + 重启,跨 session)

**依赖关系**:
```
sandbox feature 启(用户一次性,需 admin + 重启)
    ↓
修 v0.1.0 bug(W15-A 待 T7 之外的小修)
    ↓
cargo tauri build
    ↓
sandbox-verify 跑通(双击装 + 看主窗口 v1.4.0 badge + 卸载)
    ↓
bump version + git tag v1.4.1 + gh release
```

**当前 release/ 状态**:`media-to-doc_1.4.0_x64-setup.exe` 1,576,687 bytes, SHA256 `d0aa450e...`(W14-G+ 已 reviewer pass,但 UI bug 逃过测试)。

---

## 5. Win11 Pro sandbox feature 启用步骤

**当前状态**:`Get-WindowsOptionalFeature Containers-DisposableClientVM.State` ≠ Enabled(W14-G+ §5.1 撞过)。

**用户需在主机 PowerShell(管理员)执行**:

```powershell
Enable-WindowsOptionalFeature -Online -FeatureName Containers-DisposableClientVM
# 重启后验证
Get-WindowsOptionalFeature -Online -FeatureName Containers-DisposableClientVM | Select-Object State
# 应输出:State = Enabled
```

**重启后,我能跑 sandbox-verify**:
```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File "F:\soft\00selfmade\sandbox-verify\media-to-doc-ui\mtd-verify.ps1" -InstallerPath "F:\soft\00selfmade\media-to-doc-ui\release\media-to-doc_1.4.0_x64-setup.exe" -NoWait
# 预期:exit 0,日志在 C:\Users\Duanyi\sandbox-artifacts\mtd\logs\verify.log
```

**撞墙预案**(参考 W14-G+ §4.6 / §5.1):
- `Windows Sandbox 未启用` → 检查 feature 是否真的启用,可能需 30s 等待
- `msi 装包未支持` → 当前 release/ 只有 NSIS,不影响
- log 显示装到 `C:\Users\Duanyi\...` 不是 `D:\mtd-test\...` → 检查 `-InstallerPath` 是否带 `-NoWait` + sandbox 默认 /D 参数

---

## 6. 截图分析

用户附图(`C:\Users\Duanyi\.claude\uploads\7cac8911-dad9-4edf-824b-96fa6a58f0e5\1afd9f51-141c-4ee1-9758-2c09f6e3d2ac-pasted-image-1784806317165.png`):

**显示元素**:
- Top bar:蓝色 + minimize/maximize/close
- Title:`media-to-doc` + `v0.1.0` badge ← **bug**
- Status:`loading…`(top-right,说明 app_info 还在加载)
- 5 tab sidebar:Inbox(active) / Run / Output / Health / Learn
- Workspace input:`D:/training/inbox (留空用默认)` + Refresh 按钮
- COURSES section:`Loading...`

**符合预期部分**:
- 5 tab 结构(W14-C + W14-E 状态,符合 v1.4.0)
- WORKSPACE input 路径提示正确
- COURSES 在 loading(预期 `list_courses` 异步调用中)

**不符预期部分**:
- ❌ v0.1.0 badge(应是 v1.4.0)
- ❌ status 显示 `loading…` 说明 app_info / list_courses 一直没返回(?)
- ⚠️ Settings tab 未出现(W15-A 还没实装,符合预期,但 spec 已落地,可能用户期待有)

---

## 7. W15-A 计划状态(未动)

| Task | 状态 |
|---|---|
| T1: Cargo.toml + keyring_store(5 tests) | 待开始 |
| T2: llm_profiles templates + validation(17 tests) | 待开始 |
| T3: llm_profiles metadata IO + env var mapping(10 tests) | 待开始 |
| T4: runner SpawnSpec.env_vars(1 test) | 待开始 |
| T5: commands 6 Tauri commands(8 tests) | 待开始 |
| T6: lib.rs invoke_handler wiring | 待开始 |
| T7: index.html Settings tab + modal | 待开始 |
| T8: 13-step manual acceptance | 待开始 |

**实施 plan 已落地**:`docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md`(commit `f28156e`)

**阻塞**:
- ⚠️ 4 个服务商 base_url 占位值待用户决策(MiniMax / 接口 AI / 胜算云 / TeamoRouter)— 见 spec §3 + §10.1
- ❌ v0.1.0 bug 修之前不应发 v1.4.1 → 不应开始 W15-A T1(T1 需 cargo tauri build 验证)

---

## 8. 当前 git 状态

- branch: master
- 领先 origin/master:9 commits(W14-F D ~ W14-G+ + W15-A spec + W15-A plan)
- 未推送(等桌面验 / sandbox-verify 后)
- working tree clean
- 下一个 commit 应该是 v0.1.0 fix + 后续 verify

---

## 9. 后续待办(优先级排序)

### P0 — 阻塞 v1.4.1 发布
1. **用户**:启 Win11 sandbox feature(`Enable-WindowsOptionalFeature ... Containers-DisposableClientVM`)+ 重启
2. **下次会话**:修 v0.1.0 bug(`src/index.html:257` 改 `''` + 加 timeout fallback)
3. **下次会话**:`cargo tauri build` 重新打 NSIS
4. **下次会话**:跑 `mtd-verify.ps1 -InstallerPath release/media-to-doc_1.4.0_x64-setup.exe -NoWait`,捕主窗口截图确认 badge = `v1.4.0`
5. **下次会话**:bump version 1.4.0 → 1.4.1(Cargo.toml + tauri.conf.json + installer.nsi + README.md)+ git tag v1.4.1 + gh release 上传 NSIS

### P1 — W15-A 推进
6. **下次会话或更后**:用户对 4 placeholder 服务商决策(MiniMax / 接口 AI / 胜算云 / TeamoRouter)— 选项 A 维持占位 / B 替换文字 / C 删除
7. **之后**:按 plan T1 → T8 顺序实装 W15-A(预计 2-3 session 跨实施)

### P2 — 长期
8. W15-B 会话 UI brainstorm
9. W15-C UI 强化(主题 / 快捷键 / 拖拽 / 多语言 / 动效)
10. 主仓 W15-A 后续(W15-A 是子仓 UI 改造,主仓 mtd 端无改动)

---

## 10. 下次会话第一句话

> 承接 `handoff-w15-a-v014-blocker-2026-07-23.md`,W15-A spec + plan 已落地,但 v1.4.0 桌面手动验发现 v0.1.0 badge regression(`src/index.html:257` 硬编码)。请按 memory `feedback_sandbox_verify_before_release` 优先:先让用户启 Win11 sandbox feature(若已启,直接修 v0.1.0 + 重 build + 跑 sandbox-verify);W15-A 实装等 v0.1.0 修 + sandbox-verify 跑通后再开。