# iroh 升级混合负载对照结果

2026-11-12，预先冻结的[协议](IROH-WORKLOAD-COMPARISON.md)六block已结束。它检查升级取舍，不接受完整N3或全局最优。

旧dc73837e（iroh1.2/Noq1.3）与新02807c79（iroh1.4.1/Noq1.4），均rolling8/priority0；crates/scripts差异只有两依赖Cargo.toml，37份fixture/metadata逐字一致。顺序old/new/new/old/old/new；每block两w3 guards、low/high各一个fresh。所有原期限/人口/内核profile门保持，计时期间本任务无并行Turing编译/测试；共享宿主负载已保留，不声称独占整机。

证据E/iroh14-workload-comparison-v1.json SHA256 **940dda764a0f7787bf2439194b9bed6802ee3dbeed3736d60fe709358328732a**。独立复核全部六block、准确冻结metadata、102个原report文件SHA与8640完整交互人口，全部ok；12个文件attempt均完整。没有删样本、补跑替代或对失败推导吞吐。原24场景及所有原红完全保留。

每格为三次fresh范围；终端一格是三份单轮120调用p95的范围，不是混合请求p95。

| profile | 旧file秒 | 新file秒 | 旧bulk终端service p95 ms | 新bulk终端service p95 ms |
| --- | --- | --- | --- | --- |
| low | 19.006–20.063 | 18.714–19.050 | 82.449–87.925 | 86.334–92.016 |
| high | 72.435–80.323 | 64.760–76.029 | 336.727–347.027 | 318.364–681.495 |

high192MiB file中位75.729→66.124秒，但三次对应位置中不是每次新版都更快；low16MiB中位19.222→18.798秒。low bulk终端单轮p95的中位84.194→90.816ms，目录115.843→125.784ms，存在交互代价，不能只以file选择最终方案。

## 长尾保留

新block3 **114310Z-d73d001a** high bulk终端service p95/max **681.495/1102.768ms**，scheduled-to-end p95 **1893.762ms**；另外两轮新版service p95约319ms。scheduled-to-end包含串行积压，不叫网络RTT。此轮borrowed20ms probe bulk最大lag21.857ms，不能由它排除resident/PTY/传输/调度问题或归为唯一上游原因。新block6 low的idle_before终端单轮p95另有246.149ms，也须保留定位。所有逻辑请求仍在原caller期限内成功，只算功能/人口完整，不据此接受极致体验。

## 当前结论与后续

这组三fresh样本有吞吐/交互取舍和明显方差，无显著性、p99、手机能耗、完整N3或全局胜出结论。升级还存在原四秒peer恢复门红（9301772/9349684us，15条warm请求选择不符）以及首次连接间歇红，见STATE/IROH-UPGRADE，不能合入canonical或发布。

先用独立的08ecab50被动诊断补点追踪pending确认取消/Rest与原采样，不改变这六轮冻结源；再定位终端长尾。真实agent/native/phone、混版互通、其他补丁退出与完整N0–N5保持开放。当前既有sync_mixed.rs可复用，但缺OLD_KHOR时早返回不能算验收，必须核对准确二进制和实际非skip执行标记。
