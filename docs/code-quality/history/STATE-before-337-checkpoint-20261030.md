# 第二轮当前状态

更新2026-10-30。目标 **完成第二轮 active：29反馈＋5遗留＝34项**。用户已授权实施，未授权本轮main合并/ship。main仍 `/Users/zgh/Desktop/working/khor@e7c335c8dd81de091bd12b92cd704a63f25d03f3`。

逐项标准见 [34项验收总账](ROUND-2-ACCEPTANCE.md)，并遵守 PLAN、ROUND-2-EXECUTION、ENGINEERING。此前完整状态及所有失败/证据细节保存在 [255整合前历史](history/STATE-before-combined-2552334a-20261030.md)，更早记录沿其中链接追溯。本页不表示全轮完成。

## 当前收口顺序（最新，覆盖下文分阶段排队语句）

Root **6e78a03c clean**。16ee后仅四个test/preview fixture文件变化：2cc→fbc锁事实；284→ff094 AI4；0e18→5ca895 preview；9a982→ffa079 Read合法tool/broadcast；1ff→657411 ACP当前真实配置控件；901→6e78完整ChatMsgRow与有界失败日志。生产UI/Rust源码、179gen、278assets未改变。来源bundle `integration/consumer-fixtures-6e78a03c` receipt1a048e5c/bundle6f94bbde已verify0及双盘。旧16ee原生proof全frame不等当前Root，不冒当前完整App。

目前无Native/browser/App运行。最后Read96347已terminal1/17.984s，源/env/gen/tools稳、owned空/TMPabsent；完整消息DTO修已整合但尚未重验。H获授AI4 helper只额外传现有view lease给onOpen、AI4以实际chat_open授予事实独立断言answer lease，保旧生命周期/两session/重试/计数，待小片独审+Root整合。不要按旧ffa/fbc或16ee合同启动。

先完成受影响消费者及必要fixture修正（独立完整Read、完整ACP当前配置语义、AI4四组、afix/preview必要闭包）；独立脚本的失败不要使其它脚本永远未执行，所有原失败仍保。随后一次最终noEmit/Vite、C/Core定向原生、最后cold生产CLI+GUI。C会写sharedtarget CLI，故排最后生产之前；App只用最后匹配的privatecopy。A/B/Store/旧H7/Native11/6不因无关fixture变化重跑，按实际相关源不变映射原证据。

review投递若reviewer已completed，需followup_task唤醒；send_message只排消息不会触发idle工作。源writer与运行槽各单owner，当前Root不再开始GUI直到消费者收口。H source新的AI4小片在author树，不能覆盖dirty或未经审查cherry。

## 真实源码、所有权与运行资源

| owner | 当前来源 / 职责 |
| --- | --- |
| Root | `khor-round2` / `integrate/round2@6e78a03c41c7473d42d58db8385c0b9d89dbd233`，实际clean。协调集成/当前组合验收/证据总账，不另开第三条产品实现线。 |
| round2_input | `khor-round2-input` / `fix/round2-replay-node@4dd9db6c4caf6f567c7ab7940e7b483d0957ce88`；ACP/Node/CLI定向原生三批准备，等待独审/串行准入。 |
| round2_history | `khor-round2-history` / `fix/round2-replay-shared@901e5dc308791dfc464ebdbced40ce8774a3a95f`；负责Root255真实生成、noEmit、Vite及受影响browser回归准备。 |
| round2_reviewer | 非作者只读审，源码整合尾核已通过，接上述执行合同与实际结果。 |

**App无运行中的验收，Native/browser槽随current-runtime及真实句柄更新；B批已实际完成，当前空间动作结束后接browser13。** A5原App1861/driver1744/launcher1794经当前ps核不存在，实际88908 terminal1，不能继续视为live或getApp自动重开。最新组件95235 terminal0；最新生产构建42141 terminal0。内盘恢复集 `recovery/RESUME.md`、`current-runtime.json`、`current-worktree-refs.json` 与逐SHA文档镜像须随实际推进同步。

资源：共享专用 `/Volumes/disk0/Khor/caches/rust/khor-quality-target`，jobs1/incremental0/offline，原双盘5GiB门不降低。最近df外盘约7.48GB、内盘约6.10GB；每次启动前重核。用户发布target、源码和会话数据不清理；所有先前有限缓存清理及原始失败收据保在历史中。

B四已结束privatecopy已在完整gzip/耐久双盘索引后精确归档移除，逻辑439,960,128B、df实增439,934,976B分列，原target/source/proof/result不动；index029c24a5/result66019e66独审。plain不可直接执行，恢复按索引及完整解压SHA。当前生产61197已terminal1/190.34s：metadata/CLI实际0+V+copy/GUI metadata0；GUI因原5GiB外盘门被中止，source/env/gen/assets稳定、remaining[]、TMPabsent。result c6217c38；CLI copy b6360a48(355300120B)保留，原整批false不回填。后续只GUI续建，原CLI成功阶段复用，不重gen/types/Vite/CLI。

