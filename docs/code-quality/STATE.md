# 当前治理状态

更新2026-10-27。完整全仓功能/代码治理active，尚未完成。先读本文件、PLAN与对应任务卡，再核actual HEAD/diff/agents/PID/SHA。详细上一检查点保[历史](history/STATE-before-current-767aa-20261027.md)，不可据旧角色表重获写权。

## 用户决定与发布边界

- 用户要求全仓审计、分批功能修复/重构、工程规范与最终App验收，多智能体及压缩恢复已授权；“做完此目标后再结束”。首批或库基座不能代替完整目标。误发AEC研究已撤回。
- 用户明确未上线，不保旧Khor端fallback/迁移矩阵，按当前协议纯净开发；外部ACP声明能力、平台适配、认证/epoch和已有数据保护保留。
- 本轮没有main合并/ship授权。main `/Users/zgh/Desktop/working/khor`=8704a503a27b36ad58f922690de50e766f8bd485；既有release GUI SHA9c3e078f…只证明旧发布，不动用户App/服务/release。
- 权威文档本目录plan/code-quality；各实现树同名文档只是快照。资料在/Volumes/disk0/Khor对应分类，复用已有worktree/统一quality-target。

## 当前集成与已验证范围

集成 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，integrate/code-quality-wave1，HEAD **767aa23d95da6dc5fe09680d780a6c866062ae3e**，最后tracked clean。Node真实网络仍旧Sync/inline Pair路径，25MB/29MB收敛未实现。

| 已集成批次 | 证据与边界 |
| --- | --- |
| P2 GUI首批 | 键盘/最新消息、commit文件跳转已修。真实Tauri iOS run11与后续candidate JS MobileSafari/UIKit26指标通过，ACP/Khor各6次settled gap0及历史锚点，组件36/scroll34/types。最终同源全App/物理iPhone/IME仍未全验。 |
| 已有后端/交互 | autosave/conflict/lock、identity/purge、FIFO/connection owner、凭据裁剪/A2A/输入预算、上传receipt/poll、SSH生命周期已逐批集成。Q043双引擎22+38及真实App candidateJS7listen/7unlisten/childWK title；OS拖放/关闭/全屏未知。Q033真GUI→Rust上传链与Q035真实borrow lease仍open。 |
| Q039 pending/inventory/strict apply | pending丢数据先红后绿；1d2640当前inventory去旧fallback，只typed InventoryNotAdmitted可Vouch重试、逐doc继续/固定首错/epoch partial。strict apply92742250真实fence/base/epoch/candidate/no-new-pending/pen退休/flush，FlushedImport非网络ACK。原187/166等有限组独审，Windows身份仅类型/静态。 |
| Q037 owned MCP | fixture e0c6abec+4493ed3c入可杀回收process，Stop释放claim/后续Prompt可完成；作者106不同普通及Win harness typecheck，root4493 normal frontend/CLI/stub/Node+62相关不同父独审。Windows无runtime，远端可靠取消仍open。 |
| Q039 blob/Doc库 | descriptor d16d83df、init f721a2c3、outbound5da91f03、receive67927fcf、lookup/resume686f2e42都已集成。init真SIGKILL红→绿；root f72128、5da14、679222、686f49各有限组绿且独审，不累加当全仓count。最新lookup完整七源/1076inputs/六binary/17PGID+11kill子/29.402s/双盘46files，Q051独审通过。Node尚未使用这些基座。 |
| Q033 current-only小片（最新） | 作者13a9459f→root767aa23d，8源=5生产+3test；完整tracked tree均87f24fd650a7fd5183e1651bdd5aa96548dcf152。删upload_cancel/legacy_upload/Disconnected，strict当前GUI greeting，保当前IO失败reconnect/nulltoken/Stop discard Parts/同socket继续/权限。旧拒绝greeting目标真红；normalCore+greeting4+coreIO12+真host/ACP3=19不同父绿，独审及双盘/source/binary/cleanup齐。一次磁盘资源停、一次fixture缺mut编译错误原记录保留，非业务红。仅同tree源集成核对，不称最终App已验；Q033整体未关闭。 |

