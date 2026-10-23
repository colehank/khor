# 打包与装机（开发期，手工）

> 这是过渡：现在每一版都是在这台 Mac 上手工打到五台机器。规范的做法是 tag 即发布、CI 从干净 checkout 编（`release.yml` 已有 CLI 的一半，Mac app / iOS / 嵌提交号待做，见 LEDGER「问」）。在那之前照这页走，每一步都有验法，坑都是踩过的。

版本号只在 `Cargo.toml` 一处（`apps/gui/src-tauri/tauri.conf.json` 的 `"version"` 字面量不能删，删了 tauri 写 1.0.0）。装了哪一版现在看不出来——五台都报同一个号；认版本靠二进制里本次新加的字符串（下面每段的「验」）。

## 0. 前提

- 在主树 `~/Desktop/working/khor`、分支 `main`、`git status` 干净。worktree 车道都合完再打。
- `target` 链到 `/Volumes/disk0/khor-target`（系统盘不放 target）。
- 指纹采用 Rust 构建时生成的 `khor-ui-assets:/assets/index-<hash>.js`：`khor-web` 从本次 dist 生成，桌面和 iOS 启动日志打印同一收据，网页响应的 `X-Khor-UI` 也带它。先从新 dist 确定本次 hash，再在所有二进制与归档中核对完整收据。它会随前端内容变化，纯前端版本也能识别旧 Rust 库。`gui.*` 文案在手机包里压缩，不能代替这个 Rust 侧收据。

## 1. Mac 本机：关旧开新

