# W15-A Task 5 交接：侧栏 §3 Search

日期：2026-07-24
状态：DONE

## 已完成

- 在侧栏 §2 Fixed Actions 后插入 §3 Search DOM：搜索输入框、刷新按钮、清空按钮。
- 追加搜索区布局、输入框、图标按钮 hover 与侧栏折叠态 CSS。
- 新增 `initSidebarSearch()`：
  - 输入时 trim、转小写并调用 `window.__projectTreeFilter__?.(query)`。
  - 刷新按钮调用 `window.__refreshProjectTree__?.()`。
  - 清空按钮清空 query 并重新应用过滤。
  - `Cmd+K` / `Ctrl+K` 阻止浏览器默认行为并聚焦搜索框。
- Boot 阶段已调用 `initSidebarSearch()`。
- Task 6 globals 均使用 optional chaining，Task 6 尚未注入时不会产生 ReferenceError。
- 所有既有 `.tab-pane` 均保留。

## 改动文件

- `F:/soft/00selfmade/media-to-doc-ui/src/index.html`

## 验证证据

命令：

```bash
cd src-tauri && cargo build --release 2>&1 | tail -5
```

结果：

```text
warning: `media-to-doc-ui` (lib) generated 5 warnings
Finished `release` profile [optimized] target(s) in 2m 57s
```

0 errors，5 个 baseline warnings。

## 约束遵守

- 未执行 commit / add / push / reset / checkout / restore。
- 未 bump version。
- 未修改 Rust `.rs` 文件。
- 未触碰主仓 `F:/soft/00selfmade/media-to-doc/`。
- 除任务明确要求的 handoff/report 外，未新增实现文件。

## 下一步

继续 Task 6：实现侧栏 §4 Project Tree，并注入：

- `window.__projectTreeFilter__(query)`
- `window.__refreshProjectTree__()`

Task 6 应复用本任务 DOM IDs，不要重复创建搜索控件或重复绑定快捷键。
