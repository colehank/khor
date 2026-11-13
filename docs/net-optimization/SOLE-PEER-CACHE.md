# 唯一peer relay的缓存物理合同

预先协议E/sole-peer-cache-physical-plan-v1.json，固定四case各3fresh、替代0，Debug只判行为。仅测试overlay；产品源117c39未改，tree14e9f060422b3cfbd28976364d781e80daedeef7，snapshot05325c56df1ab25d53878bcc16537187a9abff7a。

远端格式准备053241Z-3a264e87已0/51.258588秒，12原files及source/hash/identity/group空核，准确fmt patch应用本地；native编译053516Z-5aa15758已0/81.366352秒，16files及原native verifier的20first-party unit/同producer CLI与test核。CLI SHA f129a57c仍与之前闭合唯一middle产物相同；旧ELF只读保全。

runtime053846Z-4b1bebf2，control09683361468cadb52033c1e83d1147c352a68703，原1/365.502303秒；36files/source/hash/identity、原有owner核干净，无强制/遗留。全矩阵原红保持，不能写成12/12。

| 场景 | 实际通过 | 说明 |
| --- | --- | --- |
| sp2 | 3/3 | 不可用的第二腿提示（原JSON null，非删map字段） |
| sp3 | 3/3 | 过时Direct/1ms第二腿提示，实际第二腿Relay |
| sp4 | 3/3 | X对目标的历史Direct提示，初始无活XY |
| sp5 | 0/3 | 暂停/恢复前置见证不满足，未证明产品负向缓存根因 |

前三类均hint前后保持、初态cold/no_relay=false、目标独有marker在原5秒返回，并见实际唯一M的Direct/Relay两腿及XRelay/YUDP真实drop counter。用现有公开DeviceDoc/load_pile/set_road/flush写隔离roster，无生产test hook或适配transport。

原首业务微秒（按固定执行次序）：
- sp2: 520744, 519725, 528708
- sp3: 528011, 547317, 524143
- sp4: 513165, 509044, 509370

sp5所有三次middle_down=false，前两次first_leg_up=false，末次middle_resumed=false。原读失败、恢复超时及全部记录都保留；既不能替换分母，也不能据此将根因归给30秒MISS缓存。下一版前置须用实际M→Y暂停拒绝、X→M真实只读和恢复真实目标内容见证，不能只依赖UI Link.down_ms。set_machine_paused接受设备名；已核lib.rs定义，不能误称名字参数无效。

准确专属proof E/turing-20261113T053846Z-4b1bebf2/sole-cache-physical-verification-v1.json；ledger已闭。不重开前三类已闭固定批次；未来修正前置需新协议、新source、独立原始分母。整个NET-16的其他策略、缺字段、多middle、移动/失败与成本仍待；N0–N5保持。

## 新前置v2/sp6也已闭：尚未进入hop负向/恢复阶段

格式准备060655Z-eab274c7已0/45.943910s，准确patch/source/hash/identity/owner核；compile061039Z-5f6b2be8已0/77.773270s，16原files及原native verifier20unit/真实CLI与test/source/hash/owner核，实际CLI仍原f129a57c。T源tree5625255d023a1f3f82fb305a64aa9cc45e2b2a15、snapshot4e2b2c66839b8401d0ed2c1e249c6f53286415ea（staged），原sp5/sp2/3/4函数保留，新sp6独立。

runtime061644Z-077fb72d、control59e3774eb74c7311ff446df472d458441302c715，原1/100.191668秒；3固定case均在“actual M-to-Y policy refuses target content”前置断言退出。各真实M→Y和X→M正控制成功，紧接外部暂停API后的M→Y请求仍成功（68802/66116/65028us）。三次未进入X→Y负向或恢复测量，不能称产品负向缓存红，也不能当恢复通过。18原files/source/hash/identity/owner核、ledger闭；所有日志/3个原Rust失败保持，替代0，专属proof E/turing-20261113T061644Z-077fb72d/sole-cache-resume-physical-verification-v2.json。

静态核pause.rs：resident进程使用自己的POLLED缓存，外部observer setter更新其文件/自身缓存；link.rs的PAUSE_POLL=500ms。正常跨进程轮询窗口是当前时序解释，尚未直接捕获resident cache内部状态。下一版必须有界等待实际M请求明确被暂停策略拒绝，才开始X负向读；恢复时也须在**原同一总5秒内**等实际M恢复内容，再做一次新X读取。不是把业务期限加长，也不是重跑旧v2挑绿。E/sole-peer-cache-resume-v3-proposal.json只为新方案提议，尚无新Rust/source freeze/编译/运行。


## v3/sp7：前置成立后的产品恢复三红

T3 tree8ff93012943b0b6c6b598142559b74e682e58829 / snapshot2c66f25350ecc698cc91b4dc180717261a3e5d4b，只新增有界等待实际暂停拒绝，原5s恢复期限保持。格式063455Z-c208e81b 0/47.923686s、compile063838Z-cdb6f871 0/75.911366s；相同产品CLI f129a57c，20unit及原native proof核。

runtime064222Z-57c213de原1/114.475143s，固定3、替代0、前置3/3成立、恢复0/3。M恢复真实内容441358/337646/554797us；X总恢复5008873/5026542/5008990us后仍timeout。首腿五次读/活路、PID及hint不变，无新hop journal（仅暂停时一次tried1）。三case owner干净，18file/source/hash/identity/ledger闭，准确proof E/turing-20261113T064222Z-57c213de/sole-cache-resume-physical-verification-v3.json。证明产品恢复失败，具体失败缓存机制待隔离修复验证；旧sp5/6失效前置和旧sp2/3/4各3绿均保留，不重置分母。


## 隔离修复R已过原sp7固定三轮

详[LIVE-MIDDLE-RETRY](LIVE-MIDDLE-RETRY.md)：9081b94d / treecf9d172b，原T3 fixture逐字不变，仅hop.rs runtime改动。完整hop库92/92后，070832Z-6b87e77c原0/98.267049s，固定3恢复3/3，总1013894/784788/878995us；原5s/实际首腿第二腿/marker/hint/PID/drop/owner全核，ledger闭。原064222Z-57c213de有效3红保持。不是full NET16/手机/能耗/N3接受，canonical/I未应用。
