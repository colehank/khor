# GitHub 构建与分发

正式构建由 GitHub 托管 runner 完成。本地只开发、Lab和定向验证；集群下载、SSH代下载上传、NFS目录与进程采用由 Khor 处理，见 [INSTALLATION](INSTALLATION.md)。发布不会自动替换用户机器上的程序。

## 版本与触发规则

产品版本按发布批次递增，不随每个commit或workflow重试变化。修复版本递增末位（如0.1.14→0.1.15）；新功能发布递增中间位（如0.1.x→0.2.0）。当前0.x阶段发生不兼容改动时也递增中间位，并在发布说明中说明迁移要求；1.0之后的不兼容改动递增首位。

准备候选前，同步Cargo工作区版本、Cargo.lock中的工作区包版本、Tauri应用版本和已跟踪的iOS版本元数据。CI再次核对tag与产品版本。`rc.N`表示该产品版本的候选轮次；Apple build number由CI生成，无需手工递增。私有npm包的`package.json`版本不作为产品版本来源。

| 操作 | 自动结果 |
| --- | --- |
| PR、普通main提交 | Quality检查与测试，不构建分发包、不上传Apple |
| 推送`vX.Y.Z-rc.N` | 全部Quality门和七平台产物通过后，上传TestFlight并确认Apple处理成功，再发布GitHub预发布资产 |
| 在通过的候选提交上推送`vX.Y.Z` | 读取已经发布的候选，复用同一批字节发布GitHub正式版本，并将同一Apple build提交App Store审核 |
| Apple审核通过 | 保留手动上线；自动流程不替用户上线 |

`rc.N`从1开始。Tag发布后不能移动或删除；修复用下一个rc。Git tag带候选后缀，Cargo、Tauri和iOS应用版本都保持数字`X.Y.Z`。不同候选由tag、源码SHA、Actions run及iOS build number区分。正式发布不改包内版本、不改build number、不重新编译。

候选tag的提交必须已经进入main，且Cargo/Tauri版本与tag的数字部分一致。先将版本变更合入main，再打候选tag。例如：

```sh
git tag -a v0.1.15-rc.1 <main提交SHA> -m 'Khor 0.1.15 release candidate 1'
git push origin v0.1.15-rc.1
```

验收后，正式tag必须指向这个候选的同一提交，不能直接指向已经继续开发的main：

```sh
git tag -a v0.1.15 'v0.1.15-rc.1^{commit}' -m 'Release tested Khor 0.1.15'
git push origin v0.1.15
```

正式流程只接受同版本、同源码、已发布且不可变的候选。若同一提交有多个候选，不猜测latest；使用`Distribute tagged release`手工重试并填写`candidate_tag`明确选择。没有候选就失败，不回退成重新编译。

## 构建与永久产物

`release.yml`只自动响应rc tag；手工`Release candidates`仍可在main进行签名或未签名诊断构建，main手工构建不会自动分发。

前端只生成一次，Quality与原生平台消费相同dist与生成类型。四个CLI（Linux x86_64/arm64、Mac Intel/Apple Silicon）、两个Mac App和一个iOS archive分别产出收据，再聚合manifest。收据包含源SHA、前端标识、run/attempt、大小与SHA256；实际原生程序检查嵌入前端，CLI/Mac App还检查嵌入Pi及Linux glibc 2.31下限。

`agent-runtime` 为每个目标平台制作一次 Pi，CLI 的 `native` 与 `mac-app` 校验源码、目标平台、run 和文件摘要后复用相同字节。两类原生构建独立调度，各有90分钟预算；Mac App不再接在CLI编译之后共用一个任务时限。中间的 `agent-runtime-*` 工件不进入最终七平台发布集合。

工具链使用Rust1.95、Node22.19、Bun1.3.14、Xcode26.5及锁文件。Rust目标目录不跨运行缓存。Actions前端保留7天，候选14天；成功rc会将七平台产物、iOS归档/构建号、manifest、原始安装器及TestFlight记录保存在GitHub不可变预发布资产。正式晋升从该预发布读取，Actions文件过期不会导致重编。

GitHub资产先上传到草稿，逐个校验摘要和大小，全部就绪才发布。重试仅补齐缺失的相同文件；发现同名不同内容立即拒绝，绝不覆盖。发布后的资产由GitHub immutable releases保护；正式Release中的程序字节与候选一致。默认latest只指向正式版本。

## Apple阶段与失败重试