## 真实未修故障与写权

| 当前owner | 实际树/基线 | 任务与权限 |
| --- | --- | --- |
| root | wave1@767aa23d；plan总账；store-owner-recovery fix/quality-current-gui-protocol@13a9459f clean | 集成/公共接口/最终验收；Q033小片已集成，无该片进程。负责独审Gen与R1。catalog host-gui-protocol两行已冻结，后续en/zh写权移交N1新增Pending键。 |
| node_transfer_owner | recovery `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery` fix/quality-node-doc-transfer@5f2d5cf7+N1 WIP | 已审真红fixtures保存dd60fc5d/audit/node-transfer-n0-red-20261027，再仅cherry库lookup686f为5f2d5cf7。N1正常Node完整接线已授权：link/proto/lib、新link_doc_transfer/doc_transfer_state私有模块、定向tests；catalog仅normal Pending对应键。不可再按旧表禁止N1。目标真实25MB/公平SeenPin/no Loaded跨await/固定plan+checked receipt/ACK，删正常旧Sync/Synced/base64；巨大Pair/Vouch恢复N2仍必须后续。暂不Cargo。 |
| remote_cancel_owner | agent-protocol fix/quality-remote-cancel@f0104d73+19源03冻结 | R1 4net+1meta有限owned Worker、独立intent/receipt、绑定身份/Status/Retry。公共link/proto/lib保01原patch且已移交后续写权给Node/root；不能再修改。当前获唯一槽：03实际CLI→libtest no-run→R0先exact，绿后其余18父清单。03尚无业务结果，编译/失败按实际记录。 |
| node_test_review（Gen实现作者） | native-io fix/quality-generated-bindings@686f2e42+3源02冻结 | 已从只读转Gen作者，root独审不能自审。完整stage/inventory/lock/journal发布恢复，保原71根；rootB1数量预算前置已02修。11普通父+1实际子worker未编/未运行，未Cargo/gen。该agent独立完成Q033最终19审与Node首红审，相关已结束卡可提交。 |

上述四树复用已有目录，不新建worktree/target。旧库作者/其他历史角色已交还源，无活槽。send_message不唤醒已completed agent；新行动用followup_task或明确新派发，先核现场。

### Node/Pair真红

- N0源码76495a0b：真实Node+hermetic relay先正常Pair，合法同设备4096×64地址history还原实际hints，Updates差集25,447,954B。4次public sync都旧cap，首及后续Seen123/Pintrue持久推进，固定VV/receive_dir仍false，最终1普通父真业务红。1075inputs/35双盘file/3PGID/cleanup独审齐。
- Pair02只是snapshot>24MiB准备失败，不能计产品红；03仅前移日志测得Snapshot9,742,899B（完整history的KV LZ4压缩，不是GC）。04仅固定12288×64与准确comment，源码e7dbda3c，Snapshot29,224,082B。首次Pair read stream too long且issuer row/票已消费；同caller retry badinvite、第三身份亦exact badinvite，最终retry断言红。VV/content/root是诊断，后续断言未执行不各计红。1普通父，24双盘file/1075stable/同树lib fresh=true+newtest541fdf/PGID89713/cleanup独审齐。
- 未降阈值/改生产编码/造设备。N1旧oversizecase可改真实>64MiB当前body限额以保错误隔离，25MB N0必须绿，epoch/inventory/首错/SeenPin oracle保。单DocTransfer 1MiB encoded/256KiB corechunk、双向合计4chunk/1MiB quantum、quiet一次Versions、按doc公平/明确Pending Apart；Pair/Vouch巨大signed roster与烧票前pure settlement、可恢复claim仍N2，不留旧Khor fallback。

### R1检查点

