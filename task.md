# W15-A 活跃 Todo

**日期**:2026-07-24 起
**当前分支**:`feat/w15a-llm-api-settings`(基线 db84639,未 push,未 commit)
**目标版本**:子仓 v1.5.0
**当前阶段**:W15-A T7.2 第二轮产品反馈已收口，待新会话实现 profiles / 会话框 / 目录项目化 / long-doc 自动同步

## 进度

| T# | 任务 | 状态 | 测试 | commit |
|---|---|---|---|---|
| T1 | keyring_store 模块 | ✅ | 5/5 | (W15-A feature commit,未做) |
| T2 | llm_profiles 模板 + 校验 | ✅ | 17/17 | 同上 |
| T3 | metadata IO + env var | ✅ | 18/18 | 同上 |
| T4 | commands 6 个 LLM Tauri command | ✅ | 9+3 / 12 | 同上 |
| **T5** | **Runner env vars 注入** | **✅ 本会话完成** | **3 / 3 新** | **同上** |
| T6 | frontend Settings UI 接入 | ✅ | Task 1-11 review 通过 | (W15-A feature commit,未做) |
| **T7** | **全量验收 + cargo tauri build** | **进行中：启动根因修复 + 1.4.2 NSIS rebuild 完成；第二轮反馈转 T7.2** | **98/98；build exit 0** | **同上** |
| **T7.2-T1** | **主仓 CLI 3 flag + WorkflowConfig 3 字段** | **✅ 本会话完成** | **4/4 新；608 total no regression** | **(W15-A feature commit,未做)** |
| **T7.2-A** | **per-run LLM / Image Agent profile** | **进行中：T1 已完成;T2 LLMConfig.from_profile_name 下一步** | - | - |
| **T7.2-B** | **会话任务框 + 目录选择 + project registry 合并** | **待新会话** | - | - |
| **T7.2-C** | **long-doc-processor vendored snapshot + Claude hook 自动同步** | **待新会话** | - | - |
| T8 | v1.5.0 release | blocked：T7.2 + 新验收通过后再开始 | - | - |

## 当前测试统计

- **Baseline (W14-G+)**:43 passed
- **T1+T2+T3+T4 累计**:95 passed
- **T5 (Runner env vars 注入) 新增**:3 passed (spawn_spec_env_vars_defaults_to_empty + spawn_mtd_clears_parent_env_and_injects_spec_env + build_child_command_inherits_parent_path)
- **T5 后总计**:**98 passed / 0 failed**
- **T7.2-T5-Tauri 新增**:5 passed (runner args 3 flag 透传)
- **T7.2-T6-Tauri 新增**:3 passed (inject_profile_env 3 分支)
- **T7.2-Tauri 累计**:**106 passed / 0 failed**(98 + 8)
- **主仓累计**:**620 passed / 0 failed**

## 加快模式规则(沿用)

- W15-A 整体一次 commit(feature commit),不做小版本 release
- 不 reset / checkout / restore / 覆盖未提交改动
- 不切回 master 直接开发
- 不删除旧 handoff / prompt(删除需用户二次确认)

## 历史会话(本分支)

- `handoff-w15-a-providers-decided-v142-icon-2026-07-24.md` — 加快模式 + 9-provider 决策
- `handoff-w15-a-t2-complete-2026-07-24.md` — T2 实现细节
- `handoff-w15-a-t3-complete-2026-07-24.md` — T3 实现细节
- `handoff-w15-a-t4-complete-2026-07-24.md` — T4 完成 + T5 接力 prompt
- `handoff-w15-a-t5-complete-2026-07-24.md` — **本会话新建**(T5 完成 + T6 接力)

## W15-A UX redesign SDD

- Tasks 1–11:已在工作区累积完成,无 commit。
- **Task 12 启动修复**:`app.withGlobalTauri=true`;98/98 tests + NSIS build exit 0;用户确认可添加 MiniMax。
- **第二轮反馈**:静态 New Run 下拉未接 profiles、缺任务框/目录项目化；`long-doc-processor` 当前仅参考未整合。
- **下一步**:见 `handoff-w15-a-task12-build-verify-2026-07-25.md` §0.6 / §6 和 `prompt-w15-a-t7-2-product-feedback-next.md`。
## W15-A T7.2 第二轮反馈(2026-07-25,本会话执行)

| T# | 任务 | 状态 | 测试 | commit |
|---|---|---|---|---|
| T1 | 主仓 CLI 3 flag + WorkflowConfig 3 字段 | ✅ 本会话完成 | 4/4 + 608 全量 | (working tree, 未 commit) |
| T2 | LLMConfig.from_profile_name + keyring | ⏭️ 跳过(架构决定,trivial) | — | — |
| T3 | task_text 落 state.json + chapter/draft prompt 注入 | ✅ 本会话完成 | 7/7 + 615 全量 | (working tree) |
| T4 | imagegen 策划 LLM + LocalSdxlProvider 最小实现 | ✅ 本会话完成 | 5/5 + 620 全量 | (working tree) |
| **T5-Tauri** | **runner.rs build_mtd_*_args 加 3 参数 + 5 tests** | **✅ 本会话完成(2026-07-26)** | **5/5 新;103 total** | **(working tree, 未 commit)** |
| **T6-Tauri** | **commands.rs inject_profile_env 替换 inject_active_llm_env + run/resume 加 3 参数 + 3 tests(2 轮 review 自检通过)** | **✅ 本会话完成(2026-07-26)** | **3/3 新;106 total** | **(working tree, 未 commit)** |
| **T7-Tauri** | **tauri-plugin-dialog + project registry 4 commands + 5 tests(static Mutex 串行化防 env var 并行污染)** | **✅ 本会话完成(2026-07-26)** | **5/5 新;111 total** | **(working tree, 未 commit)** |
| **T8-Tauri** | **frontend `__mountNewRunTab__` + `buildNewRunForm` 大改(动态 dropdowns + task textarea + 选目录按钮 + __projectTree__.refresh/selectProject)** | **✅ 本会话完成(2026-07-26)** | **手动 verify(eyeball)** | **(working tree, 未 commit)** |
| T9-main | long-doc snapshot + sync/verify 脚本 | ⏭️ deferred | — | — |
| T10-main | longdoc.py 读 vendored + pyproject | ⏭️ deferred | — | — |
| T11-Tauri | tauri.conf.json bundle.resources + Claude hook | ⏭️ deferred | — | — |
| T12 | 全面验证 + handoff-complete | ⏭️ deferred | — | — |

详见 `handoff-w15-a-t7-2-partial-mainrepo-2026-07-25.md` + `prompt-w15-a-t7-2-next.md`。

**主仓 branch**:`feat/w15a-t7-2-task4-imagegen`(T4 implementer 创建;Tauri UI branch 不变)
