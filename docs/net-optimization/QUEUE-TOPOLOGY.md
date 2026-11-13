# 高BDP传播/速率队列分离对照

2026-11-11，实施与结果前冻结；属于完整N0–N5中的模型诊断，不接受最终设计或N3。
前置burst ABBA见[当前状态](STATE.md)：四轮完整但全部低于40–60Mbit，不能只靠增大burst接受原模型。

## 固定模型与顺序

A复用hb0：x两方向同接口root netem150ms/child TBF50Mbit、40ms、65536B请求burst。
B为hq0，revision `workload_kernel_tcp_queues_v1`，PublicV4拓扑及x/y设备不变：

| 方向 | 先经过的独立速率队列 | 后经过的独立传播队列 |
| --- | --- | --- |
| x→y | pubx的实际WAN接口：root TBF 10:，50Mbit、65536B burst、latency40ms | puby面向y的接口：root netem 1:，150ms、limit1000 |
| y→x | puby的实际WAN接口：同样root TBF | pubx面向x的接口：同样root netem |

全部impairment位于转发router，endpoint无额外qdisc；每个方向的速率与传播队列位于不同接口，
避免把150ms传播队列塞在本地TCP发送设备或同一TBF层次。WAN名称由Router接口与uplink IP匹配取得，
LAN侧由Device的Ingress构造器确定后实际读回；不硬编码动态veth编号。

[netem手册](https://man7.org/linux/man-pages/man8/tc-netem.8.html)提示TCP/TSQ受延迟放置影响、
组合qdisc有约束。这支持该可检验假设，**不证明当前失败的唯一原因**。B改变队列拓扑与位置，
不能声称token-only或单参数因果隔离；原A及旧h0的红保持原样。

同一源码/二进制，固定 **A B B A**，四个fresh namespace，guarded task
`netlab-workload-high-queues`，单线程/串行，失败继续收件但最终保留首个非零退出。
关闭ss与Noq诊断，原patchbay默认/LOW/HIGH/生产网络不改；全局资源/锁/UID/netns/总期限门保持。

## 人口、物理门与证明

- 两arm均原seed、68个64B echo（4预热单列）、upload/download各96MiB逐字节/server比较。
  原2秒连接/echo、240秒整个方向传输，不重置deadline，默认TCP算法与窗口。
- min RTT仍270–420ms、双向完整payload率仍40–60Mbit；保留全部失败，不择优替换或改门。
- A保留原四份x/gateway前后TC JSON/详细文本/interface计数及两份shaper change。
- B在x/y/pubx/puby各取before/after，共八份实际读回。两端无netem/TBF；每router仅一个root TBF
  与一个root netem，必须在不同接口、正确root/handle/速率/延迟/latency/limit。
  实际burst允许请求值下16B的旧工具量化误差；两端rate-install原命令与初始读回保留。
- 每router的rate与delay接口及两个endpoint必须有双向payload相符的实际byte增量，证明流量实际经过；
  配置/接口不明或前后漂移不接受观察。层级drop各自留证，不相加冒充独立损失。
- 四轮齐全仅接受diagnostic observations；任一transport/字节/配置/人口失败则观察无效。
  即使B两轮均达物理门，也须另做三fresh独立B校准后才接受新模型，随后才能接受高profile Node策略数据。
  原HIGH失败不改绿，不把B两轮当三轮，也不替代N1/N2/N4/N5、完整公平/预览/成本/手机验收。

新source/control、收据、前置失败、实际执行及原始结果只按STATE和外部证据记录；本协议本身不是完成证据。


## 实际四轮结果

准确source-v2 1f4d90dc7c31aaf7c315727cc66105ac6f1c8f9f / tree a0c19bc7c976f2b631b556b5a3001bcdabd76e5e / snapshot384dec5a7233307d778e116e337ab28b0a621b63；control dde989632732550725f163a1511b0e5ba7d7511b。turing-20261111T210442Z-80ceec22已329.214秒101，19files/identity/原group空。

| 固定顺序 | upload Mbit/s | download Mbit/s |
| --- | --- | --- |
| A combined | 35.368370 | 28.599077 |
| B separated | 35.399825 | 34.485373 |
| B separated | 35.406192 | 34.532369 |
| A combined | 35.383369 | 28.088189 |

全部272echo与八份96MiB字节/server完整。B各八读回及两router实际流量路径均通过：endpoint无整形，root TBF/根netem在ix和v7/v9不同接口；rate/delay/lat/burst/limit正确且前后不漂移。B两轮的rate drops为79/80与80/78，两个传播netem均0drop，完整分层计数保留、不相加。**四轮全部未达40–60，不接受模型或N3**。元数据观察完整性true不改原Rust101。完整kernel-queues-verification-v1.json、source-owned contract、identity与ledger已终态。

初版47ae仅格式布局红、零业务，原件保留；准确单文件空白patch应用1f4d90dc，修正版才有上述行为人口。分离并非足以解决缺口，不能据此确定CUBIC/速率队列/CPU唯一根因。

下一项先区分链路容量与TCP单流行为。[RFC6349](https://www.rfc-editor.org/rfc/rfc6349.html)的接近瓶颈带宽结论以equilibrium为前提，并明确不预测slow start瞬态；[RFC9438](https://www.rfc-editor.org/rfc/rfc9438.html)描述CUBIC拥塞窗口随时间/拥塞变化。这是增加独立paced UDP容量诊断的依据，不是放宽原TCP门或声明通过的理由。UDP输入、pacer有效性、接收端本地稳态窗、全部sequence/缺失/内容/byte/drop证明与总期限/重复数仍待预注册；未实现/提交，Node高profile仍未开始。
