# iOS

> 2026-10-02 第一步落地：app 编得过 iOS、在模拟器里起得来；同日第二步：手机不托管，也不说它托管；同日第三步 A：手机的会话页和触屏终端；第三步之二：扫码加入、回到前台换新耳朵；同日 fix 车道：真机报的五处。**本页写现状和下一步**；
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
| 远端会话的终端、对话 | 可用 | 可用；第三步 A 起是手机的会话页：内容在上、标签与操作在下，终端有附件键，见下 |
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

## 第三步 A：会话页和触屏终端

2026-10-02（touch 车道）。负责人看了另一个手机终端：「我可能希望手机版的 session 布局大概是这样。上方完全交给内容，下方是操作与标签页」，答了三问（会话里不显示落地页的底栏、「‹」回列表；标签放不下横着滚；对话同一个样子，附件键的位置放输入框），并定了原则：**窄档就是手机的样子**，一份实现，没有只给手机或只给 iOS 的界面。宽度、指针、能不能托管三个开关各管各的（`docs/UX.md`「姿势」）。全文在《骨架与三种页型》「窄档」那一条，这里只记手机上的事：

- **会话页**（`components/SessionBar`）：没有窗口第一行、页头、底部栏，终端或对话一直画到玻璃上沿（只让开状态栏，`--safe-top`）；脚下一条底栏，第一行是「‹」· 标签（同一个 `TabStrip`，`at="bottom"`）· 滚起来以后的计数 · 键盘开关，第二行是终端的附件键或对话的输入框；底下让开主屏指示条（`--safe-bottom`）。
- **附件键**（`views/TermKeys`，只在手指下、只在终端上）：Esc · Ctrl（一次性的锁，作用到下一颗键，这一行的或软键盘的）· Tab · ⇧Tab · 粘贴（读剪贴板交给 xterm 的粘贴，程序要了就加括号；iOS 上会先弹系统的「粘贴」气泡）· `/ - | :` · ← ↑ ↓ →（照程序的光标键模式写）。按键不抢焦点。参考图行尾那颗「隐藏键盘」没有画：第一行的键盘开关是它唯一的家。
- **软键盘**（`lib/viewport`）：WebKit 不因为键盘缩短页面，只缩可视视口；壳的高度跟着可视视口走（`data-keyboard`、`--app-h`），两行坐在键盘上，终端重新 fit、远端 pty 跟着改 `cols`/`rows`；WebKit 为露出输入框做的平移收回，页面不滚。键盘升起时 `--safe-bottom` 归零。
- **视口**（`index.html`）：`viewport-fit=cover`（页面伸到状态栏和指示条下面，自己让开），`maximum-scale=1`（WebKit 会把 16 以下的输入框在聚焦时放大，xterm 的输入框就是一个——不设的话每点一下终端页面就放大一次），`interactive-widget=resizes-content`（安卓 Chrome 用，WebKit 不读）。
- **手指滚动**：程序拿了鼠标（khor 给自己的分组会话开了 tmux 的 mouse）或在备用屏上时，竖着拖换成滚轮事件交给 xterm，它照编码滚轮的方式发给程序（SGR 64 / 65）；普通屏上是 xterm 自己的触摸滚动（它的历史）。
- **中文输入**：`keepShiftedCommits`（`TerminalPane`）没动，它只接「不在组字、最后一个 keydown 只是修饰键且没抬起」的 insertText；Ctrl 的锁加在它后面（xterm 交出来的字才变控制形式），不碍它。**iOS 上没验到**：模拟器里不点屏就升不起软键盘（WKWebView 只在用户手势里聚焦时给键盘，脚本聚焦、`Toggle Software Keyboard` 都不行，这台 Mac 也不给往模拟器里发点击），拼音组字和 Shift 标点在 iOS 键盘上的事件顺序要在真机或能点的模拟器上看（《行为与门》真 app 清单那一条）。
- **在模拟器里验到的**（iPhone 17 / iOS 26.5，和一台临时桌面节点配对，handoff 001 本节）：会话页的骨架与尺寸（状态栏下 62 起、底栏两行 48 + 44、让开指示条 34）；附件键在真的远端 shell 里：Ctrl 锁 + c 打断 `sleep 100`、↑ 调出上一条、←→、Tab 补全、`/ : - |`；真 WebKit 里的手指拖动（脚本造的 TouchEvent）在 tmux 上发出 SGR 64 / 65，tmux 进了复制模式 [10/374]。**没验到、只在面门里**：软键盘升起时的让位（§19 k 用替身可视视口量）、粘贴（iOS 要点系统的「粘贴」气泡）。
- 门：`check-face` §20（`FACE_ONLY=bar`），摆放在《行为与门》的真 app 清单（2026-10-02 第三步 A）。

