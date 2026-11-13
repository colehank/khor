# feat/net 当前状态与恢复入口

2026-11-09 · owner=root，串行，无子智能体。仅本文件维护当前任务；[PLAN](PLAN.md)维护完整范围，
[RUNS](RUNS.md)保留准确源/原始结果。旧STATE原样归档到[历史](HISTORY-20261109-before-diagnostic-results.md)，其中“在途”为历史。

## 授权与位置

活动目标“完成计划”已授权N0–N5实施/验证，取代最初仅规划状态。速度/体验优先，个人可信设备网。
没有合main、发布、中继采购/部署授权。root串行；不派子智能体。

| 用途 | 位置 / 分支 |
| --- | --- |
| 产品 | `/Volumes/disk0/Khor/worktrees/current/khor-net`，`feat/net` |
| 控制 | `/Volumes/disk0/Khor/worktrees/current/khor-build-net`，`feat/net`，322835b已push |
| 基线 | `/Volumes/disk0/Khor/worktrees/current/khor-net-baseline`，detached 6485cb83d3b1f0bb93d4633825dc956623d78af9；仅paths/hop/dev.py/netlab.sh四文件fixture修改，生产原样 |
| 证据 | `/Volumes/disk0/Khor/evidence/net/` |

本机source-only：禁止本机Cargo/Rust/Node/Xcode构建、测试、格式器与下载可执行物运行。
小型Python/脚本检查可用；Linux重型用私库khor-build GitHub network job，Mac沿既有macserver lane。
不触碰其他GUI/治理worktree、现装App或用户原会话。新目录/证据放`/Volumes/disk0/Khor`分类。
图谱部分覆盖，先图工具，缺失后定向读；本树没有CodeGraph索引，不擅自建立。

## 最近诊断终态与下一轮

**当前在途：42542824071（36普通stalls）、42542984111（36动态）、42543254686（36转发）、42543928044（Noq6覆盖单元）**。
36已提交a087e1c5；前三项同basea087e1c5 clean、treec807d328e78bdfda1dbebd7bdd4c14b0eaa8cd25、controlc68b644：

| Run | 模式 | snapshot |
| --- | --- | --- |
| 42542824071 | 原p1s/p5，1线程各一次，日志关闭 | 969f8e52fa52fe7b5043ff89f333bfc368c5724a |
| 42542984111 | 原p2/p3/七相位，1线程三轮27项 | 98d696e58102978d44fe15cfb1be0a439473fc30 |
| 42543254686 | 原f4/f4s/f4i，1线程三轮9项 | bfd551912eebd65398b69d53467ca7366cf511f0 |

42543928044只加Noq6保活专项测试，无生产改动；basea087 dirty、tree87bd60cccdce5eeb412774b9409f9a256dd722fe、
snapshote4be1454ca3338aab9abc723a0218a2fa6063ad5、controlc68b644；目标437/35/9，未验证。
当前工作区该测试/KHOR修改已stage未提交。旧诊断/单元和35普通结果均已终态，下文保留准确版本解释。

42542123882已SUCCESS 436/35/9与根格式。
前者首次恢复可用性修正35的普通模式p1s/p5。
tree44fda196d1a9800c7e87e6f9175c3484661d5e51、snapshotfd0c22ffcd844d33271d337ca879961a02e2387b、basec43370ff dirty、controlc68b644。
同源单元42540515220已SUCCESS 436Noq/34路径/9选家与根格式，snapshot1f1cc72dcf35b79f43f96dceb34770455153b967。
后者base66e37760 dirty、tree32f638229d81beea7cd0dc40ccd16d67cb35282a、snapshot00e5b0a17f4364e83ba3a0c42d6a5a3361db7a0c、controlc68b644；
只加a_fault_move_probes_the_current_road_and_requires_a_post_decision_backup_sample，生产仍35。
现已终态436Noq绿/34路径pass1fail，旧分支实际Go而非Hold双向探测；fault-confirmation-red-42541459177已下载。

36候选已写并取得单元绿：leave要求备用包在max(since,began)后发出，Hold探测当前/候选；
当前选最高PTO live副本，候选选最低PTO副本，新增可选confirmation probe日志记录实际副本/结果。
原4秒兜底、收益/正常确认、routine wake旧off_direct规则不变，可能多PING当前路，必须核p3<1秒和成本。
两项同treea7f419472a9189c691a773535c70d223c2921ff9、base6c62a1bf dirty、controlc68b644：
42542123882 net-vendor已SUCCESS（snapshot2de64d8fa80f2fa067e82eddba2e2dfd7514ba76，436/35/9，fault-confirmation-green-42542123882已下载）；
42542228149 netlab-diagnostic（snapshot44cde49b6afb8e561a5e8664253aa7ddc93d218e，原f4s/p1s，非SLO）。
提交前核36策略文件与绿快照无diff。接下来对同一代码执行普通dynamic/hop/stalls与最终full；35的普通stalls仍独立保留，不混算源。
下表其余均已终态，38717126/38759858和38808023同treebe7b5abef3cc3c1ef81744075f4436efdf668fd0、
base82e0d3b5 dirty、controlc68b644；先查询，不重复调度。observer v1只改实验客户端/诊断，选路策略未改。

