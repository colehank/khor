# feat/net 当前状态与恢复入口

2026-11-09 01:42 UTC · owner=root，串行，无子智能体。
完整范围只在[PLAN](PLAN.md)，要求见[REQUIREMENTS](REQUIREMENTS.md)，准确源和全部历史结果见[RUNS](RUNS.md)。
本页只维护当前状态；旧页原样保存在[上个检查点](HISTORY-20261109-before-billing-stop.md)，其中“在途”均为历史。

## 授权与环境

活动目标“完成计划”授权N0–N5实施/验证，取代最初仅规划状态。速度/体验优先，个人可信设备网。
没有合main、发布、中继采购/部署授权。本机source-only：不运行Cargo/Rust/Node/Xcode构建、测试、格式器或下载可执行物；小型Python/脚本检查可用。
Linux重型用私库khor-build GitHub network job；Mac沿既有macserver lane。不触碰其他任务树、现装App及用户会话。
图谱先用；本树图谱仅覆盖部分代码，vendor缺失时定向读。不建立CodeGraph索引。

| 用途 | 位置 / 分支 |
| --- | --- |
| 产品 | `/Volumes/disk0/Khor/worktrees/current/khor-net`，`feat/net`；源码检查点f17b8a31；37候选及Mac入口随后提交，实际HEAD恢复时核对 |
| 控制 | `/Volumes/disk0/Khor/worktrees/current/khor-build-net`，`feat/net`；b9b27116ad286fb51edc1e211949127681c5aa27已push（历史run各绑定原control） |
| 基线 | `/Volumes/disk0/Khor/worktrees/current/khor-net-baseline`，detached6485cb83d3b1f0bb93d4633825dc956623d78af9；只有paths/hop/lab/nat.rs、dev.py/netlab.sh六文件fixture修改；生产原样 |
| 证据 | `/Volumes/disk0/Khor/evidence/net/` |

## 当前有效执行通道与37候选

第3个接续turn已发现并实际验证Mac替代通道，因此整体目标不在执行器死锁，保持active；Linux托管lane的账单限制仍在。
直接SSH macserver经既有iroh-ssh路由在banner超时，未改路由/信任或用户会话。既有worker-smoke **42549824680 SUCCESS**，已保存工具链/共享guard和C编译探针证据；它不是产品网络通过。
控制6b1616c8b816bdb72a8a031d0ff28cc46a3fb4c7已push：注册development.yml新增executor=mac便携分流，跳过托管prepare/retire；
同一Mac runner/队列/构建锁，先校验准确源码再运行。仅GitHub临时只读checkout权限、persist=false，无个人凭据/正式发行，现有准入/空间策略不改。
本机CLI和预上传任务范围检查新增，11项Python通过；默认standard不传新input，保留旧控制main兼容。

**42551075037 已终态预期产品红**，Mac正式执行同7b61b7edc12ed5ef384b7054240e7b5202f1638b、snapshota21c1a978549979a8956639d35fdbd722cc6c8df、basea087 dirty，control6b1616c。
根格式/437Noq通过，Iroh35pass/1fail仅新无接收copy快速Go用例；实际Hold双PING、预期Go。home过滤器因路径红未执行。
证据mac-current-copy-hint-red-42551075037已下载并核tree/control；Mac26.6.2 arm64、Rust1.95.0、jobs2。不是Linux性能证据。
第一次dispatch因job env错误使用runner.temp返回HTTP422，未创建run；修正为step env后使用同一request创建本run，不算一次产品失败。

37条件分支现已按实际红写候选：当前路有故障后/最近原tick接收提示时保留36的决定后备用应答与当前故障copy探测；
没有提示时允许故障后已有备用应答快速Go。故障前旧样本仍不行、4秒兜底仍在；额外false旧包/兜底检查属于待验证绿覆盖，不宣称旧源已对它们取红。
当前37生产候选、Noq2新用例、N3记录器均需下一次远端格式/编译/单元；不能以本次437绿替代新源。

## 当前执行器阻碍

