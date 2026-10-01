# iOS

> 2026-10-02 第一步落地：app 编得过 iOS、在模拟器里起得来。**本页写现状和下一步**；
> 去处的判据在 [UX.md](UX.md)「手机永远是窄档」、[CONNECT.md](CONNECT.md)「手机后台」。

同一份 Tauri 2 app（`apps/gui`），同一个嵌进去的节点。没有另起一套手机端。

## 怎么构建、怎么跑

一次性准备（本机 2026-10-02 已齐）：Xcode、`rustup target add aarch64-apple-ios aarch64-apple-ios-sim`、`xcodegen`、模拟器运行时（`xcodebuild -downloadPlatform iOS`）。**不需要 CocoaPods**：`gen/apple/Podfile` 是模板，里面没有 pod。

```sh
cd apps/gui
npm install          # 带上 @tauri-apps/cli（MIT / Apache-2.0，开发依赖）
npm run build        # khor-web 把 dist 编进去，没有它 Rust 那边编不过

# 模拟器，带热重载：前端从 vite（1430）来，先确认端口空着
npm run tauri -- ios dev "iPhone 17"

# 模拟器，只构建：产物在 src-tauri/gen/apple/build/arm64-sim/Khor.app
npm run tauri -- ios build --debug --target aarch64-sim
xcrun simctl install booted src-tauri/gen/apple/build/arm64-sim/Khor.app
```

- `tauri ios run "iPhone 17"` 在 CLI 2.12.1 上不认模拟器，落回真机目标、卡在签名——用上面两条。
- **真机要签名，仓库里不放团队号**。负责人自己：`APPLE_DEVELOPMENT_TEAM=<团队号> npm run tauri -- ios build`，或在 Xcode 里打开 `src-tauri/gen/apple/khor-gui.xcodeproj` 选团队。
- 门：`npm run mobile`（`check-mobile.mjs`）——app 对 `aarch64-apple-ios` 做类型检查，外加 `run` 还是手机的入口。改 `src-tauri` 的命令表、任何 `#[cfg(desktop)]`、按平台分叉的节点代码时跑它（《行为与门》）。
- **占地**：Rust 的 `target` 照旧；Xcode 的 DerivedData 默认在 `~/Library/Developer/Xcode/DerivedData/khor-gui-*`；每次构建把调试版静态库拷进 `gen/apple/Externals`（约 700 MB）。系统盘紧的机器把这几处链到外置盘。模拟器里 app 的数据在 `~/Library/Developer/CoreSimulator/Devices/<udid>`。

## 进了仓库的是什么

`apps/gui/src-tauri/gen/apple` 是 `tauri ios init` 生成的 Xcode 工程，**照 Tauri 的说法整个进仓库**，构建留下的（`build/`、`Externals/`、`xcuserdata/`）由它自己的 `.gitignore` 挡住；`gen/schemas` 每次构建都会重写，不进。改过的三处：

- `project.yml` 加了 `SystemConfiguration.framework`：iroh 读网卡用它（netdev、system-configuration），模板里没有，缺了就链接失败。改 `project.yml` 后在 `gen/apple` 里 `xcodegen generate`，**先把 `Externals` 清空**，否则 xcodegen 把构建产物的空目录写进工程。
- 它的 `.gitignore` 去掉了 `build/`、`Externals/` 的斜杠：把这两处链到外置盘时它们是符号链接，带斜杠的写法挡不住。
- 图标：`AppIcon.appiconset` 换成 `icons/src/mandala-glass-bleed.svg` 渲出来的（模板是 Tauri 的默认图标）；1024 那张带 alpha，上架前要去掉。

`npm run tauri -- ios init` 不要再跑：它会重写这个目录，还会自己调 Homebrew 装 / 重装 `libimobiledevice`（2026-10-02 就这样发生过一次）。

## 第一步做了什么

- **壳编得过手机**。九个错全是一种：四个只在桌面有的命令（`traffic_room`、`window_minimize`、`window_maximize_toggle`、`window_close`）照样注册，`run` 照样调只在桌面存在的 `menu::speak_at_launch`。照原有的规矩补了另一半：`traffic_room` 答 0（手机的那一行上没有系统按钮，这是真话）；三个窗口按钮和 `open_code_window` 用语料库的 `window-needs-desktop` 拒绝。
- **入口**：`run` 加了 `tauri::mobile_entry_point`，Xcode 工程的 `start_app` 调的就是它。
- **窗口**：配置里不声明窗口（桌面在 `build_the_window` 里建），手机上 `setup` 建一个最朴素的 `main`。
- **节点的家**：`Node::root_from_env` 在 iOS 上是 `$HOME/Library/Application Support`，store 在其下的 `.khor`——`$HOME` 是 app 自己的容器，容器顶层不让写；不走 `.khor-hosts` 那套共享家目录的搜索。`KHOR_HOME` 照旧优先。第一次启动铸一个新身份、自己的根（`root.key`）。
- **网页脸不在手机上起**：手机不是常驻，后台就被挂起，给出去的链接只在亮屏时通（模拟器上它还会和同一台 Mac 上的 app 抢 5467）。
- **「听不见就退出」不在手机上退**：桌面上的常驻两分钟谁也拨不通就退出、让 keeper 换一副新耳朵；手机没有 keeper、退出就是闪退，而没信号两分钟是常事。手机上只把计时清零。
- 模拟器（iPhone 17，iOS 26.5）实测：起来就是窄档，会话、机器、额度三个落地页都画得出来（空的：会话里只有本机自己那一行、机器只有本机、额度是空状态）。截图在 handoff 001 那一节记的位置。

