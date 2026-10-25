# Q037 Windows 所属进程树适配

所有者：native_acceptance。2026-10-25；Windows leaf和独立harness已完成真实 `x86_64-pc-windows-msvc --all-targets` 交叉检查，0.74s、exit0、源指纹稳定。完整Cargo槽已释放。**没有 Windows 运行证据，不能据此把 Windows 取消期限标绿。**

已提交 `b51a7d9c1b0823587af66df2403e3974652218e9`（父3d10bbaf），仅5个新增文件，工作树clean。精确交付与源快照见`Q037/windows-cross-check/delivery.json`及`committed-source/`。root接手集成/非作者复审；本智能体不再更改该候选或启动Cargo。

## 范围与版本

- 工作树：`/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery`。
- 分支：`fix/quality-agent-process-windows`，基线 `3d10bbaff062b0ccb32b3b9d36d5432275812b20`。
- 仅新增 `crates/node/src/khor_agent_process_windows.rs` 和 `crates/node/tests/windows-process-harness/`；root 拥有 common supervisor、Unix、Input、Node 接入、workspace Cargo/锁。没有改主仓、用户 App 或用户会话。
- graph first：图谱仍是旧 Q035 分支/18 文件，不覆盖新适配层；因此按真实生产源与固定 windows-sys 源核对。

## 接口与归属

`spawn(Command) -> io::Result<(Process, Pipes)>` 使用 root 指定的 `super::Pipes`（三个 `Box<dyn Read/Write + Send>`）。`Process` 内只保留内核拥有的 leader handle 和匿名 Job handle，不保留用于终止的裸 PID。

