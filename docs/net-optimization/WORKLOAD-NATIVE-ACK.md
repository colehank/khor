# 已闭：native ACK/选路诊断

固定协议E/workload-native-ack-diagnostic-plan-v2.json，三fresh、替代0。源f92abcff/9c5a4829；沿Q optional qlog编译feature但不传QLOGDIR、不采qlog；已有CLI net-diagnostics feature/NETLAB_DIAGNOSTICS=1采原ACK与选路。192MiB/720记录/三phase/2s caller/240s file/600s owned与原模型保持，非普通性能接受。

compile054642Z-d37db21c已0/300.285793s，原verifier20unit、18依赖reuse、实际CLI/test的net-diagnostics和khor_net/qlog features/opt3核；19files/source/hash/identity/owner闭。原compile.json沿用upstream_qlog标签保持并在专属audit注释，不修改原件。runtime controlab2ac080、三case所有source/actualELF/日志SHA/字节/identity/owner闭，300MiB cap均未触发；原collector界不放宽。

| run | 原退出/秒 | 记录 | 两份日志字节 | bulk TTY最大service μs |
| --- | --- | --- | --- | --- |
| turing-20261113T055445Z-f9df1c24 | 0/192.912287 | 720ok | 12,113,300 | 1664310 |
| turing-20261113T055802Z-ad8c39fd | 0/182.521238 | 720ok | 11,842,772 | 349488 |
| turing-20261113T060113Z-5af34dfa | 0/184.583014 | 720ok | 11,639,601 | 350663 |

共2160ok、三份完整file，所有原样本/错误总体留存。**未复现原2秒超时，未定原失败根因**，停止相同instrumented试验；原unlogged high1/3拒绝保持。日志和compiled feature会改变时序，不能盖过原两红。coordinator exec76657/PID64594已终态0、progress/results completed、ledger空，root恢复writer。

首轮通过但TTY bulk index7 service1664310us；其窗口有一个Direct路径copy2PTO、另一copy较新收到同tuple数据，触发peer-global silence/move decision，但move仍Hold。精确证据E/workload-native-ack-first-tail-observation-v1.json绑定原日志行号/hash、wall epoch和sample，不能说已经切Relay导致此尾。heard时间是收包证据，不自动代表post-judgment完整RTT。

静态发现read_paths仍以selected.is_ip限制旧copy wake，源remote_state.rs1620–1633；这是待检假设，不是已证根因。与另一qlog cohort的Relay发送到接收1.308秒尾分开理解。后续受控丢包或单一改动消融须同时保持健康300ms抖动/黑洞/恢复负向，不能先缩短期限、把blocked当成功或放弃速度优先。

完整N0–N5和N3仍未完成。
