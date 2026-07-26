# Handoff — W15-A T7.2 Partial Complete (主仓 done, Tauri side deferred)

**日期**: 2026-07-25
**项目**: `F:/soft/00selfmade/media-to-doc`(主仓) + `F:/soft/00selfmade/media-to-doc-ui`(Tauri 子仓)
**当前分支**:
- 主仓:`feat/w15a-t7-2-task4-imagegen`(T4 implementer 创建;无 commit)
- Tauri UI:`feat/w15a-llm-api-settings`(无 T7.2 Tauri 代码改动;只有 spec/plan 文件)
**承接**: `handoff-w15-a-task12-build-verify-2026-07-25.md` §0.6 / §6 + `docs/superpowers/specs/2026-07-25-w15a-t7-2-product-feedback-design.md` + `docs/superpowers/plans/2026-07-25-w15a-t7-2-product-feedback.md`

---

## 0.5 关键架构发现(中途确认)

**主仓 vs Tauri UI 存储设计不同**:

| 项 | 主仓 (`media_to_doc/`) | Tauri UI (`media-to-doc-ui/`) |
|---|---|---|
| Profile 元数据 | **不存在**(主仓无 profile 注册表) | `%APPDATA%/com.duanyi.mediatodoc/llm_profiles.json` |
| API key 存储 | `LLMConfig.api_key_ref`(**DPAPI 加密**) | `keyring` crate,service=`media-to-doc-ui`,name=`profile:<n>` |
| Provider 名 | Literal `ollama` / `anthropic` / `openai_compatible` | 9 个(MiniMax / DeepSeek / Ollama 等) |
| Profile name 解析 | **不解析**(用 `LLM_*` env vars) | Tauri 查 metadata + keyring → 写 `spec.env_vars` → spawn 注入 |

**结论**:
- **Tauri UI 是 profile 单一真相源**(已在 W14-D + T5 实装)
- 主仓不需要 `LLMConfig.from_profile_name` 或 keyring 集成
- `--llm-profile-name` / `--image-agent-profile-name` 在主仓**仅作存储 + log**,key 通过 env vars 注入
- 这是 spec 的设计缺陷(plan 假设主仓能读 profile name 解析);用户已确认「接受现实」

**对 T2-T4 主仓任务的影响**:
- **T2**(`LLMConfig.from_profile_name`):简化为 trivial——**已跳过**(`llm_profile_name` 字段 T1 已加;无 `from_profile_name` / keyring 工作)
- **T3**(`task_text` 落 state.json + 注入 prompt):**完整实装**
- **T4**(`imagegen` 策划 LLM):**简化实装**——`_plan_prompts` 用 `cfg.image_agent_profile_name` 仅作 gate;真出图仍走 `cfg.llm` 现有 provider(env vars 已注入)

---

## 1. 完成清单

### 主仓 `F:/soft/00selfmade/media-to-doc/`(在 `feat/w15a-t7-2-task4-imagegen` 分支,working tree only)

| Task | 状态 | 测试 | 备注 |
|---|---|---|---|
| T1 | ✅ 完成 | 4/4 新 + 604→608 全量 | `cli.py`(Typer)+ `config.py` 加 3 字段 3 flag;`mtd resume` 顺手补 `config=` 透传 |
| T2 | ⏭️ 跳过 | — | trivial:`llm_profile_name` / `image_agent_profile_name` 字段已存;无 keyring 工作 |
| T3 | ✅ 完成 | 7/7 新 + 608→615 全量 | `state.py` `task_text` 字段;`chapters.py` / `draft.py` 加 `USER_INSTRUCTION:` 前缀注入;`runner.py` 写入 state |
| T4 | ✅ 完成 | 5/5 新 + 615→620 全量 | `ImagePlan` dataclass;`LocalSdxlProvider` 改返 `[]`+ warn(删 0-byte placeholder);`SkipProvider` 写 `image_plans.json`;`_plan_prompts` 用 profile name 作 gate;`generate_images()` 集成 |

**主仓测试统计**:**620 passed / 0 failed / 0 skipped**(原 604 + 16 新)

### Tauri UI `F:/soft/00selfmade/media-to-doc-ui/`(在 `feat/w15a-llm-api-settings` 分支,无 T7.2 代码改动)

| Task | 状态 | 备注 |
|---|---|---|
| T5 runner args | ⏭️ deferred | 未开始 |
| T6 commands per-run profile(2 轮 review) | ⏭️ deferred | **2 轮 review 必跑** |
| T7 project registry + dialog(2 轮 review) | ⏭️ deferred | **2 轮 review 必跑** |
| T8 frontend New Run 大改 | ⏭️ deferred | 大文件改动 |
| T9 long-doc snapshot + sync/verify 脚本 | ⏭️ deferred | 独立,可快速跑 |
| T10 longdoc.py 读 vendored + pyproject | ⏭️ deferred | |
| T11 Tauri bundle.resources + Claude hook | ⏭️ deferred | |
| T12 全面验证 + handoff | ⏭️ deferred | |