## 身份

- **机器指纹**：`hardware::fingerprint` 在 iOS 上是 `UIDevice.identifierForVendor`（哈希后写进主人戳，和 macOS 的 `IOPlatformUUID`、Linux 的 `machine-id` 同一种用法）。改设备名（真机上「张三的 iPhone」→ 别的）不再让 `claim_store` 拒、节点起不来。它只在主线程给：app 在 `run` 一开始（主线程上）先问一次存着，之后谁问都读存着的；万一在别的线程先被问到，经主队列同步问（`dispatch2::run_on_main`）。重启后第一次解锁前 iOS 可能不给，那一次退回比主机名，和没有指纹的平台一样。
- **不进备份**：`.khor` 目录标 `NSURLIsExcludedFromBackupKey`（`backup.rs`，每个进程一次，在 `claim_store` 之后），目录连同里面的 `identity.key`、`root.key` 都不进 iCloud / 电脑备份。恢复到新手机就没有仓、起来铸一个新身份——新设备本来就该是新身份。钥匙进钥匙串（`ThisDeviceOnly`）是更强的一步，没做。
- 依赖：`objc2`、`objc2-foundation`、`objc2-ui-kit`、`dispatch2`，都是 tauri 的 wry 在 iOS 上本来就带的，MIT；`Cargo.lock` 只多几条边，没有新包。

## 第三步之二：扫码加入、前后台（2026-10-02，ios-net 车道）

### 扫码加入网络

邀请设备的二维码里装的就是配对票本身（`khor_gui_core::qr`），所以**一个加入、两种填法**：有相机扫码的那一头，「加入网络」的配对票框旁边多一颗「扫码」，扫到的票填进同一个框、走同一个「加入」（填好就按，因为人已经用对准它表达了选择；加入被拒时框里留着读到的票）。

- **在哪画**：只看壳答的一个事实 `scanner_here`——手机的构建答是（插件编进去了），桌面答否，每个 HTTP 皮（网页脸、dev bridge）答否。答否就**不画**，不画一颗按了拒绝的按钮（第二步的规矩）。模拟器也是手机的构建，所以画；它没有相机，按下去说「无法打开相机」。
- **插件**：Tauri 官方 `tauri-plugin-barcode-scanner`，MIT OR Apache-2.0，Rust 与 JS 都钉在 **2.4 线**（2.5 要 tauri 2.12，这棵树在 2.11，为扫码换整个壳不值）。只在 iOS / Android 编（`#![cfg(mobile)]`）。授权 `src-tauri/capabilities/scan.json`，只给 iOS / Android、只给 `main` 窗口，五个命令：查权限、要权限、扫、取消、打开设置——不给震动，不用插件的 `default` 全集。
- **扫的样子**：用插件的 **windowed** 模式——相机在 webview 后面、webview 变透明。插件的全屏模式**没有退出的路**（只有我们调 `cancel` 才收），所以不用。扫的时候页面给根打上 `data-scanning`：底变透明，`body` 里除了扫码那一层都隐藏（对话框、遮罩一起），那一层上面一句「对准另一台机器「邀请设备」里的二维码」，中间一个取景框，下面「取消」。墨是不随主题变的 `mark-ink-light` 压在 `mark-veil-dark` 上（下面是相机，不是纸）。只认 QR 码。
- **被拒**：相机权限被拒时不扫，说「khor 没有使用相机的权限。在「设置」里允许后再扫，或者粘贴配对票」，下面一颗「打开设置」（系统的设置页）。第一次问、被拒，同一句。
- **局域网那句**（`NSLocalNetworkUsageDescription`，2026-10-03 NET-FIX 轮补上）：没有这个键，iOS 不弹窗、直接丢掉发往局域网地址的 UDP，同一个 Wi-Fi 下的手机连 Mac 也只能走中继。抄语料库 `local-network-usage`，Mac 的 `src-tauri/Info.plist` 也带一份，`npm run mobile` 比对。允没允许：手机「设置 → khor → 本地网络」开关；拒过一次要在那里打开。mDNS 不靠它也不靠 Bonjour：直连候选（本机各网卡地址）由 iroh 在连接里交换，手机不需要多播权限。
- **相机用途那句**（iOS 第一次问权限时的系统弹窗）：`gen/apple/khor-gui_iOS/Info.plist` 的 `NSCameraUsageDescription`，抄的是语料库 `camera-usage`，`npm run mobile` 比对两处一字不差。只有中文一份：英文系统的手机看到的也是中文（要按语言分，得给 Xcode 工程加 `InfoPlist.strings`，没做）。
- **真机才看得见的**：相机真的读码（模拟器没有相机）；**深色主题下的码**——邀请对话框在深色里画浅墨深底（反色），AVFoundation 的 `AVCaptureMetadataOutput` 认不认反色码没有量过，扫不出就请对方切浅色或拷票；对焦距离、屏幕反光；扫码层在刘海与底部横条下的留白。