- 创建匿名、非继承 Job，设置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`；`STARTUPINFOEXW` 同时携带 `PROC_THREAD_ATTRIBUTE_JOB_LIST` 与 `HANDLE_LIST`。Job 在首次执行之前纳管，不用先 suspended 后猜主线程。属性不支持或 Job 限制拒绝时失败返回，不降级到脱管启动。
- 三组本机 byte-mode 命名pipe；系统BCrypt生成128bit随机名字，FIRST_PIPE_INSTANCE拒绝预先占用，REJECT_REMOTE_CLIENTS拒绝远程连接。父server端非继承、FILE_FLAG_OVERLAPPED，child同步client端可继承；HANDLE_LIST仅列stdin读端、stdout/stderr写端。Job、leader/thread handle、取消event、父pipe端均不在继承名单。创建成功后立即拥有返回的两个进程handle，关闭主线程handle与父端持有的child pipe副本。
- `exited` 只查询保留的 leader handle，既不销毁 Job，也不释放进程身份。
- `terminate` 操作该 Job；`wait` 等 leader、保存其原始退出状态，并确认 Job 的 `ActiveProcesses == 0`。**leader 先退出、子孙仍持 pipe 时，common supervisor 必须先 terminate 后 wait**；不能以 leader 已退出推断 tree/pipes 结束。
- `cancel_io(&mut self)`独立SetEvent，不终止Job；每个Read/Write都以自己的manual-reset完成event与该run的取消event等待。取消先CancelIoEx，再GetOverlappedResult确认该OVERLAPPED操作结束，之后才释放OVERLAPPED与借入的用户buffer。取消read返回EOF，取消write返回BrokenPipe，避免write_all重试Interrupted死循环。即使其他进程还持client端，本地I/O也不以EOF为取消条件。不能只提出CancelSynchronousIo然后依靠“线程已经开始内核调用”的时序猜测。
- 普通leader退出后可先terminate剩余成员+wait，继续读取pipe缓冲；只有executor收完结果或cleanup请求后才cancel_io。Stop超时则terminate+wait+cancel_io立即中断本地I/O。两种路径由root common supervisor明确区分。
- `Drop`先cancel_io，再关闭自己的kill-on-close Job；随后Rust自动关闭leader handle。Windows无Unix zombie回收契约；Drop不做无限wait，也无延迟裸PID kill。显式wait才提供实际tree已清空的证明。

## 命令语义

当前 caller 已 `env_clear()`，再填 OS allowlist 和 PATH。适配层只用 `get_envs` 的显式 `Some` 项，不读取 GUI 父环境；已删除项不进入 block。名称校验、Unicode ordinal 忽略大小写排序、UTF-16 `name=value\0...\0`，空环境仍双 NUL。字符串通过 `encode_wide` 保留非配对 surrogate，禁止内嵌 NUL。

普通 `get_args()` 按 Microsoft CRT 规则逐参引号/反斜杠转义，包括空参数、引号前连续反斜杠、末尾反斜杠，命令行含终止符不得超过 32767 UTF-16 units。解析后的 executable 单独传 `lpApplicationName`，不会由空格猜错可执行文件。

相对/绝对含目录的 program 先变为父进程基准的 absolute path；裸 `node` 在**显式子环境 PATH**中查找 `node.exe`。普通 `node + host.ts + 参数`受支持，cwd 以独立 UTF-16 参数传入。相对 program 与 current_dir 的关联在 std 本来也有平台歧义；这里明确先解析 executable，再应用 child cwd。没有 GUI 父 PATH fallback，也没有默认 shell。`.cmd/.bat` 明确返回错误；需要解释器时 caller 必须显式设置 `cmd.exe` 及其参数。

stable `Command` 不能查询 `env_clear` 标志（新 getter 仍 nightly），也不能还原 `raw_arg`、creation_flags 等扩展的语义。因此本适配器是 Khor Pi 的窄契约，不是任意 std Command 的完整替代：caller 必须使用已清空环境和普通 args，stdio由适配器固定为pipe，其他Windows创建扩展不接受为契约。当前 sidecar 构造符合这个条件。

## 失败与兼容边界

所有失败在创建 child 前均由 RAII 关闭 Job/pipe/list；attribute init 成功后才构造会调用 Delete 的 guard，init 失败正常释放 backing allocation。CreateProcess 成功后不再进行 fallible OS setup，返回 handle 当场纳入 OwnedHandle。Job 不是继承句柄，parent crash/退出也会关闭最后一个 Job handle 并杀掉其成员。

JOB_LIST 要求 Windows 10 / Server 2016+。本轮不偷偷降低为旧 OS 的 race 方案，也不声称提供更旧 Windows 支持；项目支持范围由 root 核对。Job 不开 BREAKAWAY；普通 CreateProcess 子孙归属保持，但系统服务/WMI代启动不由此证明。

**全仓并发继承仍有边界**：微软要求HANDLE_LIST中的child pipe handles自身有inherit标志。在它们创建到关闭的短窗口，另一个不使用HANDLE_LIST的并发spawn仍可能继承这些端。Rust1.95自带私有CREATE_PROCESS_LOCK仅保护std自身；本raw adapter无法取得该锁。自己的spawn不会继承别轮句柄、Job权限也不会泄露；本地overlapped取消现在独立于这些外部端的EOF，但不能据此声称全仓Windows句柄继承都已隔离。后续应审查/统一Windows spawn白名单入口；runtime harness只证明本spawn的排除名单和本地取消，不能覆盖无协调的其他入口。

## 验证

窄 harness 是独立 workspace，只依赖精确 `windows-sys = 0.61.2`。生产最小 features：

`Win32_Foundation`, `Win32_Globalization`, `Win32_Security`, `Win32_Security_Cryptography`, `Win32_Storage_FileSystem`, `Win32_System_IO`, `Win32_System_JobObjects`, `Win32_System_Pipes`, `Win32_System_Threading`。

已执行（本机已安装x86_64-pc-windows-msvc std）：

```sh
CARGO_TARGET_DIR=/Volumes/disk0/Khor/caches/rust/khor-quality-windows-process-harness \
CARGO_BUILD_JOBS=2 cargo check --offline --locked \
  --manifest-path crates/node/tests/windows-process-harness/Cargo.toml \
  --target x86_64-pc-windows-msvc --all-targets -j 2
