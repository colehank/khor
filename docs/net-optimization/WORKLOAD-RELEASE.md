# 当前公开产品与网络候选的Release共存对照

2026-11-13。固定协议E/workload-release-comparison-plan-v2.json、全证E/workload-release-comparison-verification-v2.json。完整N0–N5仍未完成。

公开main c3b098（iroh1.2/Noq1.3）对集成117c39（iroh1.4.1/Noq1.4）。双方仅相同测试overlay，opt3/thin LTO/strip/codegen-units1。模型low为40ms RTT/8Mbit，high为300ms RTT/50Mbit分离转发队列。每model按A B B A A B各三fresh，12场景各720目录/真实PTY逻辑记录、完整16/192MiB文件；原2秒caller和120/240秒file界不变。

独立两个取消guard、lowTCP和highTCP参照/UDP容量已逐arm完成；原high冷TCP吞吐门红仍原Rust101/原controller1，完整68echo/双payload、约300ms RTT、几何及UDP48.30912Mbit符合预先声明的MODEL-DECISION分层准入。未称旧TCP通过。

| block | arm | low业务 | high业务 | high文件Mbit/s | run |
| --- | --- | --- | --- | --- | --- |
| 1 | baseline | 通过 | 通过 | 18.488 | turing-20261113T041330Z-0ae9a3ec |
| 2 | candidate | 通过 | 失败 | 21.577 | turing-20261113T041753Z-362039e0 |
| 3 | candidate | 通过 | 通过 | 26.786 | turing-20261113T042344Z-6df2674e |
| 4 | baseline | 通过 | 通过 | 19.330 | turing-20261113T042741Z-cae8392c |
| 5 | baseline | 通过 | 通过 | 19.989 | turing-20261113T043203Z-d993cc45 |
| 6 | candidate | 通过 | 失败 | 22.409 | turing-20261113T043619Z-e3822070 |

全部8640预定逻辑记录和12完整payload留存，替代/追加0，来源/原始Rust/人口/网络读回/所有owner清理全核。baseline6场景全部通过；candidate low三次通过，high只有1/3通过。high候选共1925ok、3timeout、232blocked；未把blocked零耗时当成功响应。

第一红041753Z-362039e0：bulk terminal index1和directory index2各超2秒，其后该phase118终端意图阻塞。第二红043619Z-e3822070：idle_after terminal index5超2秒、其后114意图阻塞。实际终端没有隐式重接；blocked记录不是额外网络请求。两次192MiB文件仍完整，文件统计独立保留，不能把交互红误写成文件失败。

low六个phase/operation单元的分段经验p95均过预设工程退化筛查；文件median7.0949→6.9350Mbit/s。high完整文件median19.3299→22.4087Mbit/s（约+15.9%），但交互退化：例如idle_before终端计划发起至完成p95约0.303→8.229秒，其他bulk/idle_after单元有原超时/阻塞，不能以成功子集分位接受。始发延迟、排队延迟与服务时间分开在JSON保留。

**此候选不通过共存接受或工程退化筛查，不能晋升最终版本。** 此是完整分支效应，不能单归因上游、BBR、path切换、prefetch或单个补丁。原首业务peer-relay收益仍有效于其原合同，不能抵销新红。

第一红客户端probe最大lag约1.9/2.4/3.3ms，无客户端整体2秒停顿的观测；resident未做独立lag采样，不能据此排除其调度/IO。第一红有后续selectedRelay日志，第二红无相同path切换，不能把移路当已证唯一原因；before/after聚合drop不提供逐时因果。下一步使用上游qlog独立诊断，日志开销不混入普通性能样本。

有限人口分位采用nearest-rank，n=360/cell及n=120/run同时给出；失败与缺样不补零，含失败单元不声明全请求p99；无统计独立/显著性、全WAN/phone/能耗结论。此cohort未含真实sync、preview、SSH/hop混合、纯空闲CPU/唤醒或完整N3。全部原数据与早前协议保留。