### 文档(spec/plan 已 commit 等价)

- ✅ `docs/superpowers/specs/2026-07-25-w15a-t7-2-product-feedback-design.md`(brainstorming 产出,**未 commit**,在 Tauri UI 工作树)
- ✅ `docs/superpowers/plans/2026-07-25-w15a-t7-2-product-feedback.md`(12 task 计划,未 commit)

---

## 2. 主仓改动文件清单(working tree, 未 commit)

```
M  CLAUDE.md                                              (文档,跨多次会话累积)
M  src/media_to_doc/cli.py                                (T1: 加 3 typer.Option + cfg 赋值)
M  src/media_to_doc/config.py                             (T1: WorkflowConfig 3 字段)
M  src/media_to_doc/state.py                              (T3: task_text + image_plans 字段 + to_dict/load)
M  src/media_to_doc/pipeline/chapters.py                  (T3: _build_chapter_user_prompt helper + 注入)
M  src/media_to_doc/pipeline/draft.py                     (T3: _build_draft_user_prompt helper + 注入)
M  src/media_to_doc/pipeline/runner.py                    (T3 + T4: cfg.task_text → state; plans → state)
M  src/media_to_doc/pipeline/imagegen.py                  (T4: ImagePlan + Skip/LocalSdxl + _plan_prompts + generate_images 集成)
M  task.md                                                 (各 task 进度行)
?? docs/media-to-doc.png                                   (历史遗留)
?? "docs/电商店铺表.md"                                    (历史遗留)
?? tests/test_cli_profile_flags.py                         (T1: 4 tests)
?? tests/test_state_task_text.py                           (T3: 3 tests)
?? tests/test_chapters_prompt.py                           (T3: 4 tests = 2 chapter + 2 draft)
?? tests/test_imagegen_provider.py                         (T4: 3 tests)
?? tests/test_imagegen_planner.py                          (T4: 2 tests)
```

**未 commit**(加快模式遵守)。

---

## 3. Tauri UI 改动文件清单(working tree, 未 commit)

```
M  docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md   (历史)
M  docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md  (历史)
M  src-tauri/Cargo.toml                                          (历史 T1-T11)
M  src-tauri/capabilities/default.json                           (历史)
M  src-tauri/src/commands.rs                                     (历史)
M  src-tauri/src/lib.rs                                          (历史)
M  src-tauri/src/runner.rs                                       (历史)
M  src-tauri/tauri.conf.json                                     (历史)
M  src/index.html                                                (历史)
?? docs/superpowers/plans/2026-07-25-w15a-t7-2-product-feedback.md         (本会话新)
?? docs/superpowers/specs/2026-07-25-w15a-t7-2-product-feedback-design.md  (本会话新)
?? handoff-w15-a-t7-2-partial-mainrepo-2026-07-25.md            (本文件)
?? + 多个历史 handoff 文件
```

**关键**:**Tauri UI 无任何 T7.2 代码改动**。只有 spec/plan 文档 + 历史 handoff。后续 T5-T11 才会改 `src-tauri/` 和 `src/index.html`。

---

## 4. 主仓 mainrepo branch 状态(用户必知)

主仓当前在 **`feat/w15a-t7-2-task4-imagegen`** 分支(T4 implementer 创建)。
- 这是 feature 分支,不是 master,合规(用户 CLAUDE.md §5.4)
- 但与 Tauri UI 的 `feat/w15a-llm-api-settings` 不对齐
- **下一步**:用户在 v1.x.x 主仓 release 时,应把本分支合到 master(或者 cherry-pick T1+T3+T4 commits 到合适的 release 分支)
- 后续会话接力时,如果继续做 T5-T11(纯 Tauri),**不需要**主仓分支切换;如果继续做主仓 T2/T5+ 应保留本分支或合并回 master

---

## 5. Tauri side(T5-T11)设计要点(下一会话必读)

主仓 → Tauri 的 flag 透传:
- `--llm-profile-name`:`mtd run --llm-profile-name foo` → `WorkflowConfig.llm_profile_name = "foo"`(已 work)
- 但**主仓不会自己查 profile**(无 registry);Tauri 是唯一真相源
- 所以 `runner.rs build_mtd_run_args` 加这 flag,**目的仅是 logging / debug**(让 spawn cmd line 显示用户选了哪个 profile)
- 真实认证走 `spec.env_vars`(T5/T6 已有的 `inject_profile_env` 路径)

`--task-text`:主仓已经 work(state.json 落 + prompt 注入)。
- Tauri runner 加 flag 透传(简单加 1 个参数)
- 前端 New Run 加 textarea

`--image-agent-profile-name`:同上,主仓 work,仅 logging。

Tauri side 任务分解建议(下一会话可重写 plan):