| Run | 任务 | snapshot |
| --- | --- | --- |
| 42538717126 | SUCCESS 1/1，原f4s诊断；observer-hop-diagnostics-42538717126已下载 | ce0af01606b4368ff6c3a190d59afbb14a58ecba |
| 42538759858 | FAILURE 1pass/1fail，原f4s/p1s诊断；observer-full-diagnostics-42538759858已下载 | 5bd8418703baccc5a71979a984e19b861447c6f8 |
| 42538808023 | SUCCESS，根格式与66核心/1ignored；observer-client-unit-42538808023 | e62f6126eb1c9dbb2e17a3c43a33b71dedc64fa3 |
| 42539256792 | SUCCESS 436Noq/33路径/9选家，新增Backup独立ACK用例；treec0d5c1b516b5c6272f17f83e9bb6bbe597813bcd、baseff5b4d36 dirty，controlc68b644 | 09de7c0a3f15e534ccd06b298702290bf3ffcb19 |
| 42539860644 | FAILURE 436Noq绿/33路径pass1fail，仅新增首次恢复quality回归红；tree06a93054b7a08919537f8ad8dd7e5a5907d59fc6、base1a084e79 dirty，controlc68b644 | 48ff937b1c7521fa2db4f71f1078bca0001223a7 |

| Run | 任务与准确源 | 最后观察 |
| --- | --- | --- |
| 42536677230 | netlab-diagnostic；basedf5b0864 dirty；tree1113605daf5597c2663d5e6468c45dade34031c9；snapshot72b0f7e13c0cb7142db033434cbd8182a1ba610a；controlc3b117d | FAILURE 0pass/2fail，714.13秒；已下载decision-diagnostics-42536677230。f4s准备Busy未测；p1s移动2/0，零打洞/退役，最慢1.1秒；日志模式不作SLO |

诊断代码5f69db3f：CLI可选net-diagnostics加显式环境开关才安装subscriber，使用已锁tracing-subscriber0.3.23。
记录ACK raw/delay/corrected、采样包时间、迁移初始/Go的Reading与Silence、恢复hold起止。
测量后保存各节点原serve.log，默认构建不开日志。**日志模式不作为SLO成绩**，原600秒/20次300ms/忙CPU及f4s断腿/1500ms门不变。
可选CLI feature实际编译/日志非空已由该run确认；Noq/Iroh单元绿不替代它。

本run已确认可选CLI编译与subscriber有效，p1s两份原日志399888字节，分析摘要analysis-decisions.json。
**两次实际Go均failed=true**，不是普通质量/快分支。第一次当前路quality3.807ms、候选2.576ms，
仍按Silence离开；原路在Go后约238ms的下一测量tick才记answered，随后还等恢复hold。
不能用先前缓存quality反转直接归因普通收益门。实际copy级PTO/last_received仍缺，下一轮追加这些诊断，不先调阈值。
LabNode.with反复开Node争用元数据，现候选只改fixture为一次Arc观察客户端复用，并补静默copy/恢复输入日志；
没有修改生产状态保护或选路策略。observer client v1先冻结，未来A/B须同步lab.rs，不能把其负载变化算策略改善。

observer v1独立f4s现已实际启动/配对/测量成功（42538717126）：1/1、133.81秒、三份原trace696351字节。
Y回carried@8.7秒、X@9.7秒，无回退；末一腿Link heard标记14.9秒，比选路晚，饱和0不是真实零耗时。
X的Custom在monotonic324.7560有决定后应答，却quick=false等到325.7569才解除hold，随后Go；Y同路应答时quick=true立即恢复。
说明可达性仍被当前质量/类型策略门住；尚未证明它是所有f4红的唯一原因。新确定性回归42539860644要求首次新往返恢复可用性独立于quality，
重复故障仍保留原1秒/REHOLD防抖；仅先加测试，生产未改。原hold测试改为先标一次近期恢复，明确验证重复故障。

35已修并取得42540515220单元绿：首次决定后新往返恢复可用性，重复故障hold/旧包保护/闭路规则保留；
移除quick回调和两份临时向量，无额外timer/PING，copy-waking未改。提交前核策略/fixture与绿快照无diff。
NAT helper仅在失败后打印两端path诊断，原75秒PUNCH/10秒STUCK/Direct断言保持。