## 本次已完成的源码整合

Root729已整合Store两提交231e→27d；Root255随后整合Node4dd、共享47fe、恢复夹具ee341、PTY负例7d、剩余消费者969。非作者逐核37个变更路径、15个Node侧文件、20个共享存续文件及2个退役脚本，无冲突/意外改动，Root clean、main未动。

| 来源收据 | 结果 |
| --- | --- |
| `evidence/round2/integration/startup-event-store-729ac2d1/integration-receipt.json` | SHA `8616296a08a79c6e064ffcd1f95af68083b2c5fb93e373f1b0c3c46680a954ae`；bundle `73ce495efec9d690691483741bf2af1a6825cd64df7b600e412a66f06537f199`，verify0。 |
| `evidence/round2/integration/replay-retirement-merged-1/integration-receipt.json` | SHA `11d34f3604f82e28e22b60adcb1f35b657bcc63f76c519029fce0fc29aea6250`；bundle `b1920aeea3f042cae935047ddd5449e7a1f12154926305015b0f3b2a8fb7c1e2`，verify0。 |

按用户无旧端兼容决定，移除真实可达的旧Replay RPC及History/HistoryEnd、32MiB Vec链；统一canonical H/Capture及initial Load/Resume。保事实能力、PTY5、初次Load、Stop/Close、FIFO、两view互动、known-past/迟到owner及原文能力。GUI greeting改为khor-gui/2是能力/帧集合break；rmp-serde实际写variant名字，旧“enum index重排”推断已撤回，不再引用。

Root16ee另精准整合67772dff两叶：删掉paint和ChatView中两处已退役History判断。实际生成82947完成metadata/build/stage/publish后，完整noEmit发现这两个TS2367，原false保；修后只续noEmit/Vite，未重Cargo/gen。两叶source bundle与收据在 `integration/replay-note-only-16eec79e`，receipt138c2b34/bundle3c278b57，双盘且verify0。

**当前实际179生成文件及完整noEmit/Vite已通过**（见下表），定向ACP三行为已通过；Node/CLI和browser组合仍待测。私有CLI/GUI仍是较早84来源，当前生产16ee runner已固定等待准入/执行，不称当前整包已可运行。

## 已有实际证据与边界

| 组件 | 已证明 / 不能外推 |
| --- | --- |
| Store启动 | `history/startup-source-d181/event-store-native-source-3` actual95235 terminal0/118.53s，16父+2辅助。真实1025×64KiB=67,174,400B payload→67,352,665B canonical，有序完整、Ready前提交、有界raw末记录读取、Load1/New0、Close/reap；8MiB/256窗口、8MiB段及失败/释放经定向实际。result `0dc562015ba52afeb7523c1d6af52156de554e5ce3863d8d7ea6c148a1904872`。独审/双盘完整镜像通过。RSS四点只为采样，不冒全进程峰值；无bundle focused peer不冒完整KhorAgent/App；不等于全S恢复。 |
| 当前类型/前端 | `integration/replay-frontend-continuation-1` actual11451 terminal0/15.224s，noEmit0/Vite0，2331源/179gen/278assets前后同、owned空/TMPabsent，26小件双盘独审通过。frontend SHA `087f4263b5b58a20525a01f500187f06ad2458d35d8e66b51df5352fb9113817`；result1bc6f187。明确generation_head255、frontend_head16ee，完整frame仅两已审TS差，不改旧proof冒当前producer。 |
| 当前ACP行为 | 原44970编译与两个真实V producer成功、脚本str.parent早停保false；仅修Path调用，actual63174 terminal0/2.600s，3 exact各1pass/0fail/0ignored，源env稳/无残留/TMPabsent。result7eb9b958/receipt7c825028、44小件双盘及完整exe gzip独审。源仅两UI差映射不强重A3；B当前stub需新实际producer，旧proof不改frame。 |
| 当前Node行为 | B actual60432 terminal0/175.606s，当前normalstub＋Node三selected各实际no-prior V通过，16stage全0，13父各1/0/0＋7辅助强summary。source/env稳定、owned/footprint空/TMPabsent。result `45e9a355d10220ba87f1d0a2b7c0893eed72c15641b3c53e57b7b4183b6c5713`，actualreceipt63896890；113项镜像及4完整gzip独审，原target/copy/proof SHA在刷新前独立核。仅本源Node定向行为，不冒Store/Core/App/全S。 |
| SDK饱和 | 原正常帧用例red25,240,210B/384→green8,347,600B/127，真实关闭/credit归零；已Root整合，未受影响不重跑。 |
| 历史H | Native基础11/出口6、production browser7均实际通过；窗口/取消/原文/搜索组件有证。受控transport/clipboard不证明系统Save/Copy或当前整包。 |
| N状态查询 | 真实当前CLI锁竞争及root/meta八父/二辅助通过，6源整合；旧集群唯一原因仍未知。 |
| L01/L02 | 当前来源原retry exact及真实WK late-close/close-reopen已实际通过并独审；原false仍保，不冒整个S或Mac输入。 |
| L04/L05 | 既有来源反例、真实正常OS Quit→认证持久host重连/旧marker/自有清理已独审；不代替完整E/S。 |
| 生产构建84 | `validation/native-production-84c81dd0-2` actual42141 terminal0/207.56s，CLI/GUI实际custom-protocol/default/devtools、devUrl null。result `ec0662f783e84f70cf513d30ab908f154d71502fa7073a8d029598be3471ba78`。仅较早84源，不冒当前255。 |
| 真实App A5 | `history/search/mac-platform-a-5` actual88908 terminal1/119.04s。实见tauri://localhost href/origin、secure=true、ClipboardItem/write/writeText存在；实际Find命中唯一旧正文marker。actual-save阶段超时，未完成Save/Copy/正常Quit；源/env/gen/assets稳定、cleanup_errors=[]、TMP absent。只是partial，不能当NativeSave产品原因或整体通过。 |