- 原R0 seed f0104d73（未并wave1直到修绿）：真六Prompt/Stop→四实际held Cancel IPC→释放/reap后正常Read/reconcile8s仍只四Cancel，第五六永不投递，1普通父真红；本地runtime/控制peer是受控fixture，非真实外部ACP宿主终态证明。原日志2347stable/13PID+phase全退/35file双盘独审。
- 01源码独审五项阻断：known task delegate预写漏task、坏child阻兄弟、R0 Store路径错、100ms空CLI成本抖动、坏metadata回执应Unknown。01仅normal Node/CLI/libtest no-run编译成功，R0未执行。
- 02修五项+有界坏pending引用恢复；normal Node check15.101s/2350stable/current Node fresh=false/PGID退已通过。root捕获cfgtest `_scratch`漏rename和可解析错identity issue被覆盖；03仅三源修复/加真Retry/IPC边界。03冻结Q037/remote-r1/candidate03-20261026T193642Z，19全源1,474,758B/2350frame/18真正父清单，root delta审允许执行。未知task/lateSend fence是R2仍open；本地cancelled/Working ACK都不是远端ConfirmedCanceled。
- 集成R1需先f010 seed再固定生产修复，避免缺fixture；Node另一支三公共文件由root整合两个已冻意图。当前R1不可称green/集成/完整取消。

### Q041构建链

- installer31/mirror13/site7与真实Darwin Bun artifact af955540…三smoke已验；完整fresh onset/远程部署/Linux/musl/Rust嵌入尚未全部实跑，Q041d缺同源frontend prerequisite仍open。
- Gen旧producer真实红Q041/bindings-red-01：actual正常build+fresh/reused都exit0，但reused多PlaceRow/QuotaWindowKind/Wheel；1920source稳/122.384s/3PGID退/ownedscratch清/21file双盘。未动wave1原三遗留。02候选Q041/bindings-candidate-02，3源/13file双盘/1922inputs；保单71根、先preflight路径、stage全成后发布、精确backup/journal恢复；root B1数量预算前置已源码闭合，11普通尚未执行。
- root4493历史435输出=155当前gen+3旧gen+277dist，非全fresh。R1 existing-frontend实际432=155+277；2350frame另有一锁定node_modules metadata，不把433说成433 gen/dist。

## 唯一构建槽、空间与证据

- 当前唯一槽R1 owner03 focused pipeline（STEP300/TOTAL720/MIN5GiB），其他三owner只源/审查。先核实际PID/exec handle，不能只凭文件认定live。统一 /Volumes/disk0/Khor/caches/rust/khor-quality-target，jobs1（上限2）/CARGO_INCREMENTAL=0/lockedoffline/短ownedTMP。跨树需当前firstparty实际source/binary来源，后续合法artifact覆盖/清缓存不得回填历史SHA。
- 内外可用≥5GiB=5,368,709,120B；掉盘/连续3IO/越线只停own PGID。风扇坏/SMART不可读，用户允许继续；不做未经要求的硬件操作。Q033 green01 host编译触线min5,353,390,080B而停，parent退出当下group未立即ESRCH；后验六组全ESRCH已独审，后续runner加有限group drain，原记录不改。
- storage-audit-10精确5个已完成自有test executable，凭旧receipt+current SHA/uid/nlink/inode/lsof0且无编译进程，unlink 1,004,644,176B；实际free5,385,641,984→6,390,362,112B，源码/证据/用户App未动。含旧同步/Pair与R1-01 libtest缓存，后续应重建，不能据物理缓存不存在否认旧证据。历史cleanup7/8/9清单不复用清新文件。当前最近外6.31GB/内14.73GB，动态测。
- guarded runner具名STEP/TOTAL/MIN，launch AST核实际值，source/precheck失败立即停。setup/编译/guard/业务分别归因；同源已通过检查不无故重跑。
- 证据 /Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence，应急 /Users/zgh/Library/Caches/khor-code-quality/recovery。活owner未提交源以实际diff/SHA为准，不能按摘要覆盖。

## 完整目标仍缺

Node/N1真实大差集收敛、Pair/Vouch/N2恢复、R1/R2远端取消、Q033完整原生GUI链、Q035实际borrow lease、P1剩余模块深审、P4格式/职责/错误/历史/工具链治理、P5最终同源App全流程。当前only受测源码树集成，不声称最终App通过。物理iPhone/IME/Windowsruntime仍明确未知，不把metadata当跨平台runtime。main/ship不在本轮授权。
