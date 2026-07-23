# W14-G E — WiX/MSI Installer 与 NSIS 共存(Dual Bundle)

**日期**:2026-07-23
**承接 handoff**:`handoff-w14f-d-e2e-verify-2026-07-23.md` §下次会话候选 E
**承接会话承诺**:在 `media-to-doc-ui` 子仓为 v1.4.0 增加 WiX/MSI 装包,产出 `.msi` 与已有 NSIS installer 共存
**当前子仓 HEAD**:`59a74f7`(W14-E C frontend-only,无 Rust 改)
**主仓版本**:v1.3.0(PyPI latest,不动)
**子仓版本**:v1.4.0(GitHub Release latest,本次双产物仍标 v1.4.0)

---

## 1. 目标

在子仓跑通 Tauri 2 的 WiX bundler,产出 `.msi` installer,与现有 NSIS `.exe` installer 共存(dual bundle),扩 GH release assets(可选)。

**关键动机**:
- 企业 / 公司域环境更偏好 MSI(GPO 部署、SCCM、系统镜像)
- 个人用户沿用 NSIS 即可(1.58MB 体积更小)
- 双产物 → 覆盖两类分发场景,**NSIS 不退役**

---

## 2. 数据源选择

### 2.1 Tauri 2 bundler 对 Windows installer 的支持

Tauri 2.x 内置 `nsis` 与 `msi`(WiX)两个 target,通过 `tauri.conf.json` `bundle.targets` 配置:

```json
{
  "bundle": {
    "active": true,
    "targets": "all",          // ["nsis", "msi"] 或 "all"
    "windows": {
      "nsis": {},
      "wix": {
        "version": "3.14",
        "language": "zh-CN",  // 或 ["en-US", "zh-CN"]
        "arch": ["x86_64"]
      }
    }
  }
}
```

**当前配置**(v1.4.0 commit `8fd49dc`):
```json
"targets": "nsis",
"windows": { "nsis": {} }
```

**改后**:
```json
"targets": "all",          // 同时产 NSIS + MSI
"windows": {
  "nsis": {},              // 沿用 W14-C B,不改
  "wix": {
    "language": ["zh-CN"]
  }
}
```

> **注意 1**:`version` 字段 Tauri 2 会从 Tauri config version 字段自动读,无需显式指定。
>
> **注意 2**:**Tauri 2.11.4 `bundle.windows.wix` schema 不支持 `arch` 字段**(本会话 cargo test 撞 `unknown field 'arch'` 后删掉)。架构由 `bundle.targets` + 编译时 target 决定,默认 x86_64。

### 2.2 WiX 工具链

Tauri bundler 在 Windows 上要求 WiX Toolset 3.x 的 `candle.exe` / `light.exe` 在 PATH 上。

**Win11 Pro 当前状态**(本会话探查):
- `candle.exe` / `light.exe` 不在 PATH
- `scoop` 未装
- `choco` 已装(2.7.3),但 `wix` / `wixtoolset` 包没在 community source 找到(2026-07-23 探查时返回 `wix not installed. The package was not found`)

**装法候选**(按推荐顺序):
1. **手动下载 WiX 3.14 .exe installer**:`https://github.com/wixtoolset/wix3/releases/download/wix314rtm/wix314.exe`(无需管理员,解压即用,无 .msi 注册)
2. **手动下载 WiX 3.14 .zip**:无,WiX 3.14 只发布 .exe
3. **WixEdit / 第三方包**:不推荐,可能缺 candle/light
4. **choco 第三方 source**:不推荐,版本难控

**选定方案**:手动下 wix314.exe,运行 installer(管理员)装到 `C:\Program Files (x86)\WiX Toolset v3.14\bin\`。

**fallback 方案**(若 .exe 撞 sandbox 权限):用 7-Zip 解压 .exe,得到 `.cab` + 内嵌 msi,提取 candle.exe + light.exe + 依赖 .wxl 到本地目录 `F:\soft\00selfmade\media-to-doc-ui\.wix\bin\`,build 时只把这一行加到 PATH,不依赖系统注册。

---

## 3. 设计

### 3.1 tauri.conf.json 改动

唯一改动:`bundle` 段。

```diff
   "bundle": {
     "active": true,
-    "targets": "nsis",
+    "targets": "all",
     "icon": [
       "icons/icon.png"
     ],
     "windows": {
-      "nsis": {}
+      "nsis": {},
+      "wix": {
+        "language": ["zh-CN"]
+      }
     }
   }
