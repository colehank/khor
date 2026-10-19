# 打包与装机（开发期，手工）

> 这是过渡：现在每一版都是在这台 Mac 上手工打到五台机器。规范的做法是 tag 即发布、CI 从干净 checkout 编（`release.yml` 已有 CLI 的一半，Mac app / iOS / 嵌提交号待做，见 LEDGER「问」）。在那之前照这页走，每一步都有验法，坑都是踩过的。

版本号只在 `Cargo.toml` 一处（`apps/gui/src-tauri/tauri.conf.json` 的 `"version"` 字面量不能删，删了 tauri 写 1.0.0）。装了哪一版现在看不出来——五台都报同一个号；认版本靠二进制里本次新加的字符串（下面每段的「验」）。

## 0. 前提

- 在主树 `~/Desktop/working/khor`、分支 `main`、`git status` 干净。worktree 车道都合完再打。
- `target` 链到 `/Volumes/disk0/khor-target`（系统盘不放 target）。
- 指纹采用 Rust 构建时生成的 `khor-ui-assets:/assets/index-<hash>.js`：`khor-web` 从本次 dist 生成，桌面和 iOS 启动日志打印同一收据，网页响应的 `X-Khor-UI` 也带它。先从新 dist 确定本次 hash，再在所有二进制与归档中核对完整收据。它会随前端内容变化，纯前端版本也能识别旧 Rust 库。`gui.*` 文案在手机包里压缩，不能代替这个 Rust 侧收据。

## 1. Mac 本机：关旧开新

```sh
cd apps/gui && npm run build && cd ../..            # cargo 不跑 vite，不先 build 就是旧脸
touch apps/gui/src-tauri/build.rs crates/web/build.rs   # 不 touch，cargo 不重嵌新 dist
CARGO_INCREMENTAL=0 cargo build --release --bin khor-gui --bin khor
```

验：

```sh
h=$(ls -t apps/gui/dist/assets/index-*.js | head -1 | sed 's/.*index-\(.*\)\.js/\1/')
grep -a -c "index-$h" target/release/khor-gui target/release/khor   # 两个都要 > 0
grep -a -c '<指纹>' target/release/khor-gui
```

关旧开新：

```sh
ps -eo pid,args | grep '[k]hor-gui' | grep -v _host    # 只有一行，那是 app 主进程
kill <那个 pid>                                          # 不要 pkill -x khor-gui
cp target/release/khor ~/.cargo/bin/khor
(nohup target/release/khor-gui >/dev/null 2>&1 &)        # 从仓库目录起，别从已删目录起
khor sessions                                            # 对一遍：没有新的「失败」
```

坑：

- **`pkill -x khor-gui` 会把会话宿主一起杀**。宿主是同一个二进制 re-exec 的，进程名相同，带 `_host` 参数；杀了它 shell 没有退出码 → 行写「失败：宿主被结束」，没落 ring 的内容全丢（10-11 杀过 meu-use、OfficeHours）。
- `cargo check/build` 报 `failed to read plugin permissions …/<别的 target 路径>/…`：`target/{debug,release}/build/{tauri-*,khor-gui-*,khor-web-*}` 里是从别的 worktree 搬来的产物，烤了绝对路径，`rm -rf` 它们重编。
- 原生验收别点负责人真 app 的按钮（`npm run native` 起临时 app）；起窗口的门先看 `HIDIdleTime > 120 s`。

## 2. iPhone：TestFlight

```sh
cd apps/gui
T=$(defaults read com.apple.dt.Xcode IDEProvisioningTeamByIdentifier | grep -o 'teamID = [A-Z0-9]*' | head -1 | awk '{print $3}')
APPLE_DEVELOPMENT_TEAM=$T https_proxy=http://127.0.0.1:7897 \
  npm run tauri -- ios build --export-method app-store-connect
```

这一步最后报 `exportArchive Copy failed` 退出 1 是**正常的**（Homebrew 的 rsync 在 PATH 前面），归档已经在 `src-tauri/gen/apple/build/khor-gui_iOS.xcarchive`。扫码插件要从 GitHub 拉 Swift 包，所以要代理。

