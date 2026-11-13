# 当前普通 Release 与原主线基线的混合负载对照

本轮只测固定网络模型中目录、真实远程PTY和完整文件共存，不代全N3、手机能耗、同步/SSH/hop或任意WAN。原旧I1对照high超时和F3候选单臂high长尾均保留，不能由新的短绿倒推其根因已解决。

协议在新运行前冻结于E/current-mixed-comparison-plan-v1.json，SHA4b1f4b545a7ddfeac4d84ca58105c2eb755ebb44104dde960d5f56cb7af87361；控制fa5f6f6bf0edfb4f1174dea4784eaf9110e13145。原基线c3b098bd经原测试桥构建，treef6c239ae；当前DB7bacfb18/treea2f7a04c的产品代码逐字I3/F3，DB仅新增双端移动夹具。两臂11个workload/observer/calibration文件逐字相同；reduced/full entry均用PublicV4，实际几何另作新校准。完整Release、原features[]、无qlog/诊断生产覆盖、无运行编译；复用闭证ELF与CLI，运行前核源/原收据/选定五文件SHA并仅从原不可变存档恢复。

执行顺序固定：baseline五项preflight、candidate同五项，随后ABBAAB六块；每块low/high各一场，总12场、8640逻辑请求、12个完整文件。每场保留全部起止/失败/缺失/取消、计划到完成与实际服务时间，原2秒caller和120/240秒文件期限不改。每cell每臂预期360样本，有限样本nearest-rank p99不冒充总体尾部保证。原工程回归门仍为candidate p95<=baseline×1.10+20ms、文件完整率全通过且median goodput>=baseline×0.90。root唯一ledger writer，逐块采集/核验后才继续，无后台协调器/替代试次。

新baseline223755Z-939ba0ad：1/166.756497秒，17files/identity/owned全核；两个取消所有权控制、low TCP和独立UDP容量通过，high冷TCP吞吐原断言失败。candidate224138Z-9dcce531：1/172.626989秒，同17files/identity/owned全核；相同四绿一红。依据数据前原MODEL-DECISION分层准入，两臂high的68echo/双向payload/server/实际分离队列和270–420ms RTT及独立UDP40–60Mbit均满足，允许Node业务对照；不把原冷TCP红改绿。两份current-mixed-block-verification-v1.json保留完整原结果，root progress记录为2个已闭preflight。

第一业务块baseline224503Z-db31994a在途，后续五块尚未调度。下一步以E/source-refs.json和current-mixed-comparison-progress-v1.json核对实际运行，不能按此静态pending重派。最终结论必须等全部预定人口结束；未知/不安全清理停止，既有失败不替换。
