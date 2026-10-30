# 第二轮当前状态

更新2026-10-30。**目标完成第二轮 active，29反馈＋5遗留＝34项，不缩范围。** 用户已授权实施，未授权本轮main合并/ship。main仍 `/Users/zgh/Desktop/working/khor@e7c335c8dd81de091bd12b92cd704a63f25d03f3`。逐项结果以 [34项验收总账](ROUND-2-ACCEPTANCE.md)、各任务卡与对应实际证据为准；本文件不是全轮完成声明。

## 最新整合状态（2026-10-30，2552334a）

以下状态覆盖下方保留的分阶段历史：Root `integrate/round2@2552334ab1d55e227f4dacb90127f2e1b930f60f` 实际 clean。Store两提交及Node/ACP/Core/CLI/GUI旧Replay退役与相关夹具已全部整合，37个变更路径、20个共享存续文件/15个Node侧文件和2个退役脚本已由非作者逐项核对。来源收据与可验证bundle双盘保留。main未动。

**当前无运行中的测试App；Native/browser槽空闲。** A5实际88908已terminal1，原App/driver/launcher均退出。A5实见生产页面 `tauri://localhost`、secure=true、ClipboardItem/write/writeText存在，并实际Find到唯一旧正文marker；保存阶段超时，没有完成系统Save/Copy或正常Quit，不作整体通过。停止自动Inspector重试，不开A6，返回产品主线。

当前组合源**尚未真实重新生成bindings/完成noEmit、Vite、定向Native及browser回归**。Root ignored生成物179份与私有CLI/GUI仍是84c81dd0来源，不能称当前255可运行整包。H负责当前源生成/noEmit及Vite准备；input负责ACP/Node/CLI定向Native准备；共享Native与App运行串行。既有64MiB+ Store实际组件及未受影响的已绿用例不重跑。第三方agents真实模型case保持compile-only，不用不等价fakepeer冒充行为通过。

完整S恢复、桌面组合、真人Mac IME/保存退出及真手机手势/键行/视口仍待实际验收。新独立问题只记录下一轮；本轮34项目标不缩减。主文档后续应将下方过期分阶段段落归档。

## 恢复入口与当前资源

先读最新用户指令、本页、PLAN/ROUND-2-EXECUTION及自己的任务卡，再核refs/diff/真实运行句柄；不凭摘要覆盖作者dirty。内盘 `/Users/zgh/Library/Caches/khor-code-quality/recovery` 的 RESUME/current-runtime/current-worktree-refs和逐SHA文档镜像用于掉盘恢复。

**Native/browser FREE；App当前H实际A5运行中（handle88908，PID以runtime现场为准）。** 最新组件实际 **95235 terminal0**（Store16父+2helper），最新生产CLI/GUI构建 **42141 terminal0**。真实App平台A2已运行但origin阶段超时，只有CLI导入/只读尾部显示partial，未Find/Save/Copy/正常退出，不作通过。原隔离App与后续误重开的默认profile窗口均已结束；目前无测试App/模型/clipboard操作。旧18136失败终态，旧PID/历史运行语句不能当live。此前完整记录在 [生产构建前历史](history/STATE-before-production84-20261030.md)，更早SDK/整合细节在其链接历史，不改写原false。

| owner | 当前工作副本 / 来源 / 职责 |
| --- | --- |
| Root | `/Volumes/disk0/Khor/worktrees/current/khor-round2` / `integrate/round2` / **84c81dd0ff6aa04316531c4ef3e5b74cdb5b4315 clean**。协调集成/当前私有App/证据与总账，不另开第三条大产品实现。 |
| round2_input | `khor-round2-input`：Store两提交 **231e1f69→27d6434494db35ad0baf2cf22b717422433a2a6e** 已实际95235/独审通过，尚未Root整合。已授现worktree短branch `fix/round2-replay-node` 继续Node/ACP/必要CLI测试退役，不新目录、不Cargo；保原27d分支及全部实际收据。单writer，当前HEAD/diff以真实refs为准。 |
| round2_history | `khor-round2-history` / `fix/round2-replay-shared` / **ee341cbeef05f36031bbe65b59687f4b64dd923a**：共享七叶47fefbcf＋13场景迁移源码均已独审checkpoint，尚未编译/gen/browser，不与Node旧enum单独整编。原410c/8f及Root84源保留，Root未提前整合。其余旧smoke/ACP/read/preview消费迁移继续同范围。 |
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