A1–A4 setup/操作超时、A2错误重开默认profile后已正常关闭、A4后置镜像失败均原样保留；不回填原false。**停止自动Inspector重试，不开A6**，返回产品主线，系统Save/Copy并入后续新鲜真人场次。

## 最近真实结果与待修正的消费检查

Read96347修正tool/broadcast后，agent两主题及390全部有实际测量，工具/blockquote不再失败；machine仍无column/paragraph/scroller。源码核完整ChatMsgRow缺to会触发MsgLine.to.length，901只补真实DTO/删旧word-delivery，保所有正文与probe/阈值并加有界失败事实。未采旧DOM不能推唯一runtime根因，未新绿。

ACP1ff仅测试迁移已独审并Root整合：真实header设置/checkedradio/nameID/Model Large-b/fast/agent身份/default负例；usage由实际可见32%/缺席/无伪USD证明，旧hiddenattrs明确退役，不从fixture伪填probe数值；原SDK值/工具探针/墨色阈值/正文/NoLoad/login/附件发送仍保。新完整脚本尚未实际通过。

Core draft（head:null）已窄审：现成3exact FIFO/hostcodec/typedCaptureWake＋terminal_current与chat_upload_cancel仅compile；Cargo自动3normalbin也纳入6selected真实no-prior V，不执行gen/bridge/samples。只libtest实际执行需privatecopy，另5仅actualSHA/proof在target刷新前核，资源估计不作上界；不把FIFO原测试冒写前受控barrier。C仍3hermetic/agents compile-only，不调用真实模型。

S下一实际准备由input复用既有工具，不照搬PTY5：GUI HostFile child_pid=host_pid；先seed-role独立strict owned ledger，再两次App临时者strict[]，NoLoad peer原55s只允许evidence副本单常数改270（outer300/正常Quit/显式Close+reap/finally保），源/binary待pin，0运行。初批只同host正常Quit重挂/第二prompt一次，不冒resume/hostdeath/机器重启/offline。E仍另需真实devUrl observer变体/各入口与真人IME；手机本轮physical Lab和reachable连接仍缺。