**上传前必须验归档**（10-12 深夜传过一次旧库，负责人装了才发现）：

```sh
BIN=src-tauri/gen/apple/build/khor-gui_iOS.xcarchive/Products/Applications/Khor.app/Khor
ls -la $BIN                                              # mtime 是刚才
ls -la /Volumes/disk0/khor-ios-main/Externals/arm64/release/libapp.a   # mtime 也是刚才
grep -a -c '<指纹>' $BIN                                 # ≥ 1
```

为什么会传旧的：`gen/apple/Externals` 软链到 `/Volumes/disk0/khor-ios-main/Externals`，tauri 的 `xcode-script` 看 `libapp.a` 还在、指纹没变就跳过 Rust 编译，Xcode 拿旧库归档，「Upload succeeded」照样打出来。换过 target、或在别的树里编过，都可能撞上。

上传：

```sh
cd src-tauri/gen/apple
PATH=/usr/bin:/bin:/usr/sbin:/sbin:/Applications/Xcode.app/Contents/Developer/usr/bin \
  https_proxy=http://127.0.0.1:7897 \
  xcodebuild -exportArchive -archivePath build/khor-gui_iOS.xcarchive \
    -exportOptionsPlist /Volumes/disk0/khor-redesign-home/export-upload.plist \
    -exportPath /Volumes/disk0/khor-redesign-home/ipa-asc -allowProvisioningUpdates
# 看到 Upload succeeded / EXPORT SUCCEEDED
cd ~/Desktop/working/khor
git checkout -- apps/gui/src-tauri/gen/apple/khor-gui.xcodeproj/project.pbxproj \
                apps/gui/src-tauri/gen/apple/khor-gui_iOS/Info.plist   # 团队号不进仓库
```

plist 里 `manageAppVersionAndBuildNumber` 开着，构建号 App Store Connect 自己递增。之后负责人在 TestFlight 里更新。插线装是备用（`--export-method debugging` + `xcrun devicectl device install app`）。

## 3. 集群：hinton 编，另外两台自己重启

三台（turing / hinton / lecun）共享 NFS 家目录，`~/.local/bin/khor` 是同一个文件。只在一台上跑：

```sh
sh scripts/build-on.sh hinton        # 约十分钟；拒绝负载超过核数一半的机器，glibc 必须正好 2.31
```

另外两台的 keeper 盯着磁盘上的文件，变了就写一行「khor on disk changed version … restarting gracefully」自己重启，新进程从 `/tmp/khor-shield-*/` 的本地副本起。

验：

```sh
ssh turing 'tail -3 ~/.khor/serve.log; strings ~/.local/bin/khor | grep -c <指纹>'
ssh lecun  'tail -3 ~/.khor-hosts/lecun/.khor/serve.log'      # hinton 同理 ~/.khor-hosts/hinton/.khor
khor devices                                                   # 五台都在线
```

坑：lecun 的钟慢十几分钟，它日志里的 restarting 时间别拿本机时间比，看新 serve 进程的 `lstart` 在那行之后就行。旧 serve 进程会作为 keeper 外壳多挂一会儿、没有 socket，不是没重启。

安装脚本若报 `serve did not stay up`，且本次日志只有 keeper 正在把 NFS 二进制移到本地，先等它完成并核查实际监听进程。本轮遇到过脚本先超时、服务随后正常就绪：用 5467 的监听 PID 对应 `/proc/<pid>/exe` 校验和及 `X-Khor-UI` 收据确认版本。`serve.pid` 可能指向 keeper 外壳；核验完成后记录结果即可，避免重复安装打断已经正常运行的服务。

## 4. 五台一起升

协议或记录改了的版本（例：10-13 Phase 5 取消 `chat/<机器>`），新旧不互通——旧 khor 要求同步新版不认的文档，那一轮同步就停。这种版本 Mac、手机、三台服务器要一次打完，中间别停。v1 之前不做旧版兼容，车道报告里会写一行「旧版在哪断」，打包前看一眼。

