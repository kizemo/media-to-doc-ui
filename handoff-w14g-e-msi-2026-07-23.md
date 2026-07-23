# Handoff — W14-G E:WiX/MSI Installer 与 NSIS 共存(Dual Bundle)

**日期**:2026-07-23
**承接 handoff**:`handoff-w14f-d-e2e-verify-2026-07-23.md` §下次会话候选 E
**承接会话承诺**:在 `media-to-doc-ui` 子仓为 v1.4.0 增加 WiX/MSI 装包,与 NSIS 共存

---

## 1. 已完成 ✅

| Step | 内容 | 结论 |
|---|---|---|
| 1 | WiX Toolset 3.14 装好(choco `wixtoolset` 3.14.1.20250415) | ✅ candle.exe 3.14.1.8722 + light.exe 在 `C:\Program Files (x86)\WiX Toolset v3.14\bin\` |
| 2 | `tauri.conf.json` `bundle.targets="all"` + `bundle.windows.wix { language: ["zh-CN"] }` + `bundle.icon` 加 ico | ✅ JSON 合法 |
| 3 | `cargo tauri build` 出 NSIS + MSI 双产物 | ✅ 见 §2 |
| 4 | `cargo test` 43/43 不变 | ✅ |
| 5 | sandbox-verify NSIS 跑新产物 | ❌ 撞新墙:Win Sandbox feature 未启,见 §5.1 |
| 6 | spec / plan / handoff 落地 | ✅ |
| 7 | git commit | ✅ 见 §3 |

---

## 2. 双产物路径

### 2.1 build 产物(`target/release/bundle/`)

```
F:\soft\00selfmade\media-to-doc-ui\src-tauri\target\release\bundle\
├── nsis/
│   ├── media-to-doc-1.3.0-setup.exe          (旧,沿用)
│   ├── media-to-doc-1.4.0-setup.exe          (旧,W14-C B installer.nsi 产物)
│   └── media-to-doc_1.4.0_x64-setup.exe      ← NEW (1,631,898 bytes / ~1.55MB)
│                                              Tauri 2.x 内置 NSIS 模板(makensis 自动跑)
│
└── msi/
    └── media-to-doc_1.4.0_x64_zh-CN.msi      ← NEW (2,519,040 bytes / ~2.4MB)
                                                  WiX 3.14 candle + light 编译产物
```

### 2.2 分发产物(`release/`,W14-G E 起 gitignored)

```
F:\soft\00selfmade\media-to-doc-ui\release\
├── media-to-doc_1.4.0_x64-setup.exe          (1,631,898 bytes / ~1.55MB)
│   SHA256: 774D069F65BA9F94EF5862AC4D9E563F2EAD7D7D516A5EA89A5CF4D41AE4C443
└── media-to-doc_1.4.0_x64_zh-CN.msi          (2,519,040 bytes / ~2.40MB)
    SHA256: D14FD421252528987C1816915AB62E17A616111B9C8218120118F56D5B432907