```

**为什么 `language: ["zh-CN"]`**:
- NSIS installer 沿用 `SimpChinese`(W14-C B),产物里也用中文 UI
- WiX 的 `WixUI` 默认是 en-US;`language` 字段传 BCP-47 tag,Tauri bundler 会自动映射到对应 `.wxl` localization file

**为什么无 `arch` 字段**:
- **Tauri 2.11.4 `bundle.windows.wix` schema 不接受 `arch`**(本会话 cargo test 时撞 `unknown field 'arch'`,验证 schema)
- 架构由 `bundle.targets` + 编译时 target 默认决定(x86_64)
- ARM64 用户走 NSIS portable 路径,留作未来

### 3.2 产物路径

Tauri 2 bundler 输出目录:
```
target/release/bundle/
├── nsis/
│   └── media-to-doc-1.4.0-setup.exe          ← 已有(W14-C B)
├── msi/                                       ← 本会话新增
│   └── media-to-doc_1.4.0_x64_zh-CN.msi      (Tauri 默认命名)
└── ...
```

**命名差异**:Tauri MSI 默认用 `_` 分隔(`media-to-doc_1.4.0_x64_zh-CN.msi`),NSIS 用 `-`(`media-to-doc-1.4.0-setup.exe`)。这是 Tauri 2.x 默认行为,不强行改。

### 3.3 NSIS 不退役

NSIS installer 沿用 W14-C B 模板(`installer.nsi`)+ v1.4.0(commit `8fd49dc`):
- 不改 `installer.nsi`
- 不改 NSIS 命名
- NSIS 体积小(1.58MB)+ 离线安装快 → 个人用户首选
- MSI 进 GPO / SCCM 部署 → 企业首选

---

## 4. 验收清单

| # | 验证项 | 期望 |
|---|---|---|
| 1 | WiX 工具链可用 | `candle.exe --version` / `light.exe --version` 输出 WiX Toolset 3.x |
| 2 | `cargo tauri build` 跑通 | 编译 1m+WiX 编译 30s,**双产物** `target\release\bundle\nsis\*-setup.exe` + `target\release\bundle\msi\*.msi` 都在 |
| 3 | `cargo test` 不破 | 43 passed / 0 failed(WiX 配置不影响 Rust 代码,但作 sanity check) |
| 4 | `sandbox-verify` 验 MSI | `mtd-verify.ps1 -InstallerPath <msi>` exit 0,日志在 `C:\Users\Duanyi\sandbox-artifacts\mtd\logs\verify.log` |
| 5 | MSI 装机后程序可启动 | 沙箱里手动跑 / 桌面手动验收,exe 在 `C:\Program Files\MediaToDoc\media-to-doc-ui.exe`,标题 "media-to-doc" |
| 6 | NSIS 装机不被破 | sandbox-verify 跑 NSIS,与 v1.4.0 一致(回归) |

**E2E 验收**(GUI 自动化被 computer-use MCP 撞墙挡,沿用 W14-F D fallback):
- 桌面手动跑(sandbox-verify 通过后,在用户桌面手动双击 `.msi`,Next → Next → Install → Finish → 双击桌面图标 → 看到主窗口)
- 详细命令见 handoff §给用户的步骤

---

## 5. 改动清单

| 文件 | 改动 | 备注 |
|---|---|---|
| `src-tauri/tauri.conf.json` | `bundle.targets` 改 `"all"`,`bundle.windows` 加 `wix` 配置 | 唯一配置改动 |
| `docs/superpowers/specs/2026-07-23-w14g-e-wix-msi-design.md` | spec 文档(本文件) | |
| `docs/superpowers/plans/2026-07-23-w14g-e-wix-msi.md` | 实施 plan | 本会话写 |
| `handoff-w14g-e-msi-2026-07-23.md` | handoff(完成时写) | |

**不改**:
- ❌ `src-tauri/nsis/installer.nsi`(沿用 W14-C B)
- ❌ `src-tauri/Cargo.toml`(无新 Rust 依赖,WiX 是外部工具链)
- ❌ `src-tauri/capabilities/*`(权限无变化)
- ❌ `src/index.html`(W14-E C 已稳定)
- ❌ 主仓 `media-to-doc/`(v1.3.0 latest,不动)

---

## 6. 风险

### 6.1 WiX Toolset 不在 Win11 Pro 默认装

**撞墙概率**:高(本会话已确认 candle/light 不在 PATH,choco community 没 `wix` 包)
**缓解**:手动下载 `wix314.exe` 从 GitHub release,装到 `C:\Program Files (x86)\WiX Toolset v3.14\`(需管理员)
**极端 fallback**:用 7-Zip 解压 .exe,提取 candle/light 到子仓 `.wix\bin\` 本地目录,build 时 PATH 注入

### 6.2 WiX 4 vs WiX 3 schema 不兼容

**风险**:Tauri 2.x 推荐 WiX 3.x;若误装 WiX 4.x,bundler 会撞 schema 错(变量名 / 元素顺序都不同)
**缓解**:spec §2.2 显式锁 WiX 3.14,装完验证 `wix --version` 输出是 `3.x`

### 6.3 MSI 体积

**预期**:NSIS 1.58MB + MSI ~12MB(MSI 包含 .cab 内嵌)
**风险**:WiX bundler 把所有 runtime DLL / .NET deps 全内嵌(MSI 是 perMachine 安装标准),体积可能 ~10MB+
**决策**:可接受;Tauri 2.x MSI 产物参考值 8-15MB,符合预期

### 6.4 sandbox-verify 不识别 .msi(本会话已撞墙)

**症状**:`sandbox-verify/media-to-doc-ui/in-sandbox-verify.ps1:29-32` 只识别 `*-setup.exe` 和 `*-portable.exe`,不识别 `*.msi` → sandbox 报 `未找到 installer 或 portable` → exit 2 FAIL。
**根因**:sandbox-verify 当前只覆盖 NSIS(EXE 厂商签名 `Nullsoft Install`)+ portable。MSI 是 OLE compound document 格式(`D0 CF 11 E0` magic),`Get-InstallerType.ps1` 的 ASCII byte-level 检测对它无效。
**本会话决策**:**不改 sandbox-verify**(独立项目,超出本会话范围)。sandbox-verify 扩展 MSI 支持留作 W14-G+ 候选:
- 加 `*.msi` glob in `in-sandbox-verify.ps1`
- 加 `Install-MsiSilent.ps1` lib(用 `msiexec /qn /i <msi> REBOOT=ReallySuppress /l*v <log>`)
- `Get-InstallerType.ps1` 加 MSI 分支(OLE signature 检测)
- 卸载用 `msiexec /x {ProductCode} /qn`(需要 ProductCode,可从 MSI 解析)
- 估算 ~1-2h,需开新会话

### 6.5 Cargo SSL 撞墙

**已知**(memory `feedback_cargo_ssl_mitm`):`CARGO_NET_TLS_VERIFY=false` + default crates-io
**action**:build 前设环境变量,沿用 W14-B+ / W14-C / W14-D 经验

### 6.6 Tauri MSI 默认 arch 不含 ARM64

**事实**:Tauri 2.x MSI 在 Windows 默认产 x86_64(由 `bundle.targets` + 编译 target 决定)
**决策**:不显式声明 `wix.arch`(Tauri schema 不支持),ARM64 用户走 NSIS portable 路径,留作未来

---

## 7. 边界

- ❌ 不改 Rust 源码(WiX 配置是 JSON,0 Rust 改)
- ❌ 不改 frontendDist(沿用 v1.4.0 `8fd49dc`)
- ❌ 不 bump 主仓版本(主仓 v1.3.0 latest,不动)
- ❌ 不 push tag / 不 gh release(等用户拍板)
- ❌ 不强改 sandbox-verify 脚本(在独立项目,超出本会话范围;若撞墙留作 handoff 候选)

---

## 8. 后续(W14-G+ 候选)

| 候选 | 内容 | 依赖 |
|---|---|---|
| 子仓 v1.4.0 → v1.4.1(Minor bump) | gh release assets 同时含 nsis + msi + portable;release notes 标注双产物 | 本会话双产物 OK |
| MSIX installer | Win10+ 现代部署(需 WiX 4 + AppX toolchain);企业分发更优 | WiX 4 撞墙风险高 |
| 双产物哈希 + signature | 给 .msi + .exe 加 SHA256 + GPG sign | 需 code signing cert(公司付费) |
| Win11 ARM64 MSI | 跨设备兼容性 | Tauri 2.x ARM64 支持待跟进 |