Mac 构建与公证分为三个独立任务。`mac-app` 编译 App 和同版本 CLI，以 Developer ID Application 签名，再用独立的 Developer ID Installer 签发 PKG。原始 PKG 与源码、前端、Pi、文件摘要收据保存在 `mac-build-<平台>`；Tauri 构建子进程不接收公证凭据。`mac-submit` 先保存提交意图，再非等待提交 Apple，立即将请求 ID 保存为 `notary-request-<平台>`。`mac-notarize` 查询这个 ID，通过后为 PKG 和其中的 App 附加票据，验证安装包与 App 签名，再保存 PKG、供 Tauri updater 使用的 App 归档和 `notarization.json`。候选分发要求两个 Mac 包都有相同源码/run 的 Accepted 记录。

用户安装选择 `khor-macos-arm64.pkg`（Apple 芯片）或 `khor-macos-x86_64.pkg`（Intel）。两者都安装到 `/Applications/Khor.app`，并在空闲时为 `/usr/local/bin/khor` 建立指向包内 CLI 的链接。App 更新会一起替换其 CLI。独立 CLI 安装继续用 `khor upgrade`；在仅安装 CLI 的机器上可用 `khor web`，装好 App 后用 `khor gui` 打开。普通用户对 `/Applications` 没有写权限时，用新版 PKG 更新。

已有的独立安装优先保留：安装器不会覆盖 `/usr/local/bin/khor` 上的其他文件，也不会动用户目录。可用 `type -a khor` 查看实际命令来源，决定是否调整 PATH 或按原安装方式卸载；只有确认旧 `khor-gui` 是指向 App 内部程序的外部软链接时，才用 `unlink` 移除它。用户可用 `/Applications/Khor.app/Contents/MacOS/khor --version` 核对随 App 的版本。

公证查询每次最多等待30分钟，任务上限40分钟，为下载、打包和状态保存留出时间；编译仍有独立的90分钟预算。等待超时不取消 Apple 上的处理。在原 Actions run 选择 **Re-run failed jobs**，继续查询同一请求，复用原始签名包，不重新编译、签名或提交。已有最终包也会校验后复用。不要使用 Re-run all jobs 恢复公证。

Mac 签名明确返回 `A timestamp was expected but was not found` 时，构建适配器在同一作业内最多尝试三次，间隔5秒和15秒；复用当前 Cargo 输出和图标目录，保留实时日志。安全时间戳、公证与最终签名验证保持必需。其他编译、证书或签名错误直接失败；超出重试上限也明确失败。

`notary-status-<平台>-<attempt>` 保存最后状态，Apple 拒绝时同时保存诊断日志。`In Progress` 可稍后重试；查询暂时不可用会在等待时限内重试；`Invalid`/`Rejected` 需要修复后创建新候选。提交回执丢失时，按包含平台、run 与完整归档摘要的唯一名称从 Apple history 恢复 ID；历史记录有歧义或暂时查不到时明确停止，绝不盲目重复提交。原始包、提交意图和请求记录保留14天，过期后创建新候选，不混用其它 run 的产物。

TestFlight使用App Store Connect发行导出，不能使用Internal Only。上传前查询同一应用、营销版本和build number：已存在就检查处理状态，避免重复上传。Apple处理超过等待上限时明确失败；重试仍使用原build。处理成功不等同外部测试审核通过或用户已安装；测试组的自动分发设置及外部Beta审核由App Store Connect管理。

iOS build number为`1000+release workflow run_number.run_attempt`，高于历史62；超过Apple组件边界会拒绝。App Store提审核对已测试Apple build ID，Fastlane跳过二进制、截图和元数据上传，`automatic_release=false`。同一build已经在审核或待发布则不重复提交；发现另一个build正在审核则拒绝替换。协议、截图、隐私、加密申报等必要资料未齐时，错误如实保留。

重试入口是`Distribute tagged release`，选择原发布 tag 作为工作流 ref 后手工运行。签名环境仅接受 tag；不能选择 main 再填另一个 tag：

- rc分发失败：填写原`tag`与`run_id`；复用已经通过全部检查的候选。若Actions产物已过期且rc尚未成功永久发布，需要创建新rc，不能拼接其它run。
- 正式分发或提审失败：填写原正式`tag`；多个同源候选时再填写`candidate_tag`。已发布的相同GitHub资产会验证后跳过，Apple提审继续使用原build。
- 构建/Quality失败：查看失败job，修复源码后发新rc；基础设施临时故障可使用Actions的Re-run failed jobs，不通过删除检查追绿。

GitHub正式发布与Apple审核是独立阶段：GitHub可能已经发布，而Apple因资料缺失仍未提交。此时补齐资料并重试分发，不能把GitHub成功当作App Store上线。

