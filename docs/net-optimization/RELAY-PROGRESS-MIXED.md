# Relay修复后的原混合负载回归

准确F3 7a794274/tree173ce959、普通Release CLI a2a655f73f168dc10ed580e1a578c5db5c3056a6d5207aacceebc82aebba450d，runtime源逐字I3的Cargo/crates/vendor（除补丁说明）审计保持。此次Linux测试全树是F3，不冒充I3全树Quality或真机。

固定原low/high/high/low/low/high，原40ms/8Mbit/16MiB与300ms/50Mbit/192MiB、2秒caller、120/240秒文件界和全部profile/真实PTY/完整字节门保持。211514Z-c483a776 **0/721.053971秒**，18files、identity/native/CPU/owned及完整人口独立核验：**4320/4320请求成功、6文件完整**，替代0、运行编译0。

| 场/profile | 高profile文件Mbit/s | bulk目录scheduled p95 μs | bulk终端scheduled p95 μs |
| --- | --- | --- | --- |
| 1 low | — | 244129 | 81626 |
| 2 high | 23.430 | 530295 | 312281 |
| 3 high | 22.467 | 418557 | 302508 |
| 4 low | — | 328168 | 82919 |
| 5 low | — | 235725 | 79123 |
| 6 high | 22.914 | 1228888 | 904081 |

高profile第6场目录p95仍1.229秒、max1.519秒，终端max1.119秒。功能通过不关闭完整N3或用户跟手体验目标。该回归没有同期baseline，不从未重现推断旧I1/A两秒TTY超时的根因或已修复。所有完整样本和失败/blocked人口仍在各自原批次保留。

首个210632Z-119ea551 **1/7.739828秒**是root控制器漏--ignored：原生0passed/0failed/1ignored，业务0、六场全未执行。13files/identity/owned与原log完整保留，未改产品、fixture、断言或期限；修正版仅恢复旧原控制器本就使用的--ignored。不得把准备失败当产品红或原六场绿。

E/relay-progress-mixed-runtime-plan-v2.json、relay-progress-mixed-summary-v2.json及run/relay-progress-mixed-block-verification-v2.json绑定来源与完整数据。