**当时没有在途任务。** 42545673450与42545712302在prepare启动前被GitHub账单/支出额度拒绝：steps为空、runner_id=0，network skipped。
两项均无测试/远端源码artifact，不能算产品红、行为红或通过。GitHub原始run/jobs/annotation与本机提交回执已保存到对应证据目录。
已向用户异步询问恢复额度或已有Linux执行器；不擅自改账单、采购或运行本机构建。
既有Mac任务也依赖托管prepare，直接重复调度不能绕过此问题；不改变凭据/发行边界。

| 未执行任务 | 本机准确源 | 待验证内容 |
| --- | --- | --- |
| 42545673450 net-vendor | basea087 dirty；tree7b61b7edc12ed5ef384b7054240e7b5202f1638b；snapshota21c1a978549979a8956639d35fdbd722cc6c8df | 新故障边界用例预期红；没有实际取红 |
| 42545712302 baseline netlab-stalls | base6485 dirty；treebaca3064752631153c35547440da2114a36ca855；snapshot4565266b2619de01b16cbd62f88240d0551bf65f | 与observer v1六fixture匹配的基线CPU/短停顿复验，未执行 |

两个快照已经锚定本地不可变ref；这只证明提交源码可恢复，不证明远端验证。恢复执行器后先查是否产生新run，不能盲目重发或把同ID当已有结果。

## 最新有效证据：36的稳定性改善与恢复回退同时保留

前三项同basea087e1c5 clean、treec807d328e78bdfda1dbebd7bdd4c14b0eaa8cd25、controlc68b644，默认日志关闭。

| Run / snapshot | 结果 | 解释 |
| --- | --- | --- |
| 42542824071 / 969f8e52fa52fe7b5043ff89f333bfc368c5724a | SUCCESS 2/2，1118.42秒 | 原p1s 600秒/20次300ms/忙CPU：alpha/beta0/0移动，零退役/打洞，最慢865.4ms，3019包312293字节；p5原150/300ms门绿，600ms观察组不扩展为门 |
| 42542984111 / 98d696e58102978d44fe15cfb1be0a439473fc30 | FAILURE 25/27 | p3黑洞三轮1.1秒/1.2秒/818.4ms，零错误/退役，前两轮超过原1秒门；p2三轮绿，21相位全绿 |
| 42543254686 / bfd551912eebd65398b69d53467ca7366cf511f0 | FAILURE 8/9 | f4s第一轮0.7秒回carried后10.9秒再次退回Relay、28.8秒才再回；另外两轮0.4/0.6秒且无回退。全部f4/f4i绿；回退红独立保留，不宣称37能一并解决 |

日志诊断42542228149同36候选策略，2/2：p1s0/0、确认PING确实选中PTO3副本且返回Ok；f4s应答后0.3秒回carried、无回退。
**诊断不是SLO**；普通8/9和25/27不能由诊断绿覆盖。旧Watch p3三轮508–656ms、p1s3/1移动及f4s1513ms红继续保存在RUNS，不混算不同源/VM的改善比例。

## 待取红的37设计，尚未改变leave生产行为

36对所有故障都要求决定后备用新ACK，解决闲副本PTO误切，但在真实黑洞两端多加观测tick。
已有诊断的4个故障Go：同路另一copy在坏copy最老unacked之后仍收到包。这个事实只应触发“复测当前copy”，不能证明当前正向路健康。

33202c22中的remote_state改动：confirm_move从所有当前路live copy查询last_received；包必须晚于silence.began且距现在不到一个原250ms观测tick。
该布尔值已传Moving.leave但命名_check_current且**仍被忽略**，因此leave保持36行为；新增false时允许故障后备用样本快速Go的测试预期红，42545673450没有执行。
已有七个leave测试调用显式true，保留36防抖证据；Noq6新测试也在该快照中。

下一步先在远端取得该回归实际红，再最小实现：有当前copy收包提示时保持36的决定后备用ACK和当前故障copy探测；无提示时接受began之后已有的备用应答。
保留旧包保护、4秒兜底、正常两样本、速度优先与原探测节奏，不增加应用请求重放。需共同复验p3<1秒、CPU0/0及转发不回退；不能放宽门。