## 手机上只在桌面能做的事，今天怎样

手机不托管会话：没有 tmux，没有能起进程的地方（沙盒不许 `posix_spawn`），没有用户的 `~/.claude`、没有 VS Code、没有 `open`。下表是第二步的输入（a 不画；b 用话拒绝；c 坏了或说了假话）。

| 功能 | 今天在手机上 | 第二步 |
|---|---|---|
| 在本机开会话（新会话对话框、机器卡「开会话」） | **c**：对话框默认目标就是手机本身，起宿主进程失败，报的是包着系统错误的一句；终端形式先登记行再起进程，失败可能留一行死的 | 后端给一个「能托管」的事实；手机不进目标列表、默认第一台别的机器；`open_session` 在登记前拒绝 |
| 接管（`takeover`） | a：手机上的行都是远端的，菜单不画（巧合，不是设计） | 保持 |
| 远端会话的终端、对话 | 可用（经嵌入的节点转） | 触屏终端是后面的事 |
| 导入 | 终端、智能体两类可用；ACP 历史对话按本机重开，**b**（目录不存在 / 起不了宿主） | 手机上不列 ACP 候选，或交给记下它的那台机器 |
| 问智能体有哪些历史对话（`agent_ask`） | **c**：要起智能体进程，失败被吞掉，像「它没说」 | 不能托管时不问 |
| 事件钩子（机器卡、`doctor` 的修复） | **c**：往沙盒里的 `.claude/settings.json` 写，答「装好了」，其实没有 claude | 手机上不画、`install_hooks` 拒绝 |
| VS Code | **b 加副作用**：开关照画，按下会在**那台 Mac 上**起 serve-web（第一次可能下 640 MB），然后开窗口被拒绝 | 开关要能开窗口才画 |
| 打开链接（`open_link`） | **c**：iOS 上落到 `xdg-open`，系统错误；终端里的链接吞掉错误 | 用系统的打开（opener 插件） |
| 在本机打开文件（`launch`） | b，但话是「设 `KHOR_LAUNCHER`」，对手机没意义 | 手机上不给这个动作 |
| 拉到本机（`pull`） | **c**：落到 `$HOME/Downloads` 或 `$HOME`，容器顶层不让写；就算写成了，「文件」app 里也看不到 | 落到 `Documents/`、打开文件共享 |
| 发文件（`pick_and_send`） | 多半可用（对话框插件在 iOS 上用系统的文件选择器，拿到的是拷贝），**未在模拟器里点过** | 验 |
| 借网、浏览器标签（`open_page`） | b：`page-needs-desktop`；机器卡上「借网」照画 | 不画 |
| 系统验证解锁 | **c**：按钮照画（`inApp` 就画），按了什么也不发生；PIN 可用 | 不画，或接 Face ID |
| 网页脸的钥匙（安全页） | **c**（轻）：手机不出网页脸，那一行没有意义 | 手机上不画 |
| 诊断（`doctor`） | 几条警告对手机不成立（没有 CLI、没人在听网页脸、钩子、打开器）；修 serve 会去起 `khor serve`，b | 手机一套自己的检查 |
| 本机的读数 | 进程表永远是空的（sysinfo 在 iOS 上不给进程），读起来像「闲着」 | 手机自己的卡不画进程表 |
| 窗口按钮、菜单窗口 | 不画；万一被调用，按钮拒绝、菜单答「没接」让页面自己画 | — |

## 第二步

按轻重：

1. **能托管与否成为一个事实**，由后端说（不靠 `navigator.platform` 猜），上表的 c 一条条按它不画或拒绝。
2. **store 的主人戳**：iOS 上没有机器指纹（`hardware::fingerprint` 只认 Linux / macOS），主人戳只剩主机名；主机名一变（真机上会），`claim_store` 就认为 store 是别人的、节点起不来。用 `identifierForVendor` 当指纹。模拟器上主机名就是这台 Mac 的（截图里手机叫「Mac-mini」）。
3. **身份不进备份**：`Library/Application Support` 会进 iCloud 备份，恢复到新手机就带着旧手机的钥匙——同一把钥匙两台设备。`.khor` 标 `NSURLIsExcludedFromBackupKey`，或钥匙进钥匙串（ThisDeviceOnly）。
4. **前后台**：挂起回来以后节点的端点、听不见的计时怎么办（第一步只是不退出）；回到前台立刻同步一次。
5. 语言：Rust 那半只在 macOS 读系统语言，iPhone 上要读 `NSLocale`。
6. 起来就跑的、手机上白干的：每 10 秒的「一天」采样会去找 tmux（每次 `posix_spawn` 失败，静默）；`face::app_here` 把手机记成「桌面 app」。
7. 触屏终端、扫码配对、安全区、iPad（iPad 的 `navigator.platform` 是 `MacIntel`，会被当成 Mac：竖栏头上空 28 像素、快捷键走 Mac 那一支）、上架用的无 alpha 图标、推送（v1 不做）。
