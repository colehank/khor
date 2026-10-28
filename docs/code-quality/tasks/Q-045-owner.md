# Q045 owner：本次 SSH command 的本地 I/O、deadline 与回收

状态：review／本地 SSH run 批次已提交、验证通过，等待 root 非作者审查/集成；远端租约等仍开放。所有者：fanout_owner。2026-10-26。

最终作者提交：`8a22cfdbd64c5250fc53e443bd2dcdb00b923ff8`（共享单次reaped receipt，2文件）→ `a1625c7fc475b28e14d23600ccd9e0d95b2add0d`（finite SSH runner/接线/真实回归，5文件）。当前工作树 clean，旧ACP分支未动。下方准备/首红阶段记录保留，当前交付状态优先。

## 现场与边界

- 代码树 `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`，从集成 `c719741ce17fc34d761134fa76cec3af9535747b` 创建 `fix/quality-ssh-lifecycle`。保留 `fix/quality-acp-lifecycle@24b6478f64c32f3c53e797e991becf39f0552b5a` 冻结。
- 已读中央 STATE／PLAN／Q-045；本卡不覆盖 root 的总账。当前唯一 Cargo/native 槽仍归 root，未启动编译、SSH、安装、登录或发布，未改生产源码。
- 按 explore-codebase 技能先 `get_minimal_context`、再查 join/satellite 子节点。CRG 建于 `aa6011fb`、与 HEAD 不同且两文件无覆盖；因此查当前实际源码。`.codegraph` 不存在，未创建索引。
- 初始 SHA256：join.rs `d6947bc420e6d515f093d3c06e19155b0874cbb1c5c3d1c907d94d83938b61fa`；satellite.rs `7882c5b6cd8026c17d0a11cd68cffddec4f748f1540f9a8554370459d33927e5`；khor_agent_process.rs `6c36ea029fa0b027342fd98043822512267571c35a120e7f81646bced76281af`；Unix 叶 `511e96cdf0821cf3e27687179c9e35d8f58cb39be4785ff1b77935d4a6738204`；Windows 叶 `9a84ec14364e14a88b592a113797d56bc695ed1bfe7f8b3a561f004e383658b7`。

## 源码确认与尚未运行的推论

`join.rs:308–342`：直接 spawn `Dest::command("ssh")`，319–323 同步 `write_all` 且丢弃错误，324 才记 Instant；326–335 等 child 结束，336 才 `wait_with_output` 读两路输出。故“不读 stdin 可卡在计时前”和“输出填满 pipe 可使本可完成任务被误判超时”是源码风险，尚无 Q045 运行真红。`try_wait` 出错直接 `?`，std Child Drop 不负责 kill/reap，也缺统一失败清理。

输入包括小脚本、配对 ticket 和 `push_asset` 的整个 gzip archive（791）。输入不能再复制一整份交给线程。输出中 LOOK/CHECK/receipt/relay 有精确解析；不能截断 stdout 后仍报告成功。stderr 的最后一行用于错误诊断。现有 join tests 8 项只覆盖 checksum、stage shell、table 等，未执行此 SSH run。

`join::start`（175–184）明确让 add 线程长于发起请求；`Run` 只有 marks/done，没有用户取消入口。本批内部 owner 的取消回归不等同于已有 GUI/remote add 取消功能，不能凭私有 cancel 测试宣称整个 add 可取消。

`satellite::Dest::command`（195）保留用户环境／road；road（233–270）保留配置、端口、identity、BatchMode／askpass 和显式 jump 的 ProxyCommand。当前没有覆盖 ControlMaster/ControlPersist/ForkAfterAuthentication。共享 Session 的 Unix process_group／Windows owned Job 会清理本次后代，不能不经归属分析直接套用。

## SSH 配置方向（root 已审定）

本次 run 与项目生成的显式 jump 使用 `ControlMaster=no`、`ControlPersist=no`、`ForkAfterAuthentication=no`。保留 ControlPath，允许连接用户既有 master；不发 `ssh -O exit`，不删除 control socket，不查杀用户 master。保留目标、端口、identity、askpass、配置和用户 ProxyCommand 功能。