```sh
node apps/agent/build.mjs                            # 独立 Pi 运行组件，必须随宿主嵌入
export KHOR_AGENT_BUNDLE="$PWD/apps/agent/dist/khor-agent"
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
sh scripts/build-on.sh hinton        # 同时交叉编译并嵌入 Pi；拒绝负载超过核数一半的机器，glibc 必须正好 2.31
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


## 发布记录：2026-10-19（文件操作、候选交互与历史导航）

- 构建提交：`eae80f9c`，`fix/history-deck` 的五个修复提交已快进合入 main。UI 收据：`khor-ui-assets:/assets/index-BcVMfB20.js`。文件预览移入更多菜单，恢复编辑滚动不再误折叠药丸；候选列表内滚动保留键盘并独占手势，点击外部关闭推荐；普通目录不再显示右侧时间；历史详情增加一致的移动端底部返回及安全区，返回列表恢复位置；内容标签手势绑定渲染时的文档目标，修复句柄交接期间图片切 Markdown 停留旧内容的竞态。
- 主干验证：生成、默认前端检查及 TypeScript、content-model、生产构建、gates（68 scripts / 61 entries）通过。分支已用生产组件在 CUA 检查预览返回、候选点击与滚动、历史返回／安全区／横向 diff 和图片切 Markdown 的红绿回归。未改 Diff 内部横向滚动；未将浏览器操作称为 iPhone 原生验收，实际手机触控及 WKWebView 手感仍待安装后核验。
- Mac：release CLI／GUI 收据与签名通过；旧主窗口 PID 65708 退出，新 PID 92848，实际 5467 监听进程同为新版主窗口。9 个会话辅助进程全部保留，会话记录 11 → 11，失败 0 → 0；原生日志与 HTTP 收据一致。
- 集群：hinton 编译安装，三台自动换新。hinton（3763498）／lecun（2196379）／turing（3365642）的实际 5467 监听进程 `/proc/<pid>/exe` SHA-256 均为 `b493550f598936df05feb1d1bd70c0b24e7389a733a74cfc22f5b3b2ac7e9efb`，与安装文件一致，HTTP 收据相同，CLI 确认三台直连。
- iOS：先移开旧静态库强制重编，新 `libapp.a` 与归档 App 的时间及完整收据通过。Tauri 遇到已记录的 `exportArchive Copy failed`；系统 PATH 导出上传于 `2026-10-19 21:58:35 +08:00` 返回 `Upload succeeded`／`EXPORT SUCCEEDED`，Apple 已开始处理。临时两处签名团队配置按构建前基线恢复。
- 日志与收据：`/Volumes/disk0/khor-ship-20261019-history-content/`；分支交互证据分别保留在 `khor-file-preview-proof`、`khor-suggestion-proof`、`khor-history-proof`。按 bundle ID 和 lsof 无引用核验后清理两份 Khor Xcode 导出副本、旧静态库备份及本轮集群源码导出，共 820,795,073 字节；两份未确认身份的临时目录保留。保留最新正式归档、静态库及现有会话，未清系统 swap 或全量 target。系统盘约余 4.3 GiB，外挂盘约余 91 GiB。


## 发布记录：2026-10-20（浅 U 探索与即时转向）

- 合并／构建提交：`5244e07b`，`feat/pill-arc-explore` 已快进合入 main。UI 收据：`khor-ui-assets:/assets/index-8xmO67xe.js`。根会话、文件与网页共用浅 U 长按探索：压暗内容、不加玻璃底板或额外待选圈，保留真实头像状态环；中央名称使用 body 常规字重、固定两行空间，邻项 callout。回滑以最近折返点反向，无需跨过按下点；保留调速、停留连续翻选、首尾循环和松手确认。
- 验证：主干生成、27 项非浏览器默认检查及 TypeScript、生产构建通过；模型同侧反向先红后绿，分支 px／tokens／gates 通过。正式 DeckBand 的 CUA 响应／几何／手势／探索检查及 390×844 覆盖边界已验，覆盖标题稳定、内部文件、退出阶段取消和后台取消。默认 select／scroll-target 自动浏览器流程未运行；未将 Chrome 合成触摸称为原生 iPhone 验收。
- Mac：release CLI／GUI 签名与完整收据通过；旧主窗口 PID 92848 退出，新 PID 58588，实际 5467 监听 PID 一致。9 个会话辅助进程全部保留，会话记录 11 → 11，失败 0 → 0；原生日志与 HTTP 收据一致。
- 集群：hinton 首次负载略高于脚本阈值，选在 glibc 2.31、负载符合要求的 lecun 编译安装。hinton（685786）／lecun（2424670）／turing（805417）实际监听进程 `/proc/<pid>/exe` 的 SHA-256 均为 `74129abb206e42549c4ae6630be2fe6ca4105911ffaa40312c42e7609fd570ce`，与安装文件一致，HTTP 收据相同。
- iOS：先移开旧静态库以保证重编；新 `libapp.a` 与归档 App 的时间、收据和签名通过。Tauri 遇到已记录的 `exportArchive Copy failed`；系统 PATH 导出上传于 `2026-10-20 15:07:48 +08:00` 返回 `Upload succeeded`／`EXPORT SUCCEEDED`，Apple 已开始处理。临时签名配置按打包前基线恢复。
- 收尾：日志、校验与最新归档记录在 `/Volumes/disk0/khor-ship-20261020-arc-explore/`。按包身份与 lsof 无引用核验后，清理两份 Khor Xcode 导出副本、旧静态库备份和本轮源码导出，共 820,829,011 字节；未确认归属的空暂存目录保留。系统盘约余 3.8 GiB、外挂盘约余 91 GiB；保留现有会话、最新静态库与正式归档，未清系统 swap 或全量 target。iPhone 的原生手势、键盘与网页叠层仍待安装后核验。

## 发布记录：2026-10-21（目录刷新、终端拖放与状态环）

- 构建提交：`6ea64e37`；目录树 `8ac23f07`、头像环 `8819e36e`、拖放 `4d870622` 均已合入 main。完整收据：`khor-ui-assets:/assets/index-DxN-ja4e.js`。
- 目录树：修复本机变化监听不可用时永远沿用旧列表的问题。切回窗口、重新展开及慢周期刷新可见层级；折叠祖先下的隐藏层级不做兜底读取；较早的请求晚到不覆盖较新的结果。真实 ACL2027 的 `schemamem/docs/manuscripts/raw` 文件完整，未修改其文件或会话。
- 拖放：按锁定 Wry 源码修正 AppKit／GTK 的逻辑坐标，Windows 保留物理坐标缩放；原生空 paths 的 Drop 可接回本次工作区 HTML 拖放的来源机器与路径。保留向其他 App 拖出文本路径；取消、离开、新拖、超时及重复渠道分别处理，保留终端共用一次来源解析。
- 状态环：光环与头像 SVG 共用绘制原点和 viewport，仅扩展 circle 半径，避免 WebKit 单独栅格化分数尺寸造成偏心；颜色、粗细、透明间隙与呼吸不变。
- 主干验证：生成、默认前端检查及 TypeScript、生产构建、gates（74 scripts / 67 entries）通过。`tree-freshness` 五项实际 App 回归通过；正式前端与私有节点中实际鼠标从侧栏拖入终端仅发一次 `term_drop`，Finder 事件接缝与框外拒绝通过。`avatar-ring` 的 102 个头像几何检查和 51 个原生 WKWebView 像素样本通过，覆盖 18／22／32／72，最大中心误差小于 0.007 px。真正的 Finder／跨 App 拖动、Windows／Linux HiDPI、iPhone 触控仍须对应设备验收，未把事件模拟称为这些原生操作。
- Mac：release 构建、两枚二进制签名与收据通过；CLI 原子替换。旧 GUI 主进程 58588 退出，新进程 87748 启动并监听 5467，最终仅一枚主窗口。10 个既有辅助进程、10 个会话全部保留，没有新增失败；原生日志和 HTTP 收据一致。未认证 HTTP 返回 403 也携带收据，校验读取该响应头。
- 集群：hinton 编译安装，lecun／turing 的 keeper 自动换新。实际 5467 监听 PID 为 hinton 2232200、lecun 2606733、turing 67544；三台 `/proc/<pid>/exe` 的 SHA-256 均为 `9028710b41703d979834de15ed79e50120109f031aa4c461951f426af784b613`，exe 内及 HTTP 收据一致，10 个既有会话保留、无新增失败、三台直连。编译前移开远端旧缓存产物，防止脚本管道失败误取旧文件；没有反复安装或手工重启另外两台。
- iOS：移开旧静态库强制重编，新 `libapp.a`、归档 App 的时间、完整收据和签名通过。Tauri 遇到已记录的 `exportArchive Copy failed`，系统 PATH 导出上传于 `2026-10-21 02:50:29 +08:00` 返回 `Upload succeeded`／`EXPORT SUCCEEDED`，Apple 正在处理。两处源工程签名文件已按构建前基线恢复。
- 日志、签名、会话快照及收据：`/Volumes/disk0/khor-ship-20261021-workspace-terminal/`。清理三条已合并且无进程引用的 worktree；按包身份和无引用核验，清理两份 Khor Xcode 导出暂存、旧静态库备份和本轮集群源码导出，共 821,207,636 字节。保留当前静态库、正式归档与测试证据；系统盘约余 21 GiB、外挂盘约余 96 GiB，未清系统 swap 或全量 target。

## 发布记录：2026-10-21（文件动作、原生候选、编辑历史与 Codex 宿主识别）

- 最终代码提交：`f57eddef`。合入文件对象动作与预览组合、推荐菜单材质、原生网页候选叠层、Mac 原生撤销／重做与开放标签历史、药丸长按防抖、暂停 Codex 客户端识别。旧 Codex 子代理与药丸暂停参数分支已等价进入 main；旧 Lab 草稿没有覆盖后续正式实现。本次 UI 收据：`khor-ui-assets:/assets/index-BBefLb8C.js`。
- 文件树、普通目录和搜索结果统一对象菜单；本机桌面使用按系统命名的文件管理器定位。打开文件的菜单保留内容动作，预览与更多合为一块玻璃；目录重命名迁移开放标签及草稿，删除只关闭实际后代。原生候选使用共享 lease 与组件快照；跨 WebView 的材质采样不可用处采用已有厚底。
- 主干默认前端检查、TypeScript、生产构建、gates（79 scripts / 72 entries）通过；文件动作六种场景、Chrome／WebKit 菜单材质、16 项编辑历史、真实私有 Session 的定位／重命名／删除与前缀相似目录回归通过。内容门改用私有冷 Vite 缓存，避免共享优化缓存和其他 Lab 引起重载；未放宽产品断言。原生菜单协议 3 项、系统定位 6 项通过。Codex 串行相关回归最终 54 项通过、1 项显式忽略；早先并发锁夹具失败与其他首轮日志仍保留。
- 首次发布后真实核验发现 SJTWorkshop 仍回退成 shell：选择层已排除暂停客户端，最终聚合层却仍把暂停客户端计入 unbound。补完整宿主五状态回归先红后绿，再提交 `f57eddef` 并重建各端。最终真实 CLI 已将 `shell/75187ac565c2415a` 列入 Codex；未修改其进程、终端或识别配置。第一轮发布证据标记为中间版本，不用同一 UI 收据代替后端更新证明。
- Mac：最终 CLI／GUI 签名与完整 UI 收据通过；主窗口 95611 → 49317，仅一枚主窗口，实际 5467 监听 PID 一致。11 个原有辅助进程及 11 个会话全部保留，新增失败 0。最终 CLI SHA-256 为 `7d064de7352f9040327e8832d6804acce019dd21162eefb1410d0dc23246b4b7`，GUI 为 `ffa72559fd00c7e94df2a650f959907476f3cc63d496ac7914c7800a1af0eed0`。
- 集群：hinton 最终构建安装 `f57eddef`，lecun／turing 的 keeper 自动换新；实际 5467 监听 PID 分别为 3402313／2880504／1069047。运行 exe、安装文件与新产物 SHA-256 均为 `7bfedaae90b34ff4866c2f9751da3fe505a8d418012336c5a4a859c8d0e7f2c1`，二进制／HTTP 收据一致；远端 live.rs 与指定提交逐字节校验通过。11 个既有会话均保留，新增失败 0；Mac CLI 确认三台直连。没有手动重启另外两台或重复安装同一代码。
- iOS：首轮构建在发现聚合遗漏后取消，未上传。最终使用验证过且未变的 dist，CLI 临时覆盖 beforeBuildCommand=true，采用 `--archive-only` 重建新 Rust 并归档，避免重复前端构建影响并行编译；新静态库及归档的时间、完整收据和签名通过。系统 PATH 独立导出上传于 `2026-10-21 21:40:15 +08:00` 返回 Upload succeeded／EXPORT SUCCEEDED，App Store Connect 显示正在处理；上传版本为 0.1.14（55）。两处临时签名工程配置按构建前基线恢复。
- 证据与日志：`/Volumes/disk0/khor-ship-20261021-all/`、`/Volumes/disk0/khor-ship-20261021-all-ui/`。按归属和无引用核验，清理本轮 Khor Xcode 导出副本、旧静态库及两份本机源码 export，共 778,626,738 字节；空且无包身份证据的暂存目录保留。两个远端旧产物 backup 因部分进程 fd 权限限制未能完整排除引用，保留在 hinton /tmp（约 254 MB），不占 Mac 系统盘；target、最新静态库、归档、测试证据及用户数据保留。系统盘约余 18 GiB、外挂盘约余 37 GiB，未清系统 swap 或全量 target。
- Windows／Linux 文件管理器 GUI、iPhone 系统分享与触控／输入法／原生网页叠层仍待对应实机验收；编译、协议或浏览器替身通过不代表这些操作已验收。


## 发布记录：2026-10-22（会话内容甲板、网页历史与返回复用）

- 功能提交：`76a4e9e5`，原生接口登记补正：`0da5bc08`。统一 Desktop 会话内目录／文件／网页的双图标玻璃导航与单层位置药丸；文件刷新／保存进入药丸，窄屏恢复外侧返回，浏览器共用主区的历史箭头与药丸内刷新。前端收据：`khor-ui-assets:/assets/index-Bw92yO2s.js`。
- 外侧返回将当前文档标签持久重置为「新会话标签页」，再打开文件／网页复用同一枚；其它标签、顺序与固定状态保留。网页内部前进／后退仍遵循独立历史。原生查询读取平台 WebView 的实际能力，未知／不可后退时灰显后退，不可前进时隐藏前进；仅可见网页轮询站内导航变化。
- 打开网页时采用让出显示：地址编辑、推荐、菜单与任务界面暂时隐藏当前原生网页，完成后恢复同一 WebView。停止使用独立原生候选浮层；工具提示不遮盖网页，也不为提示隐藏网页。补齐冷开请求去重／串行、迟到网页事件不能改写已返回标签，以及文件菜单退出后再定位的焦点修复。
- 验证：主干生成、默认前端检查与 TypeScript、生产构建、gates（81 scripts / 74 entries）通过。分支 content-model、实际前端／私有节点的 content-toolbar、六种文件动作、browser-yield、page-history 均通过，具体红绿证据保留于 `/Volumes/disk0/khor-content-toolbar-proof/`。真实隔离 Mac App 验证文件／网页返回后同标签复用、其它标签保留、原生历史与冷开网页；原生命令表遗漏先红后绿后补理由登记，两项通过。
- Mac：release 编译、CLI／GUI 签名与收据通过。旧主窗口 49317 退出，新主窗口 97455，实际 5467 监听 PID 一致，仅一枚主窗口。11 个既有辅助进程与 11 个会话全部保留，无新增失败；HTTP 收据一致。CLI SHA-256 `f0c0fc9b2772a9ef6d7676b75b55818b2e1878e2f07366dfa2c058498e2d70b1`，GUI `45a59fcd4ce0a9fa6aef1507539db9153cf1d0f0536d1d16eb85540ec4d22f3f`。
- 集群：hinton 编译安装功能提交，lecun／turing keeper 自动换新；实际 5467 监听 PID 分别为 3703100／2969427／1263754，运行 exe 与安装文件 SHA-256 均为 `b6e1da5ffb9628aa62eff67e75cf330cde946289c9da9891b65c9159576ebada`，HTTP 收据一致。三台各 11 个既有会话保留，无新增失败，Mac CLI 确认三台直连。远端四份关键生产源码与 main 逐字节一致；后续接口登记提交只改测试，不改生产代码。
- iOS：沿用已验证 dist，采用 archive-only 归档，新静态库、归档 App 的时间、完整收据与签名通过。系统 PATH 独立导出上传返回 `Upload succeeded`／`EXPORT SUCCEEDED`；上传版本 0.1.14（56），Apple 正在处理。临时签名工程配置按构建前基线恢复。
- 收尾：日志、会话快照与发布收据保留于 `/Volumes/disk0/khor-ship-20261022-content-toolbar/`。清理本次有包身份且无引用的 Xcode 导出副本、旧静态库备份及集群源码导出，共 730,101,982 字节；未确认身份的空暂存目录保留。已移除完成并合入的 content-toolbar worktree；保留最新静态库、正式归档与验收证据，没有全量清 target 或删除系统 swap。
- 平台边界：Mac 原生网页操作已验；iOS 为编译／归档／上传，Windows getter 仅目标平台 metadata 编译，Linux getter未编译或运行。未把这些结果称为 iPhone 原生手势或其它桌面 GUI 验收。

## 发布记录：2026-10-22（原件交接与系统 Office 预览）

- 功能提交 `3ab68026`，iOS Quick Look 归档链接补齐 `b34e6273`，均已合入 main；同时发布此前已提交的 clean 工作区状态 `4e025269`。完整界面收据：`khor-ui-assets:/assets/index-pQqgYXf2.js`。
- 大文件与无法内嵌预览的文件可取回完整原件，再用其他应用打开；进度、取消、重试、来源版本检查和缓存清理共用同一路径。切走内容或隐藏页面会取消未完成请求，迟到结果不打开旧文件；移除会话文件的「在主区打开」。iOS／macOS 的 Office 文件使用原生 Quick Look 只读预览，原件交接仍可作为后备；Android 通过私有 FileProvider 与临时读取授权交给已安装应用。
- 验证：主干默认前端检查、TypeScript、生产构建通过，gates 为 82 scripts／76 entries；三项原件核心测试、真实双节点原件传输及命令表两项通过。正式组件检查了预览失败重试、原件交接、附件、导航后取消、同一内容恢复和甲板高度。真实隔离 Mac App 中 DOCX 标题／段落、PPTX 中文标题／项目布局显示正确，Esc／⌘W 关闭预览后回到原标签，其他标签保留。证据位于 `/Volumes/disk0/khor-mobile-file-proof/`。
- Mac：最终 release CLI／GUI 的签名与完整收据通过；主窗口 97455 → 94569，实际 5467 监听进程同为新版且仅一枚主窗口。12 个原有辅助进程保留，会话记录 14 → 14，无新增失败。CLI SHA-256 `8ad2e74146fda3c4104037e1153382995bfff3127afb7c621cebc60eca2c721a`，GUI `b9000a10f56c595c8612799282c3ce2625eb8450594fb75f5200fdd747f896d9`。
- 集群：hinton 构建并经 onset 安装，lecun／turing 的 keeper 自动换新。实际 5467 监听 PID 分别为 296144／3137612／1731782，运行 exe 与安装文件 SHA-256 均为 `cdce0269378193ba8cfb404a93c37419e450ade44e3e13023ed1f0c00223dda1`；完整二进制及 HTTP 收据一致，既有会话保留且没有新增失败，Mac CLI 确认三台直连。编译前移开远端旧产物，并将四份关键源文件与 main 逐字节核对；没有手工重启另外两台。
- iOS：archive-only 重新编译，最新静态库与归档 App 的时间、完整收据、签名及最终 QuickLook.framework 链接均通过。系统 PATH 独立导出上传于 `2026-10-22 15:58:13 +08:00` 返回 `Upload succeeded`／`EXPORT SUCCEEDED`，上传版本 0.1.14（57），Apple 已开始处理。两处临时签名配置按归档前基线恢复。
- 收尾：发布日志、会话快照、哈希、归档与上传收据保留在 `/Volumes/disk0/khor-ship-20261022-office-preview/`。移除本次已合并且无进程引用的 mobile-file-handoff 与 workspace-clean-status worktree；按包身份和 lsof 无引用确认后清理一份 Khor Xcode 导出暂存与旧静态库备份，共 679,788,118 字节。保留最新正式归档、静态库、目标目录和验收证据；未全量清 target 或删除系统 swap。
- 平台边界：iPhone Quick Look／系统分享仍待安装后的真机操作验收。Android 没有可用 SDK，本次未编 APK 或完成真机验收，也未宣称具备内嵌 Office 排版引擎；Windows／Linux 的外部应用 GUI 操作未实机验收。

## 本机热修复：2026-10-22（Codex 状态扫描内存膨胀）

- 修复提交 `35ee9c02`，已合入 main。用户停在终端时 macOS 报告 khor-gui 占用 21.60 GB；现场主进程采样的 physical footprint 为 5.0 GiB、峰值 7.0 GiB，重启相同代码后很快再次达到 4.0 GiB。12 个会话辅助进程逐个检查均不到 20 MiB。
- 用保留符号的相同代码重新链接，验证原始与新二进制的 `__text` 共 46,977,488 字节逐字节相同，再还原原始采样调用栈。列表刷新与后台 `record_day` 同时进入 `Codex::last_word`，反复把数百 MiB 的 rollout 全量读入并构建 JSON 值，造成大规模分配／复制与分配器保留；本机最大 rollout 约 666 MiB。
- 状态读取改为逐条流式解码少量字段，忽略工具内容／图片等值而不构造它们的字符串与对象；按文件身份、长度和修改时间缓存，增长时从最后一个完整换行继续。并发调用共用缓存锁，缓存限制 256 条；半写入、坏行、原位改写与日志替换分别处理，保留原状态及时间语义。
- 验证：57 项 Codex 相关回归通过、1 项显式忽略；包含增量／无变化零扫描、半写入、16 MiB 单条内容、坏行后继续、替换及等长改写。大内容测试进程峰值 footprint 为 4,964,832 字节，最大 RSS 为 13,385,728 字节。
- 本机：GUI 66837 → 25323，实际 5467 监听 PID 一致，12 个辅助进程全部保留；更新 CLI 后会话记录 15 → 15，无新增失败。真实会话观察数分钟，最终主进程 footprint 128.9 MiB、峰值 181.7 MiB，RSS 约 130 MiB，CPU 约 10%–13%。GUI 保留符号以便后续采样；GUI SHA-256 `4cd69a743639978b17cd4733ea719aa20a609e82ad4f0bee306c998baeb66e22`，CLI `099c7cf0ac880473ab1138a033b4abc57697fe8616d2a77c8a80918a3090ea9b`。
- 范围：本轮替换 Mac GUI／CLI，未重新发布集群或 iOS。界面资源未改变，收据仍为 `khor-ui-assets:/assets/index-pQqgYXf2.js`；后端版本以本次二进制哈希核对。原始及符号化采样、内存快照、回归与构建日志保留在 `/Volumes/disk0/khor-memory-proof-20261022/`。没有修改用户日志、终端内容或系统 swap。


## Pi 手动发布组件（2026-10-23）

Mac 的 Rust 构建必须设置 `KHOR_AGENT_BUNDLE` 为当前平台的 Pi 独立包。`build-on.sh` 会在源码 export 中按目标架构交叉编译 Linux Pi 包，再在远端 Rust 构建时嵌入；远端无需安装 Node。构建失败直接退出，不再从管道尾部成功退出后误装旧产物。iOS 不嵌入运行组件，继续通过账户归属设备执行。发布核验另检查被嵌入运行组件的内容哈希与无依赖环境下的本地模型／认证流程。

## 发布记录：2026-10-23（Pi 模型账户与移动甲板位置）

- 构建提交 `3f8fc03e`，包含 Pi 账户／模型接入 `44b4c0a9`、移动甲板位置 `22af0474` 与手动集群打包补齐。完整界面收据：`khor-ui-assets:/assets/index-DiP7Amoy.js`。
- Pi 账户使用所选归属设备的独立凭据存储，支持 API Key、自定义 Base URL、OAuth 与已有额度账户；默认模型与会话当前模型分开，使用时可切换。静态注册独立包的 OAuth 加载器，避免离开源码目录后缺少动态模块。手动集群发布现在交叉编译并嵌入 Pi，远端不依赖 Node；远端 cargo 失败直接退出，避免管道尾部掩盖失败后误装旧产物。
- iOS 底部甲板统一使用独立的垂直锚点，不再叠加完整安全区与横向间距；保留横向边距、键盘避让及 Android／窄屏桌面规则。iPhone 安全区 34 pt 时，正式组件的主导航、会话文件／网页甲板底边均为 22 pt；键盘打开时为 8 pt，收起后恢复。真实 iPhone 手感仍待安装验收。
- 验证：主干默认前端检查、TypeScript 与生产构建通过；本次独立 Mac Pi 包 40 项测试全部通过、无跳过，Linux 独立包的实际登录／凭据落盘夹具通过，没有调用付费模型。Mac CLI／GUI、Linux CLI 均核对完整 Pi 包实际嵌入；各端界面资源采用同一份已验证 dist。甲板位置与内容导航的正式组件／私有节点验收证据保留于 `/Volumes/disk0/khor-deck-bottom-proof/`、`/Volumes/disk0/khor-dock-content-proof/`。
- Mac：release CLI／GUI 的签名、完整收据和 Pi 嵌入通过；CLI 原子替换。主窗口 25323 → 60408，实际 5467 监听 PID 一致，仅一枚主窗口。16 个既有会话辅助进程与 20 条会话全部保留，无新增失败；运行日志及 HTTP 收据一致。CLI SHA-256 `fcf271101d40d063b944cabb073ae6403e082fef6901053d6a342a0dbad9c657`，GUI `7b37d8c34d131794ed6399e4662e2acfeae83180aa19e4176a021619af2c1ba3`。本次采用标准 release 配置，不沿用上次诊断构建的符号保留设置。
- 集群：hinton 构建并经 onset 安装，turing／lecun 的 keeper 自动换新。实际 5467 监听 PID 分别为 3071148／97068／3485293；运行 exe、安装文件与产物 SHA-256 均为 `b27536491755d0b8df0c995d0bcbfd2afc1c9f27910f5bf792f24e5bb5e455cc`，HTTP 收据相同。远端关键源码与 main 逐字节一致；三台各 20 条既有会话全部保留，无新增失败，Mac CLI 确认三台直连，没有手动重启另外两台。
- iOS：移开旧静态库强制重编，以 archive-only 复用已验证 dist；新静态库、归档 App 的时间、完整收据与签名通过。系统 PATH 独立导出上传于 `2026-10-23 13:43:40 +08:00` 返回 `Upload succeeded`／`EXPORT SUCCEEDED`，上传版本 **0.1.14（58）**，Apple 已开始处理。两处临时签名工程配置按构建前基线恢复。
- 构建环境与收尾：外挂盘起初仅余约 483 MiB，在无编译进程且无引用时清理可重建的 debug/incremental，释放约 14 GiB。发布后按包身份和 lsof 无引用核验，清理本轮源码 export、旧静态库备份和一份 Khor Xcode 导出暂存，共 851,050,922 字节；无法确认身份的空暂存目录保留。保留最新静态库、正式归档、当前二进制和发布证据 `/Volumes/disk0/khor-ship-20261023-pi-dock/`。最终系统盘约余 15 GiB、外挂盘约余 10 GiB；未全量清 target、删除会话或系统 swap。
- 平台边界：本次 iOS 完成编译／归档／上传，未称为真机触控验收；Windows／Android 未打包或发布。集群脚本改动通过 shell 语法检查，本次远端发布使用其等价分步命令与实际产物校验，未将其称为脚本完整端到端回归。
