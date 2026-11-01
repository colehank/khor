# GitHub 构建与分发

本地只开发、Lab和定向验证；正式构建在 GitHub 托管 runner 上进行，不需要用户集群接入 runner。集群安装、NFS共享目录与进程采用由 Khor 处理，见 [INSTALLATION](INSTALLATION.md)。

## 日常入口

1. PR运行 Quality；main产品修改运行 Release candidates（其中复用Quality）。手工可选择分支运行候选构建。默认不签名、不上传Apple、不更新用户机器。
2. 流水线先生成一次frontend，生成的类型和dist一起上传；Quality与各原生平台并行消费同一份，最终候选等待全部检查成功，不在每个平台重新生成前端。Rust 1.95.0、Node 22.19.0、Bun 1.3.14和Xcode26.5固定，依赖使用锁文件。
3. CLI保留Linux x86_64/arm64与Mac Intel/Apple Silicon；Linux用glibc2.31构建，并检查CLI和Pi的实际ABI。Mac同时产生App归档，iOS产生xcarchive。Android/Windows安装包未纳入本轮。
4. 每个产物带源SHA、前端收据、大小/SHA256、run/attempt。原生程序检查实际嵌入的前端，CLI再检查目标Pi完整字节。聚合manifest只有所有平台成功、身份相同才生成。
5. 需要分发Apple版本时手工运行候选并选signed，配置完环境才能成功；缺凭据直接失败，不产出“已签名”假成功。使用同一候选的Distribute an existing candidate，无本机编译或集群部署步骤。

GitHub runner镜像仍会更新；收据记录实际镜像版本。Rust目标目录不跨运行缓存，避免几十GB原生产物累计和旧源错配；Rust包缓存仅main保存，npm与Bun使用各自的下载缓存。前端保留7天，候选14天，分发收据90天；需长期分发的包发布到Release。过期候选要重新构建，不能拼接其它run的部分文件。已有上游成功job由Actions“Re-run failed jobs”保留；上传失败不需要重编Rust。

## 分发与权限

`distribute.yml`只能从main手工运行，输入成功候选run ID。验证该run确为release.yml、源提交已进入main、全部产物及安装器摘要匹配，然后执行所选阶段：

| 阶段 | 行为 |
| --- | --- |
| github-draft | tag必须已存在且精确指向候选源SHA；创建带CLI、Mac App、安装器与manifest的草稿。用户审阅后发布。重复tag不会覆盖旧资产 |
| testflight | 仅签名归档可上传。复用原归档与build number；成功只表示上传成功，Apple处理、可安装和真人测试分开记录 |
| appstore-review | 通过Fastlane选择相同version/build，跳过二进制上传及元数据覆盖，提交审核；automatic_release=false，审核通过后仍由用户上线 |

TestFlight使用app-store-connect导出且不是Internal Only，之后可选择相同build正式提审。iOS build number为`1000+release workflow run_number.run_attempt`，高于现有62；超过Apple组件边界会拒绝。失败上传后再次运行分发仍用原build，不改号码、不重建。Apple协议、审核资料、加密申报等需要在App Store Connect维护；工具失败不代表它们已处理。

Mac当前启用了macos-private-api，按项目既有配置走商店外Developer ID签名与公证，不将这条流水线误称Mac App Store兼容。需要Mac App Store时应单独评估能力与沙盒。Tauri提供打包和签名，Apple渠道仍需独立流程，参考[Tauri分发](https://v2.tauri.app/distribute/)、[iOS签名](https://v2.tauri.app/distribute/sign/ios/)、[App Store](https://v2.tauri.app/distribute/app-store/)。

## 一次性配置

仓库创建以下GitHub environments，并为分发环境配置所需reviewer、仅main可部署：`apple-signing`、`apple-testflight`、`apple-store`、`github-release`。`candidate`不需要凭据。环境门的实际reviewer设置不是YAML能代替的，本轮没有修改账户或环境权限。

在GitHub的Secrets界面配置，不把内容贴进聊天、日志或源码：

| 环境 | Secret |
| --- | --- |
| apple-signing（Mac） | MAC_CERTIFICATE_P12（Developer ID Application证书+私钥，base64）、MAC_CERTIFICATE_PASSWORD、MAC_SIGNING_IDENTITY |
| apple-signing、apple-testflight（iOS） | APPLE_DEVELOPMENT_TEAM、IOS_CERTIFICATE_P12（Apple Distribution证书+私钥，base64）、IOS_CERTIFICATE_PASSWORD、IOS_PROFILE（此bundle的App Store provisioning profile，base64） |
| apple-signing（公证）、apple-testflight、apple-store | APPLE_API_KEY_ID、APPLE_API_ISSUER_ID、APPLE_API_KEY_P8（App Store Connect API key文本） |

这些与APNs无关。App内通知不需要任何上述Apple发布密钥。临时keychain、profile和API文件仅在runner使用，任务退出后清理；iOS构建只临时改签名设置，结束恢复项目文件。已有私钥不会自动从用户Mac导出或上传。

## 验证范围

本地`actionlint`检查workflow与表达式，`scripts/tests/test_release_manifest.py`验证篡改、错源、缺平台、前端/Pi嵌入及未签名拒传。实际GitHub构建结果另记，不用静态检查代替原生编译；实际签名、公证、Apple处理及NFS多客户端验收也独立记录。发布回滚优先选择已验证旧版本；不删除仍可能被节点或会话使用的产物。
