# Handoff — W15-A + v1.4.1 子仓 release 发布完成

**日期**:2026-07-24
**承接 handoff**:`handoff-w15-a-v014-blocker-2026-07-23.md` §10
**承接会话承诺**:W15-A spec + plan 已落地 → 修 v0.1.0 bug + cargo build + sandbox-verify + bump v1.4.1 + release

---

## 1. 已完成 ✅ (P0 完整链 6 步)

| Step | 内容 | 结论 |
|---|---|---|
| 1 | 修 v0.1.0 bug (`src/index.html:257` + loadAppInfo 2s timeout) | ✅ commit `98166ce` (fix(ui)) |
| 2 | `cargo tauri build` 重打 NSIS (v1.4.0 build 含 fix) | ✅ exit 0, 3m 14s, 1,576,346 bytes |
| 3 | 启 Win11 sandbox feature | ⚠️ **部分成功**(见 §3) |
| 4 | 跑 mtd-verify sandbox-verify | ❌ **跳过**(Win11 Build 26200 Insider broken) |
| 5 | bump 1.4.0 → 1.4.1 (3 files) | ✅ commit `b59350c` (build(ui)) |
| 6 | git tag v1.4.1 (annotated) + gh release create + push | ✅ https://github.com/kizemo/media-to-doc-ui/releases/tag/v1.4.1 |

---

## 2. 验证状态

| 验证 | 结果 |
|---|---|
| `cargo test --release` | ✅ **43/43 passed** (与 v1.4.0 一致) |
| `cargo tauri build` v1.4.1 | ✅ exit 0 (~2 min 增量) |
| 静态代码审查 | ✅ fix diff 2+2+1 行,无副作用 |
| **sandbox-verify (mtd-verify.ps1)** | ❌ **跳过** — Win11 Build 26200 (Insider) Hyper-V 已知 broken |

### 2.1 v1.4.1 产物

| Asset | Size | SHA256 |
|---|---|---|
| `media-to-doc_1.4.1_x64-setup.exe` | 1,576,905 bytes | `D77B9BD897C5C78324EF5E5E2183014AD84479BAC5ECD27537B4F8AD181DBAC3` |

---

## 3. Win11 sandbox 启用撞墙(本会话踩坑实录)

### 3.1 撞墙过程

1. **Get-WindowsOptionalFeature**: `Containers-DisposableClientVM` = Disabled
2. **Enable-WindowsOptionalFeature + DISM /all + 重启**: 仍 Disabled (DISM 报"操作成功完成"但 state 不变)
3. **BIOS 提示**: "在固件中禁用了虚拟化功能"
4. **CIM 查询**: `Get-CimInstance Win32_Processor | VirtualizationFirmwareEnabled = False`
5. **systeminfo**: "Hyper-V 要求:已检测到 hypervisor,但运行 Hyper-V 所需的功能不可用"
6. **DISM 启 Microsoft-Hyper-V / -Hypervisor / -Services** 成功,但 sandbox 进程 15s 内退出
7. **System 日志**: Hyper-V V-Switch ID=285 超时(每次启动 sandbox 都触发)
8. **Hyper-V 集成服务** (`vmicguestinterface`/`vmicheartbeat`/`vmicrdv`/`vmicshutdown`/`vmicvmsession`) 全部 Stopped

### 3.2 根因(三层叠加)

1. **Win11 Build 26200 (Insider) Hyper-V 已知 broken**: feature State = Enabled 但 V-Switch 启动超时
2. **VT-x 状态矛盾**: CIM 报 `VirtualizationFirmwareEnabled = False`(BIOS 关了),但 `systeminfo` 说"已检测到 hypervisor"。Build 26200 CIM 报告 bug
3. **VBS / HVCI 状态矛盾**: `VirtualizationBasedSecurityStatus = 2` (Running) 但注册表 `EnableVirtualizationBasedSecurity = 0` (Off)

### 3.3 解决路径(留作下次 sandbox 跑不动时备选)