```

这只检查真实Windows API类型、production module与runtime harness完整代码，**不链接、不运行Windows，也不覆盖workspace构建**。harness在非Windows运行会非零拒绝，不能产生伪绿。Windows主机运行`cargo run --locked --manifest-path crates/node/tests/windows-process-harness/Cargo.toml`才会执行七组真实场景：

1. argv/非配对 UTF16/空参/引号/反斜杠、env清除/删除/cwd、stdin/stdout/stderr、原始退出码和重复wait；还尝试 SetEvent 一个不在 HANDLE_LIST 的父继承事件，要求原事件仍未触发。
2. 裸 `node` 从只给它的 PATH 查找，并收到 `host.ts` 与带空格参数。
3. 缺 program/PATH、batch、program/arg/env/cwd内嵌NUL拒绝。
4. leader已退出、descendant仍持stdout；先保留真实descendant进程handle，terminate+wait后要求该handle signalled和pipe EOF。
5. Drop仍活leader+descendant，保留的两进程handle均要求终止。
6. 8MiB stdin写入确实被不读输入的child阻塞，cancel_io后writer须返回错误并join，且检查此时child仍活；然后才终止tree。
7. child仍持有stdout/stderr端且不写，两个Read确实阻塞；cancel_io后两者须EOF并join，同时child必须仍活，证明没有依赖pipe EOF或child死亡。

harness父进程有30秒退出熔断；退出关闭非继承kill-on-close Job，使hang不能留下其所属child。PID只用于打开尚活fixture的句柄，此后终止断言针对保留handle，不靠回收后PID重查。Windows实际结果仍待运行。

真实check证据：`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q037/windows-cross-check/`。首次因已安装依赖source存在但archive缺失而offline退出101，未进入编译；从官方static.crates.io恢复windows-link0.2.1(6,133B)/windows-sys0.61.2(2,517,186B)，按harness lock SHA核验，见dependency-cache-receipt.json。`check-02`真实编译发现候选`command_line`泛型program缺显式`'a` lifetime，保留失败；修正后`check-03/receipt.json`及check.log为最终绿。PID/PGID7568，exit0、changed_during_run=[]；外盘9,982,013,440→9,983,610,880B。没有仍运行的本任务Cargo/native进程。

最终production SHA-256 `92f9c3f74cbaec48f85de96305eb4c5d9ee722c57208de79474e7e2b11eed175`；harness main `f23faa5d9c1e70017fe95d63e58f20e1fdcdb6a8d63b8a0a4b8c86e584db1fed`。唯一依赖新增仅harness自己的两枚lock条目；workspace Cargo/lock由root另行统一接入。

## Primary sources

- [UpdateProcThreadAttribute：JOB_LIST / HANDLE_LIST 的条件与属性值寿命](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute)
- [CreateProcessW：独立 application、UTF16、环境与 cwd](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw)
- [Job Objects：普通子孙继承、breakaway与kill-on-close](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)
- [子进程标准pipe与父端禁止继承](https://learn.microsoft.com/en-us/windows/win32/procthread/creating-a-child-process-with-redirected-input-and-output)
- [Microsoft CRT 参数解析规则](https://learn.microsoft.com/en-us/cpp/c-language/parsing-c-command-line-arguments?view=msvc-170)
- [环境block排序与格式](https://learn.microsoft.com/en-us/windows/win32/procthread/changing-environment-variables)
- [Rust 1.95 Windows source：Command环境/解析、私有spawn锁](https://github.com/rust-lang/rust/blob/1.95.0/library/std/src/sys/process/windows.rs)
- [stable Command文档：get_env_clear仍nightly](https://doc.rust-lang.org/std/process/struct.Command.html#method.get_env_clear)
- [Overlapped pipe I/O：manual-reset event与操作完成](https://learn.microsoft.com/en-us/windows/win32/ipc/synchronous-and-overlapped-input-and-output)
- [CancelIoEx：取消后不能提前释放OVERLAPPED](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-cancelioex)
- [GetOverlappedResult：确认完成后才复用buffer](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-getoverlappedresult)

资料用于核对接口，不代替目标机执行证据；实现不依赖 process-wrap 或 nightly Rust。