## 最新补丁覆盖

- 42543928044 **SUCCESS 根格式、437Noq/35路径/9选家**；basea087 dirty、tree87bd60cccdce5eeb412774b9409f9a256dd722fe、snapshote4be1454ca3338aab9abc723a0218a2fa6063ad5、controlc68b644。
  Noq6新增真实虚拟连接测试：连接5秒不抢路径15秒计时器，路径16秒仍保活，无路径计时器对照仍保活；无生产修改，不宣称旧源红或手机能耗达标。
- 42542123882 SUCCESS 436/35/9，36故障确认回归已红转绿；35首次可用性与quality分离已由42540515220 436/34/9验证，重复故障hold保留。
- 42539256792 SUCCESS 436/33/9，健康Backup独立回PATH_ACK真实连接用例；单向故障/兜底/成本仍缺。
- [VENDOR-AUDIT](VENDOR-AUDIT.md)共47项初步盘点；Noq1/5专属覆盖、Noq4其余边界、上游逐函数等价、原生代理/真机/能耗仍缺。

## 完整计划状态与顺序

N0部分、**N1仍红**、N2–N4未完成、N5部分。不能用一批单元或某个场景绿完成整计划。

1. 保存上述新结果/未执行分类，核worktree与未提交内容；不得从摘要覆盖代码。
2. 执行器恢复后取得37回归红，最小改动并取绿；先普通recovery三轮，再stalls/hop，最后准确源完整矩阵。
3. f4s普通回退需独立定因；必要时默认关闭的诊断完整保存三端trace，保持原断腿与1500ms门。
4. 本轮已推进不依赖执行器的N2–N4源码/设计审计，见NEXT-DESIGN及八文件blob收据；尚未实施/实测。N2七类预算/取消/0-RTT/写入协议已在ACCEPTANCE冻结：锁/hop/首拨未贯通by、早数据回答失去Aborts仍未实施。
5. N3交互与批量公平/成本，N4两端中继与登记故障域/容量设计，N5Mac/手机/多机和最终可信Quality，均继续按PLAN，不擅自缩范围。

## 证据与恢复

现**96**个不可变`refs/khor/net-evidence/<run>`；外部清单`/Volumes/disk0/Khor/evidence/net/source-refs.json`。
`git rev-parse REF:source`必须等于收据tree；parentless容器只用git show读取，不checkout覆盖、不push本地证据refs。
两项账单拒绝快照标tests_executed=false/remote_source_verified=false，其余终态不再pending。
33202c22已保存Noq6绿覆盖、未执行37边界用例及当轮证据；恢复仍先核branch/HEAD/diff，不假定工作区干净。所有历史红继续保留，两个main和产品feature未合并发布。
外盘缺失时只读git show feat/net:docs/net-optimization/STATE.md和PLAN；不覆盖原外盘未提交内容。
治理任务继续遵守中央code-quality STATE及外盘中断RESUME，本网络任务不代表治理授权。


本轮N2补充：不能把28秒变为长操作的总上限；沿Op现有档位的真实截止点贯通，保留Request预算。N3发现普通每连接32席位/content另预算及Custom外层队列，需分层测量后设计调度。N4默认北京双URL同机、默认自家登记集中北京，尚无在线故障实证；新增机器/登记/双家仍是提议。
本地9项remote_development Python检查通过，仅脚本；Rust/Node/Xcode未本机执行，不补记当前37回归绿。


## 本轮接续（账单阻碍第2个连续目标turn）

