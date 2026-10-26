# 当前治理状态

更新：2026-10-27。完整全仓治理活跃，尚未完成。先读本文件、PLAN及任务卡，再核actual HEAD/diff/agents/PID/源SHA/日志。详细旧证据见[上一检查点](history/STATE-before-receive-checkpoint-20261027.md)，不要从历史角色表恢复过时写权。

## 用户决定与边界

- 用户要求完成全仓功能与代码治理，授权多智能体协作、分批实现及验证，目标完成前持续工作。2026-10-27问进度，root已如实报告剩余Node/Pair大差集、远端取消、P4/P5；继续推进。
- 用户误发研究AEC消息后明确“发错了…你继续”，goal短暂paused后已active；该研究消息与本项目无关。
- 最新协议决定：“不必考虑旧端兼容，我们还没有上线，按着纯净的开发就行”。统一当前Khor，不保旧Khor fallback/迁移矩阵；平台适配、外部ACP能力、认证、epoch与已有数据保护保留。
- 没有本轮main合并/ship授权。main `/Users/zgh/Desktop/working/khor` =8704a503a27b36ad58f922690de50e766f8bd485，2026-10-27现场tracked clean。既有release GUI SHA9c3e078f…只证明旧发布版，不能证明治理版本；用户主App/服务不动。
- 权威文档本目录，plan/code-quality；实现树同名文档只是快照。资料放/Volumes/disk0/Khor对应分类，复用既有worktree，不散落新khor目录。

## 集成与精确验证

集成 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，integrate/code-quality-wave1，HEAD **686f2e42eb634ff5ba0edc34ab822fe8764f13f2**，本次tracked clean。新增DocVersion/receiver已接库，Node真实网络接线仍旧路径，不称25MB已收敛。

| 批次 | 已集成及证据边界 |
| --- | --- |
| 既有P2/GUI | 键盘与commit跳转修复已集成。真正Tauri iOS run11与后续候选JS MobileSafari/UIKit26指标通过（ACP/Khor各6次settled gap0、历史锚点）；组件36/scroll34/types绿。物理iPhone/IME与最终全源App仍待。Q043双引擎22+38、真App候选JS7listen/7unlisten/childWK title；OS drag/close/fullscreen未完整验。 |
| 既有后端 | autosave/conflict/lock、identity/purge、FIFO/connection owner、凭据裁剪/A2A/输入预算、上传receipt/poll、Q045 SSH生命周期、MCP阶段2与完整replay逐批集成；原任务卡保留实际范围。Q033真正GUI→Rust上传取消、Q035 borrow lease未关闭。 |
| Q039 pending/inventory | pending真实丢数据先红后绿；cache/compact/live absorb保未解op。当前inventory1d2640删除旧fallback，仅typed InventoryNotAdmitted可Vouch重试；逐doc错误继续+首错，epoch明确部分同步。166相关Rust/scripts绿及final26/types/catalogproducer证据已审。25MB本体仍旧cap拒，不能当已收敛。 |
| Q039 strict apply | 6e82847f→92742250；真实epoch/base/on_disk/candidate无新pending/保本地并发/真实fence/checked pen退休/flush，私有FlushedImport不是networkApplied或断电receipt。作者187不同普通相关Rust+独审；root4493中重验22Doc。Windows身份比较仅静态/类型边界，无runtime。 |
| Q046 helper | 5fe7869a将仅测试的hop projection移cfg(test)，生产算法不变。normalNode+hop9绿、独审，root4493重验9。 |
| Q037 owned MCP | 首红fixture7550→e0c6abec，生产3e676325→4493ed3c；同步MCP入可杀/回收进程，Stop释放执行权后下一Prompt可完成。作者106不同父+Windows harness typecheck，final16源/2047普通+2048Win输入/3binary/cleanup双盘均独审。root4493正常frontend/CLI/stub/Node与62不同相关父绿。Windows无runtime，远端取消仍未闭环。 |
| Q039 descriptor | 263df47d→d16d83df；canonical doc/epoch/base/target/encoding绑定既有manifest身份/长度/digest，拒非规范/超预算。正常check+5codec和独审，非bug不造red。 |
| Q039 init recovery | 真kill红477b→419f73c5→f721a2c3。singleton .initializing全gate、完整白名单/owner/links预算验证后清理、sync/close后atomic no-replace发布。作者23不同普通+9target metadata绿，原父oracle不变、11真SIGKILL子回收。rootf721 normalSync+28不同普通绿（16unit含11core+5codec，12integration，最后重复5不加计），1068输入稳/17.523s/独审Q048。metadata不是跨平台runtime。 |
| Q039 outbound | 9c353fb8→5da91f03；真实flush固定target/完整Loro mode/checksum/immutable staged body/drop大Vec，网络前可drop Loaded，resume/discard生命周期接口清楚。作者9新+5codec绿；root5da normalSync+14不同普通绿/1070输入稳/10.873s、独审Q049。作者前片step阈值420误代240已保偏差，最长6.075s，不虚构事后guard有效；root180/300真实。 |
| Q039 receiver | 3e46b6e4→67927fcf，7源匹配；DocumentVersion纯metadata canonical编码、共享BodyError校验、真实apply_staged→FlushedImport，错binding/partial/body/epoch/base/pending不伪成功，不自动清stage。作者正常Sync+8new+9out+5codec=22绿，1070输入、3binary、AST240/420/5GiB、4PGID与双盘19files独审；当前root6792 normalSync+22已执行exit0/1074输入稳/14.890s，receipt+cleanup完、独立审通过；仍非P5或Node收敛。 |

