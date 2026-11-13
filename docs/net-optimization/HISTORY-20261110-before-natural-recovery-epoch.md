# feat/net 当前状态与恢复入口

更新：2026-11-10 06:59 UTC。用户授权继续完整N0–N5，尚未完成。速度/体验优先，可信个人设备网；无main合并、发布、中继采购/生产部署授权。root串行、无子智能体。完整计划/硬要求/验收以[PLAN](PLAN.md)、[REQUIREMENTS](REQUIREMENTS.md)、[ACCEPTANCE](ACCEPTANCE.md)为准。旧状态逐字保存在[本轮历史](HISTORY-20261110-before-ping-delivery-investigation.md)与其历史链。

## 恢复先核实际句柄

- 主worktree `/Volumes/disk0/Khor/worktrees/current/khor-net`，feat/net，生产行为候选ba28fc03：已在真实红之后让incoming准入读node.paused既有缓存，serve初始pause::poll保证accept热路径无磁盘IO；该候选已Mac/Linux同source十二项全绿，详末尾06:55记录。a00c9554新增cfg(test)显式STOP，4b7e87ee为格式后的真实resume合同/barrier，原53 vendor。控制 `khor-build-net` feat/net HEAD68cadb57a0ed72d40a4ca9a843e86b9c6797bf34已推，net-node只在原remoteQUIC32后/原canonical前新增resume合同，3个source-only stub控制测试绿。
- 隔离候选 `khor-net-recovery-deadlines` detached8d91e1ec/treebbaf802b/snapshot894ed4e，无IPC v2；新增Iroh42，未合主。不把隔离候选称最终整源验收。
- Turing前驱 **turing-20261110T054132Z-da82e2b8** 已完成/采证，原group空，1142.731秒/rc0，quiet原p1s600秒20×300ms+p5全绿。full-counter诊断 **turing-20261110T060223Z-6d83f431** 已终态/采证，218.525秒/rc0/原group空/sourcecontrolbundle16artifacts核；原124/124、请求/decision/state全绿，X3585.182/Y3577.754ms，预算6/2episodes绿。只得正常counter参照，未复现前轮5.8秒红，不计性能/三轮接受。c2ec Linux net-node已终态rc101/226.685秒，source/control/bundle19artifacts/原group空核；原busywire在主动pause→resume后731行立即Cancel得paused拒绝，主I/O/cancel此前均完成，新STOP未执行。初步误称first/precondition已在ledger/proof记erratum，保留真正恢复红。第二counter诊断 **turing061146-ff132946** 已采证218.598秒/rc0/原group空，原124/124、两门/state全绿，X3285/Y3518ms、6预约/2episodes；两诊断正常不能关闭前5.8秒红，不盲刷第三诊断。quiet Iroh41 baseline f9d50e90已终态/采证1186.777秒/rc0/原group空、32ok/22CPU/allthread64/2hogs/11artifact核；原p1s1move relay245s/worst881.6ms/18punch4632pkt1706984B、p5绿。同Iroh42候选仅一Rust tie规则+KHOR文档差异、同4eeee，comparison在confirmation-copy-matched-quiet-comparison.json，单对不接受性能。初始审核误把KHOR文档排除而assert，已显式纳入两文件并留erratum，非产品红。当前Turing唯一重型 **turing-20261110T064107Z-4bcfad82**，PID1837962，06:41:28 UTC，net-node/control68cadb；同Mac候选ba28/treecf0b29d4/snapshotf9efe7c2，继续严格原/新增合同。
- Mac **42770379226** request74163b48be9c4eb686e1ef6e47c1f8dc已完成/采证全绿，验证生产候选ba28/treecf0b29d4/snapshotf9efe7c2/control68cadb，严格同四whole fixtures；合同先记df84，whole四个fixtures冻结于resume-admission-fixture-freeze.json。固定旧观察门true、next poll保持barrier，本进程resume后真实A→B list须3秒成功；旧canonical与STOP整例不改。诊断源1a6371d7/tree80a1f9a0/snapshotd2f17c01已在Mac42765006984 fmt/Noq441/iroh50/home9实际全绿（两新PING正负控制含在441），source/control/11blobs/13artifacts核；仅诊断未合主。原whole multipath/物理/paths tests保持，carried-ping-fixture-identity.json。健康参照counter显示首次API+1.08s、下tick+1.33s实际PING与新219ms RTT，+1.58s取得85/92ms；见carried-ping-counters/reference.json。不是逐packet时间，也不能倒推旧慢run的发包情况。42761341395新STOP合同实际绿，source/control/11blobs/19artifacts已核并terminal登记。macserver SSH已实际恢复（03:54与04:37后均成功），A=Mac/B=Turing天然异网，P=iPhone，不重复要求另找两电脑。
- 当前N1优先：**trace实际PING发送→ACK→RTT sample**。Iroh42第二轮仍双端约5.8秒红；API ping Ok不是实际发送/往返证明。不盲调250ms节奏/2秒确认/4秒门。