本机 `/usr/share/man/man5/ssh_config.5`（SHA256 `e10eabcd2d6c3ef08e34ba5bb543ee474ab50460135b4028746a9d6b7e3dbaff`，661–753）明确 ControlMaster=no 仍尝试已有 ControlPath；ForkAfterAuthentication 段说明后台化并隐含 StdinNull。OpenSSH portable [V_10_0_P1 ssh.c](https://github.com/openssh/openssh-portable/blob/V_10_0_P1/ssh.c#L1766) 的 `control_persist_detach` fork 后 daemon，佐证不能把新建持久 master 当普通 run 后代；这只是 upstream 行为证据，未声称本机或 Windows 已执行此配置。

任意用户 ProxyCommand 自行 detach、其外部服务、远端命令及远端临时 stage 租约均不在本次确认范围。保留实际 SSH 配置功能不能转化为“能回收任意脱离所有权的配置脚本”。本机 SSH 已回收也不证明远端停止或 stage 已回收；Q041 的 uncertain-cleanup 诊断继续保留。

## 最小私有 runner 与共享 receipt 方向（root 已批准，必须先真红）

新 `join_process.rs` 私有入口建议 `fn run(command: Command, input: &[u8], wait: Duration) -> Result<Output, String>`；`Output` 保留 ExitStatus、stdout bytes、stderr tail 和截断标记，由 join wrapper 映射既有 tuple/诊断。内部可注入现有 `Arc<Control>` 做取消回归，不扩 Node public API。输入用 scoped writer 借用 `&[u8]`，stdout/stderr 两个独立 drain 同时启动；不以 child exit 作为启动 drain 条件，不用无界 channel 传 bytes。

预算提案：stdout 最多 1MiB，超限明确失败并退役本次 command；stderr 保留最后 256KiB，始终 drain，截断明确标记；每路固定 8KiB scratch。stdin 源 buffer 属调用方，runner 不再完整复制。stdout cap 是本 helper 的捕获预算，并非 SSH 协议上限。最终常量须在首红/绿前固定，正常脚本与 receipt 保持完整。

总截止 Instant 在 spawn 前捕获，写输入、两路输出与执行共享同一个 deadline，不重置。deadline／取消／输出超预算／线程启动失败／I/O 致命错误皆由一个 guard：request_shutdown → Session confirmed reap → cancel pipe I/O → join 全部已启动 worker → 返回；自然完成必须先确认退出并 drain 到 EOF，再结束 Session。非零退出保留有界诊断；成功退出但输入未写完不能误报成功。OS spawn/kill/reap 的系统调用本身不承诺实时硬时限，测试由独立 worker watchdog 兜底。

不能只在 spawn 时调用现有 finish：`Session` 的 supervisor（154–184）自然退出先 terminate/reap，随后等待 finish/expired 才 cancel_io；若提前设 finish，reap 后立即 cancel_io，Unix Pipe::read 会先看到取消而返回 EOF，可能丢仍在内核 pipe 的尾部。也不能不观察 exit，等全 EOF 就 shutdown：进程可先关三路 IO 再继续工作。

建议共享层仅增加：

```rust
// 私有 Node 内部接口；不影响现有 Session::wait/finish 调用方。
pub(super) fn poll_exit(&mut self) -> Option<std::io::Result<std::process::ExitStatus>>;
```

语义：`Session::spawn` 创建容量 1 的 receipt channel；supervisor 在 terminate + wait 确认本次 leader／现有平台归属范围已回收后发布一次 Ok(status)，但保持现有 IO 不取消，继续等 finish/expired。任何 supervisor 错误先发布 Err，再沿现有路径返回；channel 意外关闭必须变明确 Err，不让 caller 永远等 None。首次结果之前 pending poll 返回 None；首次消费结果返回 Some；后续 poll 恒为 None。首次结果前 sender 异常关闭产生一次 Some(Err)，之后同样 None。Session::wait 仍唯一 join 并保留终态错误，不让 poll 冒充 IO 完成。runner 收到 Ok 后继续 drain EOF；到期仍 shutdown、Session.wait、join，不能因先收 Ok 放弃 deadline。此方案不修改其他平台叶，只需要 `khor_agent_process.rs` 以及共享新窄测试；root 已批准接口，但真实旧 run 首红前不得实施。

SSH flags 的单一来源也需窄扩 satellite.rs：新增专用于 finite run 的 `finite_ssh_command` 构造（保留现有 command/road 全部调用方行为），同时给 outer 与项目显式 jump 注入三个 no；避免 join 复制 road/quoting/askpass 规则。root 已授权首红后编辑这一窄路径。

## 真红设计与验收矩阵（未执行）

用新 `join_process_tests.rs` + `fixtures/join-process.py`。首红直接覆盖实际 join::run：父测试启动本测试 binary 的专用 ignored worker，仅该 worker 的 PATH 前置 scratch 假 ssh adapter，调用当前 run；不改全局进程环境、不读写用户 HOME、不连接网络。fixture 在启动即登记自己的 PID／nonce 并设置自终止闹钟，父 watchdog 限时等待/清理自有 worker。正常完成后必须验证登记 child 已退出；失败分支同样清理，不留下一轮污染。修后仍通过同 wrapper 验证，而非只测新 helper。

| 场景 | 原实现预期真红／修后断言 |
| --- | --- |
| 子进程不读输入，输入 8MiB，wait 200ms；fixture 自有 3s 闹钟 | 原实现卡在 write_all 超过 1s；修后含输入阶段在 wait + 明确调度/回收余量内返回，PID 不存活 |
| stdout 与 stderr 各写超过 pipe 容量、但低于捕获预算后正常退出 | 原实现等 exit 不 drain，误超时；修后正常成功，stdout 完整与 stderr 尾部完整 |
| stdout 连续越过预算 | 固定预算内存、明确预算错误、kill/reap/join，不把截断 stdout 当完整成功 |
| stderr 持续大量输出至 deadline | 持续 drain、不堵 stdout／输入，tail 有界且显式截断，超时后无子进程／IO worker |
| 已写完成后立刻退出，最后 stdout/stderr 在 exit 前写入 | 完整保留尾部；覆盖新增 poll receipt 不等于 IO EOF 的边界 |
| 提前拒绝并关闭 stdin | 不被 BrokenPipe 淹没认证 stderr；非零 exit 保持失败；成功 status+未写完输入明确失败 |
| 关闭三路 IO 后仍存活 | 不凭 EOF 提前成功，也不提前 kill；持续等待真实 exit 或原 deadline |
| 私有 Control 在写阻塞／读阻塞时 shutdown | 退役只涉及本次 child，confirmed reap 后所有 worker join；不冒充 GUI cancel API |
| 已有独立 sentinel/master 替身与本次子进程 | timeout/正常完成只回收本次归属，sentinel 仍活；构造参数检查 outer+jump no flags、ControlPath／Port／identity／ProxyCommand 留存 |

第一轮只写/接入上述回归并固定源取真红，再最小生产修复；不提高阈值掩盖故障。需要编译时先向 root 领取唯一槽，fresh 双盘 >=5GiB、jobs2/共享 quality-target/自有 PGID 护栏，切树显式 mtime 第一方依赖并记录字节 SHA／实际重编。验证受影响 shared process、ACP/Pi 尾部兼容与 join 正常 lib；不把 Mac green 当 Windows runtime。

root 后续允许准备独立回归草案：已新增但尚未接入/编译 `crates/node/src/join_process_tests.rs` 与 `src/fixtures/join-process.py`。三条父测试覆盖 8MiB blocked-input（200ms deadline／1s断言／fixture3s闹钟）、各128KiB双路输出（低于提案预算）、正常8192B输入/诊断控制。仅专用 worker PATH 适配 ssh、调用实际旧 run，输入时限红不靠改写有利 helper。当前仍无 Q045 运行结果；后续接入 `join.rs` 时才执行正常控制后首红。其余矩阵待接口确定后补入，不能将当前三草案称全矩阵完成。

随后 root 授权接入：`join.rs` 只增加四行 cfg(all(test, unix)) path module，旧 run body 完整未变。新测试已 rustfmt 解析、fixture 已 Python ast.parse 检查（未执行 fixture）；使用默认 SIGALRM 的内核自终止，不依赖 Python handler 唤醒。parent 在 worker 返回后等待 fixture 的 ESRCH，避免 worker 提早 panic 留短暂孤儿；不凭已回收 worker 的旧 PID 杀其他进程。固定首红候选 SHA256：join.rs `6afb04c22d13a1895bc52971f153093f8baf86fc91b00bbbf492c87d7886d5a6`，join_process_tests.rs `f90ef554744d5fe9f13fcd317d02736718b93bd4cb1d65ced474ec3a62fabe7c`，fixture `e177685811bd18ffba94f805fad64a6a5afe6f7102bfcac5ec827a548db7f716`。共享 Session、satellite 与平台叶保持初始 SHA。尚未 Cargo／编译；等待 root 明确授唯一槽。

## 真实首红与候选实现检查点

已先在旧 run 固定源取得正常控制与双红；证据根 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q045`：

- `setup-control-01`：真实 Cargo 正常交互 1/1，53.82s 含实际本树 Node 编译；fixture PID65515 返回即 ESRCH，输入/诊断完整。
- `blocked-input-red-01`：同一 binary、旧 run；200ms deadline 实际 3.036s，耗时断言失败，fixture PID65875 已 ESRCH。这是输入计时真红，不是 setup failure。
- `dual-output-red-01`：同一 binary、旧 run；两路各128KiB的有限输出却 1.040s 返回超时，正常成功断言失败；fixture PID66019 已 ESRCH。
- 三轮均 source_stable，未触护栏；最低外盘 7,396,499,456B、内盘 10,991,988,736B。完整 command/runner PGID/11输入SHA/源码 snapshots/result/test.log 各在对应目录。原测试证据 hash 以 source-before.json 为准（打印PID证据后 test SHA 与准备阶段不同）；旧 run/shared/road 均保持基线行为。

真红后按批准方向实现：新 `join_process.rs` scoped writer 借用输入，两个并行 drain，三项一次性 bounded result；单deadline含输入与读取、统一guard确认reap再join；stdout1MiB预算错误、stderr256KiB尾部及显式marker。Session容量1单次receipt、有限SSH构造已落源码；实际 first candidate 尚在编译/运行，不能计绿。新增12父测试（1纯构造+11真子进程）与2个receipt测试覆盖预算、flood、提前拒绝、成功但输入不完整、EOF先于exit、私有cancel与独立sentinel。

平台叶无最终diff；一次 rustfmt 对Unix单行格式的自身改动已撤回，并复核原 `511e96cd…` 全文件SHA。后续 rustfmt 使用 skip_children，禁止把此格式变更混入交付。

## 固定交付与验证范围

第一候选将 ForkAfterAuthentication=no 直接传给系统 ssh。随后源审发现该配置键由 [OpenSSH8.7 官方发布说明](https://www.openssh.org/txt/release-8.7) 的 New features 段新增；root 明确这属于系统工具功能需求，不人为增加 ssh8.7 下限。最终有限命令选项顺序是 `IgnoreUnknown=ForkAfterAuthentication`、`ControlMaster=no`、`ControlPersist=no`、`ForkAfterAuthentication=no`，outer／项目显式jump共用同一列表生成。只忽略该未识别键；新ssh仍强制no，旧ssh本来没有此配置fork功能，本命令也不传-f。未增加探测、网络、旧Khor兼容或未知选项通配忽略。

| 固定证据目录 | 结果与版本范围 |
| --- | --- |
| runner-candidate-01 | 新runner12父测试过，1 worker由11父真实调用；31.74s含实际Node编译。输入阻塞226.6ms、有限双输出72.8ms，未当作最终IgnoreUnknown配置验收。 |
| process-compat-01 | shared14过（含新receipt2），1专用worker由父真实调用；11.15s。 |
| acp-compat-01 | owned ACP transport/bridge12过；1.78s。 |
| host-compat-01 | host25过，2专用worker由父真实调用；63.07s。保持Close5s、Gone→ESRCH、正常大frame>2s、重复Stop与原fairness等全部原断言。 |
| pi-compat-01 | Pi29过，5专用worker由父真实调用；35.26s。 |
| join-compat-01／satellite-compat-01／normal-lib-01 | 第一候选join20／satellite35／正常非test lib check过。 |
| runner-final-01 | 最终IgnoreUnknown顺序下全join20过（8既有+12新）；24.65s含本树Node实际重编，构造顺序及11真实子进程场景过。 |
| satellite-final-01 | 最终satellite35过；0.51s，旧road调用方与local scratch scripts保留。 |
| normal-lib-final-01 | 最终 `cargo check --locked -j2 -p khor-node --lib` 过；7.37s，本树Node实际check，不仅是--lib测试构建。 |

按影响共135个不同通过测试：14+12+25+29+20+35；第一候选/后续重复运行、专用worker函数不重复计数。最终选项只改变satellite与纯构造测试，common/runner/host/ACP/Pi生产字节与前述兼容组相同，保留各自运行的确切13输入指纹，未将旧候选冒充新选项执行。

全部运行 source_stable、未触资源/墙钟护栏；全批最低外盘6,865,506,304B、内盘10,991,988,736B。guard只处理自有PGID，源码未在编译时修改。唯一Cargo/native槽已明确交还 root，作者后续未启动构建。

完整 delivery `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q045/delivery.json` 包含两提交、全部13个输入SHA、7授权交付文件SHA、未变两平台叶SHA、命令/PGID/资源/source snapshots、最终binary SHA `2d6986d9e8e7f9ccc59c75a10dd2fbcac35441ab280b8319b27fe55763ec882c`。提交hook后已再次核字节完全等于 normal-lib-final-01 的 source-after.json；36条实际fixture receipt均记录run返回立即ESRCH，交付前重查36条仍全部ESRCH。

明确限制：未运行Windows或真实SSH/master；sentinel测试仅证明独立本地进程不被本次owner触碰，构造断言保留ControlPath不等于真实mux验收。任意用户proxy detach与远端执行/租约回收不在本批确认内；`join::start`没有新增GUI/remote-add用户取消接口。`push_asset`邻接gzip.output/fetch等其他操作未纳入本helper截止。stdout捕获1MiB是本run预算，stderr尾部256KiB可显式截断；不得称所有SSH输出协议有此上限。

## 顺手非作者 CI 审查

root 的 integration `crates/node/tests/fanout_harness.rs` 与 quality.yml 两文件源审未发现阻断：dependency-free `#[path]`、PowerShell rustc 状态检查、保存测试 LASTEXITCODE 后读日志／exit、always artifact 两日志路径均合理。依据 root 提供 Mac 同 harness 10/10 和 YAML parse；我们未启动构建，Windows 尚未运行。此结论不计入 Q045 验收。