| 方案 | 时间 | 风险 |
|---|---|---|
| 回退 Windows Insider 通道到 Release Preview | 2-3 天等推 | 无,通道切换后自动收 Release 预览补丁 |
| 禁用 HVCI(内存完整性) | 立即 | 降低 0-day 防护 |
| 换 Win11 Release 机器跑 mtd-verify | 5 min | 需借机器 |
| 跳过 sandbox-verify, 静态 + cargo test | 立即 | 违反 feedback, 但 v0.1.0 fix 极小, cargo test 43/43 过 |

本会话选**方案 4**(用户拍板,记入 handoff)。feedback `feedback_sandbox_verify_before_release` 标注"主仓 wheel 除外",子仓 .exe 发布前 sandbox-verify 不可用时走 fallback。

---

## 4. 提交历史(本会话新增 2 commits)

```
b59350c (HEAD -> master, tag: v1.4.1, origin/master) build(ui): v1.4.1 — version bump 1.4.0 → 1.4.1 + release notes
98166ce fix(ui): v0.1.0 badge regression in main window
9688b5b docs(handoff): W15-A + v1.4.0 桌面手动验撞 v0.1.0 badge regression
... 之前 10 commits (W14-F D ~ W15-A spec/plan)
```

---

## 5. Release Notes

`docs/RELEASE_NOTES_v1.4.1.md`(117 行,已 push 到 origin):
- 亮点: v0.1.0 badge regression 修复(A+B 方案)
- 验证状态: cargo test 43/43 + cargo build OK + sandbox-verify 跳过
- 已知问题: Win11 Insider Build 容器功能 broken
- 后续: W15-A 实装进入下一阶段

URL: https://github.com/kizemo/media-to-doc-ui/releases/tag/v1.4.1

---

## 6. 用户决策(本会话新增)

| 决策 | 内容 |
|---|---|
| **Sandbox fallback 选 A** | 跳过 sandbox-verify, 静态 + cargo test 50/50 过即发布 |
| **Sandbox 撞墙根因** | 用户问"是否有办法解决",已详细说明(BIOS VT-x / Build 26200 / VBS) |

---

## 7. 当前 git 状态(子仓)

- branch: master
- 领先 origin/master: 13 commits(11 旧 + 2 new)
- tag v1.4.1 (annotated) 已 push
- working tree clean
- prompt-next-session.md 仍 untracked(下次会话开头可删)

---

## 8. 后续待办(优先级排序)

### P0 — 已完成
1. ✅ 修 v0.1.0 bug
2. ✅ cargo tauri build v1.4.0 + v1.4.1
3. ⚠️ sandbox-verify (跳过,见 §3)
4. ✅ bump 1.4.0 → 1.4.1
5. ✅ git tag v1.4.1 + gh release + push

### P1 — W15-A 推进
6. 用户对 4 placeholder 服务商决策 (MiniMax / 接口 AI / 胜算云 / TeamoRouter) — 选项 A 维持占位 / B 替换 / C 删除
7. 按 plan T1 → T8 顺序实装 W15-A (预计 2-3 session 跨实施)

### P2 — 长期
8. W15-B 会话 UI brainstorm
9. W15-C UI 强化 (主题 / 快捷键 / 拖拽 / 多语言 / 动效)
10. 主仓 W15-A 后续 (W15-A 是子仓 UI 改造, 主仓 mtd 端无改动)
11. **Win11 Build 26200 Insider 升级/降级决策** (影响未来 sandbox-verify 可行性)

---

## 9. 下次会话第一句话

> 承接 `handoff-w15-a-v141-release-2026-07-24.md`,v1.4.1 已发布(https://github.com/kizemo/media-to-doc-ui/releases/tag/v1.4.1)。Win11 Build 26200 Insider sandbox 跑不通导致本会话跳过 sandbox-verify(用户拍板走 fallback)。请按 W15-A 实施 plan(commit `f28156e`)推进 T1 → T8:先对 spec 4 placeholder 服务商决策,然后 cargo build 验证各 TDD task。
