# W6 · 打包 / 更新 / 回滚

本文件记录 Windows 安装包的产物、验证证据，以及更新路径的边界。凡未实测的项目一律标 `NOT_TESTED` 并写明原因，不把 skipped 写成 passed。

## 1. 产物（release 构建）

构建命令（`F:\Temp\khor-gui`，配置覆盖只为跳过 `npm run build`——本机无 npm，前端已先构建）：

```powershell
$env:CARGO_TARGET_DIR='F:\Temp\khor-target'
cargo-tauri build --config tauri.windows-build.conf.json
```

| 文件 | 大小 |
| --- | --- |
| `release/bundle/msi/Khor_0.2.0_x64_en-US.msi` | 49.31 MiB |
| `release/bundle/msi/Khor_0.2.0_x64_zh-CN.msi` | 49.31 MiB |
| `release/bundle/nsis/Khor_0.2.0_x64-setup.exe` | 35.65 MiB |

`cargo tauri build` 退出码 **0**；`Finished 2 bundles at: …`。

构建耗时：release 编译 35m53s（`lto = "thin"`、`codegen-units = 1`、`strip = true`）。

## 2. 打包工具链的坑（已解决）

`tauri-bundler` 自带的下载在本次网络下会超时：

```
Downloading https://github.com/wixtoolset/wix3/releases/download/wix3141rtm/wix314-binaries.zip
Error failed to bundle project: `timeout: global`
```

而同一 URL 手动请求 4.9s 就返回 200。修法是预置缓存，脚本：**`scripts/ensure-bundler-tools.ps1`**。

它按 `tauri-bundler-2.10.4` 的期望布局（`windows/{msi,nsis}/mod.rs`）放置：

| 目标 | 来源 |
| --- | --- |
| `<cache>/WixTools314/` | `wix314-binaries.zip` |
| `<cache>/NSIS/` | `nsis-3.13.zip`（包内 `nsis-3.13/` 改名为 `NSIS`） |
| `<cache>/NSIS/Plugins/x86-unicode/additional/nsis_tauri_utils.dll` | `nsis_tauri_utils.dll` |

`<cache>` = `dirs::cache_dir()/tauri` = `%LOCALAPPDATA%\tauri`。脚本只创建文件，不删除任何已有内容。

## 3. 安装 → 启动 → 卸载

脚本：**`scripts/windows-installer-check.ps1`**。它先打印"将要被卸载器删除的目录"，并记录安装前该目录已有什么（开发运行留下的 `runtime\` 之类），卸载后断言**只剩安装前那些**——不要求整目录消失。

### NSIS（per-user）

```
PASS  install       5334ms
PASS  layout          73ms   khor-gui.exe + uninstall.exe
PASS  start         1082ms   窗口在自己的传输上应答
PASS  stop           700ms
PASS  uninstall     1275ms
PASS  removed         33ms   installer 的文件消失，runtime\ 保留
6/6 steps passed
```

`installMode: currentUser` → 装到 `%LOCALAPPDATA%\Khor`，无需管理员。**这是干净 Windows 11 上"安装→启动→卸载"的通过证据。**

### MSI（per-machine）

```
FAIL  install   msiexec exit 1603
```

日志（`msiexec /l*v`）给出的确切原因：

```
Error 1925. You do not have sufficient privileges to complete this installation
for all users of the machine. Log on as administrator and then retry.
Property(S): ALLUSERS = 1
```

MSI 恒为 per-machine（`tauri-utils` 的 `WixConfig` 没有 install-mode 选项），需要提权；本席位审批被禁用，无法提权。

**MSI 的安装/启动/卸载 → `NOT_TESTED`**（原因：需要管理员权限，本会话不可提权）。产物本身已成功生成。

## 4. 更新与回滚

### 4.1 安装器级升级 / 回滚

脚本：**`scripts/windows-upgrade-check.ps1`**（`-Base 0.2.0 -Newer 0.2.1`）。步骤：装 0.2.0 → 证明能跑 → 覆盖安装 0.2.1 → 证明能跑 → 回装 0.2.0 → 证明能跑 → 卸载 → 断言干净。版本用可执行文件的 `ProductVersion` 判定，能跑以"窗口在自己的传输上应答"判定。

```
install dir: C:\Users\admin\AppData\Local\Khor (only what the installer creates is removed)
already present (must survive): runtime

upgrade
  PASS  install base                 5162ms
  PASS  base runs                    2027ms
  PASS  upgrade to newer             5402ms   0.2.1 覆盖 0.2.0
  PASS  newer runs                   1740ms
rollback
  PASS  roll back to base            5096ms   0.2.0 覆盖 0.2.1
  PASS  rolled-back runs             1732ms
  PASS  uninstall                    1283ms
  PASS  clean                          42ms   installer 的文件消失，runtime\ 保留
8/8 steps passed
```

**升级与回滚各一次，均通过**（`F:\Temp\khor-upgrade-check\report.json`）。

第二个版本由 `tauri.v021.conf.json` 只改 `version` 构建；实测这会触发 release 全量重编译（8m42s）。


### 4.2 应用内签名更新器

**`NOT_TESTED`**。`crates/update` 用 `TrustRoot` 校验签名的 release 产物并从 GitHub feed 取；
`ROADMAP.md` W6 明确"签名：只做验证设计，不签名"。没有签名密钥就没有可被接受的新版本产物，
因此这条路径无法在本里程碑内实测。归属：需要发布/签名授权后由发布流程验证。

## 5. 已知的发布打磨项（不阻断试用）

1. **装出来的主程序叫 `khor-gui.exe`**，不是 `Khor.exe`。Tauri 的 `mainBinaryName` 要求 cargo bin 同名，
   改它要动 crate 的 bin 名，会牵连全部构建命令与已存的 W3/W4 证据。显示名（开始菜单、卸载项、窗口标题）
   已是 "Khor"。
2. **首包 14 MB**（W5 把动态导入内联，见 [W5-GUI](W5-GUI.md) §6.4）。冷启动指标见 W7。
3. MSI 需要提权；若面向普通用户分发，NSIS per-user 是更顺手的一档。

## 6. 证据索引

| 项 | 命令 | 退出码 | 日志 |
| --- | --- | --- | --- |
| release 构建 + 打包 | `cargo-tauri build --config tauri.windows-build.conf.json` | 0 | 见 §1 产物清单 |
| NSIS 安装/启动/卸载 | `pwsh scripts/windows-installer-check.ps1 -Installer <nsis>` | 0 | `F:\Temp\khor-install-check\report.json` |
| MSI 安装 | `msiexec /i <msi> /qn /l*v F:\Temp\khor-install-check\msi-install.log` | 1603 | 同名 `.log`，Error 1925 |
