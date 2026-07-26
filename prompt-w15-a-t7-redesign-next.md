# 接力 prompt — W15-A T7.1 Settings 修 bug + UX 重设计

**承接**:`F:/soft/00selfmade/media-to-doc-ui/handoff-w15-a-t7-failed-redesign-pivot-2026-07-24.md`

## 当前状态
- 分支:`feat/w15a-llm-api-settings`(基线 `db84639`,未 push,未 commit)
- 累计未提交改动:T1 race 修复 + T1-T6 原 feature + Settings UI
- 测试:98/98 pass;build:v1.4.2 NSIS 已出炉(2,612,797 bytes)

## 必交付(顺序)
1. **修 Settings 点击 bug**:
   - 删 `%LOCALAPPDATA%\com.duanyi.mediatodoc\` + `%APPDATA%\com.duanyi.mediatodoc\` 验证 cache
   - `src-tauri/capabilities/default.json` 显式列 6 LLM commands
   - `src/index.html` `<script>` 顶部加 `window.addEventListener('error', ...)` 写 init.log
2. **删 5 tab + 删蓝色 header**:`src/index.html` 行 358-365 nav-item 删 5 个 + 行 349-356 `<header>` 整段删
3. **重建 Claude Code Haha 侧栏**:顶部 logo+collapse / 3 固定项(新建会话/定时任务/技能市场)/ 搜索框 / 项目树(按 inbox 课程名分组)/ 底部设置齿轮
4. **重 build + 装机 + 跑 13 项验收**(`cargo tauri build` 一次,装机后由用户跑 13 项;原 5 tab 验收改成"项目树展开/折叠 + Settings 链路")
5. **写 handoff**:`handoff-w15-a-t7-1-redesign-complete-2026-07-24.md`(成功)或 `-blocked-2026-07-24.md`(仍卡)

## 绝不要做
- 不 commit / push / release / reset 未提交 / 改主仓 / 删旧 handoff prompt
- 不 bump version 进 v1.5.0(T8 才做)
- 不启 sandbox feature
- 不删 5 tab 后端命令(list_courses / run_pipeline / list_outputs / get_run_metrics / list_runs 保留)
- 不删 T6 的 Settings 4 子页布局(只加侧栏 + 删 header)

## 关键参考路径
- 原 T6 实装:`handoff-w15-a-t6-complete-2026-07-24.md` §3(代码层细节)
- 加快模式 + 9 provider 决策:`handoff-w15-a-providers-decided-v142-icon-2026-07-24.md` §1
- 13 项验收口径(spec §8):`docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md` 行 350-366(部分项需改)
- 必查:`src-tauri/capabilities/default.json` + `src/index.html` 行 349-365 / 485-579

## 会话预算
<2h 活跃,撞墙立即写 handoff-*-blocked-*.md,不超时续命。
