# Release Notes — media-to-doc-ui v1.4.1

**发布日期**:2026-07-24
**子仓 tag**:`v1.4.1`(annotated)
**主仓 handoff**:`handoff-w15-a-v014-blocker-2026-07-23.md` 续(本会话)

---

## 亮点

### 1. 修复主窗口 v0.1.0 badge regression(用户桌面手动验 2026-07-23 发现)

**症状**:v1.4.0 build 启动后,主窗口 title 区域的版本 badge 显示 `v0.1.0`,与实际 `1.4.0` 不符。

**根因**:`src/index.html:257` 硬编码了 W14-B 早期的占位值:

```html
<!-- v1.4.0 之前 -->
<span class="badge" id="version-badge">v0.1.0</span>
```

JS 在 `loadAppInfo()` 里用 `app_info.version` 覆盖,但 async 调用在完成前(以及失败时)badge 仍显示硬编码值。cargo test 跑不到 WebView,code review 看不到视觉细节,这个 regression 从 W14-B 起一直未被发现。

**修复**(本版本):

- A. `src/index.html:257` 初值改为 `''` (空),初值不露馅
- B. `loadAppInfo()` 内层包 `Promise.race([invoke('app_info'), 2s timeout])`,超时/失败时 badge 显示 `'unknown'`,给用户清晰信号而非错误版本

```diff
@@ src/index.html
-    <span class="badge" id="version-badge">v0.1.0</span>
+    <span class="badge" id="version-badge"></span>
@@
 async function loadAppInfo() {
+  // 2s timeout fallback: 防 app_info hang 时 badge 一直空白/露 v0.1.0
+  const infoTimeout = new Promise((_, reject) =>
+    setTimeout(() => reject(new Error('app_info timeout 2s')), 2000));
   try {
-    const info = await invoke('app_info');
+    const info = await Promise.race([invoke('app_info'), infoTimeout]);
     $('version-badge').textContent = 'v' + info.version;
     // ...
   } catch (e) {
+    $('version-badge').textContent = 'unknown';
     $('info-error').textContent = '⚠ ' + e;
   }
 }
```

### 2. NSIS installer 同步 bump 1.4.0 → 1.4.1

- `src-tauri/Cargo.toml` `version` = `1.4.1`
- `src-tauri/tauri.conf.json` `version` = `1.4.1`
- `src-tauri/nsis/installer.nsi` `PRODUCT_VERSION` = `1.4.1`
- 产出 `media-to-doc_1.4.1_x64-setup.exe`(~1.58MB,同 1.4.0 安装逻辑)

---

## 验证状态

| 验证 | 结果 |
|---|---|
| `cargo test --release` | **43/43 passed** (W14-C A baseline + W14-B+2 +9,与 v1.4.0 一致) |
| `cargo tauri build` | ✅ exit 0(3 min 14s 首次,v1.4.1 增量预计更短) |
| 静态代码审查 | ✅ 修复 diff 2+2+1 行,无副作用 |
| **sandbox-verify** | ⚠️ **跳过** — Win11 Build 26200 (Insider) Hyper-V 已知 broken,V-Switch 启动超时(系统日志 ID=285 反复),sandbox 进程 15s 内退出,沙箱内脚本未跑到。环境问题,不是代码问题。**建议未来 sandbox feature 验证在 Win11 Release 通道或备用机器跑。** |

---

## Assets

| Asset | Size | SHA256 |
|---|---|---|
| `media-to-doc_1.4.1_x64-setup.exe` | ~1.58MB | (gh release page 显示) |

(无 portable 版本,本版本未涉及 portable 改动)

---

## 安装

### Windows(installer,推荐)

1. 下载 `media-to-doc_1.4.1_x64-setup.exe`
2. 管理员运行(perMachine 安装)
3. 装到默认 `C:\Program Files\MediaToDoc\`
4. 桌面 / 开始菜单启动 `media-to-doc`
5. **启动后,主窗口 title 区域的 badge 应显示 `v1.4.1`**

### 升级路径(v1.4.0 → v1.4.1)

- installer:覆盖安装(NSIS 自动卸载 v1.4.0)
- 配置 / workspace / inbox 不需变动

---

## 已知问题

- Rust toolchain 需 1.97+(自带 lld-link 无需 MSVC)
- 公司 VPN 用户构建时需设 `CARGO_NET_TLS_VERIFY=false`(运行不受影响)
- macOS / Linux 编译需用户自查环境
- **Win11 Insider Build 容器功能 broken**(影响本机器 sandbox-verify 跑不动),Release 通道不受影响

---

## 上游

主仓 `media-to-doc` Python 后端 v1.2.1 已是最新(无 W15-A 关联改动):
- PyPI:https://pypi.org/project/media-to-doc/
- GitHub:https://github.com/kizemo/media-to-doc/releases/tag/v1.2.1

---

## 后续

- W15-A(LLM API Settings)进入实装阶段(spec + plan 已落地,见 `docs/superpowers/specs/2026-07-23-w15-a-llm-api-settings-design.md` + `docs/superpowers/plans/2026-07-23-w15-a-llm-api-settings.md`)
- W14-F D 决定 sandbox-verify 沙箱自动化受限时,fallback 走静态 review + cargo test