- 恢复13实际98901 terminal0，result23b3e579/report6aaefd1e/21小件双盘独审，通过真实ChatView/Capture/H reader/paint+transportfixture，不冒WK/App/系统Save/完整S。
- GUI-only48383实际metadata0/Cargo0/103.54s，但原V拒生成缓存；result7d16bbfa，源/env/gen/assets稳、owned空/TMPabsent。离线一次聚合确认5个第一方script marker都current，另4个生成门和18个actual artifact后续门全过；唯一GUI `khor-gui-9093efa5a0c29633/out` 352文件/8717907B，349旧时间，其中279为实际引用。tauri-codegen2.6.3对已存在hash资产不重写，非仅孤儿文件。最终原生前仅该精确OUT完整SHA/身份/lsof核后可逆retain，再真实生成；不改mtime/helper/不加假prior。原false保，未作cold rename，旧GUI cold292等16ee准备已过期不可执行。
- ACP/read组1：41905初始row timeout，缺lock_state bootstrap事实；补后48444越初始行但旧settings/usage探针失效，4个非default mode实际也fail。原false均保。正在把原有效断言映射到真实SessionPreferences：checkedradio/nameID/model/fast/default负例/agent身份，可见上下文32%/缺席/无伪费用；不从fixture回填隐藏raw attrs、不加productattrs、不改产品failclosed。wizard只补合法hosting/home/LockFacts前置，所有有效协议/正文/NoLoad/login/工具阈值保留。
- 独立Read7229 terminal1/17.50s，agent正文/几何已有真实测量；tool_call缺必填ID被生产规则正确拒、旧machine chat未走broadcast，所以原工具/机器断言fail。两合法DTO修已Root整合，不改任何原正文/墨色/blockquote/geometry probes。重验未通过前不冒绿。
- 手机只读核：旧run11仅首轮模拟器包；正式iOS归档/上传流程与有效development签名/profile确实已有，profile覆盖已配对设备。当前仍缺本轮physical库/独立namespace签名Lab、reachable设备连接与新鲜人手；tunnel/DDI unavailable不推唯一SDK原因。0build/install/boot/userApp，报告mobile/formal-ios-readonly-1/31fafab5。

## 精确空间收口与当前续批

已结束的旧666CLI/ce69CLI+GUI三nlink1 privatecopies完整gzip可恢复归档，indexfa06ffce/resultce9dae41；旧84 CLI4/GUI5共9硬链接经H完整组报告b04a6f6b核清后，按2inode完整归档/逐剩余nlink递减删除，index79b81351/result454c480f。原9条均终态/lsof空；不重开App、不触用户release。

已授权B三target testexe仅因本批已完成、原actual/source SHA/全gzip/现CLI+GUI两raw无引用而精确移除，index40916cd7/result7c1b8e32；stub/currentCLI/其它cache不动。最后95235两private testcopies（logical310601568B）使用原49镜像完整gzip与新耐久index后归档移除；终态以作者收据为准。此后停止空间盘点，不扩其它库。8GB是清理目标不是新验收门，原Native双盘5GiB保持。

GUI-only `validation/native-gui-16eec79e/build.py` runner4082892e/admission6f5739f5已独审准入，复用原61197成功CLI phase/raw/proof/original+copy，三旧输入固定SHA，临时还原TMP仅供原require再finally恢复，前后复核；新GUI仍实际no-prior/默认custom-protocol+devtools/原300phase900total/5GiB。当前未执行。先给H恢复13独占browser实际，终态归槽再Root GUI-only，然后C；不再重整包。

## 接下来按依赖执行

1. 当前生成与noEmit/Vite完成；不重复这批、Store或未受影响旧H7。生产 `validation/native-production-16eec79e` 原四phase构建准备已固定真实frontend087f，尚待实际CLI/默认custom-protocol+devtools GUI。
2. input B按16ee实际完成：当前fullmetadata→normal ACP stub独立no-prior V→Node lib/greeting/gui_session三selected一次实际编译；13父7辅助全通过、原300总/5GiB/清理门不变。A未改Rust/ACP源不重测，但旧255全frame proof不能冒当前source；B显式实际新stub，不隐式cargo。CLI C必须新当前dist实际pin后再审/授，agents仅compile，不新造fakepeer或调用模型/API。
3. H受影响恢复13及必要消费者browser分组，不跑无影响旧H7/全smoke。必要Core FIFO/GuiNote codec等按真实变更定向补足；不重复已完成Store矩阵。
4. 构建匹配当前CLI/GUI后，实际桌面W/C/G/Q/B组合和完整S：同身份存活重挂、host死/重启/远端失联、正文/控制状态、新Shell语义；不把单NoLoad重开替代完整恢复。
5. E原passive observer通过Vite transform加载，必须匹配当前私有devUrl GUI变体，不能只替换default GUI SHA冒已接线；沿原270/300及file-only方案最小repin，不另造框架。新鲜用户可用性后才启限时Mac真实IME/路径URL拖选/失败保存取消/最后一键正常Quit与各入口草稿；真手机另验连续候选/循环/回中、组合键行/软键盘、浏览器viewport/fixed底栏/滑删。旧答复不是当前可用性。
6. 34项逐项核实，非作者Clean Code审查及最终验收审计。A是40能力源码矩阵22自动18人工，不冒40安装实测。D原卡死/B原拒绝原因缺栈则保未知，不用HTTP200或窄测试冒修复。

所有新独立问题只进NEXT-ROUND；初始Load互动暴露的既有独立缺口已记下，不追加opening API。Management在Store批仅编译，不冒独立auth peer行为。未完成、间接或缺失证据不能关闭任务；main/ship仍等用户另授。
