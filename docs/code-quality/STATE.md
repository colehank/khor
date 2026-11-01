# 第二轮当前状态

2026-11-01。用户已明确授权“先直接commit到main并ship”。**main已合并推送，Mac与三节点已更新；TestFlight 0.1.14（61）已实际上传成功，本次SHIP完成。** Apple处理可用性、手机安装与真人交互尚未查询／验收，不宣布34项整轮完成。原真人Mac IME、iPhone交互均转为发布后未验。

## 立即恢复

- 产品源 `integrate/round2@a228baef4540ac4fe03c40879d4cb50f89da69c4`，正式构建源 `main@eca881f2b30c2893b7e73f81ae1e0f5cefbe8c6c`。本次尾部仅补发布文档，不改产品。主树 `/Users/zgh/Desktop/working/khor`；协调树 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-plan`。
- 入口证据 `/Volumes/disk0/Khor/evidence/releases/20261101-round2`，最终非作者审查 `final-release-review.json` SHA `4a4f8a0705f319192e255cbf5dad6be86b06559f7c9413f7f243086f270f7aa5`。详细交付见[ROUND-2-RELEASE](ROUND-2-RELEASE.md)、[SHIP](../SHIP.md)。
- **所有编译、上传和远端安装执行槽均FREE。** 正式Mac GUI PID91072／出生2026-11-01 15:19:04仍是用户程序；不当作临时Lab清理。Root唯一实际集成发布者，input/reviewer已完成只读交接，无待集成产品提交。
- Mac实际72765已结束：Cargo0，原验证脚本成员搜索错误导致wrapper1，原false保留。22765补验0；78140切换0。iOS67257归档0；54810原upload-1 wrapper2／xcodebuild70保；31607新upload-2 wrapper0／xcodebuild0、真实build61。Linux driver126041／cargo126055已结束：Cargo0，原wrapper记录Assertion false；89593补验与stage0。70279安装driver0／原onset1，49028三节点采用探测0。**不要重启这些旧handle或重复安装。**
- 所有旧CUA/Lab绑定不可观察以免拉起App；本次正式bare executable不在CUA应用目录中，精确路径getApp返回Invalid app。正式切换沿SHIP只向fresh PID/birth/exe/5467复核的旧main79598发送SIGTERM，未向host/provider发信号。
- 当前真实状态与后续接续以内盘 `current-runtime.json`、`current-delivery.json`、`release-checkpoint.json` 和本目录结果文件为准。旧“未授权main／先真人／正在编译／等待可用性”均是历史时点。

## 已实际交付

完整共同UI指纹：`khor-ui-assets:/assets/index-B55HQiJV.js`。

- **Mac**：release Cargo0／1394.158s；CLI SHA `6e162eaae715c9b468a9c2479eac934963998a07aefb3743a7fa1fc181cc29a3`，GUI SHA `7af048369a2fbe2833d870e835550168735b26274e52e4c83665be3cbc3884e6`。两个实际程序均含完整76143842B MacPi（SHA `7672a1c1f29dc59b7a70d160452d9121ebe837ac0321d67e9a0bd7a34de066ef`），strict签名／源码gen/assets首尾冻结通过。新GUI91072持5467，HTTP403＋完整新UI；8个原_host/_ghost/_cagent的PID/birth/role全保，前后8会话ID与原列表全文相同。CLI已原子替换。见 `mac-qualified.json`、`mac-cutover-result.json`、`mac-release-review.json`。
- **三节点**：hinton单jobs1／nice19／offline复用原target，Cargo0／2064.288s。Linux CLI SHA `9117de4af9fc07b0bff2c660c12fe42c27bc6085da67f9cbee3640cea68a6c7a`，完整LinuxPi SHA `89b53264b099746af37cd05feb31fcec0de36ce13034888d998904a6b74eeac4`、ELF x86_64／GLIBC最高2.30、全source/gen/dist manifest通过。一次共享NFS安装后，hinton159488、turing989677、lecun860893实际5467 listener的exe SHA均同，HTTP403＋完整UI一致。hinton/turing各2会话ID不变；lecun前后均`document transfer state busy`，保留未知会话读结果，不冒通过。见 `cluster/adoption-summary.json`、`session-comparison.json`。
- 原Mac/Linux多字节mmap成员检查false均保留，补验改用明确 `.find(bytes,0)>=0`，没有重编或回填旧false。原onset1保留：serve已up，但serve.sh末尾空日志AND-list返回1；实际三节点采用由独立探测证明，不重启掩盖退出码。新独立缺口记[NEXT](NEXT-ROUND.md)。

## TestFlight接续已完成：0.1.14（61）

- 用户回复“通过了”后，按Apple协议已处理继续；同归档新upload-2实际exec31607／Xcode10898，exit0／119.853s，Upload succeeded＋EXPORT SUCCEEDED均存在；本次原位ContentDelivery提取真实build **61**。result SHA `3083addf77e2a6f26678325dfff6a6a86c76d46e48da6e398e97ae5754e4bf91`；version收据SHA `92bf22f9aa1612385cccdb42d0600d4cfd5b14d78ba2481fb3f25b1b44c648e8`。server已接受，不推断TestFlight处理完成或安装通过。
- upload-2当前文档HEAD2a041ddd与built eca仅精确4份发布文档不同，其它实际源码全等；App完整树／.a／gen／dist／导出选项首尾不变、原进程组空、无force。复用原归档，**没有重新编译、重装Mac／集群或手工修改账号／私钥**。所有执行槽再次FREE。
- 下列upload-1的70／协议／签名缺失是已解除的原失败，仍原样保留；旧“唯一外部阻挡／等待处理／执行upload-2”是接续前历史，不再触发重传。现无需任何发布重试。

- iOS正式归档build0／681.998s，后续架构、deep strict签名、team／正式ID和profile各phase0，真实arm64 **IOS**／SDK26.5／min15.0；App与`.a`均含完整新UI。source两签名叶恢复原bytes，gen/assets不变；旧60archive/lib同FS可逆保留。见 `ios/result.json`、`product-verification.json`、`platform-verification.json`。原工具TMP非空诊断保留，不称TMP absent。
- 正式export/upload仅尝试一次，xcodebuild70／12.359s：`PLA Update available` 与 `No signing certificate "iOS Distribution" found`。本地只读有效签名身份development1／distribution0。没有Upload succeeded／EXPORT SUCCEEDED，**该次未交付TestFlight、当时无新上传构建号**。现archive/App/lib与上传前逐SHA不变。
- 由Apple团队Account Holder登录开发者后台，审阅并处理更新协议。Root未接受法律协议、改账号／keychain／私钥或自动重传。Apple角色要求见 https://developer.apple.com/help/account/access/roles 。协议处理后再复查自动发行签名条件，不预判缺证书一定只是协议的副作用。
- 接续时沿现归档单次上传，另建`upload-2`结果；保留原70和原upload-1，不重建Rust来追绿。先读 `ios-upload-handoff.md`：若main只是本次发布文档后继，核Git差异仅`docs/SHIP.md`／`docs/code-quality/**`且其它源码、gen、dist、archive/App/lib完整等原输入；不要机械要求新HEAD==eca，也不能随意放宽产品源码。实际上传成功后才记录Apple分配的新build。

## 原34项与真人验收

功能与Clean Code具体交付见[34项总账](ROUND-2-ACCEPTANCE.md)、[结案审查](ROUND-2-CLOSEOUT-REVIEW.md)、[工程约定](ENGINEERING.md)及原任务卡。产品合并与正式构建不覆盖此前逐项实际证据，也不冒平台真人操作通过。

仍未验：Mac系统中文IME的逐字组词／候选／删除／中英切换／迟到更新／Shift选区／撤销保存；真实iPhone的WK视口与底栏、加载终态与主题、列表左滑、窄工作区、固定区、三入口连续候选、自定义组合键行与软键盘。路线见[HUMAN-ACCEPTANCE](ROUND-2-HUMAN-ACCEPTANCE.md)。此次用户决定移到发布后；不重复已绿矩阵、不预开限时实例。

## 存储与恢复

- 本次正式SHIP复用既有 `/Volumes/disk0/Khor/caches/rust/khor-target`，jobs1／incr0／external TMP；未新增Rust target或清全局Cargo。quality target原样保护；正式缓存约117GiB的既有基线不是这轮新建。当前内盘约13.33GiB、外盘约98.74GiB可用，继续按[STORAGE](STORAGE.md)保留量＋峰值准入；5GiB仅紧急线。
- 仅526个与外盘canonical逐字节相同的旧一轮恢复镜像改可逆symlink，内盘回收约0.45017GiB；唯一WIP、当前a228／恢复源码和用户session未动。索引 `recovery/old-mirror-offload-20261101.json`；原件均保留外盘。第一次0.55GiB选取触1500上限、preflight1且零修改，后缩0.45GiB实际0，原结果不掩。
- Mac旧GUI／CLI回退在本次 `rollback-mac`；相同旧installed CLI只保一份canonical＋symlink引用。旧60在 `ios/previous-60.xcarchive` 与 `previous-60-libapp.a`，更早59仍原处。不复制新大payload到内盘，不为归档TMP诊断新开治理。
- 内盘恢复入口 `/Users/zgh/Library/Caches/khor-code-quality/recovery/{STATE.md,RESUME.md,current-runtime.json,current-delivery.json,release-checkpoint.json,latest-source-bundle.json}`。disk0缺失先读RESUME，不建假挂载目录、不按摘要覆盖源。a228小bundle与原链继续保留；本次main/plan最终ref另存小bundle／Git ref收据。
- 改写前完整STATE保留于本次 `STATE-before-release-closeout.md`（SHA见`state-closeout-lineage.json`），更早详细测试／源／私有Lab位置仍见Git历史及旧checkpoint。本页以当前事实为准，不用历史live状态重新move、stage、kill或发布。