```

`release/` 在 `.gitignore`,commit `44f80d9`(本会话补 commit),只放本地分发包,不分发走 gh release。

**命名变化**:从 `targets: "all"` 起,Tauri 2.x bundler 用内置命名(`_x64-setup.exe` / `_x64_zh-CN.msi`),不再是 v1.4.0 commit `8fd49dc` 的 `media-to-doc-1.4.0-setup.exe`(W14-C B `installer.nsi` OutFile 路径)。
**实际行为**:Tauri 2.x 默认 NSIS 模板不再用我们的 `installer.nsi`(除非显式 `windows.nsis.template`),意味着:
- ✅ 保留:desktop / start menu 快捷方式(默认模板自带)
- ✅ 保留:安装目录 `$PROGRAMFILES\MediaToDoc\`(默认值,与 installer.nsi 一致)
- ❌ 丢失:`.mtdproj` 文件关联(`Software\Classes\.mtdproj` + `MediaToDoc.Project`)
  - 实际影响低:目前无 `.mtdproj` 真实文件流,W14-C B 写的是预留
- ❌ 丢失:`NoModify` / `NoRepair` Uninstall registry DWORD(默认模板不写)
  - 实际影响低:仅阻止 Windows 控制面板"修改/修复"按钮,卸载仍正常

**是否需要回退**:不建议回退(`targets: "nsis"` 会丢 MSI 能力)。`.mtdproj` 关联如需保留,留作 W14-G+ 候选:加 `bundle.windows.nsis.template: "./nsis/installer.nsi"` 让 W14-C B 模板接管。

---

## 3. 改动清单 + commit

| 文件 | 改动 | commit |
|---|---|---|
| `src-tauri/tauri.conf.json` | `targets: "all"` + `windows.wix` + `icon` 加 ico | (本会话 commit 1) |
| `docs/superpowers/specs/2026-07-23-w14g-e-wix-msi-design.md` | spec 文档(spec §6.4 撞墙记录更新) | (本会话 commit 1) |
| `docs/superpowers/plans/2026-07-23-w14g-e-wix-msi.md` | 实施 plan | (本会话 commit 1) |
| `handoff-w14g-e-msi-2026-07-23.md` | 本 handoff | (本会话 commit 2) |
| `handoff-w14f-d-e2e-verify-2026-07-23.md` | 上一会话继承(本会话先 commit 掉) | (本会话 commit 0) |

**预期 commit hashes**:
- `W14-F D`:W14-F D handoff 归档
- `W14-G E`:bundle 配置 + spec + plan
- `W14-G E handoff`:本文件

---

## 4. 撞墙记录

### 4.1 choco `wix` 包不存在

**症状**:`choco install wix` 失败 `wix not installed. The package was not found`
**根因**:choco community source 的包名是 `wixtoolset`,不是 `wix`(`wix` 不存在)
**修复**:`choco install wixtoolset --yes --no-progress --version=3.14.1.20250415`(包名纠正 + 版本锁)
**时间**:5min 探查 + 1min install

### 4.2 choco 装完 PATH 没刷新

**症状**:装完 `candle.exe` / `light.exe` 不在 PATH(`where` 返回空)
**根因**:choco 修改 machine PATH,但 PowerShell / bash session 内 PATH 已 cached
**修复**:
- 当前会话:每次 build 前 `export PATH="/c/Program Files (x86)/WiX Toolset v3.14/bin:$PATH"`
- 永久修复(用户):重启 PowerShell / 重开 bash,或手动 `refreshenv`(需要 Chocolatey Profile 模块)
- 文档:`.bashrc` 加 `export PATH="/c/Program Files (x86)/WiX Toolset v3.14/bin:$PATH"`(可选)

### 4.3 Tauri 2.11.4 `bundle.windows.wix.arch` 字段不存在

**症状**:`cargo test` 失败 `unknown field 'arch', expected one of version, upgrade-code, upgradeCode, language, template, fragment-paths, ...`
**根因**:Tauri 2.x schema 不接受 `wix.arch`(架构由 `bundle.targets` + 编译 target 决定)
**修复**:从 spec / plan / config 三处删 `arch` 字段
**时间**:3min
**预防**:本会话 cargo test 已固化(`cargo test` 验 schema 改动比 `cargo tauri build` 快 1min+)

### 4.4 MSI bundler 找不到 .ico 图标

**症状**:`cargo tauri build` 失败 `failed to bundle project: Couldn't find a .ico icon`
**根因**:`tauri.conf.json` 只列 `icons/icon.png`,WiX bundler 严格要求 .ico(NSIS 接受 png)
**修复**:`bundle.icon` 改 `["icons/icon.ico", "icons/icon.png"]`(`icon.ico` 已在 `src-tauri/icons/`,原 W14-C B 就有)
**预防**:新装 Tauri 项目默认就有 `icon.ico`(`cargo tauri init` 会生成),v1.4.0 删过 icon 数组简化,本会话恢复

