# 选路防抖职责：待验证设计提议

2026-11-12。此页记录已建立的隔离策略实验，**尚未采用或接受**；冻结物理门、已部署参数或PLAN完成定义保持。首连阶段被动诊断固定批次已闭，首连未定位保持，运行状态见STATE。

## 新证据

确认copy候选80的固定物理R2（125919Z-c38e8c1d）里，X在重接后+1.644–3.446s的真实request-open里持续报告carried质量104.231616ms、Relay约130.8–131.0ms；+1.880s已有两个近期代表样本。当前app switch_margin=max(5/10ms,currentRTT/4)，所需收益约32.7ms，而实测差约26.6ms，因此保持Relay。这是源码与原行的可核对计算，不是新的网络实验或唯一根因。

到+3.807s的请求才读到carried约91.42ms；其最终确认在+3.522s开始，+5.523s Go。Y另有+1.511s开始后重启到+5.762s的确认。该轮原四秒门红保持，不能靠单位合同修正宣告恢复已解决。证据E/iroh14-confirm-copy-r2-trace-v1.json（所有角色原行记录，分析内层请求时须明确筛x/y，M记录是另外的外层请求），SHA256 `71d9ad9367ed8474bedeceea0d1eb9237dfbad11e331a6f772fab364be06afdf`。

## 要回答的问题

app的相对25%收益门与vendor的连续2秒、双边决定后实测确认是否重复承担了防抖职责，使稳定且确实更快的路径长期没有被重新测量/选择？恢复窗口只有原有界预算，选择门没有通过后，确认探测也随之消失；不应靠不断增加后台探测掩盖这一边界。

待实验比较：保持绝对5/10ms噪声底、原Moving确认/双边新鲜性/故障快通道/原恢复probe预算，简化或撤掉重复的相对收益阈值。也可评估按真实测量不确定性决定收益，但这需要新观测字段，复杂度更高；尚不优先新增vendor接口。

早期全局提议会有意改变原pure choose helper里“60ms→48ms也保持”的旧相对门行为；必须显式记录为策略合同变化，不能偷偷改旧断言再称原负例全绿。旧源/原结果保留。5/10ms内的噪声、不具备决定后样本、当前故障副本与健康副本、同机CPU停顿、备用过期、0-RTT/取消/写入边界均须保持。

## 采用前需要的证据

独立源与预先冻结的旧/新对照；原四秒物理门完整人口；健康同速/小幅噪声/短峰值与共同调度停顿的原场景；有限probe成本、静默与无收益切换统计。若只是让一个正例过而提高抖动或闲置成本，不能接受。先收敛两个防抖层的职责，避免为每个样本再加一个补丁、计时器或预算。

混版、Mac可信Quality、真实手机换网和能耗依旧独立。即使本设计实验成功，也不替代全N0–N5或授权发布。

## 已冻结的独立实验准备

E/iroh14-single-hysteresis-plan-v1.json（SHA `dd135369ae663b8f3e798683001bc0e16309da2be745b85a71ca728742360af3`）在修改前记录源、显式合同变化与原负例/成本要求。新worktree khor-net-iroh-single-hysteresis-v1 / experiment/net-iroh-single-hysteresis-v1 从028建立，不包含80的失败修复或794的被动日志。

红准备源c4ff13cf仅在net carry/path加入两个104vs131ms行为合同，生产实现、旧相对门断言、vendor/锁/物理输入全部保持；准确tree1f174846444c23edb46633aef57a0e7b2254e317 / snapshot506701e7bd3544106c27c009fb25d3df73ba2df5。当前远端net-unit先验格式/编译与预期行为红，未出结果前不记通过或实际修复。

调用范围核对：improves还被Node hop::beats与path::selected_beats_carried使用。本实验保留API形状并一致采用同一底线；因此旧60→48、450→500、29→35的分类将有意改变。必须保留旧源/原结果，在候选里显式更新这些设计断言，不能声称新旧行为等价。更容易触发准备/更久保留carrier可能增加实际成本，预算未增加不构成能耗接受。

## 资格规则候选与首轮边界结果

c4红准备134302已终态101/53.853927s：新两合同红、原net-unit68过/原ignored1逐名一致，非编译/格式假红。1460候选仅改变switch_margin的绝对5/10ms资格底，API保留，其他生产体/vendor/锁不变，source-audit-v2机械核对；但其134620轮69过/1红/原ignored1，新增合同过而旧200→195边界断言仍按相对门，完整12命令未继续。原失败保留。

v2协议在继续前明确额外边界：原improves的<=比较符保持，因此5ms恰好可入资格、4ms不够；不是调整算法来通过该断言。3f705a2e只补充这两个边界断言，生产与1460相同，source-audit-v3。它也有意改变旧200→195分类，属于本提议的规则变化，不能列作旧行为等价。

当前source **3f705a2e07b05ad1e82eee72de6c06cdb12e2a98** / tree **fec60cfb3d627886c48658f3b06bec3ea293b896** / snapshot **0069b5e220fdf95952828c38491485e89470ddd6**；固定12门135145Z-554511fc在途，net-unit70过/原ignored1，其他门尚未终态。尚无物理/稳定性/成本接受，也未改canonical/已装App。

## 保留新建成本门，分离已测路的资格（当前候选）