上一turn为进展：33202c22/32773d44已改变权威测试/设计/记录。本轮重新读取GitHub终态和原拒绝annotation，仍无在途run，随后仅对该已终态任务做一次同源attempt2重试，见下文。未达到blocked条件时不标blocked；后续只有无法继续实质推进且同阻碍达到阈值才使用该状态。
N5继续核完Noq1/2/5的固定1.4.0相关调用链，包checksum复验通过，逐文件hash/行号摘录另存noq-1-2-5-call-chains，详细结论见UPSTREAM。
新增Noq2始终启用地址报告的0-RTT握手测试；它补原accepted在恢复时禁用报告的盲区，仅测试、未编译/执行，不记438通过或旧源红。
现有Noq437绿仍绑定旧快照，新增用例不在其中。37条件分支仍未改；下一次vendor任务需先分别确认Noq新增覆盖与Iroh预期红。
另外N3既有ignored congestion工具只保留成功echo，失败被丢且成功数标签叫asked；ACCEPTANCE已在改记录前冻结v2完整采样协议，本turn随后完成v2候选，见下文；旧摘要不能证明无错误/长尾。


为验证当前可执行性，42545673450于本turn同源重试attempt2：prepare job128936767186再次因账单/额度拒绝，runner_id=0/steps为空；network skipped，无编译/测试。不是新source，也不含本轮Noq2测试。原始run/jobs/annotation和未执行分类保存在原证据目录attempt-2，原attempt1证据保留；没有在途任务。
执行器阻碍已连续出现两目标turn，但本turn有N5调用链证据/新增协议回归/N3采样协议等实质进展，目标继续active。下轮可先修N3测量记录，禁止仅重复状态/无意义重试；如果之后无可推进工作且同阻碍达阈值，再按规则blocked。


## N3记录候选f17b8a31（未远端编译/执行）

在f0be88d4冻结v2协议后，更新congestion.rs：每个echo/typing/screen保留elapsed+ok/timeout/错误阶段，
成功延迟与全部尝试结束时间分开，空成功分布null；bulk有字节/真实时长/结束原因。记录在计时阶段后输出，保留默认工作量/seed。
所有controller/轮输出后，对请求错误或不完整bulk失败退出；没有性能数字或新性能门槛。只改测量与脚本，未改生产选路/预算或37分支。
control 6447289f4f9260df4f5887adc7c1e2f2270bbcce的net-congestion已push并核远端SHA一致：Noq模拟、1线程3轮、默认几何、独立log、slo=false。
本机driver桩测试1项通过，证明首轮失败仍执行后两轮并保留最终exit8；prepare1项、产品remote_development9项及bash-n/diff-check通过。
这些是脚本检查；Rust格式/编译、Noq2新用例与真实congestion都未运行。没有新run，不增加94 refs或已通过数字。
产品dev.py新选项已同步baseline，仍仅原六fixture，生产6485原样；baca306旧快照不改写。N3与N5新增代码仍待远端验证。
下一步仍先取得37实际红；之后Noq2/N3格式/编译、原recovery/stalls/hop和最终矩阵，N2–N5全范围保留。


本turn补齐Noq接口/ACK/保活/备用恢复入口静态核查，源hash/摘录在upstream-20261109/noq-interface-and-recovery-call-chains；未执行上游，不删除补丁。37已取实际红；原“先等红/未改leave”段落为此前状态，以本页当前有效通道段为准。


下一control b9b2711仅补Mac入口说明及只读host-capabilities.json（Hypervisor支持/CPU/内存/已装VM工具路径），不启动VM、安装软件或改空间策略。当前95源ref均终态，37候选下一次Mac检查尚待调度；调度后立即记录新ID与source/control，不能恢复时重复旧run。


## 37候选正式调度

37及Mac CLI入口已提交72b1d891c879cab63aaed032e201aee7baf1734c，工作区当时clean。
**42552723913 net-vendor --executor mac**：treef6d0e84996723b8e9ad4659b6a2be8b625cb74b5、snapshotbafedc8a33390550c2bd0b10de0103e305500d31、controlb9b27116ad286fb51edc1e211949127681c5aa27。
新源含Noq2始终reporting测试与N3记录器；要先看根格式/编译，不预认438/36/9通过。额外只读host-capabilities.json用于判断Linux后端是否可行，尚未启动VM。
96个source refs已锚定，以上run pending；恢复只查询该ID，不重发。Mac单元即便绿也仍不能收原Linux p3/CPU/hop门。