## N1：已证事实与未闭合项

- Iroh41同source f07db0c9：ad62734d原124/124、请求51/51、双端Go3816/3613ms绿；repeat2 34871ed6仍124/124但Go3747/7238ms红。三轮达标未成立。Iroh40 fc435/3d3两端约5.4秒红，Iroh39 fa99请求门绿但Go4060/5799ms红；现代匹配baseline53aa约7674/7913ms红，历史不覆盖。
- Iroh41只在有限恢复episode前2秒，真实post-epoch样本至少向下降25%才重启quality窗口、最多3次；各copy继承epoch，旧ACK不刷新。正常2秒/两fresh/收益门、3probe/250ms/2秒保持。335b真实paths49绿1红→1514516e代表confidence改为当前recent数后50绿，whole tests SHA5c4da4b4保持，439/50/9绿；主cherry0c223+724582。
- 诊断8c90acc0准确5a39源，124/124、请求51/51却X3565/Y7549ms红：Y1549ms在copyA取84ms开始Moving，普通confirmation切copyB，101ms样本成为road代表后撤销确认。另一copy同刻PTO未记录，不声称同刻必为tie。Iroh42 8d91e1ec仅让candidate confirmation复用已有recovery_copy_rank：低PTO优先、同PTO保留恢复copy；current最高PTO规则不动。无新增PING/时间门/强owner，54补丁候选未合主。
- Iroh42 vendor turing051032-74e68abe准确894/control69d：fmt/Noq439/iroh50/home9全绿，原tests SHA5c4da4b4不变，source/bundle/control/14artifacts/group空核。
- Iroh42物理1：turing051404-4f2d591b，原124/124、请求50/50绿，XGo3262ms；Y从未离开同wire Custom，首post stream14ms/首业务成功762ms，全32post请求同wire且选carried、无targetpeer重拨/路径变动。原严格decision因缺Go仍红，不能单凭缺事件记绿。新增补充continuity合同后回溯绿只算探索，不计prospective repeat。
- **Iroh42物理2：turing052323-2c9d5898**，原124/124但请求门52个post4s仍Relay、XGo5825/Y5870ms，原decision与新state均红。X实际三次ping调用Ok在1073/1324/1574ms、同copy、PTO0；但直至2323ms RTT count/timestamps仍旧值49/1.1646s，另一copy也没新样本。Y没有RecoveryCheck观测episode，稍后自然样本才开始Moving。已读exact noq1.3.0 Path::ping：lock_and_wake→proto.ping_path，只置pending_ping。下一追发包/阻塞/ACK，而非猜测调参。
- 新`scripts/netlab-peer-state.py`不改原物理/selection/decision：只有full logs+原请求门绿、真实pre/post同wire carried、首业务≤4秒、无postpeer Go/reset/path/dial/retire且hash核，才允许“始终保留好路”补充。18个复制数据正负控制全部符合预期（17负），回溯9真实run所有旧慢恢复红保留。详peer-state-negative-calibration.json与peer-state-retrospective-calibration.json；冻结后才允许未来prospective≥3轮，不回填旧验收。
- Noq12 selection-only ACKfloor不改transportSRTT/PTO/CC，真实红266/270→271单元439绿、旧fixture纠错/whole hash保留。但quiet modern matched baseline两轮零move/worst539/635ms，candidate1/2moves、约1s/719ms，第二原at-most-one红。Iroh40 quiet71aa原p1s/p5绿但1move/worst1.3s，不是整体性能接受。当前Iroh42 quiet试验p1s已原20×300ms/600秒绿，两端0move、worst671.7ms、6punch/3362pkt/1325754B；p5全部三个停顿档也绿、全32outcomes ok；22CPUmarkers、全部live线程CPU64/2hogs/end610845ms/11artifact/source/control/bundle已核、原group空。quiet-stalls-audit-v2.json并入原比较，单轮不算matched重复接受。
- 040407-e3bf/042825-2a7fe04f有初连10.032秒超时未replug，保留cold-start独立红；不是恢复时序。Noq1/5专门coverage/上游等价仍缺，不凭成功样本删保护。

## N2：期限、回复所有权与实际STOP