Mac继续走Developer ID签名、公证和商店外分发。当前`macos-private-api`配置不等同Mac App Store兼容。Android/Windows安装包未纳入本轮。

## GitHub一次性配置

发布环境保留main用于明确的诊断或重试，并允许相应tag类型：

| 环境 | 允许的tag | Secrets |
| --- | --- | --- |
| apple-signing | `v*-rc.*` | MAC_CERTIFICATE_P12、MAC_CERTIFICATE_PASSWORD、MAC_SIGNING_IDENTITY、MAC_INSTALLER_CERTIFICATE_P12、MAC_INSTALLER_CERTIFICATE_PASSWORD、MAC_INSTALLER_SIGNING_IDENTITY、APPLE_DEVELOPMENT_TEAM、IOS_CERTIFICATE_P12、IOS_CERTIFICATE_PASSWORD、IOS_PROFILE、APPLE_API_KEY_ID、APPLE_API_ISSUER_ID、APPLE_API_KEY_P8 |
| apple-testflight | `v*-rc.*` | 上表除 MAC 六项外的七项 |
| apple-store | `v*`，脚本再严格要求正式版本 | APPLE_API_KEY_ID、APPLE_API_ISSUER_ID、APPLE_API_KEY_P8 |
| github-release | `v*` | 使用工作流GITHUB_TOKEN，不存个人token |
| updater-signing | 仅 `v*` tag，脚本严格核对当前 ref | TAURI_SIGNING_PRIVATE_KEY、TAURI_SIGNING_PRIVATE_KEY_PASSWORD |

另外启用仓库immutable releases，以及禁止更新/删除`refs/tags/v*`的规则。Tag创建仍由有推送权限的人执行，创建正式tag即确认发布和提审意图；不额外加一轮重复人工审批。签名私钥只通过Environment Secrets提供，临时keychain/profile/API文件在runner退出时清理。

## 应用内与 CLI 更新清单

六个桌面/CLI 构建的收据包含程序实际返回的 `_release_info`，绑定源码、候选 tag、原始构建 run、平台和协议契约。跨架构检查通过 Rosetta 或 QEMU 执行，不在 Python 中复制 Rust 的协议常量。iOS 继续由 TestFlight/App Store 更新。

分发中的 `updates` job 在 TestFlight 阶段成功后运行于 GitHub 托管机器及 `updater-signing` 环境。它只签署已经验证的原始归档、`khor-release.json` 与相应签名，生成 Tauri SDK 使用的 `khor-tauri-update.json`；最终由原生 `_verify_release` 对内嵌信任根、六个原包、摘要和签名进行复验。私有租机和本地开发不接收正式私钥。

独立的 `update-publication` 工件保存签名和清单收据，发布后随 `release.json` 永久保存在不可变 Release 中。稳定版复用候选原包与原包签名，只重新签署发布身份；`built_from` 和归档地址仍指向原候选。重试先恢复已有签名；未完成草稿必须恢复原签名工件，不能生成不同签名覆盖已上传资产。静态清单位于各 tag 的 Release 下载路径，不使用一个可覆写的 latest 文件。

## 验证

Python测试覆盖tag分类、错源/错run拒绝、失败Quality阻断、公证超时/回执丢失恢复、拒绝结果、未公证包阻断、过期产物恢复、部分上传续传、同名冲突、重复Apple上传、错误build提审和正式晋升字节一致。公证状态测试使用隔离环境和虚拟凭据，不调用 Apple 服务。安装器用真实临时文件/进程验证rc、版本错配和原子激活；actionlint与ShellCheck检查工作流。

这些检查不代替实际GitHub签名、Apple上传/审核和真机验收。实现或配置发布流程时不创建试验发布tag，不用真实上传验证单元测试。

官方参考：[Tauri GitHub流水线](https://v2.tauri.app/distribute/pipelines/github/)、[Apple测试与正式分发](https://developer.apple.com/documentation/xcode/distributing-your-app-for-beta-testing-and-releases/)、[GitHub不可变Release](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases)。

公证参考：[Apple自定义公证工作流](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow)、[Tauri macOS签名](https://v2.tauri.app/distribute/sign/macos/)。

macOS 更新／原生 WebKit 借网和 iOS 模拟器 UI 分别运行在 `mac-updates` 与 `ios-native-ui` 两个独立托管作业中，均消费当前提交的同一前端工件。模拟器生命周期不再与 Mac 原生网络探针共用宿主；全部断言保留，候选聚合作业继续要求整个 Quality 成功。