## 当前写权与依赖

| 所有者 | 树/分支/源 | 任务及授权 |
| --- | --- | --- |
| root | wave1@686f2e42 clean；plan总账 | 集成、公共协议/接口、独立审、恢复。root6792组合22/1074已绿并独审。lookup8ffe→686f已集成，root49/1076源稳/29.402s/6binary/17PGID收尾已完，Q051限定独审通过。维护Q041/P4/P5。 |
| remote_cancel_owner（新干净上下文接管） | agent-protocol，fix/quality-remote-cancel@f0104d73 +R1 WIP | 原mcp_worker_owner已交R0/契约后退出；root先前send_message未唤醒，实际查树clean后已spawn本owner开始生产。R1 async here/on/with、4net+1meta、独立truth、Status/Retry/flat BoundAction/stableBinding按root授权实现；暂不Cargo。新卡Q-037-remote-r1.md。R0 fixture未并wave1直到修绿，audit ref保留。 |
| doc_outbound_owner | native-io，fix/quality-doc-transfer@8ffe6aa2 clean | lookup/resume candidate02（transfer50e71755）已normalSync+49不同普通绿/1072稳/28.725s；6binary/fingerprints/17PGID+11kill子/AST240/600/5GiB/双盘53file最终独审无阻断。已commit8ffe6aa2→root686f，7源码匹配；源码写权归还root，无PID/slot。卡Q-039-transfer-lookup。 |
| node_transfer_owner | recovery，fix/quality-node-doc-transfer@67927fcf，恰2测试dirty | N0 candidate02 SHA76495a0b与Pair N0b candidate02 SHA0f21a850已独审并实际首跑：normalNode check绿；N0真业务红（25,447,954B差集，4次public sync均旧cap、Seen123/Pintrue保留但VV/receive_dir不收敛）；N0b在首次Pair前snapshot大小准备断言失败，无配对故障结论。source1075稳定、总129.283s、3Cargo PGID已退，35file双盘/两binary/cleanup最终核齐；03仅前移安全snapshot日志，SHAf9de265a，未运行。不得改oracle假绿；先分析Snapshot与Updates差异，再定合法大快照准备。未Node生产；link/proto写权仍R1，禁N1并改。 |
| receive_review | 只读固定库源，独占receive review卡 | receiver源码+22真实证据已审，无阻断；commit exactblob与root组合最终核对可另派。 |
| doc_apply_review | 只读各固定源/独占review卡 | MCP106/delivery、R0真红、Doc/descriptor/init/outbound及root4493/f721/5da审完成。R0非ACP终态证明；后续R1与lookup需再次派审。 |
| 旧长上下文owners | 已交源/卡，无PID | capacity失败不自动反复重试；新分工按本表，所有dirty以现场为准，禁摘要覆盖。 |

## 最新真实红与框架边界

