# W15-A Task 8 Tab Manager 交接

## §1 改了什么

- `src/index.html`: Task 8 only `+198 -8` lines。
- 主 `<main>` 改为 `#main-area`，新增 `#tab-bar` 与 `#tab-content-host`。
- 保留 `run/output/health/learn/settings` 5 个旧 `.tab-pane` source，供 Tasks 9–11 `cloneNode` 复用。
- 新增 tab bar CSS、Tab Manager、3 类占位 builder、localStorage persist/restore、dedup/focus/close 逻辑。
- Cmd/Ctrl+K listener 已从 `initSidebarSearch()` 删除，并统一迁移到 `initTabManager()`；全文件仅 1 个 keydown listener。
- boot 顺序按用户裁定为 collapse → actions → search → projectTree → settingsGear → tabManager → loadAppInfo。
- `.superpowers/sdd/2026-07-24-w15-a-ux-redesign/task-8-report.md`: 新增 Task 8 报告。

## §2 Build evidence

`cargo build --release 2>&1 | tail -3`:

```text
warning: `media-to-doc-ui` (lib) generated 5 warnings
    Finished `release` profile [optimized] target(s) in 2m 40s
```

`cargo check 2>&1 | tail -3`:

```text
warning: `media-to-doc-ui` (lib) generated 5 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.09s
```

额外验证：embedded module JavaScript 经 `node --input-type=module --check`，exit 0；结构断言确认 5 个 source pane、1 个 keydown listener、Task 8 diff `+198/-8`。

## §3 下一步

- 继续 Task 9：实现 Session Tab 内容与 mount 接缝。
- 不 commit/reset/checkout/restore；继续保留 Tasks 1–8 累积工作区。

## §4 Defect 1 reminder

Tasks 9/10/11 dispatch prompts 必须注入以下 reminder：

> 实现 module-scope `function __mountXxxTab__` 后，必须追加 `window.__mountXxxTab__ = __mountXxxTab__;`。具体为 `window.__mountSessionTab__ = __mountSessionTab__;`、`window.__mountNewRunTab__ = __mountNewRunTab__;`、`window.__mountSettingsTab__ = __mountSettingsTab__;`。否则 Task 8 的 `window.__mountXxxTab__?.(...)` 不会调用 mount 函数，tab 内容将一直停留在占位状态。

## §5 Concerns

- 无新增 Task 8 blocker。
- 已知 `.sidebar` 缺 `display: flex; flex-direction: column;` 的 Task 7 layout gap 按裁定留 Task 12 兜底，本 task 未修改。