## 5. 顺序

1 → 2 → 3，Mac 先装是因为 `khor sessions` 和 `khor devices` 要用新 CLI 看结果。三步加起来约二十五分钟（Mac 编七八分钟、iOS 归档十分钟、集群十分钟，后两步可以并行）。


## 6. 磁盘收尾（2026-10-16）

本轮查到独立测试每轮遗留约 417 MiB 的 bridge/CLI 副本、旧 Codex fixture 的后台 updater、Xcode 导出暂存和 Chrome 签名副本。主 target 在外挂盘，也不能保证系统盘不会涨；Rust 并行编译还可能增加系统管理的 swap。

- 打包前后记录 `df -h /System/Volumes/Data /Volumes/disk0`。iOS 构建/导出使用外挂盘临时目录：先 `mkdir -p /Volumes/disk0/khor-tmp`，为构建命令设置 `TMPDIR=/Volumes/disk0/khor-tmp`；`target`、Externals 和 archive 继续沿用外挂盘路径。
- `content-ui`、`attachment-ui`、`session-content` 在 finally 关闭私有会话/浏览器/服务后删除 scratch 中的 bridge/khor 副本，保留截图、日志和 fixture 状态。手动中断后核对遗留进程及副本。Codex fixture 的 HOME 下 updater 也须退出，不能只关测试前台进程。
- 上传成功后清理已确认属于 Khor 的 `XcodeDistPipeline.*` 暂存副本，保留本次归档与发布收据。Chrome 的 `code_sign_clone` 是浏览器产生的可重建副本，只有 lsof 确认无引用时才清；不清用户浏览器 profile。
- 大扫除只处理已识别的测试副本和可重建缓存。没有 cargo/rustc 运行时可清 debug/incremental；保留正在使用的发布二进制、用户会话、测试证据与最新归档，不全量 cargo clean，也不手动删系统 swap。

## 发布记录：2026-10-18（选择框、底栏手势、即时已读）

- 代码提交：`48abccbd`；合入 `picker-consistency`、`fix-content-pan`、`fix-immediate-read`。本次 UI 收据：`khor-ui-assets:/assets/index-CCoiSSoh.js`。
- 主干验证：前端默认检查、生产构建、门索引、iOS 编译门通过；GUI core 命令表 2 项及已读快照 6 项通过。`page_sheet` 已按原生专用接口补入带理由的命令表例外。
- Mac：release 编译、CLI／GUI 收据与签名核对通过；旧主窗口 PID 58629 退出，新 PID 57858；11 个既有会话辅助进程全部保留，装机前后列表没有失败项。
- 集群：hinton 构建并安装，turing／lecun 的 keeper 自动换新。三台 5467 实际监听进程的 `/proc/<pid>/exe` SHA-256 均为 `601798415bd4501e084011a30759bcc8a0f4ca673840565f53e342082a003892`，HTTP 收据相同，CLI 确认三台直连。首次 rsync 断线，加 SSH 保活后重试成功；自动换新期间监听短暂为空，待进程就绪后核验，不重复安装。
- iOS：新 `libapp.a` 与归档可执行文件均验到本次收据；Tauri 遇到已知 `exportArchive Copy failed`，系统 PATH 的导出上传在 `2026-10-18 14:18:52 UTC` 返回 `Upload succeeded`／`EXPORT SUCCEEDED`，App Store Connect 开始处理。签名团队的临时工程改动已恢复。
- 收尾：保留正式归档与发布日志 `/Volumes/disk0/khor-ship-20261018-read/`；按 bundle identifier 确认归属、`lsof` 确认无引用后，删除 2 个 Khor Xcode 暂存目录（218 MiB）及本次集群导出源码。系统盘可用从约 12 GiB 到约 11 GiB，外挂盘约 105 GiB。
- `TMPDIR` 并不能保证 Xcode 的所有导出副本都去外挂盘：本轮 `XcodeDistPipeline.*` 仍出现在系统临时目录。上传后同时检查系统临时目录和指定临时目录，按归属与引用核验后清理；不碰正式归档、会话及系统 swap。
- iPhone 实机的网页手势、选字及跨设备已读手感仍需安装此包后验收；编译和发布成功不代表完成了这些真机操作。