### 前后台：回到前台换一副新耳朵

iOS 在 app 离开屏幕几秒后挂起它，节点手里的东西都聋了：中继的连接、对端的 QUIC 连接，真机上连 socket 都会被系统收回。桌面的常驻聋了两分钟就退出（`DEAF_EXIT`）、keeper 再起一个；**手机没有谁来再起**，退出在系统看来就是闪退。所以手机的节点**就地换耳朵**：

- **信号**（2026-10-08 ios-rest 车道改过，见下）：UIKit 自己的三条通知，从默认通知中心听（`src-tauri/src/lib.rs` 的 `watch_the_front`）：`UIApplicationDidEnterBackgroundNotification` 放掉所有连接（`link::suspending`），`UIApplicationWillEnterForegroundNotification` 换新耳朵（`link::resume`），`UIApplicationDidBecomeActiveNotification` 时问常驻一句「你在歇着吗」（`link::resting`），在歇着也换新耳朵——兜底，不管是什么让它歇着的。决定是一个小函数 `lifecycle_act`，三件事交给一条线程按系统说的顺序做（`front_worker`）：放掉答完了才轮到后面的恢复；「在歇着吗」和恢复走同一扇门，排在刚发出的恢复后面，所以回前台紧接着变活跃只换一次耳朵。选原生的而不是页面的 `visibilitychange`：它早（页面还没跑一行，节点已经在换了）、不靠页面活着。**不是** `RunEvent::Resumed`——那是事件循环每次从 poll 醒来，一直在发。
- **为什么不用 tauri 的 `WindowEvent::Suspended` / `Resumed`**：tao 0.35 把 `Suspended` 接在 `applicationWillResignActive` 上，`Resumed` 接在 `applicationWillEnterForeground` 上，`applicationDidEnterBackground` 留空。两半不成对：**拉下通知中心或控制中心、系统弹窗（权限请求）、Face ID、来电横幅**都会「将失去活跃」而不去后台——节点歇下了，却没有谁叫醒它，直到下一次真的去后台再回来。这期间别的机器拨进手机，常驻把进来的连接丢掉（`link.rs`：歇着时 `continue`），noq 丢掉一个 `Incoming` 就是答 CONNECTION_REFUSED。负责人看到的就是这个：从手机往 Mac 拖一张图，Mac 照规矩回头拉文件（`Request::DropFile`，收的那头拨手机），得到「无法连接 localhost：server refused to accept a new connection」（`localhost` 是手机在设备表里给自己起的名字）。门：`lib.rs` 的单测 `the_phone_lets_go_in_the_background_and_wakes_in_front`；`cargo test -p khor-node --test resume` 的歇着那条量 `link::resting` 的三个答（醒着、歇着、刚发出恢复就是醒着）。安卓没有 UIKit，照旧用 tauri 的窗口事件（那边是成对的），走同一条线程。
- ⚠️ **不要为此打开 UIScene 生命周期**（`UIApplicationSceneManifest`）：tao 的 scene delegate 是空的，`gen/apple` 没开。三条 `UIApplication…Notification` 在两种生命周期下系统都发，但别的东西（tao 的窗口事件）会跟着变。
- **按顺序做什么**（`khor_node::link::resume`，等的那段是 `khor_gui_core::rejoin::resumed`）：
  1. 常驻 serve（手机上是 `Node::serve_resumable`）**绑一个新端点**，同一把钥匙；旧端点在后台关掉；挂在旧端点上的都放掉——留着的连接（`wires`）、远端会话前面的管子（`reaches`）、远端编辑器前面的管子（`codes`）。不靠 iroh 的 `network_change`：它只在网卡变了时才动，而回到前台的手机多半还在原来那张网上。
  2. 新地址写回设备表自己那一行和 `endpoint.json`；同步计时清零。
  3. 耳朵好了就**叫醒所有在等重连的远端终端和对话**（见下），然后**马上同步一轮**，不等下一拍。
  4. 那一轮回来，壳给页面发 `node:woken`：会话、机器两张表立刻再读一次（`src/hooks/use-woken.ts`）。页面自己的 `visibilitychange` 也会先读一次，那时读到的是节点同步之前的。
