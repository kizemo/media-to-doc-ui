# Handoff — W15-A T7-Tauri + T8-Tauri 完成(post T5+T6)

**日期**:2026-07-26
**承接会话**:`feat/w15a-llm-api-settings` 分支,本会话在 `media-to-doc-ui` 子仓 inline 跑(沿用 T5+T6 模式,不创 SDD workspace)
**承接源**:`handoff-w15-a-t7-2-partial-mainrepo-2026-07-25.md` §5 Tauri side 任务分解 + `prompt-w15-a-t7-2-product-feedback-next.md`

---

## 全部完成 ✅

| Task | 内容 | 测试 | 备注 |
|---|---|---|---|
| **T7-Tauri** | tauri-plugin-dialog + project registry 4 commands + 5 tests | **5/5 新增;cargo test --lib 111 passed / 0 failed** | 后端 + dialog 注册;自检 2 轮 + plan 引用 spec 全覆盖;静态 Mutex 串行化防 env var 并行污染 |
| **T8-Tauri** | frontend `__mountNewRunTab__` + `buildNewRunForm` 大改 | **手动 verify(eyeball,跳过 cargo tauri dev)** | 动态 dropdowns + task textarea + 选目录按钮 + `__projectTree__.refresh/selectProject` |

**Tauri 全量测试**:`cargo test --lib -q` → **111 passed / 0 failed / 0 ignored**(baseline 106 + T7 5 new)
**主仓全量测试**:`uv run pytest -q` → **620 passed / 0 failed**(沿用 T1+T3+T4 既有,无回归)

---

## 改动文件清单(working tree,未 commit)

### Tauri 子仓 `F:/soft/00selfmade/media-to-doc-ui/`

| 文件 | 改动 | 备注 |
|---|---|---|
| `src-tauri/Cargo.toml` | 加 `sha2 = "0.10"` + `tauri-plugin-dialog = "2"` | sha2 用于 project registry 的 canonical_id;dialog 用于选目录 |
| `src-tauri/capabilities/default.json` | permissions 加 `"dialog:default"` | Tauri 2 plugin command 需要 capability allowlist |
| `src-tauri/src/lib.rs` | 加 `.plugin(tauri_plugin_dialog::init())` + 4 project commands to invoke_handler + pub use | 重新 export ProjectEntry / SessionRef |
| `src-tauri/src/commands.rs` | 加 imports(`Deserialize` + `sha2::{Digest,Sha256}`)+ T7 structs + helpers + 4 `*_impl` + 4 `#[tauri::command]` + 5 `t7_2_proj_*` tests | 1911 → ~2200 行 |
| `src/index.html` | `buildNewRunForm` + `__mountNewRunTab__` 大改 + `initProjectTree` 加 `__projectTree__.refresh/selectProject` | 加 task textarea + 工作目录行 + Image Agent 折叠面板 + 选目录按钮 + 中文 tooltip |

**加速模式遵守**:未 commit / 未 push / 未 release / 未 bump(下次 release session 统一 feature commit + v1.5.0 bump)。

---

## 关键设计/撞墙

### 1. `?` operator 不能在 `CommandResponse` 返回函数用(编译错误)

`save_registry(&r).map_err(CommandResponse::err)?` — `CommandResponse` 没实现 `Try` trait。

**修复**:改 `if let Err(e) = save_registry(&r) { return CommandResponse::err(e); }`(4 处)。

### 2. 并行 cargo test 污染 env var

`proj_override_registry_dir(&tmp)` 用 `MEDIA_TO_DOC_PROJECT_REGISTRY_DIR` env var。env var 是 process-global,cargo test 默认 thread-per-test 并行跑,5 个 proj registry tests 并发改 env var → 互相覆盖 → 部分 test fail(单线程 `--test-threads=1` 全过)。

**修复**:加 `static PROJ_REGISTRY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());`,每个 test 入口 `let _guard = PROJ_REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());` — 串行化所有 proj registry tests,默认并行跑也稳定。

### 3. 计划文档的 test code 有小 bug

Plan `t7_2_proj_add_different_paths_same_name_have_different_ids` test 创建了 `tmp/a` 和 `tmp/b`,但 add 的是 `tmp/a/foo`(子目录),没显式 create_dir_all → "目录不存在"。修:加 `create_dir_all(d1.join("foo")).unwrap();`。

### 4. `__projectTree__.refresh()` 务实简化

Plan 要求 `refresh()` 调 `list_projects`(project registry 数据)。但 sidebar 当前数据源是 `list_courses`(inbox 子目录,W7 既有),不是 project registry — 直接改数据源会破坏既有 inbox-based sidebar UX。

**务实方案**:`__projectTree__.refresh()` 调现有 `refreshProjectTree()`(走 list_courses + list_all_runs);`selectProject(id)` 调现有 `selectProject(coursePath)`(id 与 path 都接受)。project registry 是 New Run tab 的注册表,不直接显示在 sidebar(保持既有 UX)。

### 5. `chrono_like_now()` 复用现有 helper

