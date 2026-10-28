# Q037 Pi取消期限：夹具预审与最小监督设计

状态：设计/夹具预审，native_acceptance；**未运行Cargo、未取得目标红、未修改生产**。当前native-io fix/quality-agent-cancel-deadline基于3d10bbaf，root持有test-only改动。具体源指纹见Q037/review/source.json。普通ACP GUI 10s另批，先Pi，不混Q033上传Stop语义。

## 夹具

现测试通过真实Configure→Prompt→Stop home-handler；假账户/root/sidecar均隔离，127.0.0.1:1假key不连接模型。脚本read首条start后发布ready，收到真实abort写.abort，却不done继续读stdin。shell仅内建read/printf/case，没有子孙进程；worker故障时OS关闭stdin，脚本读EOF退出。外父25s保护、子worker15s取消断言和下一prompt可用是合理的红前置，未通过删除pipe/模拟receipt作弊。

最初仅查cancelled/running移除/下一prompt还不能证明旧进程死亡。root已采纳：脚本ready前写$$；成功前signal0要求ESRCH，活进程与僵尸均拒绝，之后才下一prompt。不要改用link::pid_running，它会waitpid并改变被观测对象。此fixture只覆盖会读stdin但不响应取消的runtime，不覆盖阻塞stdin或继承pipe的后代；不能把它一绿当整个期限完成。

## 当前真实阻塞点