observer v1完整诊断42538759858：1pass/1fail、808.44秒；f4s Y@9.6秒回carried（腿heard8.7秒、差0.9），X@15.1秒，最慢8.6秒、无回退。
p1s原20次300ms/忙CPU，alpha5次移动、beta0，零退役/4轮打洞，最慢703.8ms，3619包521832字节；诊断非SLO。
实际5个Go中4个failed=true、1个普通质量回迁。4个故障Go时，同一路的另一副本last_received均晚于故障副本最老未确认包，
3次selected Direct且wake_allowed=false，1次selected Relay已允许wake仍在同tick立即Go。
因此只去掉Direct wake门不足：候选可能用决定前样本直接离开，Hold又不探测当前路。下一步先加决定后备用应答/当前故障副本探测回归取红，
再最小改Moving.leave与探测副本选择；不把quality/ACK校正阈值当作这组切换定因。

## 当前阶段判断

N0部分、N1仍红、N2–N4未完成、N5部分。完整范围和逐项缺口见[REQUIREMENTS](REQUIREMENTS.md)，不能以单元或p6绿完成整计划。

最新Watch生产代码519ad4aa，后移路Backup修复ff399658；5f69db3f只加可选诊断。
后续提交主要是文档/验收覆盖，恢复时仍核实际HEAD/branch/diff，不按本页覆盖工作区。

| 最新普通模式证据 | 结果与限制 |
| --- | --- |
| 42532849927，Watch动态三轮 | SUCCESS 27/27；p2 1.1秒/831.8ms/6.2秒，max50.38/52.60/75.08ms；p3 626.1/655.5/508.1ms，零错/退役；21相位绿，闲置phase0 18.42/19.53/19.20秒，20秒裕量仍小 |
| 42532770351，Watch p1s/p5 | FAILURE 1pass/1fail；p1s原600秒、20次300ms、忙CPU，alpha移动3次、beta1次，零打洞/退役，最慢671.8ms；p5 150/300ms原门绿。CPU稳定性仍红 |
| 42532877270，Watch转发三轮 | FAILURE 8/9；f4恢复0.8/0.0/0.3秒；f4i34.3/15.1/12.6秒；f4s0.0/**1513ms**/0.2秒，中轮超1500ms；均无回退。饱和0不代表真实零耗时 |
| 42531361022 vs 42531408799，相同准备v2 | ff候选2pass/2fail，baseline648为3pass/1fail；两者p1/p1j移动0/0，p1s均7/0，ff打洞更多，p5红而baseline绿。不能声称候选全面胜出；早于Watch修正 |
| 42531477192，ff转发三轮 | FAILURE 8/9；f4s中轮1.7秒超1.5秒；旧红保留 |

上述源和artifact目录见RUNS。不同源/VM/任务组合不计算“14降3”的因果改善比例。
旧15秒黑洞请求超时42527295249、旧full40为28pass/12fail的42521539154等历史失败继续保留，
不以最新三绿证明它们全部定因或最终源全矩阵达标。

N5覆盖补齐：新测试a_backup_probe_is_acked_when_the_responders_data_road_is_blackholed使用真实虚拟连接，
主数据路不可达时要求Backup PING的PATH_ACK走健康本路。42539256792已SUCCESS 436Noq/33路径/9选家；
没有改传输生产代码，没有在旧源取红的宣称，单向故障/额外ACK成本仍是缺口。

最新36诊断42542228149 **SUCCESS 2/2、812.64秒**，suspect-probe-diagnostics-42542228149已下载：
p1s原20次300ms/忙CPU，alpha/beta **0/0移动、零退役/打洞**，最慢600.2ms，2629包276760字节。
原trace中alpha44次confirmation probe、beta24次，已确认选到PTO3的故障copy且ping返回Ok，两个节点无Go。
f4s应答后0.3秒回carried，无回退，最慢9.9秒包含8秒断腿和人为超慢Relay。
日志模式不作普通SLO，不以一次0/0删除历史红；正在等上表三项日志关闭复验。
35普通42540582927已FAILURE 1/1、1119.21秒，availability-stalls-42540582927：p1s alpha2/beta2、零退役，
最慢763.8ms；p5 150/300ms零Relay切换/打洞，仍绿。不同源/模式/VM不计算改善比例。

## 已有修复与边界：不要重复造机制

