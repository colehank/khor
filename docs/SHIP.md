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

## 4. 五台一起升

协议或记录改了的版本（例：10-13 Phase 5 取消 `chat/<机器>`），新旧不互通——旧 khor 要求同步新版不认的文档，那一轮同步就停。这种版本 Mac、手机、三台服务器要一次打完，中间别停。v1 之前不做旧版兼容，车道报告里会写一行「旧版在哪断」，打包前看一眼。

## 5. 顺序

1 → 2 → 3，Mac 先装是因为 `khor sessions` 和 `khor devices` 要用新 CLI 看结果。三步加起来约二十五分钟（Mac 编七八分钟、iOS 归档十分钟、集群十分钟，后两步可以并行）。
