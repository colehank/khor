# 高BDP resident初始化顺序对照

2026-11-11 22:34 UTC，任何v3实现/数据前预注册。完整N0–N5与MODEL-DECISION的分层准入保持。

v2预检2e4c439f三high完整2160请求/三份192MiB，但第二轮结束仍Relay，Direct门失败，原exit101/contract false永久保留。配置是在配对并首次读Direct后才施加：日志先4–13ms，随后300ms，出现smoothed与fresh quality明显不同。这里存在旧状态混入新模型的候选解释，不据此确认唯一根因或宣称产品选路错误；速度优先允许更好的Relay胜出，Direct是本受控比较的人口条件。

新revision **resident_mixed_workload_high_v3** 只将同一个high构造器提前到x/y两个resident启动和配对之前；端点/router已建立，实际队列参数、可用Relay与几何不变。LOW配置继续在原位置，避免改变旧回归人口。新manifest增加impairment_installed_before_residents=true，由静态实际调用顺序及初始journal交叉核，不单信自报值。v2旧数据不并入v3。

保留192MiB/原seed/256KiB/PULL_AHEAD8、400ms计划/120每类每phase/单在途/2秒caller/240秒整文件、原PTY10秒、20ms借用probe/512行/20000cap/Skip、≥30完整重叠、完整字节/关闭/结束Direct/实际八TC读回与下载路径字节门。不得移除可用Relay、添加直连硬优先或延长等待追绿。两w3→三low→三high，前置失败阻止high，阶段开始后完整执行该阶段所有轮次并保留首非零退出。

若v3通过，说明该固定profile的真实业务准入得到验证，不是证明初始化顺序是唯一原因、旧v2修好或完整N1/N3接受。需要分别核首次/后续path与fresh quality/年龄/样本数；如果仍选Relay或启动失败，保留全部失败继续查选路/比较条件，不能把门改成任意可用路径来启动原Direct比较。高priority A B B A A B仅在有效preflight/准确源码配对之后运行。