## 发布记录：2026-10-18（药丸恢复内容）

- 合并提交：`455e9c2a`，修复提交：`4e3ce55c`。药丸恢复最近的工作区、浏览器来源页、文件或网页；会话列表与内部标签列表不再覆盖内容恢复位置。UI 收据：`khor-ui-assets:/assets/index-CCOkGSI1.js`。
- 主干验证：默认前端检查与 TypeScript、`content-model`、生产构建、门索引通过。首轮默认检查与生成并行，读取了未完成的生成文件；生成结束后完整重跑通过。独立节点的实际 App 窄屏操作已在修复分支验过，未进行 iPhone 真机操作。
- Mac：release 编译完成，CLI／GUI 签名与收据通过；旧主窗口 PID 57858 退出，新 PID 54135。11 个既有会话辅助进程全部保留，发布前后会话行未丢失、没有新增失败项，最终仅一个主窗口进程。
- 集群：hinton 构建并安装，turing／lecun 自动换新。三台 5467 监听进程的二进制 SHA-256 均为 `61b658b437064fed6f2e7555cb477fc518d547da971a794cd4730dc50f58129e`，HTTP 收据一致，CLI 确认直连。turing 首次核验遇到重启空窗，待就绪后通过，未重复安装。
- iOS：先移除可重建的旧 `libapp.a`，新静态库与归档均验到本次收据。Tauri 遇到文档已记录的 `exportArchive Copy failed`，系统 PATH 的导出上传于 `2026-10-18 16:28:58 +08:00` 返回 `Upload succeeded`／`EXPORT SUCCEEDED`；App Store Connect 正在处理。工程仅临时增加的两处签名团队配置已恢复。
- 日志与收据：`/Volumes/disk0/khor-ship-20261018-pill/`。保留最新正式归档；按 bundle ID 和无进程引用核验后清理本轮两个 Khor Xcode 暂存副本（约 203 MiB）及已完成的集群源码导出（约 47 MiB）。系统盘约余 11 GiB，外挂盘约余 104 GiB。
- 此版未包含后台返回时键盘区域空白的修复。

## 发布记录：2026-10-18（统一弹层、窗口轮廓与会话内容身份）

- 代码合并提交：`a61b8a43`。本次合入 `fix-window-outline`（`144522ec`）、`fix-dialog-bounds`（`287235b8`、`5b5a083d`）与 `fix-content-identity`（`17a724f4`），同时发布此前 main 中的 `618a4d59`／`92743b77`：空浏览标签关闭与重复点击药丸时侧区同步收起。UI 收据：`khor-ui-assets:/assets/index-87SAChJZ.js`。
- 主干验证：默认前端检查与 TypeScript、`content-model`、`content-sheet`、`overlay-policy`、生产构建、门索引通过。各修复分支已通过隔离生产组件／实际 App 的 CUA 操作检查；本次打包没有声称完成 iPhone 真机惯性、键盘动画或 Windows/Linux 原生合成器验收。
- Mac：release 构建完成，CLI／GUI 签名与收据通过；旧主窗口 PID 54135 已退出，新 PID 25312。9 个既有会话辅助进程全部保留，装机前后 12 条会话记录未丢失、没有新增失败项。
- 集群：hinton 构建并安装，turing／lecun 自动换新。三台 5467 监听进程的二进制 SHA-256 均为 `52becc8647fd1f69bff321fd9a0379820a9e6eea66960f8d2e55b29a98f33b1b`，HTTP 收据一致，CLI 确认三台直连。
- iOS：重建后的 `libapp.a` 与正式归档均验到本次收据及新时间。Tauri 遇到已记录的 `exportArchive Copy failed`，系统 PATH 的导出上传于 `2026-10-18 20:13:12 +08:00` 返回 `Upload succeeded`／`EXPORT SUCCEEDED`，App Store Connect 开始处理。临时增加的两处签名团队配置已恢复。
- 日志、校验与清理清单：`/Volumes/disk0/khor-ship-20261018-overlays/`。发布后按包身份、构建已结束和 lsof 无引用检查，清理旧 Khor 源码导出、Khor Xcode 导出副本、退役 Codex 工作区夹具、Khor Xcode intermediates 与 npm `_cacache` 下载缓存；保留 `_npx`、node_modules、现有会话、最新正式归档与静态库。VS Code 的暂存版本更新于安装版，保留其待安装更新。
- 系统盘释放约 1.53 GiB，可用由约 8.6 GiB 增至约 10 GiB；外挂盘另清掉约 47 MiB 的本轮源码导出。未清理系统 swap 或全量 cargo target。

