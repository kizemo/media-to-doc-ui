# Prompt — W15-A Task 12 用户反馈接力(新会话第一句话)

承接:`F:/soft/00selfmade/media-to-doc-ui/handoff-w15-a-task12-build-verify-2026-07-25.md`(特别是 §0.5 用户反馈)

## 背景(20 行内)

- W15-A 12-task plan 已完成 Tasks 1-11,代码 working-tree 累积在 `feat/w15a-llm-api-settings` 分支,**无 commit**(加快模式)
- Task 12 build 已完成:NSIS installer `src-tauri/target/release/bundle/nsis/media-to-doc_1.4.2_x64-setup.exe`(2.6 MB,2026-07-25 02:03)
- 用户按 handoff §2 装机后跑 13 项验收,**3 个 visible bug + 1 个 functional bug**(详见 handoff §0.5)
- 加快模式:本任务**不 commit**(留 T8 release 会话统一)

## 用户反馈的 3+1 bug

| # | 用户描述 | 代码反推 |
|---|---|---|
| 1 | 顶部"蓝色栏"未取消 | `<div class="tab-bar" id="tab-bar">` Task 8 加的占位 div(背景 #252525 = 看起来蓝灰),内容空 |
| 2 | 右侧缺会话窗口 | `<div class="tab-content-host">` 同样空 |
| 3 | 所有按钮没功能 | boot init 链某处 throw,后续 init 没跑 |

## 根因假设(按可能性)

**H1:JS boot throw(最可能)** — tab-bar + tab-content-host 已挂 DOM 但内容空 → `initTabManager()` 没跑 → 整个 boot 链 throw 阻断。**最具体嫌疑**:`initSidebarActions()` 行 848-849 没 null 守卫

**H2:WebView2 cache** — W14-G+ 已知问题,强清可能漏 `EBWebView` 子目录

**H3:race condition** — 可能性低

## 你(新会话)的第一步

1. **读 handoff §0.5**:`F:/soft/00softmediasoft00selfmade-media-to-doc-ui/handoff-w15-a-task12-build-verify-2026-07-25.md`
2. **读 plan / spec / ledger**:`docs/superpowers/plans/2026-07-24-w15-a-ux-redesign.md` + `docs/superpowers/specs/2026-07-24-w15-a-ux-redesign-design.md` + `.superpowers/sdd/2026-07-24-w15-a-ux-redesign/progress.md`
3. **诊断 throw 点**(H1):在 `src/index.html` 的每个 init 函数开头包 `try { ... } catch (e) { console.error('[init] <fn>', e); }`,定位实际 throw 行(可能 `initSidebarActions()` 行 848-849 `$(...).addEventListener(...)` 在某种 DOM 时序下 `$` 返回 null)
4. **真机再跑**:`cd src-tauri && cargo build --release`(快,~2 min);用户重装 1.4.2(强清 `EBWebView` 兜底,见 handoff §0.5 H2);F12 看 console
5. **修 + 再 build + 强清缓存装机**

## 绝不要做

- **不 commit / push / release**(加快模式)
- **不动后端 Rust**(`src-tauri/src/*.rs` 零行变更)
- **不 reset / checkout / restore / 覆盖任何已工作区内容**
- **不 bump version**(仍是 v1.4.2,T8 release 才 bump v1.5.0)
- **不删 handoff / prompt**

## 关键参考

- **Handoff**:`handoff-w15-a-task12-build-verify-2026-07-25.md` §0.5(本反馈)+ §3(13 项验收清单)+ §4(known parked Minor)
- **Plan §8 review**:Tasks 9/10/11 实现完整,已通过 reviewer + 2 个 fix round
- **Ledger**:`.superpowers/sdd/2026-07-24-w15-a-ux-redesign/progress.md` Tasks 1-11 ✅,Task 12 in progress
- **W14-G+ cache 经验**:`feedback_cargo_ssl_mitm.md` + `feedback_tauri_dev_static_server.md`(已知 WebView2 缓存问题)

## 预期交付

- 诊断 H1/H2 哪个为真
- 修 H1 的具体代码改动 + build + 强清装机
- 13 项验收重跑,≥11/13 PASS → 写 `handoff-w15-a-t7-1-redesign-complete-2026-07-25.md` 接力 T8 release session
- <11/13 PASS → 写 `handoff-w15-a-t7-1-blocked-2026-07-25.md`
- **仍不 commit**(加快模式)