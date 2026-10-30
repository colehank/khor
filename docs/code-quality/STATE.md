# 第二轮当前状态

更新2026-10-30。目标 **完成第二轮 active：29反馈＋5遗留＝34项**。用户已授权实施，未授权本轮main合并/ship。main仍 `/Users/zgh/Desktop/working/khor@e7c335c8dd81de091bd12b92cd704a63f25d03f3`。

逐项标准见 [34项验收总账](ROUND-2-ACCEPTANCE.md)，并遵守 PLAN、ROUND-2-EXECUTION、ENGINEERING。此前完整状态及所有失败/证据细节保存在 [255整合前历史](history/STATE-before-combined-2552334a-20261030.md)，更早记录沿其中链接追溯。本页不表示全轮完成。

## 真实源码、所有权与运行资源

| owner | 当前来源 / 职责 |
| --- | --- |
| Root | `khor-round2` / `integrate/round2@16eec79ead2152b0b1afab08597cd782b5929084`，实际clean。协调集成/当前组合验收/证据总账，不另开第三条产品实现线。 |
| round2_input | `khor-round2-input` / `fix/round2-replay-node@4dd9db6c4caf6f567c7ab7940e7b483d0957ce88`；ACP/Node/CLI定向原生三批准备，等待独审/串行准入。 |
| round2_history | `khor-round2-history` / `fix/round2-replay-shared@67772dff02e131bd041d3f5abbd0a9c489c92793`；负责Root255真实生成、noEmit、Vite及受影响browser回归准备。 |
| round2_reviewer | 非作者只读审，源码整合尾核已通过，接上述执行合同与实际结果。 |

**App无运行中的验收，Native/browser槽随current-runtime及真实句柄更新；当前B批已条件准入。** A5原App1861/driver1744/launcher1794经当前ps核不存在，实际88908 terminal1，不能继续视为live或getApp自动重开。最新组件95235 terminal0；最新生产构建42141 terminal0。内盘恢复集 `recovery/RESUME.md`、`current-runtime.json`、`current-worktree-refs.json` 与逐SHA文档镜像须随实际推进同步。

资源：共享专用 `/Volumes/disk0/Khor/caches/rust/khor-quality-target`，jobs1/incremental0/offline，原双盘5GiB门不降低。最近df外盘约7.48GB、内盘约6.10GB；每次启动前重核。用户发布target、源码和会话数据不清理；所有先前有限缓存清理及原始失败收据保在历史中。

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

## 接下来按依赖执行

1. 当前生成与noEmit/Vite完成；不重复这批、Store或未受影响旧H7。生产 `validation/native-production-16eec79e` 原四phase构建准备已固定真实frontend087f，尚待实际CLI/默认custom-protocol+devtools GUI。
2. input B按16ee实际完成：当前fullmetadata→normal ACP stub独立no-prior V→Node lib/greeting/gui_session三selected一次实际编译；13父7辅助全通过、原300总/5GiB/清理门不变。A未改Rust/ACP源不重测，但旧255全frame proof不能冒当前source；B显式实际新stub，不隐式cargo。CLI C必须新当前dist实际pin后再审/授，agents仅compile，不新造fakepeer或调用模型/API。
3. H受影响恢复13及必要消费者browser分组，不跑无影响旧H7/全smoke。必要Core FIFO/GuiNote codec等按真实变更定向补足；不重复已完成Store矩阵。
4. 构建匹配当前CLI/GUI后，实际桌面W/C/G/Q/B组合和完整S：同身份存活重挂、host死/重启/远端失联、正文/控制状态、新Shell语义；不把单NoLoad重开替代完整恢复。
5. E原passive observer通过Vite transform加载，必须匹配当前私有devUrl GUI变体，不能只替换default GUI SHA冒已接线；沿原270/300及file-only方案最小repin，不另造框架。新鲜用户可用性后才启限时Mac真实IME/路径URL拖选/失败保存取消/最后一键正常Quit与各入口草稿；真手机另验连续候选/循环/回中、组合键行/软键盘、浏览器viewport/fixed底栏/滑删。旧答复不是当前可用性。
6. 34项逐项核实，非作者Clean Code审查及最终验收审计。A是40能力源码矩阵22自动18人工，不冒40安装实测。D原卡死/B原拒绝原因缺栈则保未知，不用HTTP200或窄测试冒修复。

所有新独立问题只进NEXT-ROUND；初始Load互动暴露的既有独立缺口已记下，不追加opening API。Management在Store批仅编译，不冒独立auth peer行为。未完成、间接或缺失证据不能关闭任务；main/ship仍等用户另授。