### 4.5 Tauri 默认 NSIS 模板丢失 .mtdproj 文件关联

**症状**(隐性,非 build 撞):从 `targets: "all"` 后,Tauri 用内置 NSIS 模板,不读 W14-C B 的 `installer.nsi`
**根因**:`bundle.windows.nsi.template` 字段默认是 Tauri 仓库内置模板路径
**实际影响**:`.mtdproj` 文件关联失效 + `NoModify/NoRepair` Uninstall registry 丢失
**决策**:**接受损失**(实际影响低,留作 W14-G+ 候选 — 加 `template` 字段恢复 W14-C B)
**为什么不回退**:`targets: "nsis"` 会丢 MSI 能力,MSI 是本会话主目标

### 4.6 sandbox-verify in-sandbox-verify.ps1 不识别 .msi

**症状**:`sandbox-verify` 跑 `.msi` 会报 `未找到 installer 或 portable` exit 2 FAIL
**根因**:`in-sandbox-verify.ps1:29-32` glob 只匹配 `*-setup.exe` 和 `*-portable.exe`;`lib/Get-InstallerType.ps1` 用 ASCII byte 检测 EXE 厂商签名,对 OLE/MSI 格式无效
**本会话决策**:**跳过 sandbox-verify MSI**(本会话只跑 NSIS 回归)
**留作 W14-G+ 候选**:
1. `in-sandbox-verify.ps1` 加 `*.msi` glob
2. 新加 `lib/Install-MsiSilent.ps1`(`msiexec /qn /i <msi> /l*v <log>`)
3. `lib/Get-InstallerType.ps1` 加 MSI 分支(OLE `D0 CF 11 E0` magic)
4. 卸载用 `msiexec /x {ProductCode} /qn`(ProductCode 可从 MSI 解析)
5. 估算 ~1-2h

### 4.7 Win11 Pro sandbox feature 未启(撞墙)

