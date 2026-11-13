# Release首业务：当前公开产品与网络集成候选

2026-11-13。协议E/sole-peer-release-comparison-plan-v1.json在执行前声明；与原Debug固定3次是不同问题，旧批次保持关闭。源码仅加同一测试夹具，baseline生产实现没有移入候选代码。完整目标与剩余项见[GOAL](GOAL.md)/[STATE](STATE.md)。

baseline为公开main c3b098，iroh1.2.0/Noq1.3.0；candidate为集成117c39，iroh1.4.1/Noq1.4.0。原Release配置opt3、thin LTO、strip、codegen-units1相同；分别在稳定owned source/target目录编译一次，原native_provenance核同一实际test producer及真正CLI。compile-only025353Z-e298b390与030746Z-850f325b分别0/630.716674秒、0/655.111898秒，产品runtime零。两编译收件/identity/profile/源/hash/group空均已闭证。

四份共享夹具字节及SHA逐份相同，原sp1业务函数未改；同一lab.rs观测client和owned取消、私有namespace、首目录5秒及生产hop3秒预算。X到仍配置的公共Relay被双向阻断，M是唯一可达邻居，Y的UDP全部阻断，仅可经真正Relay与M通信。每次独立新Lab，配对roster后不做X→Y预热，不等待长期no-relay宣告；六次请求前均报Idle/no_relay=false。

| 顺序 | 源 | 首目录结果 | 耗时/预算 | run |
| --- | --- | --- | --- | --- |
| 1 | 旧公开产品 | 超时，未返回 | 5001736μs / 5秒 | turing-20261113T032327Z-4c665996 |
| 2 | 网络候选 | 目标独有marker返回 | 345236μs / 5秒 | turing-20261113T033307Z-8dc292aa |
| 3 | 网络候选 | 目标独有marker返回 | 366587μs / 5秒 | turing-20261113T033352Z-7b79eb32 |
| 4 | 旧公开产品 | 超时，未返回 | 5007568μs / 5秒 | turing-20261113T033436Z-dc3369dd |
| 5 | 旧公开产品 | 超时，未返回 | 5003745μs / 5秒 | turing-20261113T033530Z-8a405fd4 |
| 6 | 网络候选 | 目标独有marker返回 | 366320μs / 5秒 | turing-20261113T033620Z-3d6f0e1b |

按原A B B A A B共6次，额外/替代0，原Rust基线每次0过/1失败/0忽略，候选每次1过/0失败/0忽略。候选实选Forwarded via M，Direct第一腿/Relay第二腿；旧源仍Idle。所有原nft规则/counter、实际start/finish、完整日志、来源及进程所有权留证，无强制清理或遗留。首个A在剩余串行批次前约10分钟已结束，该时间差及各load记录保留，不把它抹去。

E/sole-peer-release-comparison-verification-v1.json绑定全部source、proof SHA、版本与数据；E/sole-peer-release-comparison-results-v1.json为原串行结果。三旧请求只是五秒右删失，真实完成时间未知；不能当作五秒成功样本、计算有限倍数加速或推算p99。此结果证明整个候选分支在该受控首业务场景从0/3到3/3；不单归因某一补丁，不外推任意WAN/移动、缺失或过时hint的全部矩阵、能耗或完整N0–N5。
