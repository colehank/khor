# 第二轮当前状态

更新2026-10-30。**目标完成第二轮 active，29反馈＋5遗留＝34项，不缩范围。** 用户已授权实施，未授权本轮main合并/ship。main仍 `/Users/zgh/Desktop/working/khor@e7c335c8dd81de091bd12b92cd704a63f25d03f3`。逐项结果以 [34项验收总账](ROUND-2-ACCEPTANCE.md)、各任务卡与对应实际证据为准；本文件不是全轮完成声明。

## 恢复入口与当前资源

先读最新用户指令、本页、PLAN/ROUND-2-EXECUTION及自己的任务卡，再核refs/diff/真实运行句柄；不凭摘要覆盖作者dirty。内盘 `/Users/zgh/Library/Caches/khor-code-quality/recovery` 的 RESUME/current-runtime/current-worktree-refs和逐SHA文档镜像用于掉盘恢复。

**Native/browser/App当前全部FREE。** 最新生产构建实际 **42141 terminal0**，H下一平台A只在真实合同填齐/独审/Root授槽后运行；目前无App/CLI种子/store/模型/clipboard操作。旧18136失败终态，旧PID/历史运行语句不能当live。此前完整记录在 [生产构建前历史](history/STATE-before-production84-20261030.md)，更早SDK/整合细节在其链接历史，不改写原false。

| owner | 当前工作副本 / 来源 / 职责 |
| --- | --- |
| Root | `/Volumes/disk0/Khor/worktrees/current/khor-round2` / `integrate/round2` / **84c81dd0ff6aa04316531c4ef3e5b74cdb5b4315 clean**。协调集成/当前私有App/证据与总账，不另开第三条大产品实现。 |
| round2_input | `khor-round2-input` / `fix/round2-startup-bounds` / 最后fixed **e3268412f672799071915bda8d6c5656f73b5fea**；EventStore/ACP lib/lifecycle/event_fence及必要Node gui_host/journal startup交接dirty仍未编译验收，保真实diff/作者恢复副本。Node/ACP这些叶单writer。 |
| round2_history | `khor-round2-history` / `fix/round2-browser-shared` / **410c1f1b3dd151a0b272fbbce716d08c66bcb88c**，生产父8f65423e；产品源未再变。当前填H平台A实际合同，随后S真实App恢复；Replay退役当前仅清单/交接，不与input双写。 |
| round2_reviewer | 非作者只读，已核当前42141真实四phase/原始Cargo/私有copy/双盘小件；接平台A合同/跨层退役语义审。偶有容量报错不改模型，不冒未审。 |

## 当前可运行的正确生产变体

目录 `/Volumes/disk0/Khor/evidence/round2/validation/native-production-84c81dd0-2`，**42141 terminal0 / 207.560619s**。metadata/CLI/metadata-gui/GUI四phase全0，实际selected GUI **custom-protocol/default/devtools**、devUrl null；是私有默认嵌入页面的验证构建，未发布。18136的真实成功Vite已校验原phase/log/source-after/assets逐字节全等后复用，18136整体false仍保。

| 项 | 实际来源 |
| --- | --- |
| result | **ec0662f783e84f70cf513d30ab908f154d71502fa7073a8d029598be3471ba78** |
| private CLI | `bin/khor` / **5f52ce3c79ad4da9f3e8e488d1710e5a0a3a3f3892003929331af3dca3aade94** / 355053048B |
| private GUI | `bin/khor-gui` / **533dba425b38e2df5ce6a7d21109c304f4aa3ed331ee626c3178977f3d8cf0a4** / 388737672B |
| provenance | CLI **a97aa10d…822c** / GUI **4dc04074…0bf8**；各selected在原CargoJSON、no-prior/实际fresh=false，原helper不变 |
| frozen inputs | 当前2326源字典＝prepared/receipt/beforeafter；**179gen、278assets**实际SHA全同；source/env/helpers/tools稳定 |
| cleanup | 无timeout/forced/remaining/footprint；TMP absent；30个小件双盘manifest全SHA已独审 |

原900总/300phase/jobs1/incremental0/offline/5GiB/provenance门保持。外盘最低5863014400B，完成后约6205861888B；内盘约6303002624B。**这里只证明构建，尚未证明真实App交互。** 本批不重旧Native11/6/catalog或无影响L01/L02。