## 发布记录：2026-10-19（药丸连续预选）

- 构建提交：`d63e8d43`；功能 `45a38f7c`、原生网页避让 `0c3abeb2`、本地占位依赖锁文件 `8885fe44` 均已合入 main。前端收据：`khor-ui-assets:/assets/index-B7CmfXd0.js`。
- 根会话与内部内容药丸共用直接横滑／长按预选，按起点偏移调速、逐项停靠、首尾循环，松手只打开最终目标；原生网页为选择带让出底部区域，保留 WebView。
- 验证：锁定依赖的 `npm ci --ignore-scripts`、默认前端检查、TypeScript、生产构建、content-model、content-sheet、gates 通过。CUA 在正式 DeckBand 上分别检查根层和内部层的 11 个事件／取消／收尾场景；真实 Chrome 鼠标横拖验证末项到首项只提交一次。完整 face 浏览器门、iPhone 原生触摸／键盘／WKWebView 手感未实测。额外 palette 门停在旧未读元素夹具的超时，未声称该门通过；本轮没有配色 token 改动。
- Mac：最终 CLI／GUI 签名与收据通过，旧主窗口 PID 45358 已退出，新主窗口 PID 10447；8 个既有会话辅助进程全部保留。会话行数 10 → 10，没有新增失败；HTTP 与运行日志均为本次收据。
- 集群：hinton 负载超过脚本上限，改在 glibc 2.31 的 lecun 编译并安装。lecun／hinton／turing 的 5467 实际监听进程 SHA-256 均为 `1a8e75e2b360478f5a2b2bf62f521860383242a156da16d95bfdd9e952a36a66`，三台 HTTP 收据一致；校验的是 `/proc/<pid>/exe`，不是 keeper 的 PID 文件。
- iOS：新静态库与归档 App 均验到本次收据；Tauri 遇到已记录的 `exportArchive Copy failed`。系统 PATH 下的导出于 `2026-10-19 04:01:14 +08:00` 返回 `Upload succeeded`／`EXPORT SUCCEEDED`，App Store Connect 正在处理。临时签名团队改动已恢复。
- 构建环境：初次临时安装绕过不完整的锁文件，带入了 Tauri JS 2.12；随后补齐 elk 本地占位包的锁记录，以冻结安装恢复 API 2.11.1／CLI 2.12.1，与 Rust 2.11.5 配合。最终 Mac、iOS 与集群均重新构建并按同一收据核验；冻结安装未改依赖版本。依赖留在外挂盘的 `/Volumes/disk0/khor-gui-locked-20261019/node_modules`。
- 收尾：按 bundle identifier 与无进程引用确认后，清理两个 Khor Xcode 导出临时目录，共 243,284,757 字节；清理了未使用的临时依赖缓存与旧静态库备份。保留本次归档、当前 Externals 静态库和发布日志 `/Volumes/disk0/khor-ship-20261019-pill-scrub/`。

前端依赖外置时，物理目录必须保留 `…/node_modules/包名` 层级，不能把包直接放在改名后的目录根，否则 Node 的上级模块解析与 TypeScript 会失效。锁文件缺失本地 `file:` 占位包时，应补齐锁记录后执行 `npm ci`；不以 `--package-lock=false` 绕过锁定版本。