- R0 `Q037/remote-red/run-20261026T163311Z`：frontend87.90s/CLI11.14s/testcompile53.58s绿，exact1普通父11.73s目标business红。六真实Prompt→正式durable child link→Stop全PiESRCH/cancelled/mapfalse/claimfree；前4真实IPC Get/Cancel屏障持四freshCLI，5/6未admit。释放原4后四PID退出，再正常Read/reconcile8s仍只4Cancel。断言前cleanup三项全Ok，outer cleanup=[]/全部ESRCH；2347输入、三源、实际binary前后稳，13+phase PID消失、独占250files3,362,456B限定清理、35files双盘独审。
- R0 fixture不声称真实delegate/ACP远端完成；它用production Store.accept/Execution Working与track_child构造受控peer，真实IPC投递是唯一oracle。R1保该oracle，async迁移/稳定身份增强不能靠拒绝所有请求假绿。
- 本地cancelled与远端确认分开；cancellation_requested/Working不是ConfirmedCanceled。R1计划的metadata lane最多1、network step最多4；只metadata成功可DurableQueued，未知写入结果不假NotPersisted或成功。R2未知task解析/lateSend fence仍open，不重发Send/Answer。
- Node接线必须单一当前小/大DocTransfer、固定持久sender plan、每doc4chunk/1MiB轮转、独立doc继续+确定首错、Pending/Apart显式；Applied须checkedflush+进程重启可恢复receipt后返回，FlushedImport不直接当ACK。Pair/Vouch巨大signed roster走同有界Blob层，保现trust规则、烧票前pure root冲突验证、绑定caller/tokenHash/nonce可恢复claim，不新造crypto/普通陌生人读权限。

## 构建、整体验收与资源

