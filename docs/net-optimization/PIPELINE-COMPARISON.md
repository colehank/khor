# 批量屏障与滚动补齐的独立对照

本协议在任何rolling计时数据前保存。priority两套18轮已完整结束；批量prefetch1/2/8矩阵正在执行，
其准确源与control保持冻结。滚动实现仍在独立experiment/net-pipeline-v1，不能混入该矩阵。

## 假设与匹配输入

原pull_into等整批join_all结束再补下一批；候选在按序提交一个前缀后补一个请求。
目标是减少批量屏障造成的空槽，同时控制交互长尾、乱序响应内存和取消所有权。
四个逻辑测试与跨窗口真实文件wire已在Mac执行通过；files库的Finder回收站超时两次复现，
整轮验证仍失败。此范围不证明Noq上的实际补齐时刻、服务端取消、吞吐或最终N3接受。

先完整复核批量prefetch矩阵，再在首个rolling数据前记录比较宽度与理由。
primary宽度沿用原8作为共同参考；若prefetch显示更小宽度存在值得保留的交互/完成时间取舍，
在首个rolling数据前确定对应宽度的独立补充对照，不能看rolling结果再选择性补样本。
priority固定0，两种实现完全相同；当前Mac候选仍为-1，不能直接拿来与prefetch源比较。

匹配baseline与candidate均包含相同私有Window模块、四个逻辑测试和同一份大于八片的files_wire载荷。
baseline保持原batch pull_into，私有helper不被生产代码调用；candidate只替换该调度循环及必需短读重启。
核完整树、原batch函数体与全部fixture，明确相对已测prefetch源新增的未调用helper和测试载荷。
不通过只看两行常量来宣称两个不同实验源等价。Noq/iroh、切片大小、priority、并发上限、文件内容、
控制入口、网络配置、caller期限、诊断开关和记录方式全部匹配。

## 首轮性能顺序与结果

固定A=batch、B=rolling，顺序 **A B B A A B**；每block先两个真实w3守卫，再low/high各一个fresh。
每profile每arm三场景，共12场景；low/high独立报告，不能将旧prefetch场景充作本轮baseline。
守卫失败不进入计时；业务开始后low失败仍执行high，保留首非零和失败原位置。
任一场景缺少完整逻辑人口时先调查，不自动补跑或将缺失当成功。

原low v4与cold high v3保持16/192MiB、120/240秒file caller、100/400ms计划、每类单个在途、
三phase每类各120、2秒operation caller、原PTY边界、20ms借用probe/Skip/cap和512行日志。
每场景720个逻辑请求、实际传输/失败attempt人口、全部结果和调度滞后均保存。
成功文件核完整字节和原路径/关闭门；超时只报告被原caller截断的attempt，不能生成verified payload rate。
记录实际配置和路径，不能切断Relay或强制Direct来制造候选胜出。

采用准确冻结source metadata复算、独立原始人口/单调时间戳、收据/原group为空和全部report文件hash复核。
报告逐场景服务/计划到完成分布、文件完成时间、全部失败、成本及对应位置差值；
三场景/arm只作探索，不宣称显著性、p99或全局最优。

## 行为、成本与最终接受

计时之外仍需真实文件合同：有序写入、首片cap、逐片版本/mtime、短片续读、断线续传、
远端变化后重启、拒绝/取消清理、landing提交与原本地复制/sponsor SCP。
慢头/慢尾必须观察真实RPC的已发/已完成/待提交数量；逻辑future被poll不等于网络请求已发。
取消须覆盖用户动作到client/remote读任务释放、外层host/child退出、晚到旧版本响应及资源上限；
本地drop计数不能替代这些端到端证明。

完整前台文件/预览、公平持续推进、共享/分连接、背压与1/3/5设备CPU/字节/唤醒/能耗仍是N3必需范围。
若rolling只改善文件速度但增加交互长尾、CPU或缓存，应记录取舍并继续设计，不能以吞吐一项宣布完成。
最终集成还需准确源码的Mac/Quality、全部相关回归、真实手机自动换网/peer relay与能耗验收。
Finder红项单独调查并保留，不在本协议中豁免；没有合并、发布或候选App安装授权。


## 完整prefetch后的宽度冻结（首个rolling数据前）

18场景/12960交互/153原件已完整复核，结果见[PREFETCH](PREFETCH.md)。
主比较宽度8保持；预先增加独立宽度2补充，原因是low文件完成接近8且目录延迟较低，high则原三次超时，
须验证滚动补齐是否改变取舍。每宽度仍各自A B B A A B、每block low/high各1，共12；先完整8再完整2，合计24。
不将原prefetch作为baseline、不混合宽度/profile/失败，也不在看rolling结果后选择补充宽度。
宽度1不增加滚动计时：单个在途没有本轮要隔离的批量空槽并发差异。
上述是实验输入选择，不是最终产品参数；准确源对、逻辑/wire/取消与全部N0–N5门保持。