## 发布记录：2026-10-19（药丸裁切与文档标签保留）

- 合并／构建提交：`39d05083`，修复提交 `6a6c9ff5`、`e3fe715d`。本次收据：`khor-ui-assets:/assets/index-DPFftrn2.js`。圆弧轮盘仅留在 `lab/pill-arc`，未合入或发布。
- 药丸：隐藏邻项不再因内边距露出头像；右滑以固定内部排版原地揭露；长按选择带按整条甲板对齐，手指移出药丸或向下移动仍持续选择，松手才确认；连续反向后的收尾保持运动方向。
- 内容标签：仅空白起始标签首次填入文档；从来源列表再打开其他文件／网页时新增标签，重开已有地址复用已打开文档。返回浏览只改变观看位置，不改文档标签身份；新文档保留来源目录、查询与列表滚动；网页内部跳转、地址编辑和重命名仍更新原标签。
- 主干验证：生成、默认前端检查与 TypeScript、`content-model`、`content-sheet`、生产构建、`gates` 通过。修复分支的正式 DeckBand 根层／内部层几何检查、长按移出／收尾与真实鼠标拖动已验；完整旧版 face／content-ui 浏览器门及 iPhone 原生触摸、键盘、WKWebView 手感未作为本轮通过项。
- Mac：release CLI／GUI 的收据与签名通过，旧主窗口 PID 10447 退出、新主窗口 PID 62879 启动，最终仅一枚主窗口。8 个既有辅助进程全部保留，会话快照 11 行 → 11 行（含表头），失败数 0 → 0；原生日志与 HTTP 收据一致。
- 集群：hinton 负载符合脚本要求，在其上编译并安装；hinton／lecun／turing 的 5467 实际监听进程 `/proc/<pid>/exe` SHA-256 均为 `5e3729585863d83f53392af6c9b81b66027cde0fe1612e8635467861f0b53b92`，与安装文件相同，HTTP 收据一致；CLI 确认三台直连。
- iOS：重编后的静态库与归档 App 均通过新时间及完整收据核验。Tauri 遇到已记录的 `exportArchive Copy failed`；系统 PATH 导出上传于 `2026-10-19 09:24:33 +08:00` 返回 `Upload succeeded`／`EXPORT SUCCEEDED`。App Store Connect 继续处理；临时增加的两处签名团队配置已恢复。
- 日志与校验：`/Volumes/disk0/khor-ship-20261019-pill-fixes/`；药丸几何证据在 `/Volumes/disk0/khor-pill-geometry-proof/`。构建与临时目录外置；上传后按 bundle ID 与 lsof 无引用核验，清理两份 Khor Xcode 导出副本及已替换的旧静态库备份，共 770,845,250 字节，保留最新归档、静态库和发布收据。收尾系统盘约余 9.8 GiB，外挂盘约余 101 GiB；未清系统 swap 或全量 cargo target。


## 发布记录：2026-10-19（长按栏轻玻璃与选中高亮）

