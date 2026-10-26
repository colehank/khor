# 当前代码治理状态

更新：2026-10-27。完整目标仍 active，未完成。先读本文件、PLAN与任务卡，再核现场 HEAD/diff/agents/PID/SHA；不能按摘要覆盖未提交代码。前一检查点完整保留在 [历史](history/STATE-before-670347d7-20261027.md)。

## 用户决定与交付边界

- 目标是全仓功能审计、按风险修复/重构、Clean Code工程治理和最终App验收，含多智能体协作及压缩恢复；不能以首批或库基座代替完整目标。误发AEC消息已撤回。
- 用户已明确未上线，不保旧Khor端fallback/迁移矩阵，统一当前协议；外部ACP声明能力、平台支持、认证/epoch和数据保护保留。
- 本轮没有main合并/ship授权。main `/Users/zgh/Desktop/working/khor` 仍 **8704a503a27b36ad58f922690de50e766f8bd485**，最后现场clean；不动既有用户App/服务/release。
- 权威协调目录就是本目录的plan/code-quality分支；实现树中的同名文档只是历史副本。外盘资料统一放 `/Volumes/disk0/Khor`，不再新建散落目录/worktree/target。

## 当前集成

`/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，`integrate/code-quality-wave1`，**670347d766191b2b30fbc0e91f31fbf4e87cb3c2**，最后tracked clean。Node正常网络仍旧Sync/inline Pair路径，N1/R1尚未集成。

| 已集成范围 | 当前证据与边界 |
| --- | --- |
| 首批GUI | 键盘/最新消息和commit文件跳转已修；真实Tauri iOS run11及后续MobileSafari/UIKit26指标、ACP/Khor尾部与历史锚点回归有证据。最终整合App、物理iPhone/IME尚未全验。 |
| 编辑/身份/资源/agent | autosave/conflict/lock、identity/purge、FIFO/connection owner、凭据裁剪/A2A/输入预算、上传receipt/poll、SSH生命周期已逐批集成并独审；确切范围看AUDIT与任务卡，局部绿不等于全用户流程。Q033完整GUI上传链、Q035真borrow lease、Q043 OS拖放/关闭/全屏仍open。 |
| Q039 Doc/blob库 | strict apply92742250、descriptor d16d83df、init f721a2c3、outbound5da91f03、receive67927fcf、lookup/resume686f2e42已集成并有限组独审。init真SIGKILL红绿，lookup root49/1076inputs/6binary/17组及kill子核齐。库FlushedImport不是网络ACK；Node尚未使用这些基座。 |
| Q037 owned MCP阶段 | e0c6abec/4493ed3c已集成，可杀回收process并释放claim；旧106相关作者组/root62有记录。远端可靠取消R1/R2另做，Windows仅类型/静态不称runtime。 |
| Q033当前GUI协议 | 作者13a9459f→767aa23d，5生产+3tests；作者/集成完整tree同87f24fd6。删旧upload_cancel/legacy_upload/Disconnected，严格当前GUI greeting；normalCore+greeting4+coreIO12+真host/ACP3=19不同父通过、独审。磁盘guard及一次fixture缺mut编译失败原记录保留，非业务红。实际GUI→Tauri→Rust→host→ACP仍未全验。 |
| Q041e bindings生成（本轮完成） | 作者c89aa135→**670347d7**，只3冻结源。先真fresh/reused多3旧TS红；后11普通父、真实Busy/SIGKILL恢复、实际runtime默认/显式路径、旧输出接管、standardgen/types通过，root核1922inputs/190双盘file/2binary。集成又实际normalbuild64.098s→gen6.729s→types5.164s绿，1923inputs稳/3组退出，旧157TS成为154TS+1管理marker，恰清PlaceRow/QuotaWindowKind/Wheel；旧原件保留。TMP94个npm缓存另凭精确清单清理，原tmp_absent=false记录不改，post-cleanup证明已清。全Q041/finalApp未因此关闭。 |

## 当前所有者与写权

| owner | worktree / branch / baseline | 当前行动 |
| --- | --- | --- |
| root | wave1@670347d7；plan总账；native-io Gen c89aa135源码已交还 | 集成、独审、公共接口协调和最终验收。Gen小片已完成；负责N1 delta、R1 paired与后续terminal独审。 |
| node_transfer_owner | `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery`，fix/quality-node-doc-transfer@**5f2d5cf7**+N1 13源 | 正常DocTransfer接线01已normal check15.307s，1083输入稳；独审确认B1真实raw1MiB门漏、C1测试proof方向。02已修并冻，仅5源delta。当前获唯一槽仅B1真实raw父红→绿，运行时在冻结red/02四源间精确切换；不要当场覆盖其源。link/proto/lib及Pending catalog en/zh仍归其本片；暂不自动N0/11unit。 |
| remote_cancel_owner | `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`，fix/quality-remote-cancel@**8242b5bda383b456a0122fc2613ca4e274620187**，19源checkpoint已commit；仅delegation_wire新增WIP | R1 18 focused不同父已过且root证据独审；已知child可靠取消片尚需旧受影响24+真paired Bound父。paired01 root发现Unix libc helper破坏Windows编译，已授仅该test最小平台guard修订/paired02冻结；不Cargo。R2未知task/lateSend仅设计，生产不动。原link/proto/lib保01原patch，后续写权已交N1/root。 |
| node_test_review（现terminal作者） | `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-store-owner-recovery`，fix/quality-terminal-current@**13a9459f**+Q052 WIP | Gen实现/卡已交root，不能自审；N1独审已交root。当前实现Q052终端current-only清理：host/GUI term与必要commands/CLI/测试调用机械适配，无UI/依赖/公共link/proto/lib/catalog写权；只源码/定向测试及freeze，不Cargo。 |

所有树均复用旧目录，不增worktree。send_message不唤醒completed agent；有动作须followup_task。旧库作者源权已归还root。作者不能自审自己的片。

## 当前唯一槽与真实运行

- **唯一Cargo/native/gen槽：node_transfer_owner，仅B1 red→green两exact**。已现场核无其它编译后启动red：runner67142 / Cargo PGID67160 / exec session30075；实际是否仍live以该handle/ps为准，不因timeout重启。红若setup/编译/资源/清理或意外结果，立即停且恢复冻结02；只有真实raw-cap业务红才续同父green。
- 两runner STEP300/TOTAL420/MIN5368709120，jobs1/locked/offline/incremental0，统一target `/Volumes/disk0/Khor/caches/rust/khor-quality-target`，短独占TMP、源前后/实际binary/有限group drain/双盘回执。跨树必须核当前firstparty source与fresh=false实际产物；不得把后续被覆盖binary哈希回填旧记录。
- 外内可用都≥5GiB；掉盘/越线/连续3IO只停自有组。外盘风扇坏/SMART不可读，用户已允许继续。最新B1启动外5,969,137,664B/内13,232,922,624B，动态测。
- storage-audit-10已凭精确SHA/uid/nlink/inode/lsof0回收5个已退自有test缓存约1.004GB，含旧R1-01/Node/Pair；源码/证据/App没动，缓存删除不否认历史结果，不复用旧清单删新文件。

## N1/N2实况与下一步

- 真N0：dd60fc5d/audit ref保存原测试；真实Node+relay、合法4096×64地址历史，差集25,447,954B，旧4次public sync不收敛但Seen123/Pintrue持久前进，1普通父业务红。原oracle完整保留。
- 真Pair04：合法12288×64历史，Snapshot29,224,082B首次回包过cap且票已消费、同caller重试badinvite，1普通父业务红。02/03之前只是准备失败，不能另算产品红。巨大Pair/Vouch/可恢复claim为**N2仍open**。
- N1当前单DocTransfer1MiB encoded/256KiB chunks、quiet一次Versions、每doc双向合计4chunk/1MiB quantum、visit32MiB/30s、固定Preparing/Ready、checked flush→durable receipt→Applied与exact ACK/GC。删正常旧Sync/Synced/base64，不留兼容fallback。源码01独审除B1/C1无新增确认阻断；实际性能/收敛未验。
- B1新增父使用真实Pair身份、同连接2MiB普通Versions admission正对照、padded canonicalWrite/Confirm、合法256KiB Exchange Stored正对照。01必须显式出现原Missing/Confirmed及创建transfer dirs才算业务红；02必须exact cap Refused且无state副作用，正常chunk仍可用。
- 冻结：external Q039/node-transfer-n1-b1-red-candidate-01、node-transfer-n1-candidate-02，internal recovery同名（不带Q039层），各19files/1083inputs。test-only另4files。源当前运行状态优先，不按“02已冻”覆盖red测试中的四源。
- B1后再授11新unit、原normalPair/agents/knownhost/inventory/epoch/64MiB隔离和N0实际收敛；quiet1Versions、fairness与postflush/prereceipt reopen需实证。任务卡Q-039-node-transfer、Q-039-node-n1-review。

## R1/R2实况与下一步

- R0真六Prompt/Stop，四held实际Cancel释放后旧实现第五六永不投递；现修复真正送达。原R0是受控peer/真实local runtime/IPC，非外部ACP全终态证明。
- R1 4有限网络Worker+1metadata lane、durable intent/child receipts、stable native/HTTP binding、明确Durability/ACK/Terminal状态、坏记录隔离、退避/公平/暂停均已实现。01独审五项及root03两项已修。
- 03执行原R0/auth-parent两父通过；随后fixture缺runs目录setup失败。04建目录后前五新父过，Completed fixture误把合法Refused当服务错误而停。05只修该测试共享reply harness，16新父全通过；总**18不同父**，不累计04重复。root逐rawlog/source2350/19源/双盘/2binary/161PID独审通过；生产相对03无变化。checkpoint **8242b5bd**=受测19源，未集成。
- 当前新paired父必须真实ResolveBinding→BoundSend/Get→actualACP permission；错caller/home/反向绑定走真实认证transport拒绝，原任务保持，再validCancel→freshGet Canceled、原parent/单Task不变，cleanup先断言。不得拿旧裸Send/Get/Cancel的旧paired测试替代。补平台guard中，原默认测试保留；新父explicit selector不nested Cargo。
- 后续最小相关清单24旧不同父+新paired1（20lib、wire2、GUI2、paired1），需要当前stub provenance及相关必要test targets；只复用05真实libtest不冒充重跑18。方案/runner在Q037/remote-r1/paired-candidate01-*与next-validation-plan-*，修后另冻02。
- **R2未知首次Send task lookup/lateSend fence仍open**，不得重放Send/Answer、从List起点无限扫描或将未知标全取消。集成顺序先R0 seed f0104d73，再R1固定提交及新test；三公共源与N1意图由root整合。任务卡Q-037-remote-r1/root-review。

## 完整目标剩余

N1大差集真实收敛、N2 Pair/Vouch恢复、R1剩余验证/集成与R2、Q052终端当前协议、Q033真GUI→Rust上传、Q035实际borrow lease、P1余下模块深审、P4命名/职责/错误/失效历史/工具链治理、P5最终同源App全流程。Q041剩fresh onset/远程/Linux/musl/Rust嵌入；Q045真实SSH/master/远端lease等按任务卡继续。物理iPhone/IME/Windowsruntime未知，不把元数据或类型检查当平台runtime。main/ship不在本轮授权。

## 恢复与证据

证据根 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`；内盘 `/Users/zgh/Library/Caches/khor-code-quality/recovery`。掉盘先读该处RESUME/current-code-quality-docs/current-worktree-refs.json，原外盘未提交内容不能据摘要覆盖。Gen集成证据Q041/bindings-integration-670347d7；其余精确SHA/细节进入各任务卡及历史，不将局部count累加为全仓通过数。