3f的135145轮已101/228.604469s：net70过/ignored1，hop88过/1红，失败是原“6ms收益不应准备新middle”的成本负例，后续门没有执行。原结果留证。E/iroh14-single-hysteresis-plan-v3.json在继续前明确保留该负例：原25%/5ms门继续用于新建middle估计成本，只简化已存在、已测量路径的选择与保留；恢复原Node准备断言，不扩大新建候选范围。

当前source **67c320135b40fb2e34d3d9a37523af2d912304e9** / tree **4077911ceb2c45a8c2ce990a5fd7ca5b35401779** / snapshot **c0d1c50fda47e827e21a7d43fa34ed4adaaa4834**，新增跨层合同要求同一104/131案例在已测路可入确认、新建估计仍保留成本门。生产只改switch_margin和hop::beats；后者重现原准备成本公式，其他生产token/vendor/锁不动，source-audit-v4机械核对。没有新增恢复状态、计时器或probe预算。

完整12门140132Z-2d53d83b在途，预期原273命名人口+net2/hop1=276/ignored1；尚未终态。原physical-plan-v1绑定3f、未执行物理slot；新的physical-plan-v2在任何产品测量前冻结67的三fresh/原7输入/原四秒门与v2 auditor，失败全留、三轮停止。未出准确physical/健康噪声/静默成本前不采用。保留carrier的实际成本可能变化，不能由预算不变宣称能耗不变。

## 后续健康负对照的覆盖提醒（尚未执行）

原p1/p1j/p1s的RTT约2/20/2ms，其25%项通常不高于5/10ms绝对底；只重跑这些例子不足以覆盖本次改动真正改变的高RTT区间。因此若三轮原peer物理screen显示收益，除了保留原10分钟健康/jitter/共同停顿窗口，还应给旧/新同样加入一个约130ms、同均速带小幅jitter的负例，直接复用one_speed现有拓扑、持续时间、最多一次切换和零wire退役断言。不能缩短原窗口或把低RTT绿扩称高RTT稳定。

该新负例当前只是执行前设计提醒，未新增源码/运行或改变原门。旧/新必须各自同一fixture overlay、准确source/provenance与串行计时；新建中继成本门已恢复，已存在carrier保留的实际成本仍需静默/交互并行负载对照。

完整12命令门140132Z-2d53d83b已0/363.333291s、276过/原ignored1，原命名人口+新增3合同、原取消合同、21原hash/source/control/group空核验，proof/ledger闭。已启动准确同67源三fresh物理R1：141018Z-445b94fe/pid1979830。保留原10秒持续前后条件和四秒双向decision/stream门，当前尚未终态；未接受策略。

## 固定物理screen结论（2026-11-12 14:31 UTC）

准确67候选三轮已全部终态：141018 R1原warm/decision过，Go3403177/3646386us；141413 R2与142410 R3各124业务全ok但后续没有持续carried，原warm/decision均失败、双向无Go。身份、输入hash、原group空、proof与ledger全闭。结果E/iroh14-single-hysteresis-physical-results-v1.json SHA92e5b6e59c46691d8c845f9f961e8cda7a591292479366e1f20e1ae0cdffdfda，固定三轮停止、accepted=false，不晋升。

降低已测路资格门并保留原准备成本门，虽通过全部276应用合同，仍不足以保证物理恢复。先读取R2/R3的carrier退役、内层样本及外层路径事实；不直接追加同源采样挑绿，也不以此进入长稳接受。首连问题、完整N0–N5与Mac/混版/手机保持。physical-plan-v3只修v2数据审核把final carried成功当先决断言的问题，v3 auditor记录失败并照常运行同一原四秒解析器，未改产品或门。

## 同app策略、旧依赖诊断对照（已冻结，未出结果）

为缩小因果范围，old I dc738应用准确028→67三文件patch，得到16e943d8；与新67相比全部crates树只有net/node两Cargo清单不同，应用Rust/测试/物理/scripts全部逐字一致，旧vendor/锁保持iroh1.2/Noq1.3。source-audit-v1 SHA9286dc4460c8d9fdb4606214228af5c9e19334fea12faee0b2b1446718261877。compile-only143631在途；随后原control69三固定物理诊断，原七文件/四秒门/期限不动，全部结果保留、三轮停止，不增加新67采样。

E/iroh14-single-hysteresis-dependency-comparison-plan-v1.json SHA7122efcde90f4d44cbac5f3aebb8835d0c955eaf054882d5b40a827106a889f5，在源和执行前明确这是观察失败后选择的诊断对照；不能宣告唯一上游回归、失败率或旧栈最优。新67仍拒绝晋升。

## 旧依赖同策略诊断结论与下一步

旧16的三fixed全部终态：144049/144427/144738各124全ok，原四秒warm/decision均过，双向Go3423765–3730444us。三轮后停止。新67原固定三轮一过两红保持，result SHA608b6c78e26a269ace71a28b78597f9a1403dc7b480bbe072372d620af2145c7。观察新失败后选择、顺序不匹配和小样本，不能声称失败率或唯一上游因果；两个候选都不因此接受。

接下来bc3064af只改CLI opt-in日志filter，复用原生Noq路径/STREAM trace，无新vendor/state/timer/probe/策略；两fixed诊断capture后停止，日志开销不作为性能接受。原七文件/四秒parsers/期限保持，准确source与在途见STATE。先辨别路径容量、实际path-open/本地status、排队STREAM偏移/重传/接收推进；共享质量或写入flush不能独立证明交付。
