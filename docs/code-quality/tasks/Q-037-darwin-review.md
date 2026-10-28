# Q037 Darwin EPERM 独立调查

作者：agent_redaction；生产源码只读，无 Cargo，无主仓修改。root 第一候选正常下一 turn 失败记录见 `Q037/first-green-diagnostic/cargo.log:43`。检查时 Unix 候选 SHA-256 为 `5ee38e55906392945eccd4a2038b26170ca53a957e53c09d18633a209aab5808`；此后 root 可继续编辑，不将本报告当最终候选审查。

## 已证实机制

Apple XNU 的 group signal 遍历先排除 SZOMB，再按有权限的可处理成员累计 nfound；group 存在但没有合格成员时，POSIX 路径返回 EPERM。因此未 reap 的单一 zombie leader 会触发 EPERM，不代表本次存在活的权限不同进程。保留 leader 作为 PGID anchor 的方向仍正确，不能先 wait/reap 再 killpg。[Apple kern_sig.c 固定 f6217f8，killpg1](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/kern/kern_sig.c#L1675)

`proc_listpgrppids` 的原始返回值是 **PID 个数**，内部调用返回 bytes 的 proc_listpids 后除以 sizeof(int)。本实验直接记录其 raw int，不额外除数。proc_listpids kernel 在进程列表锁内枚举 allproc 和 zombproc；buffer 满可能截断，不额外报“还有成员”。wrapper 还可能把底层失败折为 0，因此 0 不能认作已空。源码缓存及 SHA 见证据目录。[Apple libproc.c](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/libsyscall/wrappers/libproc/libproc.c)，[Apple proc_info.c](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/kern/proc_info.c)

## 推荐最小修法（root 已采纳方向）

仅 `target_os="macos"`、killpg 返回 EPERM、仍拥有未 reap leader 时，先 WEXITED|WNOHANG|WNOWAIT 确认这一个 leader 确已退出，再以固定 **2 个 pid_t / 8 bytes** 缓冲调用 proc_listpgrppids。仅 raw_return==1 且 member[0]==leader 才把该 EPERM 当作无需再杀的已退出 group。0、负数、>=2、非 leader 的唯一 PID、waitid 未退出/出错，均保留原 EPERM。不先 reap，不改 PGID，不再向已经丢失归属的组发信号；确认后仍由原唯一 wait 负责 reap。

这是“只剩我们持有的死 leader”证明，内存固定、无需 proc_pidinfo 结构体/动态枚举。不会把 dead leader + active descendant 的权限拒绝吞掉。固定2槽取得1项可排除截断；若 group 有3人，原始返回2，明确拒绝放宽。

保守边界：同组多个 zombie 也返回 >=2，因此暂仍失败，不能宣称覆盖任意后代都已退出。若后续要接受它们，不应凭一次 PID 列表后逐个 pidinfo 即声称原子安全：枚举后、状态查询前某活成员可能先 fork 再退出，产生第一次列表之外的新活成员；PID 重用及 query 失败也不能视作 zombie。当前最小例外只依赖 held leader 不能再 fork 的终结事实；不防应用之外的同 session 主动 join 干预，与原 process-group 归属模型边界一致。

Linux/BSD 保留原有 errno 处理；未经平台事实证据不复用 macOS 特例。iOS 不加 libproc 分支，保持 fail-closed。当前 macOS SDK 的 libproc.h 标了旧 iOS availability，iPhoneOS26.5 SDK 的 libproc.tbd/libSystem.tbd 确有相关 symbol，但无公开 libproc.h，且库头将这些接口注明为 private；这仅证明可见符号，不能据此宣称 iOS SDK/API/沙盒/运行行为已支持。本调查没有 iOS 设备运行证据。[Apple libproc.h](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/libsyscall/wrappers/libproc/libproc.h)

## proc_pidinfo 备用方案事实

若将来确需多 zombie 诊断，SHORTBSDINFO (flavor13) 可读取状态与 pgid，且 kernel 对其不要求同 uid，但仍可能遭 MAC policy 拒绝，失败必须保守。**arg=1 才找 zombie；arg=0 对已经退出的 leader 返回0/ESRCH**。本机 SDK shortinfo 实际64B（有末尾 rfu 字段），不能照旧网上60B定义；容量错误返回0/ENOMEM。不要以“ESRCH / 看不到”当作已无活成员。[Apple proc_info.c 固定源码](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/kern/proc_info.c#L2167)

## 真实隔离实验

环境 Darwin25.6.0 arm64，uid501。所有 signal 仅指向本实验自建 PGID；权限拒绝通过 sandbox deny signal 模拟 MAC 授权拒绝，不冒充已经实测不同 uid descendant。外层未受 sandbox 限制的原 owner 负责 kill 清理，worker waitpid 收自己的 leader。组 leader 在判定前一直 WNOWAIT，不 reap。

证据目录 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q037/darwin-eperm-review/`：

- `probe.py` + `probe-result.json`：3/3 真机隔离场景通过，64槽诊断所有成员状态。
- `two-slot.py` + `two-slot-result.json`：4/4 通过，直接调用固定8B原始 wrapper；zombie-only返回1/leader，接受；sandbox live leader返回1但 waitid 未退出，拒绝；dead leader+live member返回2，拒绝；dead leader+两个live成员（三人组）仅返回2，验证截断仍拒绝。
- zombie-only：killpg errno1；waitid exit0；arg0 shortinfo ESRCH，arg1 返回64B/status5；最终 waitpid0。
- 两个真实 live EPERM 对照：live 状态2可观察，dead leader 状态5，不接受为成功；外层 signal 后 leader 正常 reap。没有付费模型、用户凭据或项目 sidecar。
- 最初 ctypes 短结构少 rfu，60B query 返回 ENOMEM，是实验 setup 错误，保留 `probe-v1-setup.py`；补为 SDK64B 后才取得上述真实通过，未把前置失败算作业务结论。
- `apple-source.json` 固定官方源码 commit `f6217f891ac0bb64f3d375211650a4c1ff8ca1ea` 与文件 hash；官方源副本 pinned-*.c/h 可供离线核对。未声称该公开源码版本逐字等于当前运行 kernel；本机系统行为另由实验验证。

复跑：`python3 <evidence>/two-slot.py`。Python ctypes 调用系统 libproc，不需 Cargo、安装依赖或改变生产源码。
