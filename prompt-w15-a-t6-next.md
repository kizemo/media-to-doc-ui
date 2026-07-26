# W15-A T7 — 手动验收 + NSIS 重打接力 prompt

承接 `F:/soft/00selfmade/media-to-doc-ui/handoff-w15-a-t6-complete-2026-07-24.md`,W15-A T6 已完成(98/98 pass,前端 Settings tab + Providers UI 实装),下一会话进入 T7。

## 当前状态
- 分支:`feat/w15a-llm-api-settings`(基线 db84639,不动)
- 已完成:T1+T2+T3+T4+T5+T6(累计 98 / 98,前端无新 Rust 测试)
- T7 改动待 commit(如果验收失败需修),W15-A 整体一次 commit

## T7 必交付(plan Task 8 + spec §8)
1. `cargo test --lib` 确认 98/98 仍 pass
2. `cargo tauri build` 重打 NSIS installer
3. 桌面安装 + 13 项手动验收(用户执行,因为 sandbox-verify 受 Win11 沙箱功能阻塞)
4. 含 v0.1.0 badge regression 复检(主窗口 title 显 v1.5.0)
5. 写 `handoff-w15-a-t7-complete-2026-07-24.md`(如果验收失败,先修后写)

## 避坑
- 13 项验收清单见 spec §8 + plan Task 8;每项独立判断
- 失败项不阻塞其它项,记录后下一会话修
- 测试连接成功的关键链路:填密钥 → 后端 save_llm_profile 写 keyring → test_llm_connection 读 keyring → HTTP probe
- env var 注入:T7 步骤 7-8 需在 Run pipeline 后查 mtd.log 看 `[env] OPENAI_API_KEY=sk-...(4 chars)` 等

## 执行顺序
跑通 13 项 → 写 handoff → 等用户拍板是否进 T8(v1.5.0 release)

## 绝不要做
- 不 commit / push / release / reset 未提交 / 改主仓 / 删旧 handoff prompt
- 不 bump version 进 v1.5.0(T8 才做)
- 不启 sandbox feature / 不强求 sandbox-verify(已知受 Win11 限制)

## 必读
1. 本文件 2. `handoff-w15-a-t6-complete-2026-07-24.md` 3. spec §8 + plan Task 8 4. `docs/RELEASE_NOTES_v1.4.0.md`(参考 changelog 风格)5. `src-tauri/tauri.conf.json`(version 字段位置)6. `git status --short --branch`

会话预算:<2h 活跃,撞墙立即写 handoff。