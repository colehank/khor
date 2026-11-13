# 相同配置下的Relay可用性隔离

2026-11-13，诊断已闭，完整N3仍未完成。

源为953fd88f5f39fa99bc726296e5a84559996ce5cc，现已提交为实验分支f6b9a756；runtime逐字I117，BBR不变。CLI SHA256为a7e3565f15ed15d59f5215bceeb3900d153d053dd853846f623d86efbe6e4ac9，**与原先出现超时的C producer040254Z-2e151f7f完全相同**。新夹具、Noq独立模型和Python构建证明工具的改动不构成产品修复。

原空Relay清单实验125829Z-17b3241d前置无效：chosen_relays会关闭空清单的only_mine并恢复内置Relay。这是已有兜底设计，保留原失败、一个已完成normal场和四个未执行场。新实验不修改该产品语义。

## 固定协议与实际前置

E为`/Volumes/disk0/Khor/evidence/net`。协议relay-availability-runtime-plan-v1.json，SHA256 d901305ab1f394b08c403d9478475955fef5aec10f465a9408a6064b51426983。

两端始终配置相同、非空的Lab Relay清单，实际endpoint.json须各只发布该URL。正常组实际TCP探测可连；故障组只停止并wait私有Lab Relay原child，实际探测必须ConnectionRefused。变化在resident启动前发生；两组均须先建立Direct，之后才计时。

固定available/unavailable/unavailable/available/available/unavailable，每场720个目录/真实PTY意图与192MiB完整文件；原2秒caller、240秒文件、600秒owned、Direct前后、完整字节/重叠/PTY关闭门保持。CPU准入idle≥80%、pressure avg10≤5、无同UID编译；计时期间编译0、替代场0。Main目录/文件与Tunnel终端原本就是独立连接。

## 结果

runtime132935Z-b055e94a完成1/1161.211608s，control78f4657adb796765bf3317a044d2829ba0296096。18份原artifact、source/control/实际producer、资源准入及全部owned清理闭证；六场前置有效，原计数4320全部保留。

| 条件 | 原native门 | ok / timeout / blocked | 完整文件 | 文件吞吐中位 |
| --- | --- | --- | --- | --- |
| Relay可用 | 1/3 | 2095 / 2 / 63 | 3/3 | 23.302977Mbit/s |
| Relay不可用 | 3/3 | 2160 / 0 / 0 | 3/3 | 23.427441Mbit/s |

可用组第4场在idle_after终端index82触发原2秒超时，后37意图blocked；第5场在idle_before终端index93触发原2秒超时，后26意图blocked，其开始时间已比原调度迟10.444035秒。blocked是失败后未发送的意图，不算网络超时；失败未被隐式重连掩盖。两场文件完整、PTY关闭且direct_after均通过。卡顿发生在文件传输前和后，单纯“大文件挤占带宽”不能解释全部现象。

bulk终端360个意图在两组都成功，scheduled-to-end p95为可用组5.874877秒、不可用组0.336874秒；bulk目录对应0.786266/0.506660秒。这是端到端排队体验，不能直接叫网络RTT。idle_before/after可用组有失败，其成功样本分位数只能作条件统计，不能给全部调用补零或宣称整体p95。预冻screen因可用组原业务失败不可接受，不能把“关闭中继更优”作为产品结论。

完整E/relay-availability-comparison-verification-v1.json；各run下relay-availability-block-verification-v1.json。原日志不可变，仅按预冻规则对每场一个独立、严格验证的readiness控制行建立分析视图，全部业务/失败/时刻/footer/capture/journal原样保留。

## 结论与下一步

该固定小样本支持继续调查**可用Relay与传输/选路交互**。它不证明某个具体Noq调度、ACK、拥塞或路径确认是根因；Relay不可用组仍有死Relay重试开销，也不是完全无Relay候选的端点。可用组peer journal记录了Relay路径及切换，但普通journal不能独立将每个TTY流与某个Noq copy精确绑定。

生产仍保留Relay和peer relay救通能力，速度优先方向不变。下一控制保持Relay可用、BBR、原业务与配置，只比较当前RoadOrder和iroh内建选择器，以区分自定义策略与更底层传输交互。内建默认把Relay列为backup，**不符合用户速度优先完整合同**，仅用于定因，不能作为最终方案。它也含当前vendor补丁，并非未修改的完整上游对照。若这项控制不能区分，再按实际失败捕获请求/连接/路径事实，避免周期性诊断开销改变普通Release行为。


## 后续选择器控制（已闭，未定因）

原定RoadOrder/内建/内建/RoadOrder/RoadOrder/内建六场，Relay均可用、BBR与全部业务门保持。各3/3原native门、2160ok，六文件完整、替代0。bulk目录scheduled p95 584911/409883us、终端441411/316267us，文件中位23.548850/23.542792Mbit/s。同期RoadOrder没有原2秒超时，因此预冻diagnostic screen为false；不把不同批次的失败挑来补同期red，也不归因或晋升默认Relay=backup的内建选择器。完整E/selector-comparison-verification-v1.json及原6块13files收据。下一步记录真实PTY两端的业务时刻，保留原程序控制和原期限，不能继续只凭路径RTT猜测输入、程序处理或返回链路。