- **远端终端、对话怎么接回来**（`khor_gui_core::rejoin`）：没有第二条路。管子断了（换耳朵会把旧管子全关掉），读线程不再说「这个终端结束了」，而是**重新够一次**——就是每次打开远端终端都走的那个 reach（新加的本地 ipc 操作 `Rejoin`，只是答复分得清「没够到」和「够到了、会话不在」），在新耳朵上新起一条管子，远端宿主照旧把它的环（1 MiB）倒给新客户端。终端的 feed 开新的一段，脸上光标落在头外，于是照它原有的规矩重置、重放（`TermChunk::reset`）；对话的帧同理，脸看到 `lost`、清掉再要历史（`ChatBatch::lost`）。**远端答「会话不在了」才算结束**；够不到就等（1 秒起翻倍、最长 30 秒），回到前台立刻再试；没人看着了（`term_leave` / `chat_leave`）就停。这一条只在手机上打开（`rejoin::set_on`）；桌面上远端终端的管子断了，照旧是结束。
- **挂起时**：进程被停住，什么都不转（模拟器里量过：后台 40 秒、90 秒，CPU 时间一秒不长）。被停过的进程醒来，节拍不补发错过的那些（同步、暂停、三个采样器改成 `MissedTickBehavior::Skip`）；桌面上 `Instant` 在系统睡眠里不走，所以没有这种补发，改了也看不出。
- **前台时聋了两分钟**（`DeafWatch`）：手机上不退出，也是就地换耳朵。端点在底下被关了：桌面的 serve 照旧结束，手机的先聋着、等前两样之一。挂起后真机可能把 hand-off 的本地监听收回：手机的 serve 在 accept 出错时重绑一个新端口、写回 `endpoint.json`，不在错误上空转。
- **线上格式没变**：没有新帧、没有新文档、没有新的设备表键。`Rejoin` / `Unreached` 是本机 app 和它自己的常驻之间的 ipc（同一个二进制），尾部追加；老 CLI 不发它。**三台 Linux 不用为此升级。**
- **笔记本合盖**：同一个 `resume` 谁都能调，节点里没有手机专属的东西；桌面没接（桌面 serve 不是 `serve_resumable`，管子断了照旧结束），要接就是挂一个系统唤醒通知去调它，再把 `rejoin` 打开。iroh 在桌面上本来就用墙钟跳变察觉睡醒、自己重绑（`netwatch`），手机上它把那个检查关了（每小时一次）。
- **门**：`cargo test -p khor-node --test resume`（两台真连：关掉一台的端点后它谁也听不见、谁也问不到它；`resume` 之后**同一个 serve、同一个进程**听见对端、那一轮带回对方期间写的东西、对方又问得到它；远端终端的管子随端点断掉，那时重连答「没够到」，`resume` 后重连拿到的回放里有期间写的输出，对一个不存在的会话答「不在了」）。把「换新端点」变成「留着聋的那个」时两条都红。gui-core 单测：重连后终端 feed 让每张脸重置、只给新的一段；对话让每张脸 `lost`；kick 叫醒等待。
- **模拟器实测**（iPhone 17 Pro / iOS 26.5，2026-10-02）：和一台临时桌面节点配对、打开它上面的一个终端，切到「设置」40–90 秒，期间临时桌面上的程序写了一行、改了名字；切回来：同一个进程（pid 不变），回到前台后 **0.46 s / 0.66 s** 新写的那一行在终端里（两次），机器列表上的新名字 **0.57 s**，`node:woken` 在 0.59 s / 0.74 s / 0.67 s 到（那一轮自己 196–280 ms）；没有「结束了」、没有重复的行或会话。临时节点只用一台本机回环上的临时中继（随机密钥），模拟器里的节点用临时家目录（`KHOR_HOME` 经 `SIMCTL_CHILD_` 传进去），都不碰 Khor 中继和 n0。**真机才看得见的**：系统收回 socket 之后的样子（模拟器里只量到进程被停住，socket 有没有被收回量不出）、切到蜂窝网再回来、锁屏很久以后回来、走出 Wi-Fi 时前台的重连。

