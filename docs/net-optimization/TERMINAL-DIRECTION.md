# 终端超时方向诊断

此页记录独立诊断，产品修复与整体网络优化尚未完成。当前执行状态以 [STATE](STATE.md) 和实际 receipt 为准。

在相同配置下，上一批可用 Relay 组出现 2 次原 2 秒终端超时，而不可用组 3 场全部成功。之后选择器交错对照两组均成功，尚不能归因于 RoadOrder。原失败保持，见 [RELAY-AVAILABILITY](RELAY-AVAILABILITY.md)。

本轮保留真实 PTY，在原 shell 与带私有记录管道的 Python echo 间固定交错 6 场。网络、Relay 可用性、720 项意图、192 MiB 文件、2 秒 caller 和 600 秒 owned 上限不变。产品 CLI 与原失败版本同 SHA `a7e3565f15ed15d59f5215bceeb3900d153d053dd853846f623d86efbe6e4ac9`。原生 producer `145811Z-6f3704a7`、test SHA `499340a0655fc827e5e792af7c364ade14188560901a1a8aa332975c05462954`，构建和计时分离。

记录器通过私有非阻塞 FIFO 报告程序处理完整输入行、写出回复及测量后 summary。FIFO 写满不会阻塞 echo，会报告丢记录。原始原生日志、所有业务结果和 summary 后的事件均保留；只分析第一条 summary 前连续、无丢失的记录前缀。记录器分片、多行、满管道和独立 summary 的 4 项远端 helper 检查已通过。数据分析另有 6 项纯元数据检查。

时间戳采在 I/O 之后，是完成时刻的上界。只有在时钟 namespace 一致、epoch bracket ≤ 1 ms、无记录丢失/读取错误/观察溢出时，才作以下有限解释：

| 观察 | 可得结论 | 仍不能确定 |
| --- | --- | --- |
| reply_written 早于本地观测超时下界 | 程序在本地观测超时前已完成 stdout 写入 | PTY 读取、host 队列/写者、Tunnel/Noq、客户端读取和输出匹配哪一段停顿 |
| input_received 早于超时，但没有早期 reply_written | 程序已处理该完整输入行 | 回复实际完成时刻；晚记录本身不能证明晚写出 |
| 完整 summary 前缀没有该输入 | 程序截至 summary 尚未处理该完整行 | 包是否已到内核，以及 frame/ledger/PTY/程序调度中哪一段停顿 |
| 只有超时后的记录 | 时间戳上界不足以判断方向 | 不据此声称网络送达晚或程序回写晚 |

带记录的程序增加诊断开销；其结果不代替最终性能和能耗验收。本轮若没有重现失败，也不视为修复。运行前冻结协议 `E/terminal-direction-runtime-plan-v1.json`，SHA `a6893aaa724d3f121c3c9f316ccdfbaf7b1808b9e99a5ea37f25f5475c7330ef`；E 为 `/Volumes/disk0/Khor/evidence/net`。


固定6场已闭：151349Z-c16a9107，0/1125.157539s，原shell/capture各3/3、各2160ok，六个192MiB文件完整；三个capture均724条完整前缀、输入/回复各361、epoch bracket130ns，无丢记录。原/capture文件吞吐中位22.720202/22.759749Mbit/s仅作描述。未重现旧超时，无法归因；不据绿宣称修复。完整E/terminal-direction-comparison-verification-v1.json。

对原Relay可用组两次红的纯数据复核发现：两个2秒TTY超时期间，本地20ms probe各推进100次，最大tick lag1489/1711μs；同时Main目录请求约303–305ms成功。不能据schedule_lag大就认定CPU停顿，它还包含等待前面串行业务。此观察只把问题收敛到终端连接/PTY及相关链路，未确定具体环节；E/red-terminal-locality-verification-v1.json保留原log SHA和全部数据来源。


后续持续交互诊断154840Z-ed908e42已闭：两个同一PTY的10窗口会话，各2400ok，共4800ok，无bulk；capture2404条完整前缀、输入/回复各1201、epoch bracket120ns，无丢失。TTY scheduled p95原shell/capture为303409/303297μs，目录305919/305500μs。原/capture各只有一个长会话，量测条件也不同于原混合负载；不证明修复、等价性能或bulk唯一因果。完整E/terminal-persistence-comparison-verification-v1.json。