H平台A准备位置 `history/search/mac-platform-preparation`：outer **f267abd7** / launcher **5ffae61d** / template **6ffaba2c**（后续填合同应重新固定字节）。修复了env-i后错误直接校验build env的早停：精确producer env临时构造→原require/copySHA→finally还原，实际runtime仍私有白名单。待真实当前receipt/context/gen/assets填齐再独审；不拿旧devURL或失败产物代替。真实CLI ws/hook/import只种1201条403053B合成Claude记录，无模型/手造binding；全App正文/worker/native IO→系统Save完整184544B原文→CUA正常Quit。原240外/200内/Save35门保。生产主文档实际href/origin/isSecureContext必须现场观察，不硬填；khor-ui-assets只是UI_REVISION标识。Copy/Paste并入后续新鲜真人Mac IME场次，不读取旧剪贴板、不用JXA/AppKit。

## 已有实证与边界

| 包 | 已证明 | 仍未完成 |
| --- | --- | --- |
| H长历史 | 基础Native11、原文出口Native6、真正177bindings+catalog；Root raw七叶/notice两叶/零内容高度取消修正已整合。production browser7 actual37853 **7/7**、result **fe3a6d87…656d5**，document/zero-height各exact cancel→IO release/finally Drop→leases0/no late reads；25小件双盘核。B1–6原false保。 | 正确生产WK/系统Save/真人Copy、完整恢复组合；受控browser transport/clipboard不外推系统。 |
| SDK启动入口 | 锁定SDK3.5.0原archive9bbe69c8…bd75a，114 pristine＋补丁合120最终文件与作者e326逐SHA全等，Root82b45cf3已整合。真实90088 red **25240210B/384** →同原用例79152 green **8347600B/127**（8MiB/256内），真实child关闭/credit0/Load1/New0/无Ready/strict净；正常帧组件独审完。 | 不涵盖EventStore、ByteStreams长base/巨帧/tiny count独立、全进程RSS或整个App；pending完整行65730B另计。 |
| N状态查询 | 十提交/6源整合，当前CLI真实IO6＋root/meta两positive＝8父/2helper，原ID/锁序/等待预算保。 | 不冒原集群唯一cause或所有网络/GUI已验。 |
| L01 | 当前666 Node actual10596，原ignored exact1/0/0、8.45s；fresh不饿死/旧retry不早于due，13CLI reaped；result8744edb8…c6。 | 原040唯一cause未知；不是SDK/EventStore证明。 |
| L02 | 当前666真实WK原late-close/close-reopen两case actual83173通过，CUA exactApp正常CmdQ0/attester终态前ack/最终handles0、严格清理，receipt4e8631b7…341。 | 首次app-l02-current收尾timeout原false保；不冒原4Q035、E真人或完整S。 |
| L04/L05 | 既有源/环境/ACL/生成规则及实际正常Quit→认证持久host重连/旧marker/自有清理均已独审；当前42141再提供真实最新构建。 | 不替代真人输入退出或完整S恢复。 |

SDK七链/精确raw/V红绿与source bundles、此前Native6/11、原L01/L02和所有false细节看历史与任务卡。内盘 `round2-notice-sdk-82b45cf3.bundle`（依赖666）及 `round2-history-ui-final-84c81dd0.bundle`（依赖82）已verify；不按摘要重建未提交源。

## 必须继续的实际工作