## 真机报的五处（2026-10-02，fix 车道）

负责人把第三步装到真 iPhone 上，报了五处。逐条：根因、改法、门（每条在 ee144e3 上先红过），模拟器（iPhone 17 / iOS 26.5，和一台临时桌面节点配对：gui-core 的 dev bridge 当节点，临时 `KHOR_HOME` / `HOME` / 身份，私有 tmux `-L khorfix`，只用本机回环上的 `khor relay --dev`，没有网页脸）。截图在 `/Volumes/disk0/khor-wt-fix-shots/`。

1. **「这个 session 没有宿主」**。不是手机走错了路：手机照常 `reach` 到了 Mac，是 **Mac 答错了**。Mac 上那一行是坐在 tmux 里的 Claude Code，khor 给它的终端是一个 **viewer**（`_host tui/… -- tmux new-session -t … ; select-window …`），地址写在 `view.json`；而托管它的机器答远端的 `Reach` 时（`link.rs` 的 `Request::Reach`）先让桥把 viewer 立起来，然后**不管座位**去读 `host.json`——读不到，答 `no-host`。Mac 自己的窗口没事，因为它的终端门读座位（`term_open`）。改：座位说是哪个文件，一处（`LiveKind::terminal_address`），`Reach` 和本机自己 reach 自己都用它；不能托管的机器的本地门（`attach_tmux`、`session_dir`、`is_hosted`）不读任何宿主文件，`Reach` 问到手机先拒。**线上格式没变**（`Reach` / `Reached` 一字未改），所以三台 Linux 不必升级——但它们托管的、坐在 tmux 里的会话，要等它们升级到这一版，手机才打得开。模拟器：临时桌面跑 ee144e3 的 bridge 时，手机点那一行就是负责人看到的那句（`00-old-desk-far-tmux-session-no-host.png`）；换成这一版的 bridge，同一个手机、同一行，0.8 秒打开，屏上是 tmux 里那个程序的输出（`01-new-desk-far-tmux-session-opens.png`）。
2. **机器掉线时 app 卡住**。量出来的根因：**tauri 把不带 `async` 的命令放在主线程上跑**（WKWebView 的 IPC 回调在主线程，同步命令在那里跑完才放）。`term_open` 是同步的，它在主线程上 reach 那台机器，机器不在就等到 reach 的期限。模拟器里 ee144e3 的 app、临时桌面杀掉：点那一行，**整个 app 停了 10.1 秒**——rAF 一个 10 146 ms 的空档，一个只读内存的同步命令（`scanner_here`）在主线程后面排了 9 859 ms。改：会等的九个命令（`term_open`、`chat_open`、`term_drop`、`close_session`、`usage`、`agent_ask`、`takeover`、`takeover_term`、`resume_session`）改成 `async`、身子经 `off_main` 去 tokio 的阻塞池；打开终端、对话时不在等的那段攥着登记表的锁（别的终端的轮询、按键照常），早到的「离开」记在 `landing` 里；连宿主的握手有期限（`host::HANDSHAKE` 15 秒，以前读 `Welcome` 没有期限）；终端在等的时候画骨架（以前是一块空白的板，20-states「绝不」那一格）。这一版同样的场景：等 10.4 秒后说没连上，**这期间 rAF 没有空档**（只有进出会话页那两帧的重画，292 / 143、132 / 93 ms），57 次 `scanner_here` 除了打开那一下（291 ms，页面在挂）全在 100 ms 以内，切会话、机器两张表 23–105 ms。回到前台：开着它的终端时杀掉临时桌面、切到「设置」17 秒再回来，之后两分钟 573 次 `scanner_here` 最慢 121 ms、rAF 只有回来那一帧 352 ms；终端留着最后一屏、不说「结束了」，在后台按 1→30 秒的节拍重连（`02b` 骨架、`02c` 失败的话——英文，见下「第三步」1，`05d` 回来后的会话页）。
3. **窄档回到列表还亮着**。`SideList` 的选中在窄档不画（`useNarrow`），文件树打开的那个文件同理。`05a` / `05c` 里刚开过、退回来的那一行不亮；ee144e3 上亮着（`04-old-list-phone-light-app-dark.png`）。
4. **会话行写着「来自 xxx」**。撤了（`from-device` 进墓碑），第二行只有预览。`04`（旧）有，`05a`（新）没有。
5. **状态栏那一条是灰的**。两半。**页面那一半**是根因：窄档非会话页的让位是外层盒子的 `padding`，露出的是页面自己的底（框 `canvas` 的灰，暗色 `rgb(36, 36, 34)`），压在 L0 的 `side`（`rgb(25, 25, 23)`）上面。改成一条 `side` 色的带子（`data-safe-top`）。会话页本来就对：纸一直画到玻璃上沿，底栏垫着指示条。**原生那一半**：`set_window_theme` 在 iOS 上是空的，窗口的明暗跟着手机走；现在穿在每个窗口上（`overrideUserInterfaceStyle`，跟随时清掉），状态栏的默认样式按窗口给墨。量得到的是窗口确实换了：手机浅色、app 深色时页面读到的 `prefers-color-scheme` 旧版是浅、新版是深；手机深色、app 浅色新版是浅；「跟随系统」跟着手机。**墨本身在模拟器里没看到差别**：iOS 26.5 模拟器里旧版的状态栏墨也已经是白的（看起来按下面的内容取墨），所以「墨看不清」只有负责人那台 iOS 27 的真机能确认。截图：`04`（旧，灰带）、`05a` 手机浅 / app 深、`05b` 同组合的会话页、`05c` 手机深 / app 浅、`05d` 同组合的会话页。