原900总/300phase/jobs1/incremental0/offline/5GiB/provenance门保持。外盘最低5863014400B，完成后约6205861888B；内盘约6303002624B。**构建已通过；真实App2只读打开有partial，完整系统保存/交互验收未过。** 本批不重旧Native11/6/catalog或无影响L01/L02。

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

1. **S启动组件已完成并独审，待Root整合**：27d的单owner EventStore/真实8MiB+256窗口/8MiB段spool、两FD/单IO与read结果、互动credit、finite prefix与退出/磁盘失败通过实际95235（见下节）；仍需完整同源S App恢复与被退役旧Replay的语义闭包，不冒整S完成。Management本批只编译，无独立auth peer行为证明；test-onlystats getter警告后续必要清理片cfg(test)，不为此立即重跑已绿矩阵。
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


## Store实际收口与后续交接（最新）

`history/startup-source-d181/event-store-native-source-3` actual **95235 terminal0/118.530588s**，fixed **27d64344** / runner **2e78be19…93c4** / contract **12b8df98…7690**。完整host metadata＋ACP/Node各actual no-prior producer、原Cargo唯一selected row/严格来源/私有copy，ACP finite-prefix6＋Store6＋原SDKpressure1，Node scheduler/failed-bootstrap/longLoad **3parent＋2owned helper**，合**16父+2辅助，不是18独立场景**。每阶段0/强count、无timeout/forced/remaining/footprint、TMP空且absent、source/env稳定；2323源与prepared/proof/前后逐SHA，49小件＋两完整exe gzip双盘解压SHA已非作者核。

真实ByteStreams：**1025×64KiB＝67,174,400B payload→67,352,665B canonical**，有序/无重复/Ready前完整提交、真实有界raw末记录读取、Load1/New0与Close/child reap全部到达。RSS四checkpoint **13136→27072→27120→38000KiB** 只采样，不冒kernel峰值或8MiB全进程硬限；无bundle为当前明确支持的focused Python peer分支，不证明KhorAgent embed能力。source/raw-inline阈值未改，原32768总count/32MiB总量不作为Store上限。

result **0dc562015ba52afeb7523c1d6af52156de554e5ce3863d8d7ea6c148a1904872**；actual receipt **97b82bea3d3e3a500798c990c9b20549246860330b91176007a00fef1f016388**；handoff `event-store-handoff-27d64344/component-handoff.json` **d6792b1e642396042e48622ce725cd0545470251cf67c9efa7bb4c76a2f42294**（只取231e→27d两个提交，SDK七链已Root整合）。内盘 `startup-event-store-27d64344-actual1/mirror-manifest.json` **8d8ef24a…00f7**。Node/ACP退役在作者现有worktree新短branch继续，保d92表所有当前用户能力/原时限；不动vendor SDK/不重复load/不加第二spool/不保旧兼容，真实gen与组合只在跨树源一致后排。

为本批预留空间，独审 `validation/obsolete-666-libraries-prune-1` actual **45563 terminal0/6.731s**：20个原666 raw中的firstparty rlib/rmeta，当前84两raw和两proof递归路径均不引用；UID501/regular/nlink1、真实ar/rust、.d/fingerprint、双lsof/SHA/耐久删除前索引均核。逻辑2236342827B，实际free6329876480→8566181888B；只可再生旧缓存、不byte备份，旧666 privatecopy/当前84/proof/source/用户target保。actual summary **a00c56a7**，不回填旧proof。

## 真实App A1/A2的失败与下一步

