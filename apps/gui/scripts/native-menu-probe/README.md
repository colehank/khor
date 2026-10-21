# iOS 原生候选菜单手动验收夹具

这份夹具复现已经完成的 27 项验收，不维护第二份菜单或原生适配器。Rust 的 `menu_overlay.rs` 和 `ios.rs` 通过外部目录中的符号链接直接编译当前仓库代码；只用事件收集器替换 `tauri::AppHandle/Emitter`，UIKit、WebKit、原生层级、消息桥和输入焦点均是真实实现。

## 运行

需要 Apple Silicon Mac、完整 Xcode、可用的 iOS 模拟器、Rust `aarch64-apple-ios-sim` target，以及仓库锁定的离线 Cargo 依赖。前端仍用项目原有 Node、依赖和系统 Chrome。

在仓库根目录运行，**产物放在仓库外**：

```sh
python3 apps/gui/scripts/native-menu-probe/run.py \
  --out /Volumes/disk0/khor-native-menu-proof \
  --simulator EBBE393D-4ABB-4AFB-B360-44812E929058
```

默认先运行 `npm run native-menu-ui`，通过现有 `NATIVE_MENU_SNAPSHOT_OUT` 导出正式 `OmniMenu` 快照。也可复用已导出的快照：

```sh
cd apps/gui
NATIVE_MENU_SNAPSHOT_OUT=/Volumes/disk0/menu.html npm run native-menu-ui
cd ../..
python3 apps/gui/scripts/native-menu-probe/run.py \
  --out /Volumes/disk0/khor-native-menu-proof \
  --snapshot /Volumes/disk0/menu.html \
  --simulator EBBE393D-4ABB-4AFB-B360-44812E929058
```

该快照是现有前端门的 30 项暗色候选，包含 `m:local` 与 `m:remote`，以及故意带脚本文字的标签；不是产品文案或设计提案。更改前端夹具时应同步这份 Swift 验收的行数与候选身份。

脚本从当前 `Cargo.lock` 取得依赖版本，在指定外部目录生成临时 Cargo 项目、构建静态库和签名模拟器 App。没有复制生产源文件、生成源码或二进制进入仓库。`Probe.swift`、`bridge.h`、`lib.rs` 是手动验收源，外部目录是构建及证据目录。

只安装／启动／删除 `dev.khor.native-menu-probe`，不碰真实 Khor、home、会话、凭据或网络。只操作指定模拟器；原来关闭的在结束后关闭，原来已启动的保持启动。输出目录必须为空，或带有脚本自己创建的标记。不要在运行途中修改被测源文件；前后哈希不同会拒绝本次结果。

## 检查范围

- 真实网页兄弟 WKWebView 上方的菜单层级、测量尺寸、暗色快照、30 行内部滚动。
- 原网页文档身份与 `scrollY=240` 在选择前后保持；菜单更新不丢候选滚动位置。
- 实际 `UIView.hitTest` 在菜单外落到网页，外点观察器不取消／延迟原触摸。
- 合法候选、当前 owner、过期 lease、无效候选、非法桥消息、旧视图清理、旧关闭与替换。
- 原地址字段的真实 first responder 保留；菜单不能取得 first responder，显式调用 `becomeFirstResponder()` 也失败。

原生回归曾实测先红：只覆盖 `canBecomeFirstResponder=false` 时，WKWebView 的 `becomeFirstResponder()` 仍返回 true 并抢走原字段。仅菜单的运行时 WK 子类同时拒绝两者后，27 项通过。主界面与网页类不变。

## 产物与边界

外部目录中的 `probe-results.json` 是逐项结果，`native-menu.png` 是模拟器原生截图，`probe-manifest.json` 记录实际源与快照哈希。Cargo 目标、App 和这些证据留在外部供复验；脚本在完成或失败后清理模拟器上的测试 App。

这不替代完整 Tauri 通道、物理触摸、真机中文输入法和手感验收。新模拟器可能出现 QuickPath 首次键盘教学：first responder 检查仍有效，不能因此声称键盘实际输入已验收。Desktop、Windows/Linux 与其它系统 WebView 不在这个夹具的覆盖范围。
