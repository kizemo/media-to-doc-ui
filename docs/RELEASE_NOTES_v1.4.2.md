# Release Notes — media-to-doc-ui v1.4.2

**发布日期**:2026-07-24
**子仓 tag**:`v1.4.2`(annotated)
**承接 handoff**:`handoff-w15-a-v141-release-2026-07-24.md` 续(本会话)

---

## 亮点

### 1. 替换应用图标 — `docs/media-to-doc.png`

之前 v1.4.x 一系列发布的应用图标是 W14-B 临时生成的占位蓝色播放键。本版本正式采用项目 logo `docs/media-to-doc.png`(深蓝底 + 白色播放键 + 文档线条 + 圆角)替换全部图标文件:

| 位置 | 文件 | 改动 |
|---|---|---|
| Windows installer / 桌面 / 任务栏 | `icons/icon.ico` | 67KB → 67KB(重生成) |
| Windows 任务栏 32×32 | `icons/32x32.png` | 229B → 2.3KB(占位替换) |
| Windows 任务栏 64×64 | `icons/64x64.png` | 4112B → 6.4KB |
| Windows 任务栏 128×128 | `icons/128x128.png` | 15KB → 20KB |
| Windows 开始菜单 | `icons/128x128@2x.png` | 62KB → 62KB |
| Tauri icon ref | `icons/icon.png` | 210KB → 157KB |
| Windows Store | 7 个 `Square*.png` | 全部重生成 |
| macOS DMG | `icons/icon.icns` | 1280KB → 1280KB(重生成) |
| iOS / iPadOS | 14 个 `AppIcon-*x*.png` | 全部重生成 |
| Android | 14 个 `mipmap-*/*.png` | 全部重生成 |

源图来自项目 `docs/media-to-doc.png`(128×128 RGBA),用 Pillow LANCZOS 重采样到 1024×1024 后,再由 `cargo tauri icon` 自动生成全套 38 个图标。

### 2. NSIS installer 同步 bump 1.4.1 → 1.4.2

- `src-tauri/Cargo.toml` `version` = `1.4.2`
- `src-tauri/tauri.conf.json` `version` = `1.4.2`
- `src-tauri/nsis/installer.nsi` `PRODUCT_VERSION` = `1.4.2`
- 产出 `media-to-doc_1.4.2_x64-setup.exe`(预计 ~1.58MB,同 1.4.1 安装逻辑)

---

## 验证状态

| 验证 | 结果 |
|---|---|
| `cargo test --release` | **43/43 passed**(与 v1.4.1 一致) |
| `cargo tauri build` | (build 中) |
| 静态代码审查 | ✅ 版本号 + 图标替换,无逻辑改动 |
| **本机装 v1.4.1 → 启动看主窗口 badge** | (待用户桌面手动验,见 §安装) |
| **sandbox-verify** | ⚠️ **跳过** — 同 v1.4.1,Win11 Build 26200 (Insider) 沙箱跑不通 |

---

## Assets

| Asset | Size | SHA256 |
|---|---|---|
| `media-to-doc_1.4.2_x64-setup.exe` | ~1.58MB | (gh release page 显示) |

(无 portable 版本)

---

## 安装

### Windows(installer,推荐)

1. 下载 `media-to-doc_1.4.2_x64-setup.exe`
2. 管理员运行(perMachine 安装)
3. 默认装到 `D:\Program Files\MediaToDoc\`(W14-G+ 决策,D 盘优先;C 盘 fallback)
4. 桌面 / 开始菜单启动 `media-to-doc`
5. **桌面快捷方式图标应为新 logo**(深蓝底白播放键 + 文档线条)
6. 启动后,主窗口 title 区域的 badge 应显示 `v1.4.2`

### 升级路径(v1.4.0 / v1.4.1 → v1.4.2)

- installer:覆盖安装(NSIS 自动卸载旧版)
- 配置 / workspace / inbox 不需变动
- 用户在 v1.4.1 桌面手动验发现的主窗口 badge regression 在 v1.4.1 已修,本版本图标同步

### 卸载

- 控制面板 → 程序和功能 → MediaToDoc → 卸载
- 干净卸载,无残留(NSIS SectionEnd 内 RMDir `$INSTDIR` + DeleteRegKey)

---

## 已知问题

- Rust toolchain 需 1.97+(自带 lld-link 无需 MSVC)
- 公司 VPN 用户构建时需设 `CARGO_NET_TLS_VERIFY=false`(运行不受影响)
- macOS / Linux 编译需用户自查环境
- **Win11 Insider Build 容器功能 broken**(影响本机器 sandbox-verify 跑不动),Release 通道不受影响

---

## 上游

主仓 `media-to-doc` Python 后端 v1.2.1 仍是最新(W15-A 是子仓 UI 改造,主仓无对应改动):
- PyPI:https://pypi.org/project/media-to-doc/
- GitHub:https://github.com/kizemo/media-to-doc/releases/tag/v1.2.1

---

## 后续

- **W15-A(LLM API Settings)** 实装进入下一阶段(spec + plan 已落地,见 `docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md` + `docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md`)
- 用户已决定 4 个 placeholder 服务商(MiniMax / 接口 AI / 胜算云 / TeamoRouter)→ **替换为真实服务商**(下一步需用户告知替换为哪家)
- Win11 Build 26200 Insider 沙箱 broken:用户决定降级通道 / 关闭 HVCI / 借机器 / 接受 sandbox-verify 跳过 4 选 1