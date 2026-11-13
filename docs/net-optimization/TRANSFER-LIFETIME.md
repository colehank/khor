# 文件传输的取消与资源归属

2026-11-12。这是N3的设计与验证边界；各源版本、实际run和结果以STATE为准。没有完成的合同不因本文存在而算通过。

## 已测范围

| 入口 | 实际验证 | 仍缺 |
| --- | --- | --- |
| Path拉取的caller future被丢弃 | 控制QUIC responder收到全部等待流的显式STOP_SENDING，同wire Sessions仍可回答 | 生产文件server和宿主进程资源 |
| road.stop / Job.cut的异步通知 | 停止前后订阅、多waiter已注册后暂停poll；Path首片和8片等待的真实红→绿 | 所有产品停止入口、最终提交竞争 |
| 远程DropStop | 实际receiver Node.handle/dispatch；签名roster准入；陌生设备/畸形token/错误token负控；正确与重复Stop | 配对协议、生产文件source、移动网络 |
| 会话文件停止与发送端cut | 四项首片/八片真实Offer等待红→绿；同fixture/期限；stop_sent统一通知，pull_one借用等待通知 | run_take完整清理、最终发布竞争、生产source与host-child |

Path的rolling测量与Notify修复是不同冻结源，不能互相冒充性能或功能验证。Offer目前仍使用自己的Fetch批量循环；Path的对照结果不能直接推广给它。

## 所有者与停止的含义

1. caller拥有拨号/请求future和窗口。可放弃的读取停止时，丢弃其等待流与已解码但尚未提交的片；不得另起一个无主任务继续拉取。对远端已接受的写入或持续job，不从读取取消推导它们可重放或被撤销。
2. Job拥有任务停止事实和异步通知。所有产品入口必须通过同一通知方法；只改atomic会让已注册的等待者继续睡眠。一个borrowed通知future可跨片复用，不为每片增加timer、任务或RPC。
3. Offer的pull_offer拥有pulling marker；pull_one保留精确partial供run_take决定删除、续传或记录cut。测试中drop后清理和取消后自行返回必须分别记录。低层返回不冒充run_take、消息fate和完整收尾已验。
4. server正在执行的blocking读拥有其工作额度直到实际读结束。取消async等待不能中断已开始的OS读；若把额度随future提前释放，会允许后续请求绕过限制堆积。应分别记录已排队、读中、已编码、发送中和已停止，验证其额度生存期。
5. 一个发送stream的STOP、连接错误、permit归还和fixture teardown是四种事实。只有实际transport的显式STOP算STOP；仍能在同一条wire上完成独立控制请求才支持“没有误伤连接”。

## 最终落盘竞争的拟定合同

目前“检查stopped后rename”之间仍有跨线程竞争。拟定规则是以最终发布文件的动作和停止请求共享同一个短提交边界：

- 取消先取得边界：不发布最终文件，返回取消，按原resume/cut规则处理partial。
- 发布先取得边界：文件已落地，不能再把该次发布报告成未落地；后到取消可停止尚未完成的后续文件。
- 发布失败：保留真实IO错误及原清理规则，不能将mutex获取或停止标记当成功回执。

边界不得跨网络await、完整复制、digest或阻塞子进程等待；否则取消本身会被长任务拖住。需先审计一个Job承载多个文件、Offer receipt/fate及cut语义，再实现；本文不等于该边界已存在。测试要控制两种顺序，并核实际目标文件、partial、receipt和最终Job状态。

## 下一轮验证顺序

先完成原24个匹配对照和当前四个Offer合同；修复仅限统一通知入口与borrowed网络等待，不借机变更Offer调度。准确源相关回归通过后，合并隔离候选并重新验证。随后补生产server blocking所有权与提交竞争，再扩到前台预览、公平/背压、1/3/5设备成本与实际移动/中继矩阵。全N0–N5和真机验收保持独立要求。