1. **S启动**：input的单owner EventStore/真实8MiB+256窗口/8MiB段盘spool、两FD/受控单IO作业与read结果、interaction credit到回答或drop、finite accepted prefix、canonical完整前不Ready、关闭唤醒/join及磁盘失败均待实现收口/真实验证。NodeByteStreams≥64MiB合法base/4096原fence/巨帧与取消合同不能用SDK绿替代。
2. **S恢复**：完整同身份存活重挂、host死/重启/远端失联、GUI正文/历史/控制状态与新Shell语义仍待真实App；H固定 `session-recovery/full-app-readonly-preparation` plan **efc35f85** 只是准备，不把单个NoLoad重开冒全覆盖。
3. **E**：真正Mac中文IME/路径URL拖选/失败保存取消/最后一键正常OS Quit；源有定向门，真实人工仍未完。只有新的可用性答复后才启动限时场次；ce69旧人工备用还保。
4. **W/C/G/Q/B**：当前App文件/工作区最终布局、独立commit预览固定/长列表/实时外部变化与草稿、普通/Cmd打开/窄屏软折行/回cwd、连续玻璃/深浅三材质、括号公式、真实WK进度/背景/错误原地址。分层Core/CLI/DOM已有证据不等于当前组合已验；HTTP200不冒WK诊断。
5. **M/B手机**：真手机候选跟手/循环/回中、键行组合与软键盘、同页viewport/fixed底栏及浏览器滑删。真实TerminalPane→PTY→隔离Codex完整press+keyup已与常规自动化键盘事件路径同；不冒实体键盘/该vendor自带ShiftLeft选区。
6. **A/D**：ACP能力40源码矩阵22自动18手动，不冒40installed vendor实测。原旧卡死/原WK拒绝现场仍缺栈/错误链，限定调查保持未知，不凭H或HTTP绿宣称唯一cause已修；按34项原标准和真实当前复现收口。
7. 最后逐项做非作者Clean Code审查和完整验收审计；名称/唯一owner/IO边界/取消/迟到结果/失效分支/必要注释都算，格式/零注释/测试数量不算完成。新独立问题只进NEXT-ROUND；main/ship等用户另授。

## 旧Replay路径的明确收口决定

H只读审计 `history/session-recovery/replay-retirement-analysis/RETIREMENT.md`（当前 **d92b6479dffc2a167c7b77e237cf1f892c006f7082b595cbb37c21e9c76cb5ea**）与22源source-inventory **9c588a771c86c4ce88edda0f6ab11f8e82c2fdebc747d76dea14e71e6104c82e**。普通canonical readonly、存活重挂、初次Load不走Replay；但旧HTTP/Tauri RPC仍真实可达32MiB Vec。当前MainUI/CLI/MCP无有效用户caller，Root原则决定按用户“无旧端兼容”**退役整条失效Replay API，统一H/Capture/initial Load，不加第二套spool**。

Node/ACP仍input单writer；H只准备Core/Tauri/API/gen/UI片，须先固定跨层清单/契约和实际producer后实施。不保兼容stub。Facts.replays真实能力事实/显式LoadResume保；旧API自身codec/error-bracket可退，当前历史完整、Stop/Close/取消/互动、多view/FIFO/known-past/原文必须在真实当前入口保有证据。reviewer补出cagent/codexagent/agents等真实测试caller，不能直接删整case或只删调用追绿；不得将“没有UI caller”当“不需处理”。目前只清单/交接，未改产品。

## 空间治理结果与恢复约束

失败18136（result **1eb68939…aac6**）原样保；5GiB门从未降低。所有动作仅在专用quality-target和已结束私有验收副本，用户发布target/源码/会话/证据不动。文件逻辑大小与实际df余量分列，未证APFS clone原因；diskutil实际无snapshots。

| 已结束实际动作 | 结果 |
| --- | --- |
| old-cli-99-archive-1 /31879 | 旧私有99CLI354461976B→gzip107863775B；保旧666/ce69。 |
| old-quality-artifacts-archive-1 /83712 | 215旧rlib/Node缓存逻辑5144998560B→gzip1265301357B；实际free+571469824B。 |
| old-quality-executables-prune-1 /13141 | 283旧Cargo MachO逻辑12706273336B；实际free+1023594496B，可再生非byte备份。 |
| completed-quality-tests-archive-1 /73590 | 11已结束test cache1062039336B完整gzip308544378B；actualfree6787735552→7849693184。 |
| old-orphan-rust-objects-prune-1 /59289 | 322旧>1MiB无exe/rlib/rmeta/a/dylib伴产物的Rust对象1377881928B；actualfree7848951808→9226272768，可再生非byte备份。 |

以上后续三组summary已独审。每份固定清单/代码SHA/UID501/regular/nlink1/属性/lsof/耐久双盘删除前记录与实际终态都在 `evidence/round2/validation/<name>`；完整归档在内盘recovery/round2的对应archived-*索引，解压全SHA已核。原plain已不存在，旧原路径prior不可再用；恢复也不得覆盖已有路径，须按原来源规则重新验证。旧proof不回填。10GiB只是清理提前停目标，原构建门仍5GiB。旧测试private9份和0478/ca83旧Lab4份的更早归档看历史索引；不反复为独立已绿用例构建。