- Q041 installer31/mirror13/site7及真实Darwin Bun artifact af955540…三smoke已过；Linux/musl/Rust嵌入/完整fresh onset/远程部署未实跑。Q041d缺同源GUI prerequisite与Q041e旧gen遗留待修：root4493现存435输出=155当前gen+3旧gen(PlaceRow/QuotaWindowKind/Wheel)+277dist，不能称435全fresh；原字节/mtime保留，未为审查删除。
- root4493 source2230稳定/正常构建+62不同父；第一次SUN_LEN是root嵌套TMPDIR过长、SSH broker bind前失败，改短ownedTMP同source/binary才完整绿。一次runner SyntaxError在执行前，原失败脚本保留；没有隐藏业务失败/削弱断言。Q047/048/049独立卡记录实际计数与共享artifact被后续合法编译覆盖，不拿当前新binary冒充历史。
- 唯一Cargo/native/gen槽必须root明确派发。root6792与作者/root lookup49均已结束并收尾；Node N0/N0b首跑刚结束并归还槽，作者收尾中，R1 candidate01 normalNode/CLI/libtest no-run均成功且归槽，已知setup问题导致原R0未执行。Node03诊断已结束归槽，实际Snapshot9,742,899B（完整history，KV LZ4压缩，与Updates25.45MB不同），仍是体积准备失败；已授权最小04固定12288×64同设备历史后exact，保原>24MiB/<=64MiB及Pair断言。R1作者修candidate02，无Cargo。其他未授权不编译。统一quality-target、CARGO_INCREMENTAL=0、jobs1(上限2)。切树mtime字节不变+actual compile来源检查，禁止新散落target/动主release。
- 内外可用每次≥5GiB=5,368,709,120B；掉盘/连续3I/O错误/越线只停ownPGID。风扇坏/SMART不可读，用户已允许继续。最近min内约12.85GB/外6.59GB，仍动态重测。不动用户App/服务/系统swap。cleanup7/8/9精确自身缓存回执在证据；不复用旧manifest清新文件。
- 验证runner使用独立命名的STEP/TOTAL常数，launch前AST解析并核真实值；precheck/source失败必须check=True立即停止，不能shell继续变更。构建/setup/cleanup失败与业务红分开保原始日志，不为了已正常短时完成的超时常数偏差重复无关测试。
- 证据 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`，应急 `/Users/zgh/Library/Caches/khor-code-quality/recovery/{RESUME.md,current-code-quality-docs}`。备份不代表全部dirty已覆盖；外盘未提交源只能按真实diff/SHA恢复。
- 完整目标仍含Node/Pair真实25MB收敛、远端取消闭环、Q033/Q035、P1剩余深审、P4风格/职责/错误/工具链与历史分类、P5最终同源App全流程。物理iPhone/IME/Windows runtime未知；本轮无main合并/ship授权，不以中间基座代替完成。

## 当前lookup集成收口

作者8ffe6aa2（parent3e46b6e4，恰7源）→root686f2e42，七源testedfreeze=commitblob=live一致。原core/store/Node/锁/旧tests未漂移。root `integrated-686f-final-01`正常Sync+49不同普通全绿，1076输入/49mtime稳定，6实际binary前后匹配，17ownPGID均退，scratch零files后空容器删；named STEP240/TOTAL600/MIN5GiB AST外launch及runner首部核值，存launch脚本SHA。总29.402s，min外6,581,297,152/内12,840,390,656B，完整源/日志/receipt双盘。限定独审Q-051-review已通过；不另把作者49+root49加成98。Node仍旧4MiB路径，已授权首跑取得N0真红，N0b仅setup失败，均不称真实收敛。

## 当前并行收口（2026-10-27 更新）

remote_cancel_owner实际18路径WIP、rustfmt解析及diff检查通过，但尚未编译/运行；计划candidate01冻结后独审。proto仅Bound Send保40s已登记，agent_accounts为既有helper async机械适配；不借此扩大功能。node_test_review已完成N0/N0b候选源码与Q051证据独审，当前只读定界Q041e生成目录完整更新/失败安全方案，不运行生成器、不删除3遗留类型。N0b准备失败保留原日志，后续新源新证据，不计Pair真红。

## R1 candidate01 当前独审与构建边界

冻结18源及1916 tracked输入双盘：Q037/remote-r1/candidate01-20261026T181911Z。normal libcheck 15.104s/2349 runtime输入稳/current Node实际recompile/ownPGID gone，非测试绿。CLI build已成功，libtest no-run进行中，不执行原R0（独审发现fixture task建在controlled-peer-store，ResolveBinding却查真实node Store，须先修setup）。源码独审node_test_review已确认known-task delegate续输入before-send漏task、单坏child使合法兄弟饥饿；root另确认100ms空CLI重启不遵守资源层backoff的成本问题。统一审查卡后做candidate02，01不覆盖、不称已修绿。Q041e最小完整生成目录方案已只读完成，尚未实施。

## 当前独审交付与下一步

Q-037-remote-r1-review已完整交付：known-task delegate预写task、单坏child兄弟饥饿、R0 Store setup、100ms空CLI成本退避、坏metadata Reply分类共五项有阻断。root已授权作者最小修订及必要定向case；bad pending仅允许当前有界marker页按合法hash直定位同hash权威intent，保护原件、显式issue、不卡合法run，不扫历史。candidate01源码/normal编译/CLI与libtest no-run三段仅是编译证据，不是业务绿。Q-039-node-n0-review已独立核首跑35file/1075/两binary/3PGID：N0一普通父真业务红，Pair02仅准备失败。Q041e最小方案定界完成未实施；reviewer现只读Q033 current protocol旧host分支删除与最终原生上传验收计划，不改源码/不占槽。

## 已进入实际接线与current-only实现

Pair04真实产品红/独审通过：Snapshot29,224,082B，首次read stream too long但issuer已入row烧票，同caller重试badinvite、第三身份亦exact badinvite；最终retry断言红（其它VV/content/root为诊断，后续独立assert未执行）。24files/1075源/真实新testbinary541fdf/current同树lib fresh=true/3004205GiB/PGID89713退出与scratch清完整。两个首红fixtures保存dd60fc5d和audit/node-transfer-n0-red-20261027，lookup686f单独cherry为5f2d5cf7，Node owner该clean树已获正常Node完整N1接线权。R1作者已冻结link/proto/lib原01 patch并移交后续公共写权给Node/root；不能再按早期表禁止N1。Node N1无Cargo。

Q041e root已得真实旧producer红：bindings-red-01正常build+fresh/reused都exit0但3旧类型留存，1920输入稳/122.384s/3PGID退/ownedscratch清。native-io已复用fix/quality-generated-bindings@686f，从root移交node_test_review为实现作者（不再自己审该片），按Q-041e-root/plan最小owned stage/inventory/发布恢复实施，无Cargo。root将独立审。

root另复用store-owner-recovery clean树→fix/quality-current-gui-protocol@686f，按用户当前协议指令删GUI upload旧host轴，不动外部ACP/terminal。先加真实TCP拒绝非当前greeting测试，原生产不改；唯一Cargo exec38946正在跑exact首红。资源及运行句柄以现场优先，R1和Node/生成器作者当前均仅源实现。Q033最终App链仍open。
