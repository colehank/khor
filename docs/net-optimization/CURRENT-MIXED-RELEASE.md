# 当前集成版混合负载回归

本页记录R/I2产品runtime的首个固定双profile普通Release回归，不代表旧偶发超时已修复或完整N3接受。

源I2 `3093f866`，测试树 `a5de5685` / tree `90a1d7813377d611ea93b3276a36d3876e28e09c`。沿用[MOBILITY-RELEASE](MOBILITY-RELEASE.md)同一原生CLI `7e8c52bc…`、完整netlab目标 `e4f2f52c…`；原始目录、真实PTY与完整文件场景不改，没有运行时编译。协议在发起前固定为low/high/high/low/low/high，共6场、每场720请求；替代0。

`171546Z-dfd9502c` 0/713.189051秒，18files、源码/control、原生产物来源、CPU准入及全部owned清理已核。**4320/4320请求成功，6个文件完整**。每场保持原2秒请求期限。low为40ms RTT、8Mbit/s、16MiB文件、100ms交互周期；high为300ms RTT、50Mbit/s、192MiB文件、400ms周期；两者队列40ms。

| 场次/profile | 文件有效吞吐Mbit/s | bulk目录scheduled p95 μs | bulk终端scheduled p95 μs |
| --- | --- | --- | --- |
| 1 low | 7.184 | 251325 | 78482 |
| 2 high | 25.452 | 939412 | 867320 |
| 3 high | 24.606 | 1673215 | 1064111 |
| 4 low | 7.220 | 181675 | 81911 |
| 5 low | 6.935 | 276977 | 80020 |
| 6 high | 22.509 | 705757 | 322414 |

表中延迟从计划发起时刻计至完成，包含等待前一个串行操作的时间；不是单独OS调度延迟。所有样本、原结果、传输重叠子集及service/scheduled分布均保留。没有同期baseline对照，不作相对提速声明；每组只有3场，不宣称p99或统计显著性。high仍有超过1秒的交互尾延迟，功能通过不等于体验目标达成。

早先I1/A上的原超时及blocked请求继续有效。本轮没有定位其根因，也不能仅凭未重现认定R的窄peer relay修复解决了交互问题。下一步是同源完整Quality/原生Lab及剩余真实业务、资源、换网与custom验收；不继续无变化重跑来选绿。

证据E=`/Volumes/disk0/Khor/evidence/net`：`current-r-mixed-summary-v1.json`；run下`current-r-mixed-block-verification-v1.json`与全部原始report。冻结协议`current-r-mixed-runtime-plan-v1.json`，SHA256 `8124e47150e4ce62a56d60d05c6635b182cef7bc7714165c98be24e82c0f209e`。