- caller短期限427298真实5绿1红→427332 IPC6/portable绿，whole原块SHA10838747。本机32原427371真实5秒回收红、wholewire SHA945c22aa；lease首次427389 SIGABRT保留，Box::pin大型future后427399原整fixture0.33秒绿。
- 独立process lease：427531实际约1006ms，TCP writer仍open/barrier未释放、server32permits回收、Node真正join/child reap；坏cookie/多余输入不触发Begin，legacy halfclose保持。ipc-independent-process-proof.json；不是手机/Windows证明。
- 原远端QUIC32：427545实际barrier32/permit0、callers abort/join后5秒回收门红。17d只为Resolve/Window/Record在dispatch/prepare监听send.stopped，Begin/Cancel/写job/Watch保持原语义；首427551 E0425不计行为。757返回(frame,resp)并持resp至write后，同canonical tree2d218/snapshot3f0：Mac427556+Turing042313-d964所有net-node绿，remote32为1.53/2.37秒、原busy-wire2.35/4.34秒、process约1006ms。quic-response-owner-32-red-green.json核两个whole fixtures/red-green命令前缀同，resource层已闭合。
- **新实际STOP/same-wire层Mac42761341395绿1.93秒**，准确a00c9554/tree028f96b6/snapshot6ff42190/control69d：32Window实际编码屏障、全部caller abort/join、32个Ok(Some(STOP code))、零connection-close替代、同B stable wire、barrier仍持有时同wire Begin/Cancel完成。quic-explicit-stop-wire-proof.json标明仅Mac强层；Linux这层仍待，不冒称WAN取消/accepted写job/phone。新whole stop fixture SHA19bf3701，原quic32 whole bb4e8b69及原local32/busywire函数体不变。主源码仅cfg(test)观测。

## N4：当前整源真跨网分项

- 当前cohort coordinator4cee410c97601afe/Mac42757732776，同757/tree2d218/snapshot3f0/controlccd。两端fresh ID相符，双向各30目录+30固定29B正文按字节验证，**120/120全部ok**。Mac binarySHAcd9be1a8、Linux0e063201；源/control/全artifact核。wan-current-directory-body-proof.json。
- worker elapsed_us最近秩p50/p95/max：Mac→Linux目录55382/58815/76133、正文61322/64630/66470；Linux→Mac目录59527/64622/65616、正文55671/58949/59727。全outcomes无过滤。实际均公共Relay101.200.80.77:3340；不是Direct/peer/IPv6/mobile/p99/吞吐/手机能耗，也不与旧ff608 cohort作匹配性能比较。
- 两端resident实际join、worker0/owned空/Linux原group空/锁释放，one-time GH secret已删；两个business fresh identities及8个lifetime copies均直接验证删除，仅本次自有资料。当前wan-binary-manifests同3f0 cohort，旧ff608逐字备份。
- 当前source stop/EOF/kill：Mac3/3+Linux3/3真实绿，wan-current-lifecycle-proof.json。最初两stop的postobserver IndentationError保留、readonly恢复但不计pass，新六轮才算。observer三条snippet语法核，guardian69e7346与currentccd字节同。
- 原ledger registration误标directory_v1保留，并以artifact_measurement_revision=directory_and_content_v2与erratum补正；两个actual records本为v2。ops未来label已修，不重跑已证业务。旧ff608双向目录263、旧lifecycle证据仍锚原cohort。

## 完整剩余范围，不缩减

| 包 | 必须闭合 |
| --- | --- |
| N0 | 最终目标/容差/样本量/p99充分性、全outcomes、同量测baseline/candidate/固定好路/负向校准 |
| N1 | 先PING真实链路与cold-start；同source至少3轮双端四秒、原CPU稳定性/黑洞/成本；冷暖多候选/多middle双向、201/208/244/242慢方向因果；原hop/recovery/dynamic/stalls/full矩阵与162/182失败 |
| N2 | Linux强STOP层、accepted写job所有权、两middle总3秒、输者/晚候选、地址epoch/重启/jitter合并/quiet epoch/close-vs-offer、写不重放/legacy |
| N3 | Custom1024帧/256KiB外层队列、SSH/hop混合公平/共享vs分连接、1/3/5设备CPU/字节/唤醒/手机能耗；低层704请求不能替代 |
| N4 | U12 IPv4/IPv6独立/同时/地址族切换、peer移动、Mac/Turing/iPhone冷暖不对称、登记/出口/整机故障域、有数据的布局容量运维方案 |
| N5 | 主53+隔离Iroh42逐项vendor coverage/上游等价/退出、Noq1/5缺口、最终源可信Quality、原生App/代理/多机/手机能耗；证据前不升级删补丁 |

## 执行与证据规矩

