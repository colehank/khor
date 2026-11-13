# 高BDP共存qlog诊断（已闭）

本批只诊断，不是普通Release性能接受。原[Release对照](WORKLOAD-RELEASE.md)中的117c39候选仍因high两次失败而被拒绝。

预先固定三次fresh high病例、替代0；源码Q tree d372d293f69f4980bc0926797bf99f99ffa60f3a，snapshot 7e328dc5d000873f1cca809683fa41c0fa472e99，base117c39。仅启用上游optional qlog、隔离trace目录及壁钟锚点，没有改算法/重试/业务期限。

编译045808Z-c6558108已0/617.837255秒，原native verifier核Release opt3、实际qlog feature、20个first-party unit与同producer CLI/test；19原files及source/hash/identity/group空均核。先前引用前置失败045344Z-11d9f8d9原1/25.822100秒保留，零native编译/产品运行。

| run | 原退出码 / 秒 | 样本 | qlog字节 | EOF不完整末条 |
| --- | --- | --- | --- | --- |
| turing-20261113T051025Z-5907dd5c | 0 / 180.620308 | 720 ok | 326,868,694 | 6/8 |
| turing-20261113T052027Z-1edb3353 | 0 / 182.737763 | 720 ok | 333,501,091 | 6/8 |
| turing-20261113T052344Z-509f2cd6 | 0 / 184.584695 | 720 ok | 330,356,066 | 5/8 |

全部2160条原记录成功、三份完整192MiB文件。所有raw trace保留并绑定SHA/字节；部分连接退出时最后一条JSON-SEQ记录截断，明确保留原尾字节/hash，不能以此证明后续事件不存在。700MiB采集保护均未触发，原collector512MiB压缩/1GiB展开与2秒caller/240秒file/600秒owned门未放宽。

结论：三次未复现原超时，**对原故障未定因**，停止追加相同诊断。qlog有运行开销且是不同instrumented产物，三绿不能抵销原unlogged两红，不能认定修复或完整N3通过。接续按明确的新问题做静态定位或受控消融，不靠重复挑绿。

E/workload-qlog-diagnostic-summary-v1.json绑定三个专属proof；E/workload-qlog-diagnostic-progress-v1.json/results-v1.json的coordinator仅负责后两次，故其runtime_executions=2不是全批只有2。exec68965/PID40415已终态0，ledger无pending，root恢复唯一writer。

上游元数据于2026-11-13T05:25:07Z核：iroh1.4.1、noq/noq-proto1.4.0仍最新稳定；见E/upstream-latest-recheck-20261113T0526-v1.json。GitHub Noq latest UI旧tag不可代registry latest。

附加定时相关（不是原超时定因）：E/workload-qlog-tty-delivery-correlation-v1.json对case2同CID/stream0/方向/offset/长度匹配727个完整frame。该TTY bulk index38用时1459798μs；X发送输入63.686781s、Y接收63.836960s、Y发送回复63.837503s（Noq path1）、X接收65.146285s，均相对workload epoch，文件名壁钟有毫秒量化。回复两个qlog传输观察点相差1308.782ms；Y转发前仅约0.543ms。path1按双方tuple_assigned及iroh fd15:70a:510b:1::/64定义核为Relay；其后Y回复用path0直连。packet_sent不是NIC线速时间，尚不能细分Relay TCP队列/链路。数据来自通过的instrumented病例，不能当原失败根因或修复证据；但明确继续核双向选路、共享样本与旧路径在途恢复。n0-qlog0.2.0 streamer.rs55已核MicroSeconds实际as_micros/1000，事件数值单位毫秒。
