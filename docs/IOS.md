# iOS

> 2026-10-02 第一步落地：app 编得过 iOS、在模拟器里起得来；同日第二步：手机不托管，也不说它托管。**本页写现状和下一步**；
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

## 第二步做了什么：手机不托管，也不说它托管

2026-10-02（ios2 车道）。手机是网里一台**看别的机器、操作别的机器**的机器：不托管会话（没有 tmux、没有能起进程的地方），没有 VS Code、没有钩子、没有网页脸、没有有用的进程表。第一步时前端不知道这件事，于是手机上画着一串会失败或说假话的东西。第二步在根上改：

- **一个事实**：`khor_core::hosts_here()`——手机的构建（iOS、Android）答否，别的构建答是；构建时就定了，不靠 `navigator` 猜。节点开起来时把它写进**自己在设备表里的那一行**（`khor_sync::devices` 的 `hosts`，扁平寄存器，只由本机写，每次开都写，没变就不写），于是网里每台机器都知道——桌面也不会去手机上开会话。界面读 `DeviceRow.hosts`：本机这一行经 `src/lib/here.ts` 往下传，别台的读它自己那一行。
- **线上格式**：没有新的帧，`Vitals` 一个字没动。只是设备表（loro 文档）里多一个键 `<id>/hosts`。**老节点照常同步、原样转发这个键、从来不读它**；新节点读一行没有这个键的（老节点写的、旧嵌套形状的）当作电脑——那是默认，不是兼容垫片：第二步之前的 khor 全跑在电脑上。所以**三台 Linux 机器不用为了网能用而升级**；它们自己的 CLI 不读这个键，所以在它们上面敲 `khor open --on 手机` 不会被提前拒，会拨过去、由手机的节点用 `machine-cannot-host` 拒（手机醒着时）。升级了就在拨号前拒。
- **不能托管的节点什么也不起**：发现关死（`Discovery::empty().with_procs(Procs::default())`）——不读 vendor 目录、不找 tmux、不读进程表。第一步说的「每 10 秒的『一天』采样会去找 tmux」就是这一条：`day_sampler` → `record_day` → `sessions` → 扫一遍。现在一次也不起（`tests/no_host.rs` 用一个记账的假 tmux 量的，电脑做对照）。
- **门**（绕过界面来问的：CLI、别的机器、过期的页面）：开会话、终端、对话、接管、重开 ACP 对话在**登记一行之前**拒，话是 `machine-cannot-host`；问智能体 `agent-ask-needs-a-computer`；装钩子 `hooks-need-a-computer`（不再答「装好了」）；在手机上打开文件 `launch-needs-a-computer`；拉到手机 `pull-needs-a-computer`。别台问手机时照手机那一行**先拒、不拨号**，手机睡着也说得出。自检的 serve、hooks 两个修复在手机上拒。
- **白干的活**：上面的扫描；`face::app_here` 只在桌面写（手机的卡不再自称「桌面 app」），`face::faces` 在手机上是空的（不再去探 5467）；钩子不轮询；VS Code 不问；导入不读智能体；安全页不读网页脸的钥匙。
- **身份**：见下面「身份」一节。

## 手机上只在桌面能做的事，今天怎样

a 不画；b 用话拒绝；c 坏了或说了假话。第二步之后没有 c。

| 功能 | 第一步时 | 现在 |
|---|---|---|
| 在本机开会话（新会话对话框、机器卡「开会话」） | **c**：默认目标是手机本身，起宿主进程失败，报包着系统错误的一句；终端形式先登记行再起进程 | **a**：新会话默认第一台能托管的机器；挑到手机时智能体、shell 两段置灰，只剩留言（手机的窗口照样能留言）；手机的机器卡没有开会话——**电脑看手机也没有**。**b**：节点在登记前拒（`machine-cannot-host`），别台照表先拒 |
| 接管（`takeover`） | a：手机上的行都是远端的，菜单不画（巧合） | a 照旧；**b** 是设计出来的了：`takeover` / `takeover_term` 在手机上拒 |
| 远端会话的终端、对话 | 可用 | 可用（触屏终端是后面的事） |
| 导入 | ACP 历史对话按本机重开，b | **a**：手机上不列 ACP 候选、不画「问一次」那一节；终端、智能体两类照旧 |
| 问智能体有哪些历史对话（`agent_ask`） | **c**：要起智能体进程，失败被吞掉，像「它没说」 | **a**：新会话「接一个」之后手机不问；导入不画。**b**：`agent-ask-needs-a-computer` |
| 事件钩子 | **c**：往沙盒里的 `.claude/settings.json` 写，答「装好了」 | **a**：机器卡不画、不轮询；自检没有这一行。**b**：`install_hooks`、自检的修复拒 |
| VS Code | **b 加副作用**：开关照画，按下在那台 Mac 上起 serve-web，然后开窗口被拒 | **a**：那台不能托管、或本机是手机（没有第二个窗口），开关都不画，`code_here` 也不问——serve-web 不会再被起 |
| 打开链接（`open_link`） | **c**：落到 `xdg-open`，系统错误 | **可用**：交给系统（`UIApplication.openURL`，同一份 http/https 白名单 `web::page_only`），在系统浏览器里开 |
| 在本机打开文件（`launch`） | b，话是「设 `KHOR_LAUNCHER`」 | **a**：设置里 store 那行只写路径、不画「打开」。**b**：对手机（本机或别台）`launch-needs-a-computer`。从手机让一台电脑打开它自己的文件照旧可用 |
| 拉到本机（`pull`） | **c**：落到容器顶层，不让写；写成了也看不见 | **a**：树里、文件页上都不画「拉过来」。**b**：`pull-needs-a-computer`。落到 `Documents/` + 文件共享留给第三步 |
| 发文件（`pick_and_send`） | 多半可用，未点过 | 未变（见 handoff 001 本节：模拟器里没点到系统文件选择器） |
| 借网、浏览器标签（`open_page`） | b；机器卡上「借网」照画 | **a**：手机上机器卡不画借网；浏览落地页里开页仍是 b（`page-needs-desktop`） |
| 系统验证解锁 | **c**：按钮照画，按了什么也不发生 | **a**：只在 Mac 上画（`LockFacts.system_door`）。顺带：Linux / Windows 桌面上它也一直是按了没反应，现在一起不画 |
| 网页脸的钥匙（安全页） | **c**（轻） | **a**：那一节不画，钥匙也不读 |
| 诊断（`doctor`） | 几条对手机不成立；修 serve 去起 `khor serve`，b | 手机一套：身份、仓、serve、中继、对端、名单、道路、磁盘；不跑的项根本不起（版本检查会起 CLI），serve 不带修复 |
| 本机的读数 | 进程表永远是空的，读起来像「闲着」 | **a**：不能托管的机器（手机自己，或电脑看手机）不画进程表；读数和 CPU 一天照画 |
| 身份页「本机开着哪几张界面」 | 手机列着「桌面 app」 | **a**：手机上那一节不画（`face::faces` 空） |
| 窗口按钮、菜单窗口 | 不画；被调用时按钮拒绝 | — |