- 全承载RoadOrder按实测速度比较；Direct没有硬优先。收益/确认统一，hop发现与清理共用门槛。
- Noq最新ACK样本计数含等值；选路样本使用与原平滑器相同的ACK-delay校正，原raw RTT/PTO/拥塞数学保留。
- Reading近2秒最多8个观测tick中位数，年龄来自采样ACK时间；初始估计不是ACK，读stats不刷新年龄。
  窗口缩小同步重算质量，空窗保留最后已知值；没有额外timer。ACK时间是协议接收路径时间，不是内核时间。
- 同一代表副本的quality/age/count共享缓存，读取不发包/不通知paths_stream；冷/活副本偏差与CPU污染仍待解。
- Moving快分支也需决定后两个新观测；Hold时复测当前路。故障leave仍独立，无应用请求重放。
- Watch普通慢质量需原2秒窗和决定后两个观测才打洞；探测预算不增。真实无应答1秒、RTT超2秒探索及30秒节流保留。
- Noq坏/明显慢Backup重排在飞可重传帧，健康同速不重排、MTU排除/原loss与CC/QUIC去重保留。
  既有Backup LossDetection也重检，覆盖先移路后坏，未新增timer/逐发送扫描。
- journal事件measurement是回调时缓存，**不是实际决策输入**；u128 JSON回读兼容已有journal28绿。
- 稳定性准备v2先冻结再实现：Relay设备100ms往返仅引导Direct，清除后原30秒settle及600秒测量，观察期不限制类型。

## 最新单元与补丁审计

- **42536711138 SUCCESS**：根格式、435Noq/33iroh；tree1113605d、snapshot71a5b250ed41979a7e8d66d01991b06254b813d3、controlc3b117d。
  `decision-diagnostics-unit-42536711138`已下载，5f69db3f全部vendor/CLI/脚本与快照无diff。
- **42537392595 SUCCESS**：根格式、435Noq/33iroh/**9项选家/QAD/网桥**；based5e926bf clean，
  treed7f8f38cdeaabefc398f5580a747c7e599209c49、snapshota256f6708e9ad3920c1d01561b785e6e1425d1bd、control322835b。
  `vendor-home-coverage-42537392595`已下载。不是iroh全部158项；旧33过滤器遗漏的9项现在才取得当前源实证。
- 产品9项Python、control准备1项与sh-n/bash-n/diff-check通过。本机不运行Rust格式器。
- [VENDOR-AUDIT](VENDOR-AUDIT.md)逐项盘点45条目标/覆盖/退出；Noq1/4/5/6专属回归、上游逐函数等价、原生代理/真机/能耗仍缺。
  固定发布源码及PR事实见[UPSTREAM](UPSTREAM.md)，不升级依赖、不按一个补丁退出删除整树。

## 接下来按顺序做

1. 42536677230已终态，下一轮先验证observer client v1与copy级日志；新run回执落盘后只查其ID，不重复旧run。
2. 对p1s每个Go/当前路关闭事件，关联实际Reading和ACK raw/delay/corrected；对f4s关联静默判定、决定后应答、hold解除和Go。
   现有缓存快照提示候选质量可能残留故障值，但尚未证明根因；不要凭快照调gain或加固定等待。
3. 由确定性行为回归先取红，再最小修复。保留速度优先、故障快通道、健康备用/探测预算和不重放应用操作。
4. 关闭日志后重跑相应普通模式三轮、稳定性与最终准确源完整矩阵，不能用诊断模式收N1。
5. N1稳定后继续N2–N5：总期限/取消/竞速/缓存/随机退避与0-RTT，混合负载公平/能耗，中继与登记故障域/容量方案，Mac/手机/多机及最终可信Quality。

N2七类预算/取消/写入行为已在ACCEPTANCE实现前冻结；静态发现锁/hop不受总by约束、普通首拨未传by、早数据回答交出后失去Aborts。
尚无N2代码或行为红。NAT helper只看selected Direct，需先核opened/有效样本，不能把d2/d4未选Direct直接当穿透失败或回填通过。
具体审计入口见[NEXT-DESIGN](NEXT-DESIGN.md)。

## 源码证据与恢复规范

现92个本地immutable `refs/khor/net-evidence/<run>`，清单`evidence/net/source-refs.json`。
重要快照不依赖七天artifact保留；`git rev-parse REF:source`须等于收据tree。
快照是parentless容器，读取用`git show REF:source/<path>`，不得直接覆盖工作区，也不push这些本地ref。
收据终态及时刷新pending；当前为36普通三项及Noq6覆盖单元。

恢复先核本目录、branch/HEAD/diff与已有run，保留未提交内容。两main及产品feature未合并发布。
外盘缺失时，从共同仓只读`git show feat/net:docs/net-optimization/STATE.md`与PLAN恢复已提交文档；不覆盖原外盘未提交内容。
治理工作若需接续，遵守中央code-quality STATE及外盘中断RESUME入口，不把本网络任务冒充治理任务。