门：`cargo test -p khor-cli --test far_view`、`cargo test -p khor-node --lib handshake`、`cargo test -p khor-gui-core --lib landing`、`npm run mobile` 第 4、5 条、`check-face` 第 21 节（`FACE_ONLY=iphone`），登记在《行为与门》。真机清单在《行为与门》「2026-10-02 真 iPhone 报的五处」。

## 第三步

按轻重：

1. **Rust 那半的语言**：手机上节点的拒绝和设置页里节点给的字是英文的（`machine-cannot-host` 在模拟器里读出来是 「Mac-mini is a phone — …」，设置页「Confirm dangerous actions」），因为 `config::apply_lang` 只在 macOS 读系统语言；iPhone 上要读 `NSLocale`。这是第二步里最显眼的一处没做。
2. **拉到手机**：落到 `Documents/`，`Info.plist` 开 `UIFileSharingEnabled` / `LSSupportsOpeningDocumentsInPlace`，「文件」app 里看得见；然后把「拉过来」放回来。
3. ~~**前后台**~~：做了，见「第三步之二」。桌面合盖醒来、手机换网（`link::network_changed`：Mac 的 `NSWorkspaceDidWakeNotification`、手机的 `nw_path_monitor`、常驻自己的墙钟跑过单调钟）和终端重连时的「正在重新连接」，2026-10-03 NET-FIX 轮做了，见 NET.md「长连接」。
4. 设备名：手机的频道名是主机名（模拟器里是这台 Mac 的名字），真机上该用 `UIDevice.name` 或让人起名。
5. 钥匙进钥匙串（`ThisDeviceOnly`），备份排除之外再加一层。
6. 存储被拒时的界面：主人戳不是这台的（模拟器里把指纹改掉量过），节点起不来，界面停在骨架上不说话；该说 `store-belongs-to` 那一句。
7. 网页脸的字：安全页「锁定 app 窗口和网页脸」在手机上提到了它没有的网页脸。
8. ~~触屏终端（附件键）、安全区和键盘让位~~（第三步 A 做了，见上）、~~扫码配对~~（见「第三步之二」；相机用途那句按语言分还没做）；iPad（`MacIntel` 被当成 Mac）、上架用的无 alpha 图标、真机签名、推送（v1 不做）。
9. 第三步 A 没做、看见了的：触屏上选字拷贝的手势（今天：普通屏 xterm 自己的；tmux 鼠标开着时手指拖动是滚动，选字没有手势）、捏合改字号、横屏（手机横过来超过 760 宽就是宽档）、工作区会话栏里打开的会话（窄档仍是旧的头，没有底栏）、~~状态栏的墨跟 app 的主题~~（fix 车道做了，见「真机报的五处」5）。