本机source-only，禁止Cargo/Rust/Node/Xcode产品build/test/formatter或下载执行物；Git/Python stdlib元数据/数据checker/纯stub控制合同允许。Turing owned `/tmp/khor-netlab-feat-net-1110` UID1110、单flock槽、Rust1.95/jobs16/nice10、50GiB准入/40停/90min；只fresh user/netns、无sudo/宿主网络更改，计时不夹编译。Mac走既有run-build/build_guard单槽/jobs2/50–40GiB/100GiB target，resident在guard生命周期内，不绕过installed policy/ensure-budget。

SSH BatchMode/ForwardAgent=no/StrictHostKeyChecking=yes；无个人key/PAT/agent/签名/模型凭据给workers，不重设HOME/CODEX_HOME。临时WAN capability只私管道/一次secret，禁止日志正文/票据。GitHub本机SSH上传不改worker认证。不确定timeout先查receipt/GH/proc/PGID，已有bundle/run先recover不重发。stage/launch显式完整source/control、前驱终态/组空/锁核；exec返回session不等于完成，commit hook结束后才snapshot。ledger避免并发writer。

图谱先，缺vendor覆盖才定向读；不自建.codegraph。旧baseline/control WIP不reset；只终态+所有权/hash核后清本次临时资料。`verify-*-record.py`仅核identity，必须读行为日志分fmt/编译/fixture/执行器/产品红。证据根`/Volumes/disk0/Khor/evidence/net`，source-refs审计最近329项（39ops），新增逐条pin，稳定后刷新全量。旧goal服务paused不是用户暂停，不因此停止；无子智能体/merge/release/deploy授权。

用户真机补充（2026-11-10 06:32UTC）：iPhone在手边，可切换Wi-Fi/蜂窝，已确认纳入P真机验收，不再问设备可用性。已异步询问当前khor安装/与A(macserver)、B(Turing)配对状态，以决定具体短操作；真实App/binary/source/cohort、IPv4/v6实际候选、首个成功业务/选路恢复/稳定与成本须分别留证，不能仅据网络名称推断地址族。手机答复期间服务器开发继续。

Mac新resume合同：42768368485仅workspace fmt一处assert换行失败，未编译/未执行行为，source/control/11blobs/9artifacts核。精确remote format.patch已inspect、whitespace-only应用4b7e87ee；新运行42769067590，request5d5db3d5c9b04c0b844af8b02133e51f，source4b7/tree751e2978/snapshotca77b24d/control68cadb，待实际行为红。生产准入尚未改；不得把fmt红当行为红。原wire_tests从struct Server到EOF整段与a00逐byte相同，旧canonical/local32及新STOP/QUIC32整fixture保持，见resume-admission-fixture-freeze.json。

实际resume红与候选（06:39UTC）：42769067590准确4b7/tree751e2978/snapshotca77/control68cadb fmt/portable/IPC/remoteQUIC32绿，新真实合同在fixture41被暂停拒绝、case6.25秒；四whole fixtures/source/control/11blobs/18artifacts核。resume-admission-red-green.json保留实际红，production fix ba28只初始化既有pause cache并让incoming查该缓存；不改500ms poll/持久job/重试/期限/旧fixture。候选Mac427703+Linux4bc同canonical并行验，pending不提前标绿。

06:55UTC：Mac427703与Linux4bc同ba28/treecf0b/snapshotf9ef/control68全部12项net-node实际绿。真实resume红427690→同whole四fixtures绿（case1.74/1.83秒），原canonical含pause/resume6.87/4.24秒绿，actual32STOP/samewire1.70/2.16秒绿；分别11controlblobs/20与21artifacts、Linux原group空核。resume-admission-red-green.json与quic-explicit-stop-wire-proof.json已更新同source双OS，旧Mac-only proof逐字备份。Linux f1d19b07同诊断1a已vendor441/50/9绿、14artifact/原group空核，所有当前run均terminal，无已知重型active。下一Noq3 stats发送状态补充仅观测，见NEXT-DESIGN，Iroh42仍未合主/5.8秒红仍未闭合。

当前（06:59UTC）Turing唯一heavy：**turing-20261110T065737Z-505de602**，PID1853108，06:58:00 UTC，net-vendor/control68cadb，诊断源f0a5db0e/tree1cfb126a/snapshot92afa1e5。仅Noq3 stats瞬时bytes_in_flight/ping_pending/pending_other_frames与两模型pending断言，不改selector/CC/PTO/发包；旧whole Noq RTT/多路/物理fixtures完全同字节，carried-send-state-fixture-identity.json。前驱f1d19b07完整采证，51.790秒/441+50+9绿、14artifact/原group空。下步collect当前vendor，绿后新版send_state物理诊断一轮；原5.8秒红仍在。新版数据extractor保留旧副本extract-carried-ping-before-send-state.py，v2 measurement要求三字段存在，字段不能缺省伪装0。