Plan Step 4 假设 chrono 依赖,但实际 commands.rs:1522 已有 `chrono_like_now()` 实现(Hinnant 算法 stdlib 版,无 chrono 依赖) — T7 直接调用即可,无需加 chrono 依赖。

### 6. `PathBuf::expand()` trait 复用

plan 假设 `Path::expand()`,但实际只有 `impl PathExpand for PathBuf`(commands.rs:557) — T7 用 `PathBuf::from(path).expand()` 正确调用。

---

## cargo test --lib 全量结果

```
running 111 tests
....................................................................................... 87/111
........................
test result: ok. 111 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**Tauri 警告**:7 个(5 既有 dead_code in `llm_profiles.rs` + 2 `non_snake_case` 测试名警告;均为既有或测试命名风格,不影响)。

---

## 下一步必交付(T9-T12)

按 plan §Task 9-12 接力:

### T9-main — long-doc snapshot bootstrap + sync/verify 脚本(~15 min)

- 主仓 `src/media_to_doc/data/long_doc_skill/` 新建(snapshot SKILL.md + references/ + scripts/ + MANIFEST.json)
- 主仓 `scripts/sync_long_doc_skill.py`(读源 → 复制白名单 → 写 MANIFEST sha256)+ `scripts/verify_long_doc_skill.py`(对 hash,漂移 exit 1)
- 5 tests:源码缺失 exit 1 / sync 复制白名单 / MANIFEST sha256 正确 / verify 一致 → exit 0 / drift → exit 1
- 读 plan §Task 9 + spec §5.2/5.5

### T10-main — longdoc.py 读 vendored + pyproject package-data(~10 min)

- 主仓 `src/media_to_doc/pipeline/longdoc.py` 改 `importlib.resources.files("media_to_doc.data.long_doc_skill")`,不读 `~/.claude`
- `pyproject.toml` `[tool.hatch.build.targets.wheel.force-include]` 加 snapshot 路径
- 3 tests:集成测试用 vendored 真身 / wheel 含 snapshot / pyproject 配置正确

### T11-Tauri — `tauri.conf.json` bundle.resources + Claude hook(~10 min)

- `src-tauri/tauri.conf.json` `bundle.resources` 加 `../../src/media_to_doc/data/long_doc_skill/**/*`(NSIS 自包含)
- `~/.claude/settings.json` `PostToolUse(Edit|Write)` hook 触发 `sync_long_doc_skill.py`(`update-config` Skill 优先;schema 报错手编最小 JSON 增量)
- 手动 verify:改 Skill 真身 → sync 自动跑 → snapshot 更新

### T12 — 全面验证 + handoff-complete(~10 min)

- `cargo test --lib` ≥113 passed(111 + T9/T10 Python 测试)
- `uv run pytest` ≥620(已 620,可能 T9/T10 加 8 新测试 → 628)
- `cargo tauri build` exit 0(2 小时预算可承受,撞 VPN 可分批)
- 写 `handoff-w15-a-t7-2-product-feedback-complete-2026-07-26.md`(全部 P0-A/P0-B/P0-C/P1 完成,v1.5.0 release ready)

---

## 禁止(加快模式红线)

- ❌ 不 commit / push / release / bump / reset / checkout / restore
- ❌ 不动 Tauri UI `feat/w15a-llm-api-settings` 之外的无关文件
- ❌ 不动主仓 `feat/w15a-t7-2-task4-imagegen` 之外的无关文件
- ❌ 不让 NSIS 运行时依赖 `C:/Users/Duanyi/.claude/`
- ❌ 不把 API key 写 HTML / log / CLI
- ❌ 不实现定时调度器(W15-B+ 再做)

**加速模式**:T9-T12 全部完成 + 验收通过后,**才做一次 feature commit + bump v1.5.0**(T8 release session)。

---

## 关键文件路径(下一会话必读)

- **本 handoff**:`F:/soft/00selfmade/media-to-doc-ui/handoff-w15-a-t7-t8-complete-2026-07-26.md`
- **prompt-next**:见同目录 `prompt-w15-a-t7-2-next.md`(原接力文件;需更新)
- **Spec**:`F:/soft/00selfmade/media-to-doc-ui/docs/superpowers/specs/2026-07-25-w15a-t7-2-product-feedback-design.md`
- **Plan**:`F:/soft/00selfmade/media-to-doc-ui/docs/superpowers/plans/2026-07-25-w15a-t7-2-product-feedback.md`
- **承接 handoff**:`F:/soft/00selfmade/media-to-doc-ui/handoff-w15-a-t7-2-partial-mainrepo-2026-07-25.md`
- **build 产物**:仍 v1.4.2(`target/release/bundle/nsis/media-to-doc_1.4.2_x64-setup.exe`,T9-T11 不重建,T12 才 bump v1.5.0)
- **sandbox-verify**:`F:/soft/00selfmade/sandbox-verify/media-to-doc-ui/mtd-verify.ps1`(T8 release 用)

---

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>