## 2026-11-12 04:17 UTC：真实RPC合同闭合与首block

源、原件及四树完整审计在E/pipeline-source-quad-v1.json，调度在E/pipeline-campaign-v1.json。
宽8A0c4e4e35/B e5527f93，宽2A ea414bfc/B2438172c；priority0，宽度内仅pull_into循环和必要短读重启不同。
所有helper/四逻辑/三真实合同/长files_wire与vendor锁相同；A原batch函数逐字未改，B函数逐字来自57ff原型。
新增共同Python wrapper复用原low/high recorder的全部期限/人口/失败门，只准确标记本次pipeline来源和arm；原prefetch结果没有被充作baseline。
固定timing control d28538241f29a1608a7fed606da25d2fde57470f 保留原prefetch分支，新增分支除标记/wrapper外原调度逐字相同；六dispatcher和九wrapper合成准入通过，合成不算网络行为。

| 宽度/arm | 实际Turing run | 原结果 | 范围 |
| --- | --- | --- | --- |
| 8 / batch | turing-20261112T040425Z-36a4357d | 0 /108.097333s | 四逻辑+三真实合同 |
| 8 / rolling | turing-20261112T040708Z-436c6664 | 0 /90.080052s | 同上 |
| 2 / batch | turing-20261112T041034Z-0a61687c | 0 /90.094105s | 同上 |
| 2 / rolling | turing-20261112T041334Z-19528761 | 0 /94.184242s | 同上 |

每run十三原files/准确source metadata复算/客户端已解码回复与远端实际RPC人口/原hash/identity/原group空及ledger终态已核验，各pipeline-wire-verification-v1.json。
宽8慢尾未释放时，A已发9/解码8/提交1片，B已发11/解码8/提交8片，真实补齐不同；慢头两者已发9/解码8/提交1片且无第十个RPC。
取消在两宽两arm均为原pull future丢弃→全部等待流STOP_SENDING→同wire独立Sessions成功。这里的responder是受控端，不是生产server、Job stop或外部host-child；这些范围仍未闭。
两份完整十三片文件逐字一致且无part/note，亦不替代短读/版本变化/恢复/完整预览等其他合同。
宽8RPC源与最终计时源只差新增共同Python wrapper，crates/vendor/Cargo逐字一致。
首个未格式化合同源2f5366db的5f783fc8原1/零行为已保存；准确远端patch映射为b020a801，不把格式红算行为过。

首block **turing-20261112T041552Z-e46a2819** 已唯一启动（宽8A，PID1044381/child1044594，source0c4e4e35），尚无timed block已收件验证。
随后依原8→2、各ABBAAB、各block guard2→low1/high1，总24场景；失败留原位、缺人口不补跑。
Mac固定12 net-node **43153603327** 正运行准确e552源码；整体验收与旧Finder红项仍保持，当前没有候选手机接受或最终N3/N5完成。


## 2026-11-12：宽8主比较完成、取消链与宽2接续

宽8六block/12fresh/8640交互全ok，所有文件完整；E/pipeline-width8-comparison-v1.json冻结102原件hash、准确metadata与独立原始人口复算。来源、逐场景service/planned/schedule-lag与对应位置差值均保留，没有选winner或显著性/p99声明。
low文件batch18.737180–18.808332s、rolling18.791625–19.548505s；high文件batch82.563249–85.975258s、rolling61.708015–76.625635s。high对应位置文件三次rolling更快，但终端service p95两次更慢/一次更快，目录/计划时延也有取舍；不能以文件一项决定默认。
宽2首batch（block7，turing-20261112T050750Z-7f2df1ec）low19.614184s完整，high原240.001025s caller超时、未完整、verified rate为NULL；两profile各720交互全部ok。原101保留，不能把attempt当文件完成。block8 rolling turing-20261112T051657Z-96f322fb在途，继续原ABBAAB且不补样本。

独立取消树31720fa6真实重现并修复public road.stop不能唤醒挂起pull：同fixture/control，Mac43156779606两个红→43157963440两个绿；只变road/link（含通知unit），原文件/metadata gate不变。E/pull-job-stop-red-green-v1.json保存准确绑定。
新f3ccd2f8仅修观测total与最终短片metadata，生产字节等于31720；Mac43159900154六组29过且独立frozen RPC v2准入过，旧43158565837的原metadata失败不改绿。此修复树不混入冻结性能四树。
这些只闭public Job→caller与受控responder及相关unit/wire，生产server编码回复/阻塞读任务、远程DropStop RPC、host-child、最终commit等仍须验；全N0–N5与手机/能耗保持未完成。用户澄清没有进行Lab验收，不记人工接受。
