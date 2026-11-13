# 高BDP v2分层校准与业务比较决策

2026-11-11，首次高profile Node实现/计时前保存。完整N0–N5保持；本决策只定义新模型准入和业务fixture。

## 证据与判读

- 原h0组合模型、burst、长流与分离TCP原件全部保留，原270–420ms/40–60Mbit门没有通过；不改旧report或区间。
- 分离模型两fresh TCP：68 echo/双向96MiB内容/server完整，min RTT300381/300378us；
  冷TCP平均约35Mbit仍未达旧吞吐门，不能称TCP性能已解决。见80ceec22/kernel-queues-verification-v1.json。
- 相同分离几何独立UDP三fresh：六方向各145833/145833全部发送，实际输入70.000320Mbit，
  本地接收窗口各48.309120Mbit；24实际配置/路径计数通过，见c50ab6ef/kernel-capacity-verification-v2.json。
- [RFC6349](https://www.rfc-editor.org/rfc/rfc6349.html)的接近瓶颈带宽结论以TCP equilibrium为前提，
  并排除slow-start瞬态；[RFC9438](https://www.rfc-editor.org/rfc/rfc9438.html)描述CUBIC窗口行为。
  因而把冷TCP完整payload平均率当作IP容量的唯一准入条件不充分。这是基于实测的工程判读，
  **不是确认本次唯一根因、宣称TCP门通过，或将UDP速度当作应用SLO**。

新profile v2允许以独立链路容量、延迟几何/echo与实际队列配置三者校验后进入应用比较。
这是明确版本化的准入谓词变更，替代v1“冷TCP96MiB必须填满80%名义带宽才可测Node”的前置要求；
旧要求仍为未达标TCP参照记录，不重标绿。完整N3接受仍要求实际应用体验、完整文件、预览、
公平/背压/成本等工作包，不由新准入代替。

## v2固定模型与业务输入

| 项目 | 固定值 |
| --- | --- |
| 网络 | 转发router WAN各root TBF50Mbit/40ms/65536B；接收方向LAN各root netem150ms/limit1000；endpoint无额外整形 |
| 对源码的校准适用性 | 必须核共同队列构造器与c50ab6ef已编译源码字节相同；新Node每轮另读实际配置/流经路径，不只信manifest |
| revision / mode | resident_mixed_workload_high_v2 / buffered；原低profile v4参数与入口保持 |
| 文件 | **192MiB**、原seed、原256KiB切片和当前PULL_AHEAD=8；逐字节比较，整个文件240秒总界 |
| 交互 | 每phase目录/真实PTY各120次；400ms计划、各最多一个在途、每次2秒总界；原10秒PTY ready界 |
| phase / 记录器 | idle_before、bulk、idle_after；720完整人口、512行/phase、20ms借用probe、20000 cap/Skip，flush另记 |
| 传输重叠 | 每类至少30个完整请求处于实际文件传输内；同时报告全bulk phase/开始在传输中/完整在传输中三个人口 |
| 前置 | 两个真实w3守卫；泛化fixture先三fresh原低profile w2回归，再三个high fresh功能/人口预检 |

文件从原提案96改192MiB在首次Node数据前决定：低profile终端约两倍基础RTT是已有观测，
按2×300ms作保守采样预算（不是协议定理），96MiB名义16.1秒不足保证30个完整TTY请求；
192MiB名义32.2秒提供更充足机会。**保留30样本门并增加负载**，不缩减样本/取消deadline以求绿。
400ms计划可能产生真实schedule lag，完整保留，不把它从体验分布剔除。

## 验证与后续比较

Node单向文件为y→x：x RX、y TX、puby WAN rate TX、pubx LAN delay TX必须有至少192MiB的实际byte增量；
反向控制/ACK路径完整记录但不伪要求它也传192MiB。两个endpoint和两个router各before/after共八读回，
root/接口/单位/queue/配置漂移严格核验，安装原命令/初始读回另留。各层drop不相加。

固定控制任务先守卫，再三low回归；任一前置失败保留完整分母并阻止high阶段，不替换失败轮。
首非零退出保留。high预检只证明可运行/人口/完整文件/读回，不选优先级胜者、不接受N3。
只有此预检通过、源码配对与校准适用性核实后，才按 **A B B A A B** 每block三fresh启动18场景priority0/-1比较；
两树仅priority const不同、prefetch固定8。低profile18旧结果不并入新模型。独立prefetch1/2/8、预览、
连续控制下bulk公平、共享/分连接、背压/取消、独立退出与1/3/5设备成本仍按完整计划进行。

Mac既有运行时证据仅在全Git差异证明零生产runtime变更时复用；新Linux fixture必须实际编译/执行。
无合并/发布/生产部署/候选手机安装授权的推定。新source/control/run和状态以STATE为准。

## 高profile优先级campaign的固定入口（首个policy数据前）

预检与策略数据分开：只有原netlab-workload-high回执终态、source-owned metadata与独立原始人口/配置/路径字节核验通过后，才可启动策略campaign。新固定task为netlab-workload-high-policy；每个block先两个真实w3守卫，再三个同wh2 high fresh，关闭生命周期诊断；不重复low人口混入策略分布。守卫失败阻止业务，一旦业务开始保留全部三轮及首个非零退出。新metadata单独输出workload-high-policy-contracts.json，原preflight/旧low validator不变。

首次提交前冻结完整A/B树与固定control，证明树之间仅priority const的0/-1一行不同，PULL_AHEAD=8、共同整形构造器与已验证预检逐字同；如果只有文档/metadata增加则记录全Git差异以复用已有Linuxfixture行为和Mac生产runtime证据。沿已声明A B B A A B共18轮，计时前落盘campaign；失败仍保留在其预定位置，任何改变另冻版本，不按结果挑轮。此入口已准备不等于预检已通过或campaign已启动。