khor_agent.rs::send持Running.input Mutex执行writeln+flush。Stop同步调用它；execute首次发送start也会阻塞。将10s判断只加在events.recv_timeout分支无效：上述write、done后的Child.wait、reader/drain join都能绕过那个循环。ChildGuard Drop只kill/wait直接child，没管理所属树；标准Child既不会因drop自动结束，wait又会阻塞。[Rust Child官方说明](https://doc.rust-lang.org/std/process/struct.Child.html)

## 建议的最小共同控制面

新增仅Pi使用的AgentProcess/Supervisor，隔离spawn/观察/终止/回收，不重构整个node进程管理。

1. 一个监督线程独占Child/Job所有权与终止许可；Running只持每run控制句柄和首次Stop时间。Stop同步入场先记录`Instant + 10s`（重复Stop不延长），通知监督器，再尽力入abort写队列。它不能等input锁/flush/Child锁。控制通知拥塞也不能丢期限：监督器定时读取同一短锁/原子状态。
2. 只让一个writer worker拥有stdin并串行写start/MCP/abort；调用方只入队。大start写塞住时，abort可能暂时写不出去，但监督器仍能独立在期限终止进程。不能每次Stop另起一个永久等待同一input锁的线程。
3. pipe reader继续用Q038的预算与固定stderr buffer。read error立即请求终止；done或EOF进入收尾预算，不能随后裸wait/join。用户Stop的10s从入场算，不能等done后重新获得10s。
4. 只有进程/所属资源确实终止、直接child被reap、writer/reader/drain完成后，才写最终cancelled/释放运行claim；然后下一prompt可进入。先drop event receiver，避免reader卡在满channel。join只在worker完成信号或is_finished后做；监督器不能为了join停掉自己的期限控制。
5. 监督器、输出worker和控制句柄都捕获原run对象；收尾撤销原终止许可，不能按root重新lookup当前run并杀后来prompt。失败要保持“正在收尾/收尾失败”的真实状态，不能伪造cancelled并丢失后台进程。

## Unix：保留leader身份直到组终止完成

用已有稳定CommandExt::process_group(0)在spawn时创建专属PGID，信号仅发该组。[Rust官方API](https://doc.rust-lang.org/std/os/unix/process/trait.CommandExt.html#tymethod.process_group)；[POSIX负PID信号语义](https://pubs.opengroup.org/onlinepubs/009604499/functions/kill.html)。这里是进程组归属，不宣称能回收主动setsid逃离组或远端进程；不得改成按进程名扫杀。

**监督期间禁止Child.try_wait/wait提前reap leader**。最小做法是waitid(P_PID, pid, WEXITED|WNOHANG|WNOWAIT)只观察退出；leader即便已退出，未reap的PID仍作为归属锚。正常完成、parent提前退出或10s到期，均先终止剩余组，再撤销组终止许可，最后由唯一owner Child.wait回收leader。之后任何旧watchdog/Drop都不得再按该数字PGID发信号。意外ECHILD表示归属锚丢失，应报错，不能对可能复用的PGID补杀。

本机Xcode SDK sys/wait.h直接声明WNOWAIT/WEXITED/waitid；[Apple XNU waitid_nocancel](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_exit.c)仅在未传WNOWAIT时reap，WNOHANG无事件可能不改siginfo，故每次须先清零。现源码无全局SIGCHLD/SA_NOCLDWAIT或waitpid(-1)；仅link::pid_running会对指定PID reaping，Pi监督路径不得调用它。后续新增全局reaper须遵守同一owner契约。

## Windows：Job对象，不是PID名单

所属树放入独立Job；应启用kill-on-close，未允许breakaway，保存Job句柄直到清理结束。10s时TerminateJobObject后确认完成，再释放run；[Microsoft Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)与[TerminateJobObject](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-terminatejobobject)是准确能力来源。

创建时就加入Job最稳：Windows10+可用CreateProcessW/STARTUPINFOEX的PROC_THREAD_ATTRIBUTE_JOB_LIST。[Microsoft创建时Job方案](https://devblogs.microsoft.com/oldnewthing/20230209-00/?p=107812)说明它避开suspended→assign之前崩溃留下孤儿的窗口；[属性API版本边界](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute)明确Windows10/Server2016起支持。不要默默改变项目最低Windows支持版本。

stable std的main_thread_handle/spawn_with_attributes官方仍nightly，不能假设当前rustc1.95能直接使用；见[ChildExt](https://doc.rust-lang.org/stable/std/os/windows/process/trait.ChildExt.html)。若不写自有Win32 spawn/stdio适配，可审计复用process-wrap的suspended-assignment backend，但仍需应用层监督与失败清理；它不是期限保证本身。

## process-wrap 10.0.1不能直接当作已解决

核对固定tag源码而非仅README：

- [std/process_group.rs](https://github.com/watchexec/process-wrap/blob/v10.0.1/src/std/process_group.rs)的try_wait会waitpid(WNOHANG)/inner.try_wait并缓存Exited，start_kill仍killpg数字值。先reap后等pipe仍有本题复用窗口；不能用其try_wait替代上述不回收观察。
- [std/job_object.rs](https://github.com/watchexec/process-wrap/blob/v10.0.1/src/std/job_object.rs)调用make_job_object(handle,false)，start_kill能杀Job，但wait本身阻塞等待；默认std wrapper不自动满足kill-on-close/独立期限。
- [windows.rs](https://github.com/watchexec/process-wrap/blob/v10.0.1/src/windows.rs)通过CREATE_SUSPENDED、分配Job、枚举并resume owned-process线程解决std缺主线程handle；不是创建时JOB_LIST。任何采用都需保留spawn失败kill/wait路径，并明确上述崩溃窗口。

建议：Unix先用现有libc/稳定process_group组成小而明确的owned process adapter；Windows要么补可靠Job后端，要么在审计/小补丁后复用crate。不要为引入通用库而省掉应用层时钟、stdin阻塞和owner测试；Windows尚未实跑时不能报跨平台完成。

## 真实验收次序

1. 当前“确收abort但不done”home-handler红→绿，要求实际旧PID已回收、下一prompt成功。
2. 单进程不读stdin，足够大start真实塞满pipe：Stop调用仍能入场，10s监督不受write锁影响；fixture另有晚于断言的自退出保险/精确owned PID清理，不能靠EOF假定它会退出。
3. done后不退出、parent退出而后代持stdout/stderr、取消时后代持管道：记录所属PID/PGID或Job，证明实际树终止且三个worker完成。失败清理只能针对隔离fixture。
4. 正常cooperative done/abort及时收口、不白等满10s；剩余组退役与deadline强制终止分别记证据。重复Stop不续期、Stop早于spawn/晚于新run不误杀、reader channel已满/输出噪声、取消后的下一prompt。既有MCP审批和Q038输出回归继续过。
5. Unix/macOS与Windows Job测试分别记录平台，Windows未实跑明确保留，GUI ACP普通Stop另步，不能以Pi替代。

本预审没有运行/安装任何fixture或进程组、没有加依赖，也没有把方案声明为已修复。
