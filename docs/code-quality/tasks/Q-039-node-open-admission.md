# Q039/N1：已改 Node 打开准入边界的集成收尾

状态：design working；owner completion_audit，root审查/集成/唯一运行槽。不是新独立模块或新增治理批次；当前只做有界设计，未实现/编译。基线wave1 integrate/code-quality-wave1@495d096d54ea083304031423ca052f87dd9bc75e，tracked clean。main8704不动，无merge/ship授权。

## 为什么属于已开始工作

用户要求Clean Code贯穿治理，且“开始了做完，新问题只记下一轮”。main已有GUI每API新开Node方式；本轮14931新增RootGate/try-lock准入，21b1延长同root fence覆盖recover/register_self/bootstrap。exact Busy错误来自该已改边界，不能把当前GUI回归一概称main既有问题。源链是native khor_agent→core khor_agent→open(root)→Node::open，devices也有blocking重开；未发现所谓Engine缓存或AppResident标记导致长期持gate。raw不含唯一gate/owner，故当前是本轮准入契约缺口/回归候选，须最小红证据后再修，不能按假设直接改锁。

## 实际影响

P5/Q043-pi-owned-run01账户读取/设备目录Busy、Connect disabled，实际未manage/提取/启动；P5/Q043-os-owned-run01正常Shell Machine空/Directory与Create disabled。Q033既有attachment可继续并不证明重新open的读可用。OS权限default.json/tauri.conf与main逐字同、fullscreen未成功回调是另一既有候选，留下一轮，不混入本片。

## 窄契约与禁止项

正常同进程同root普通Node打开应可协调，不因另一个正常open直接失败；保持有界等待、明确所有者和返回前释放，不跨await持构造permit。重入不能死锁，registry/排队有界且退役，取消/关闭不留permit，既有Stop/启动取消不退化。保持恢复、认证、完整性和真实trust/Doc/wire/Pair业务Busy/冲突语义。跨进程未知owner仍fail-closed，不承诺清除所有Busy。

禁止笼统将RootGate或全局metadata gate改成等待，禁止生产sleep重试、吞Busy、绕过恢复或新增Node缓存/大模块/公开配置。catalog纯查询特判不能代替普通设备/read请求恢复。不得追加全矩阵、新诊断服务或工具链治理。先读refactor-safely并图优先impact，核当前源/refs再最小设计。

## 交付与验证边界

第一步只交双盘P5/node-open-admission-design01设计：具体锁/准入契约、允许路径、已有target里能先红的最小行为测试、重入/限时/资源/跨进程/async blocking边界，以及现有可复用clean worktree。root审后才授权实现/单独分支，不新建worktree。所有Cargo/native由root唯一槽授予；原300/720/5GiB/jobs1/inc0/来源/owned/TMP/dual门保持，不改公共f7/fcb。

修正必须有原行为红→候选绿、非作者审查和精确集成版本；再刷新受影响GUI/core实际producer并继续被阻塞的P5原流程。已过115及Q033不重复充数；已有旧证据只按明确影响映射使用。尚未达成任何新pass，不以本卡代替实现或整轮完成。

## 静态锁序核对（2026-10-28）

root与非作者审查共同确认，不能只在Node::open外再套一层constructor锁：resident trust_tick/approve/revoke及Pair结算也使用RootGate，仍会使单个普通open失败。需要恢复的open先取root再取metadata，而Pair路径可以先metadata再try root；保留metadata与控制路径的fail-fast，避免把原拒绝改成循环等待。拟议普通open等待必须在本次root/metadata均未持有时进行，限时/限额、重入明确拒绝，其他进程的OS lease仍try并fail-closed。尚未选定或实现具体策略，也未证明GUI根因唯一。

Clean Code验收：统一一处表达准入规则与释放顺序，资源由RAII所有者释放；不在调用方各自加重试或重复状态。同步改异步不得产生脱离请求/关闭所有者的后台构造写入，裸spawn_blocking方案不接受。功能通过与结构质量分别提供证据。