- 构建提交：`6d69a497`，已合入 main。UI 收据：`khor-ui-assets:/assets/index-BOhLSlqv.js`。长按选择带移除顶部序号、缩短高度、增加项目间距；中央复用现有 `glass-sel` 高亮与轮廓，头像状态环语义不变。单标签不展开选择带；标题保留单行省略，普通横滑的药丸内序号保留；减少透明度时使用厚底。未采用圆弧轮盘或两行标题布局。
- 验证：主干生成、默认前端检查与 TypeScript、`content-sheet`、生产构建、`gates` 通过。CUA 对正式 DeckBand 完成根层 9 项几何／11 项手势检查、内部层 9 项几何检查，核对单标签无浮层、材质模糊与中央 `glass-sel`；Lab 深浅主题已查看。完整 face 浏览器门、iPhone 原生触控／键盘／WKWebView 手感未作为本轮通过项。
- Mac：release CLI／GUI 收据与签名通过；旧主窗口 PID 62879 退出，新 PID 63652，最终仅一个主窗口。8 个会话辅助进程全部保留，既有会话 10 → 10，失败数 0 → 0；原生日志与 HTTP 收据一致。
- 集群：hinton 编译安装，三台自动换新；hinton／lecun／turing 的实际 5467 监听进程 `/proc/<pid>/exe` SHA-256 均为 `1f1b001ca3146014b511381e61580d1107260cbce48c62cbb7826f973e584b1c`，与安装文件一致，HTTP 收据相同；CLI 确认三台直连。
- iOS：先移开旧静态库强制重编，新 `libapp.a` 与归档 App 的时间、完整收据通过。Tauri 遇到已记录的 `exportArchive Copy failed`；系统 PATH 导出于 `2026-10-19 11:02:09 +08:00` 返回 `Upload succeeded`／`EXPORT SUCCEEDED`，Apple 正在处理。临时两处签名团队配置已按构建前基线恢复。
- 日志与证据：`/Volumes/disk0/khor-ship-20261019-selector-glass/`。按 bundle ID 与 lsof 无引用核验后清理两份 Khor Xcode 导出暂存及旧静态库备份，逻辑大小合计 770,848,787 字节；保留当前静态库、正式归档和收据。系统盘约余 8.8 GiB、外挂盘约余 100 GiB；未清系统 swap 或全量 target。


## 发布记录：2026-10-19（终端可读性与长按响应）

- 构建提交：`f27bb64d`；终端修复 `c9388e4d`、药丸响应 `f27bb64d` 均已合入 main。本次 UI 收据：`khor-ui-assets:/assets/index-DZVvR-5R.js`。可读性保护补齐 DIM 完整对比度目标，光标按所在格子背景取色，保持形状与闪烁策略；不改 PTY 字节、原始缓冲或颜色应答。长按唤出 380 → 320 ms，长按后翻选等待 400 → 180 ms；按项目宽度计算响应并容忍轻微抖动，保留标题停顿、首尾循环和松手才进入。
- 验证：主干生成、默认前端检查与 TypeScript、生产构建、门索引通过。终端分支的真实 xterm DOM／WebGL 对比、关闭保护、光标形状、主题和渲染器切换已验；药丸根层与内部层各 4 项响应／11 项手势／9 项几何检查通过。iPhone 原生触控、系统触感与浅色终端仍待安装后真机验收，未将浏览器合成事件称为真机手感验证。
- Mac：release CLI／GUI 收据与签名通过；旧主窗口 PID 63652 已退出，新 PID 65708，实际 5467 监听进程也是新版主窗口。9 个会话辅助进程全部保留，会话 11 → 11，失败 0 → 0；原生日志与 HTTP 收据一致。
- 集群：hinton 编译安装，三台自动换新。hinton／lecun／turing 的实际 5467 监听进程 `/proc/<pid>/exe` SHA-256 均为 `39f0101f42b2e842aaee8a367d23a57af375e76f96ab53601b42a95c9f472ba8`，与安装文件一致，HTTP 收据相同；CLI 检查通过。
- iOS：先移开旧静态库强制重编，新 `libapp.a` 与归档 App 的时间、完整收据通过。Tauri 遇到已记录的 `exportArchive Copy failed`；系统 PATH 导出上传于 `2026-10-19 17:22:17 +08:00` 返回 `Upload succeeded`／`EXPORT SUCCEEDED`，Apple 正在处理。两处临时签名团队配置已按构建前基线恢复。
- 日志与证据：`/Volumes/disk0/khor-ship-20261019-legibility-response/`；分支证据在 `/Volumes/disk0/khor-terminal-legibility-proof/` 与 `/Volumes/disk0/khor-pill-response-proof/`。按包身份及 lsof 无引用核验后清理两份 Khor Xcode 导出副本、旧静态库备份与本轮集群源码导出，共 820,731,073 字节；未确认身份的临时目录保留。系统盘约余 5.6 GiB，外挂盘约余 92 GiB；保留最新归档、静态库、发布收据与现有会话，未清系统 swap 或全量 target。
