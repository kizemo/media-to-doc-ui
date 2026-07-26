# W15-A T5 — Runner env vars 注入接力 prompt

承接 `F:/soft/00selfmade/media-to-doc-ui/handoff-w15-a-t4-complete-2026-07-24.md`,W15-A T4 已完成(95/95 pass),本会话进入 T5。

## 当前状态
- 分支:`feat/w15a-llm-api-settings`(基线 db84639,不动)
- 已完成:T1+T2+T3+T4(累计 95 / 95);T5 改动未 commit,W15-A 整体一次 commit

## T5 必交付(plan Task 4 + spec §5)
1. `SpawnSpec` 加 `env_vars: HashMap<String, String>`(`src-tauri/src/runner.rs`)
2. `spawn_mtd()` 加 `.env_clear().envs(&spec.env_vars)` — 防父进程 HTTP_PROXY 污染
3. `run_pipeline` / `resume_pipeline` 改:`get_active_profile()` → `keyring_store::read_key()` → `to_env_vars()` → `spec.env_vars`
4. `let mut spec = ...`(原 `let spec` 改 mut);≥1 个新单元测试(不真 spawn,只验 env 构造)

## 避坑
- `env_clear()` 清父进程 env(W14-D 撞过 SSL)
- `ACTIVE_PROFILE_REQUIRED:` 错误直接 `return CommandResponse::err(...)` 传播
- 已有 `runner::tests` 6 个改 SpawnSpec 时不能挂

## 执行顺序
TDD:先写 ≥1 测试 → RED → 最小实现 → runner:: 定向全过 → 全量目标 96+ → 独立 review

## 绝不要做
- 不 commit / push / release / reset 未提交 / 改主仓 / 删旧 handoff prompt

## 必读
1. 本文件 2. `handoff-w15-a-t4-complete-2026-07-24.md`(上下文真相)3. spec §5 + §4 4. plan Task 4 5. `src-tauri/src/runner.rs` 6. `commands.rs` 行 ~995-1110 7. `git status --short --branch`

会话预算:<2h 活跃,撞墙立即写 handoff。