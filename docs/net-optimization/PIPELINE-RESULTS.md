# batch / rolling 匹配对照结果

2026-11-12。协议[PIPELINE-COMPARISON](PIPELINE-COMPARISON.md)预先冻结；完整24场景已结束。该探索没有完成整个N3，也不证明全局最优。

## 输入与完整性

priority固定0，先宽8再宽2，各ABBAAB；每block两项真实w3 guard，然后low/high各一个fresh。四个冻结源码同宽仅pull调度循环/必要短读处理不同，跨宽仅PULL常量；source_quad及准确tree见STATE。Mac同时工作不在Turing上制造构建竞争。

原始证据`/Volumes/disk0/Khor/evidence/net/pipeline-comparison-v1.json`，SHA256 **14f62be1fafaaac2ae57fdb61ee892d50a8f3860c94d08b47d302af966279c90**。独立重新验证12个block、准确冻结metadata、204个原report文件SHA、完整17280交互人口；24文件attempt中21完整、3达到原240秒期限。所有交互最终ok，失败文件仍失败，没有为超时推导有效吞吐。

## 文件结果

每格3个fresh；秒数是完整文件的范围，超时只记录attempt。

| profile | 宽度 | batch | rolling |
| --- | --- | --- | --- |
| low，16MiB | 8 | 3/3完整，18.737180–18.808332s | 3/3完整，18.791625–19.548505s |
| high，192MiB | 8 | 3/3完整，82.563249–85.975258s | 3/3完整，61.708015–76.625635s |
| low，16MiB | 2 | 3/3完整，19.571980–19.650718s | 3/3完整，18.482548–18.776145s |
| high，192MiB | 2 | 0/3完整，均240秒期限 | 3/3完整，170.928677–181.190526s |

同宽、同profile对应位置的比较是描述，不是逐请求因果或统计显著性。宽8 high的文件对应三次均rolling较快，但bulk终端service p95对应两次更慢、一次更快；目录与计划延迟也存在取舍。文件速度不能独自决定交互策略。

## 不能漏掉的终端长尾

block9（`turing-20261112T052628Z-e17ce649`，宽2 rolling，low）三阶段都出现终端尾部。原raw SHA **ed1fc1d070c45b4673b6cc9f0fbfa87922bb44710ee2e4ee519abf5f3b216530**，独立逐条复算见`E/pipeline-block9-terminal-tail-v1.json`。

| 阶段 | 终端service p95 / max | 终端scheduled-to-end p95 / max | 120次中service超过500ms |
| --- | --- | --- | --- |
| idle_before | 663.547 /1226.700ms | 8122.275 /8466.806ms | 12 |
| bulk | 775.951 /1508.990ms | 7078.141 /7237.756ms | 9 |
| idle_after | 827.850 /1562.095ms | 9167.501 /12192.905ms | 18 |

scheduled-to-end包含前序操作变慢后形成的串行积压；不称这些秒数为网络RTT。每个调用仍在原2秒caller期限内成功，不能把“功能绿”当交互体验达标。

同轮bulk目录service p95为100.972ms；idle_after目录为62.299ms。本地borrowed 20ms probe三个阶段的最大lag分别2.498/2.465/4.673ms，未cap。这些只说明该探针没有同步长停顿，不能排除resident、host、PTY或传输环节问题。长尾在bulk前后都存在，尚未证明由rolling、宽度2、带宽争用或整机CPU负载唯一导致。原block和所有人口保持，不能删掉后重算候选胜出。

## 当前决策与下一步

宽2不设为全局默认；它在high的完成代价较大，而且存在上述未解释尾部。保留宽8 rolling作为下一轮整体验证候选，canonical仍是原宽8 batch，隔离候选未发布。这个选择是缩小下一步比较范围，没有接受最终调度策略或“最优”结论。

先以原宽2 rolling准确冻结源码，独立预声明3个low诊断fresh；原w3 guards与所有期限/人口保持，开启现有lifecycle并保存两端resident日志，诊断结果不混回普通性能24样本。无论能否重现都保留三次，不能反复跑到想要的结果。若当前记录不能定位，增加最小分层观察后另立诊断合同。

同时补齐会话Offer取消遗漏；生产server/阻塞worker、最终发布竞争、前台预览、公平/背压、1/3/5设备成本、真实换网/peer relay和手机能耗仍按完整PLAN执行。


三次独立诊断已完成：turing-20261112T062130Z-c432d9c1（source2438172c、control8b8620ac），25原files/6resident日志、准确冻结metadata与2160交互人口已核验。三次终端service最大84.365/87.069/83.094ms，未重现原尾部；没有修复或唯一因果结论，普通24和block9均不改。完整接续见STATE。