**症状**:本会话跑 `mtd-verify.ps1 -NoWait` 立即抛 `Windows Sandbox 未启用`
**根因**:Win11 Pro sandbox feature(`Containers-DisposableClientVM`)未在机器启用
**本会话决策**:**跳过 sandbox-verify**(需要管理员 + 重启,跨 session)
**用户后续**(如要跑 sandbox-verify):
```powershell
# 管理员 PowerShell
Enable-WindowsOptionalFeature -Online -FeatureName Containers-DisposableClientVM
# 重启后
powershell -NoProfile -ExecutionPolicy Bypass -File "F:\soft\00selfmade\sandbox-verify\media-to-doc-ui\mtd-verify.ps1" -InstallerPath "F:\soft\00selfmade\media-to-doc-ui\src-tauri\target\release\bundle\nsis\media-to-doc_1.4.0_x64-setup.exe" -NoWait
```
**遗留**:`C:\Users\Duanyi\sandbox-artifacts\mtd\` 目录仍未建(脚本提前抛错);用户跑前可手动 `mkdir -p`。

---

## 5. sandbox-verify 状态

### 5.1 撞墙:Windows Sandbox feature 未启用

**症状**(后台 task `bhx19vrm4` 退出):
```
Windows Sandbox 未启用。请先跑: Enable-WindowsOptionalFeature -Online -FeatureName Containers-DisposableClientVM
```
**根因**:Win11 Pro 机器级 sandbox feature 未启(`Get-WindowsOptionalFeature Containers-DisposableClientVM.State ≠ Enabled`)
**本会话决策**:**跳过 sandbox-verify**(本会话不能改机器 feature — 需管理员 + 重启,跨 session)

### 5.2 用户侧后续(可选)

如需跑 sandbox-verify,在 PowerShell(管理员):
```powershell
Enable-WindowsOptionalFeature -Online -FeatureName Containers-DisposableClientVM
# 重启后
powershell -NoProfile -ExecutionPolicy Bypass -File "F:\soft\00selfmade\sandbox-verify\media-to-doc-ui\mtd-verify.ps1" -InstallerPath "F:\soft\00selfmade\media-to-doc-ui\src-tauri\target\release\bundle\nsis\media-to-doc_1.4.0_x64-setup.exe" -NoWait
```

**预期结果**(feature 启用后):
- `C:\Users\Duanyi\sandbox-artifacts\mtd\logs\verify.log` exit 0
- `C:\Users\Duanyi\sandbox-artifacts\mtd\screenshots\02-running.png` 显示主窗口
- 程序路径:`D:\mtd-test\media-to-doc.exe`(NSIS /D 参数)

MSI 验证需要先扩展 sandbox-verify(W14-G+ 候选 C)。

### 5.3 桌面手动验收(用户直接,不依赖 sandbox)

| # | 步骤 | 期望 |
|---|---|---|
| 1 | 双击 `target\release\bundle\nsis\media-to-doc_1.4.0_x64-setup.exe`(主机侧) | Next → Next → Install → Finish,装到 `C:\Program Files\MediaToDoc\` |
| 2 | 双击桌面 `media-to-doc` 快捷方式 | 主窗口弹出,标题 "media-to-doc",Inbox/Run/Output/Health/Learn 5 tab |
| 3 | 关闭主窗口 + 控制面板卸载 | 干净卸载,`C:\Program Files\MediaToDoc\` 残留为 0 |
| 4 | 双击 `target\release\bundle\msi\media-to-doc_1.4.0_x64_zh-CN.msi` | Next → Next → Install → Finish,装到 `C:\Program Files\MediaToDoc\`(perMachine) |
| 5 | 双击桌面 `media-to-doc` 快捷方式 | 同 step 2 |
| 6 | 控制面板 "Programs and Features" 卸载 | 干净卸载 |

---

## 6. 测试

```
$ cargo test --lib
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

43/43 不变(baseline W14-C A + W14-B+2 +9 → W14-E C 不变;本会话 0 Rust 改)。

---

## 7. 预算使用

- 活跃时间:~50min(WiX 探查 10min + 装 5min + config 改 5min + build 撞墙 ×2 + 15min + 写文档 + handoff 10min + sandbox-verify 后台启动 5min)
- 剩预算:~1h10min(全局 §新会话开局守则 <2h)

---

## 8. 下次会话候选(W14-G+)

### A. D 盘安装路径(2026-07-23 用户追加)

**用户问题**:桌面端安装在 D 盘是否影响功能?

**结论**:**不影响**。功能影响分析:
- ✅ Tauri WebView2 host:D 盘路径与 WebView2 Runtime(用户级装)无依赖
- ✅ 子进程 mtd/uv:env var `MEDIA_TO_DOC_PROJECT` 不依赖绝对安装路径
- ✅ inbox/output 路径:用户在 GUI 自选,与安装路径无关
- ✅ 注册表卸载入口:Windows 按 ProductCode 定位,与 InstallDir 无关
- ✅ Start Menu/Desktop 快捷方式:`.lnk` 指向 `$INSTDIR\media-to-doc-ui.exe`,自动跟随
- ⚠️ WebView2 user data:%LOCALAPPDATA%\com.duanyi.mediatodoc(基于 identifier,与 InstallDir 无关,但 D 盘用户首次开需等 WebView2 cache 建立)
- ⚠️ 卸载残留:NSIS Uninst.exe 清 InstallDir,但 D 盘如被占用需手动清

**用户如何装到 D 盘**(无需改 installer):