- `mac-platform-preparation` actual **89074 terminal1**，在entry绝对import前未注册native_provenance而setup早停，0 CLI/0 App；原source/contract/root-ready记录全保。只在fresh `mac-platform-a-2` 修outer/inner模块注册名→native_provenance，真实只读导入/function identity预检及独审通过，不改原helper/预算。
- A2 actual **70439 terminal1**，外240/内200/原5GiB仍保。真实4CLI都0、导入alias由CLI实际解析，App **51240** 真实CUA显示尾1104..1199/Read original record1/Read-only/Search，AX URL **tauri://localhost**。但Root未能在origin60s内取得并记录DOM能力，inner **60.771855s** timeout origin-and-history；未Find/Save/Copy/正常Quit，outer清理原App、remaining[]/TMPabsent/源env等稳。owned result **2256c2fe…ff2fb**，inner **a8c82d01…682ba**。保原false，不能当产品导出错误。
- 原App超时后Root错误用getApp(bundleid)触发系统自动重开默认profile窗口，已实际CUA CmdQ关闭；PS原PID及私有App路径均空。独立 `root-postrun-observation.json` **d707965f…03b7**记录，不折进原guard或改成正常Quit绿。后续每次绑定前核真正PID/birth存活，不能在过期后让getApp自动重开。
- 已源/非作者核的现有Inspector候选：**Search输入框内右键→系统Inspect Element**；普通行/背景被useNoWebMenu拦截。CmdAltI依赖Tauri内部toggle权限，现ACL未grant，尚无运行时拒绝错误，不称唯一原因。菜单本无Inspector项。下一fixture可hardlink同receipt匹配CLI到Contents/MacOS/khor以还原普通安装，但短self子进程无argv不能归因缺CLI。
- 诊断 `mac-platform-a-2-readonly-diagnosis/REPORT.md` **e456400e…86a2** 已双盘审。Root接下来先固定已有可执行UI路径/最小安装envelope，再独审新fresh合同运行，不延长原时限或重Producer。原主文档合法URL可无尾斜线，不能把test prefix假设当真实origin事实；仍须现场观察，不猜Clipboard flags。Mac真人/手机仍待后续新鲜可用性。


### App A3/A4后续与A5（最新）

A3 actual73392原origin阶段超时/outerfalse，第一次**实见Search输入系统菜单Inspect Element→Inspector＋Console prompt**，但未取得DOM JSON或执行Find/Save；无超时重开。A4由H接手actual42116，CLI4=0/App94067真开，同Inspector路径可到；origin61.219s超时、无来源JSON/旧hit/Save/Copy/正常Quit，原pid/driver/launcher已不存在、remaining[]/TMPabsent。Console AX只是拼接文本且没有独立syntaxerror，不能将其冒唯一输入机制或产品原因。原终态outer42ec6675/inneraf1d8729为false；H提前预建finalmirror导致后置assert错误原样保，另29小件在独立terminal镜像逐SHA核实，terminal-manifest15ddddf1。

Root只再授**最后一次自动Inspector A5**：fresh目录mac-platform-a-5、四Python源逐字节同A4/原所有门，操作改用文档支持的AX setValue和实际截图确认，禁止paste/旧clip读取/虚构JSON；准备镜像与终态mirror明确分离。最终contract **bd245a1ab2410fbdd3715e0be548e33f5487efd9239d6b126449df1d2322622b**。H已实际启动**88908 / driver1744 / launcher1794 / 初始App1861（18:52:35）**，唯一App/CUA，无Native/browser并行，Root不操作CUA/不改Root84；当前结果未定，按同handle观察，不自动A6。结束如仍无法取证，返回产品主线/后续真实人类场次，不扩工具治理。

Store27d已收口后，input获授fix/round2-replay-node source单writer：Node/ACP/必要CLI integration测试和既有私有fixtures按d92语义退役；GUI_GREETING升khor-gui/2仅因能力/帧集合break，实际rmp-serde写variant名字，已撤回“index重排”猜测。PTY HOST_VERSION5不动，旧GUI1及当前GUI2都作为PTY负例。H共享来源已到**7d123593**（47fe+ee341+单GUI2拒绝测试），源checkpoint/13 browser场景迁移已审，不等于compile/gen/browser绿；NodeCancel/Python/agents及其余smoke/ACP/read/preview消费仍在迁移。

初始Load内Elicit暴露是既有独立能力缺口，已记NEXT-ROUND，不新增opening API。Management在Store批仅编译，本轮不冒独立auth实际结果。所有有效Prompt/Control互动、历史完整/StopClose/FIFO/两view/known-past与raw能力仍在退役迁移表中，不能删整case追绿。