1. **T5-Tauri**:runner.rs `build_mtd_run_args` / `build_mtd_resume_args` 加 3 flag 透传(5 tests)
2. **T6-Tauri**:commands.rs `run_pipeline` / `resume_pipeline` 加 3 参数 + `inject_profile_env(spec, llm_profile_name_opt)` + 5 tests(2 轮 review)
3. **T7-Tauri**:tauri-plugin-dialog + project registry 4 commands + 5 tests(2 轮 review)
4. **T8-Tauri**:前端 `__mountNewRunTab__` 大改 + `__projectTree__.refresh()`(手动 verify)
5. **T9-main仓**:long-doc snapshot + sync/verify 脚本 + 5 tests(独立)
6. **T10-main仓**:longdoc.py 读 vendored + pyproject + 3 tests(独立)
7. **T11-Tauri**:tauri.conf.json `bundle.resources` + Claude hook(手动 verify)
8. **T12-verify**:cargo test --lib ≥113 + uv run pytest ≥620 + cargo tauri build + handoff-complete

---

## 6. 下一会话接力点(Subagent-Driven 或 Inline 都行)

**新会话第一条 prompt** 建议读:`handoff-w15-a-t7-2-partial-mainrepo-2026-07-25.md`(本文件)

**任务优先级**(时间预算紧 → 选最重要的):
1. **T5-Tauri runner args**:简单(纯加 3 参数 + 5 测试,~10 min)
2. **T6-Tauri commands per-run profile**:关键改动,2 轮 review(~30 min)
3. **T7-Tauri project registry + dialog**:关键功能,2 轮 review(~25 min)
4. **T8-Tauri frontend**:大文件改动(~20 min)
5. **T9-main仓 long-doc scripts**:独立,可快速跑(~15 min)
6. **T10-main仓 longdoc vendored**:独立(~10 min)
7. **T11-Tauri bundle + hook**:依赖 Claude Code 行为(~10 min)
8. **T12 verify + handoff-complete**:最后,~10 min

**总预算**:**~130 min = 2.2 hours**。单会话难以跑完,建议:
- 下一会话做 **T5 + T6**(最关键,Tauri commands 改 commands.rs)
- 后续会话做 **T7 + T8**(frontend + registry)
- 后续会话做 **T9 + T10 + T11**(long-doc 整合)
- 最终会话做 **T12 + 装机验证 + v1.5.0 release**

**禁止**(加快模式红线):
- ❌ 不 commit / push / release / bump
- ❌ 不 reset / checkout / restore / 覆盖未提交工作区
- ❌ 不实现定时调度器
- ❌ 不动 Tauri UI `feat/w15a-llm-api-settings` 之外的无关文件
- ❌ 不动主仓 `feat/w15a-t7-2-task4-imagegen` 之外的无关文件
- ❌ 不让 NSIS 运行时依赖 `C:/Users/Duanyi/.claude/`
- ❌ 不把 API key 写 HTML / log / CLI

**加速模式**:**全部 12 task 完成 + 验收通过后,才做一次 feature commit + bump v1.5.0**(T8 release session)。

---

## 7. 验收门槛(全部完成后)

- 主仓:`uv run pytest -q` ≥620 passed
- Tauri UI:`cargo test --lib -q` ≥113 passed
- `cargo tauri build` exit 0
- 7 项新验收(原 spec §7.2):
  1. New Run tab LLM 下拉列 MiniMax 等 profile
  2. Image Agent 折叠面板两层独立
  3. Stop after 各阶段 tooltip 中文
  4. task textarea → state.json.task_text 落
  5. 选目录按钮 → 左侧立即出现
  6. 同路径 add 重复 → 合并
  7. long-doc Skill 改一行 → sync 自动跑过
- P1 自动验收:source Skill 改 → sync → hash 一致;漂移 → verify exit 1
- 13 项既有验收继续 PASS(用户已确认)

**完成后**:`handoff-w15-a-t7-2-product-feedback-complete-2026-07-25.md` + v1.5.0 release session 接力。

---

## 8. 关键文件路径(下一会话必读)

- **本 handoff**:`F:/soft/00selfmade/media-to-doc-ui/handoff-w15-a-t7-2-partial-mainrepo-2026-07-25.md`
- **prompt-next**:见同目录 `prompt-w15-a-t7-2-next.md`(本会话同步生成)
- **Spec**:`F:/soft/00selfmade/media-to-doc-ui/docs/superpowers/specs/2026-07-25-w15a-t7-2-product-feedback-design.md`
- **Plan**:`F:/soft/00selfmade/media-to-doc-ui/docs/superpowers/plans/2026-07-25-w15a-t7-2-product-feedback.md`
- **承接 handoff**:`F:/soft/00selfmade/media-to-doc-ui/handoff-w15-a-task12-build-verify-2026-07-25.md`
- **各 task 报告**:`F:/soft/00selfmade/media-to-doc-ui/.superpowers/sdd/task-{1,3,4}-report.md`
- **build 产物**:仍 v1.4.2(`F:/soft/00selfmade/media-to-doc-ui/src-tauri/target/release/bundle/nsis/media-to-doc_1.4.2_x64-setup.exe`,T7.2 不重建)
- **sandbox-verify**:`F:/soft/00selfmade/sandbox-verify/media-to-doc-ui/mtd-verify.ps1`(T8 用,T7.2 不必跑)

---

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>