门：`check-face` 第 18 节（`FACE_ONLY=phone`）、节点单测、`tests/no_host.rs`，登记在《行为与门》；摆放在那里的真 app 清单。

模拟器（iPhone 17 / iOS 26.5，和一台临时桌面节点配对，2026-10-02）里看过：新会话开在那台电脑、挑到手机只剩留言；手机自己的机器页只有「文件」；看电脑的机器页有开会话和文件、没有借网和 VS Code；安全、设置、身份、自检四页；链接交给 Safari；从手机直接调开终端、装钩子拿到的是语料库的拒绝。临时桌面那一侧：它的机器页看手机没有开会话、VS Code、进程表，新会话挑到手机只剩留言，`khor open --on 手机` 在拨号前答「… 是手机，会话只开在电脑上」。截图在 handoff 001 本节记的位置。

## 身份

- **机器指纹**：`hardware::fingerprint` 在 iOS 上是 `UIDevice.identifierForVendor`（哈希后写进主人戳，和 macOS 的 `IOPlatformUUID`、Linux 的 `machine-id` 同一种用法）。改设备名（真机上「张三的 iPhone」→ 别的）不再让 `claim_store` 拒、节点起不来。它只在主线程给：app 在 `run` 一开始（主线程上）先问一次存着，之后谁问都读存着的；万一在别的线程先被问到，经主队列同步问（`dispatch2::run_on_main`）。重启后第一次解锁前 iOS 可能不给，那一次退回比主机名，和没有指纹的平台一样。
- **不进备份**：`.khor` 目录标 `NSURLIsExcludedFromBackupKey`（`backup.rs`，每个进程一次，在 `claim_store` 之后），目录连同里面的 `identity.key`、`root.key` 都不进 iCloud / 电脑备份。恢复到新手机就没有仓、起来铸一个新身份——新设备本来就该是新身份。钥匙进钥匙串（`ThisDeviceOnly`）是更强的一步，没做。
- 依赖：`objc2`、`objc2-foundation`、`objc2-ui-kit`、`dispatch2`，都是 tauri 的 wry 在 iOS 上本来就带的，MIT；`Cargo.lock` 只多几条边，没有新包。

## 第三步

按轻重：

1. **Rust 那半的语言**：手机上节点的拒绝和设置页里节点给的字是英文的（`machine-cannot-host` 在模拟器里读出来是 「Mac-mini is a phone — …」，设置页「Confirm dangerous actions」），因为 `config::apply_lang` 只在 macOS 读系统语言；iPhone 上要读 `NSLocale`。这是第二步里最显眼的一处没做。
2. **拉到手机**：落到 `Documents/`，`Info.plist` 开 `UIFileSharingEnabled` / `LSSupportsOpeningDocumentsInPlace`，「文件」app 里看得见；然后把「拉过来」放回来。
3. **前后台**：挂起回来以后端点、听不见的计时、回前台立刻同步一次（第一步只是不退出）。
4. 设备名：手机的频道名是主机名（模拟器里是这台 Mac 的名字），真机上该用 `UIDevice.name` 或让人起名。
5. 钥匙进钥匙串（`ThisDeviceOnly`），备份排除之外再加一层。
6. 存储被拒时的界面：主人戳不是这台的（模拟器里把指纹改掉量过），节点起不来，界面停在骨架上不说话；该说 `store-belongs-to` 那一句。
7. 网页脸的字：安全页「锁定 app 窗口和网页脸」在手机上提到了它没有的网页脸。
8. 触屏终端（附件键）、扫码配对、安全区和键盘让位、iPad（`MacIntel` 被当成 Mac）、上架用的无 alpha 图标、真机签名、推送（v1 不做）。