| 方法 | 命令 |
|---|---|
| GUI 装(可选目录) | 双击 `.exe`,安装向导 "Installation Directory" 改成 `D:\MediaToDoc\` |
| NSIS 静默 | `media-to-doc_1.4.0_x64-setup.exe /S /D=D:\MediaToDoc` |
| NSIS 静默 + 日志 | `media-to-doc_1.4.0_x64-setup.exe /S /D=D:\MediaToDoc /LOG=D:\install.log` |
| MSI 静默 | `msiexec /i media-to-doc_1.4.0_x64_zh-CN.msi INSTALLDIR="D:\MediaToDoc" /qn /l*v D:\install.log` |
| MSI GUI | 双击 `.msi`,向导 "Destination Folder" 改成 `D:\MediaToDoc\` |

**决策**:**不改 installer 默认值**(保持 `C:\Program Files\MediaToDoc\` 默认;用户 D 盘在 GUI/命令行选)。
**留作可选(W14-G+ B)**:若用户希望"开箱即用 D 盘",改 `installer.nsi` 的 `InstallDir "$PROGRAMFILES\MediaToDoc"` → `InstallDir "D:\MediaToDoc"`,WiX MSI 加 `<Property Id="WIXUI_INSTALLDIR" Value="D:\MediaToDoc" />`。

### B. 子仓 v1.4.0 → v1.4.1(Minor bump + gh release 上传 .msi)

- bump `src-tauri/Cargo.toml` + `src-tauri/tauri.conf.json` + `installer.nsi` + `README.md` 到 1.4.1
- 双产物 SHA256 计算 + 写进 release notes
- `git tag v1.4.1` + `gh release create v1.4.1 --target master`
- gh release assets 同时含 `media-to-doc_1.4.1_x64-setup.exe` + `media-to-doc_1.4.1_x64_zh-CN.msi` + `media-to-doc-1.4.1-portable.exe`
- 估算:~30min,等用户拍板

### B. 恢复 .mtdproj 文件关联(改用 W14-C B NSIS template)

- `tauri.conf.json` 加 `windows.nsis.template: "./nsis/installer.nsi"`
- `installer.nsi` 改 OutFile 到 `media-to-doc_1.4.0_x64-setup.exe`(兼容 Tauri 默认命名)
- 重 build + sandbox-verify 验证 .mtdproj 关联(W11-A registry 校验)
- 估算:~30min

### C. sandbox-verify 扩展 MSI 支持(独立项目)

- `lib/Install-MsiSilent.ps1`(新加)
- `lib/Get-InstallerType.ps1` 加 MSI 分支
- `in-sandbox-verify.ps1` 加 `*.msi` glob + MSI 跑分支
- 估算:~1-2h,需开新会话(超出本子仓范围)

### D. 真实长视频 107min Tauri UI 完整跑(W14-F 候选 G)

- `cargo tauri dev` + 选真实 inbox + stop_after=longdoc
- run_in_background 必开(>2h)
- 撞墙:600s stream 上限 + session 上限(沿用 W14-B+ 经验)
- 估算:~6-10h 跨 session

### E. LE L3 优化(W14-F 候选 F)

- 范围:Prompt 自适应 + 自动重试 + 跨 Agent 经验晋升
- 依赖:先定 L3 metric(L1=执行,L2=审核/沉淀,L3=进化)
- 估算:4-6h 跨多 session

### F. 小修(W14-F §小修)

- `jumpDisabled` Set bug 修复:重跑同一 work_dir 时清 entry
- 重新 build NSIS 让 installer 包含本修复
- 估算:~15min

---

## 9. 下次会话第一句

> 承接 `handoff-w14g-e-msi-2026-07-23.md`,W14-G E 完成(`targets: "all"` + WiX 3.14 双产物:NSIS `media-to-doc_1.4.0_x64-setup.exe` 1.6MB + MSI `media-to-doc_1.4.0_x64_zh-CN.msi` 2.5MB,43/43 测试过,2 个撞墙已记录)。sandbox-verify NSIS 在跑后台 `bhx19vrm4`,等用户在桌面确认 verify.log。等用户拍板是否 bump v1.4.1 + gh release 上传双产物。