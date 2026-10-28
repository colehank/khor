# Q037：只读独立审查与writer/supervisor接入边界

审查者：store_owner。状态：设计/测试审查完成，等待root实现后另行非作者审查；本卡不是生产修复通过证据。

## 固定现场

读取中央STATE/PLAN、Q-037任务卡，并先查图谱。图谱当前指向main，未覆盖这些新增符号（semantic search无结果），随后读真实worktree。

工作树 `/Volumes/disk0/Khor/worktrees/current/khor-quality-native-io`，HEAD `3d10bbaff062b0ccb32b3b9d36d5432275812b20`。当时未提交只有khor_agent.rs的test-only模块入口与新khor_agent_cancel_tests.rs，没有生产期限改动。未修改源码、未启动Cargo、未运行真实用户服务。

SHA256：khor_agent.rs `6298b2c3090040443f069180c4b83b01d2b73dbe8f45a6220ea7087b7401d76e`；cancel_tests `f6bcf4a5449f2751df0c114ea631dc0c7149b2ee5177a3fc6929bc025bf6ac1e`；khor_agent_io.rs `e45c6a510c18503724951f26fc2d77a64bf837124bcead666264264e882b1387`；gui_host.rs `87d49780cb4866cecfc6256cbd1696f2fa5a789cb9f5693604e819347ce571a2`；acp/src/lib.rs `fb388f35767347783cf4b6beab9a1d4381efb16b336b1fa6593690873b283f4d`。

## 确认的生产调用边界

1. **Stop同步阻塞在stdin/锁，recv deadline不足。** khor_agent.rs:379–388先置stopped，再同步send；send:171–180持active.input锁完成writeln/flush。execute:544发布同一stdin，:556写start，:562写abort，:594/:609写MCP响应；收尾:665取同一锁关闭。child不读或start写满pipe时，Stop本身可能等写/等锁；executor也到不了recv_timeout中的新deadline。必须让Stop只记录取消/非阻塞唤醒，不触碰持锁的OS写。
2. **直接wait与两个join均无界。** execute:676 child.wait，:677 reader.join，:678 drain.join，ChildGuard::drop:88–89同样kill+wait。done事件只退出读循环，child仍可不退出；仅kill直接child无法关闭子孙继承的stdout/stderr，join仍可卡住。既有drop(events)在join前是正确的，需保留，否则reader可能堵在sync_channel(64).send。
3. **取消状态不能先于实际退出。** 当前结尾:693仅凭AtomicBool选择cancelled；改动必须让“本次进程已wait/try_wait确认回收、持有的writer/reader已退出”成为最终状态前置。kill返回成功不代表reap成功。强制清理失败/超时应为明确未确认的错误/中断，不能硬写cancelled并释放executor让后台继续。
4. **MCP不在独立可取消边界。** McpSession::message/poll同步进入mcp_request:861附近的mcp::call_with_parent；其mcp.rs:379/436/460等runtime.block_on可占住executor。只把事件recv和pipe做期限不能承诺整个executor取消有界。不得把有副作用的MCP线程detach后发布cancelled/释放claim；旧写可能在新Prompt后完成。需给实际工具future传播cancel/期限与提交语义，或明确另立缺口；supervisor仍应能独立杀本机sidecar。
5. **末尾子任务取消仍会挡住归档/claim。** execute:690–692同步cancel_children；delegation.rs:674–770按child links串行runtime.block_on远端list/get/cancel，其中list可分页。这是最佳努力的远端取消请求，不是远端停止确认。不要把它排在本机reap/管道收口的唯一监督线程上；也不能把本机进程停止误称所有已派任务停止。

## 最小可靠writer/supervisor契约建议

- **Run身份固定**：一个run一个owner，所有输入句柄、取消标志、退出收据关联此owner；Stop只查原run并原子设置一次最初取消时间（重复Stop不能推迟期限），随后非阻塞唤醒。Stop不能等待ChildStdin mutex或队列容量。
- **单输入writer**：唯一拥有ChildStdin；start/MCP/abort都排同一帧序列，不允许并发writeln。队列按项/字节有界，用try_send或可取消入队，不用无界队列替代OS阻塞。写完成收据需区分“已入队”和“完整JSON帧已写”；不能把排队当原send成功。
- **帧边界优先级**：abort可在完整帧边界优先，必须在start后；如果半个JSON帧已写，不能丢余下半帧再插abort（会造非法行）。Unix对stdin使用O_NONBLOCK+poll/有限等待检查取消/关闭，最多当前帧的剩余写在grace内继续，截止后直接终止本run，不靠补写abort救场。
- **独立supervisor**：独占Child与try_wait/reap，持有从spawn时建立的process_group(0)身份，取消deadline与writer/reader/MCP执行栈解耦。grace耗尽后只终止明确归属的group，随后确认直接child回收。进程组只覆盖未主动脱组的后代，不能宣传任意逃逸进程树都被清除。
- **I/O收口不能只等EOF**：stdout/stderr reader应能被取消唤醒/有限poll退出；即使后代继承FD或主动脱组，不能无限read/join。reader向有界channel发消息也需可取消，或监督方能确定drop对应receiver。线程JoinHandle::drop仅detach，不构成收口证据。
- **单调完成**：只由确认reaped的completion走最终save与claim释放；成功/失败/取消三条路径及panic/early-return复用同一监督清理。若无法确认，保留明确cleanup pending/错误归属，不把detached工作当完成；不要在所有路径最后再经过无界ChildGuard::drop.wait。
- **跨平台**：Unix group+非阻塞FD是平台模块，公共层只依赖submit/request_stop/confirmed_exit契约。Windows需要单独受控process/job与取消I/O实现、实跑证据；不能把直接Child.kill fallback说成等价进程树回收。本轮尚无该平台执行证据。

## 当前测试的有效范围与需要补的区分

现有fake sidecar真实读完start，标记ready、记录abort却不发done，能验证“取消已经送到但对端不合作”的期限和后续Prompt/claim恢复。token修正为合法16–64字符后才可能得到目标红；首次不合法token失败不计业务红。Worker隔离、假账户与独立store方向正确，fixture当前无后代且EOF退出，避免碰用户服务。

尚未覆盖/建议独立worker：

- child完全不读stdin，合法start或MCP帧超过pipe容量；Stop调用本身应迅速返回请求收据（不等整个grace），并最终reap与恢复下一Prompt。
- 重复Stop不重置最初deadline；Stop在spawn/输入writer安装前到达；初始start部分写时取消不得产生半帧后的abort伪成功。
- done后不退出、stdout EOF但child活着、直接child退出而同组子孙持stdout/stderr、事件channel满时Stop；分别证明wait/read/join收口。
- 取消后伪造done不能被计为成功；有合作abort→done→exit的正常路径仍保留数据、稳定收据与后续Prompt。
- 当前测试先观察cancelled+running移除，再给PID ESRCH额外1秒，这允许状态先发布再reap；若目标是“Cancelled只在reap之后”，应在第一次观察终态时立即断言ESRCH（直接child zombie仍有PID），不再给宽限。第二Prompt在已确定agent_busy时短重试是合理的，不能盲重发不确定Prompt。
- MCP执行中或delegated cancel请求卡住必须单列，不能因上述pipe fixtures通过而宣称整个工具取消期限已闭环。

## ACP相邻范围

普通GUI Stop在gui_host.rs:1253只handle.cancel，没有设cancellation_sent；现有:1124起10秒期限仅delegated.is_some时运行，普通Stop不受它保护。并且biased select事件在timer前，高速无穷notes可饿死deadline；应在每轮顶部检查绝对期限，或把已到期timer置于保证可推进的位置，不能只给idle循环加timer。

khor-acp Handle的_close只随Handle最后一个实例释放；gui_host把Handle包Arc并让in_turn/replay/control/set任务克隆。离开loop后只丢主Arc不足以关transport，JoinHandle drop也不会停止任务。需要显式可共享的shutdown能力，先关连接/进程再结束任务；若要报告Cancelled还需拿到退出确认，不能仅发送close通知。

本机固定依赖agent-client-protocol 3.5.0/acp_agent.rs已读：spawn process_group(0)；ChildGuard Drop会kill group+Child.kill，但Drop本身不是同步reap收据。其stderr有1秒shutdown grace，这不替代Khor的普通Stop期限或确认接口。不改依赖，不把第三方注释当实际已验证回收。

## 结论

当前仅test入口，不能批准“Q037已修”。root实现common/Unix时，上述Stop非阻塞、writer完整帧、独立进程监督、管道可取消、确认reap后终态是同一收口链；MCP/远端子任务的在途副作用是需要显式处理或另记的边界。等待固定实现再做源审与实际测试核对。本审查没有自行运行Cargo/新fixture，全部动态结果以root任务卡与后续真实日志为准。


## 第一候选追加源审（未编译，2026-10-25）

root通知后读取三新叶模块与execute差异。此时SHA：input `e263681ea8ba0e0dcb8e2f42999a7ac2ad704939cc6c5782f9929a17677d3981`，process common `cd9a9c2fccc20b7bf96dd330c400c93c61373ed71059acb3c4ebbd6eda35e36e`，process Unix `5ee38e55906392945eccd4a2038b26170ca53a957e53c09d18633a209aab5808`，khor_agent `b068d19262b9f3e0611c4e72c29e53ae7ce47bcf63aafaeb59adc62f3cfef35f`。源码继续由root修改，本记录只适用于这些内容。

已采用的正确边界：Running.request_stop只设stopped/Control最初deadline与Input.abort，无OS写；send先clone Input再写不持active.input锁。独立supervisor推进期限；Unix非阻塞pipe检查cancel。waitid WNOWAIT保留leader zombie作为PGID锚点，killpg再wait；自然exit后先reap但不cancel_io，等execute消费stdout缓冲done后finish再取消I/O，避免天然exit的尾部done被提前抹去。正常收尾drop(events)→finish确认wait→cancel_io→writer/reader/drain join→状态保存，process_stopped条件拒绝仅凭stopped标Cancelled。上述判断仅源审，尚无作者/审查者实跑结论。

需补的异常收口：Input线程持owned.clone，空队列时Condvar.wait；execute正常路径显式input.close，但若中途panic被外层catch_unwind接住，Session Drop杀/reap+cancel_io不能唤醒空队列writer，JoinHandle drop只detach，线程可能永久留存。建议独立输入guard在Drop保证close并join（不能依赖Input最后一个Arc drop，因为writer自持Arc），或将writer生命周期放Session统一监督。已向root指出，不修改作者文件。

测试撤掉了1秒ESRCH宽限，但仍只有“cancelled且running已移除”才跳出loop做probe；这仍允许Cancelled先发布、之后清理、最后移除running。建议每次首次观察cancelled立即probe，running/claim释放另断言；当前候选生产publish顺序源审正确，指出的是回归不能防未来倒序的证据缺口。

MCP同步调用与cancel_children未解决，root已明确不以这个候选关闭完整Q037。未运行Cargo或新的动态probe，未修改任何生产/测试源码。


## MCP/delegation深入检查点（暂停交接）

root要求优先Q033新独立红，本项只读分析保存后暂停，未实现/运行新fixture。

确认不能简单timeout顶层future：mcp::runtime每次新建current_thread Runtime；link::delegation_local:5960的spawn_blocking进入同步delegation_here，local search:6305、ls:7959亦spawn_blocking。已启动blocking任务不能靠drop JoinHandle取消，Runtime析构还会等其退出；A2A更会嵌套进入a2a_http::handle自己的runtime.block_on。因此timeout/select顶层后仍可能在runtime Drop卡住。local read_at:7450直接files::read_slice连yield都没有，local search对同步fs::read的SEARCH_BUDGET只在文件之间检查，不是单次OS读的截止。

可取消的叶级等待：ipc::exchange的Tokio TCP read/write/连接；mesh ask_on的网络响应；A2A handle_async/client的reqwest send/chunk/get/wait睡眠；本地task_wait每100ms sleep循环可改为检查run cancellation。但“drop客户端等待”只表示本地不再等，不能撤回已被resident/远端接收的Open/Send/Answer/Cancel/Tell。原请求不应自动重放。路由via_serve会把执行移到另一个resident task，断开IPC不等于对端取消。

不可伪装取消的本地提交：mcp.delegate先track_child intent（write_private临时文件+rename），调用后再记录确切task；ACP Store.accept/request_cancel、a2a Files.transaction都涉及文件锁和同步提交。a2a Files.transaction即使receipt/get也写ledger，锁等待gate.lock无界。a2a Send在intent durable后才send_message，未收到回复必须按既有request id恢复，不能清intent或重送；Cancel先写cancellation_requested再发远端请求，它只代表意图。Connect完成网络验证后写credentials/service/endpoint，Disconnect删除endpoint；Tell的post_with在self.chat.post后已提交正文，再await推送；Open/Close/Answer有已启动/已发送副作用。以上同步写开始后必须收口并保留结果，不能detach正在写的worker、释放原executor再对用户称未发生。

具体下一批建议：增加只供Pi内部使用的run cancellation context，不改变外部MCP默认语义；在dispatch前/文件锁等待时/未提交的网络预检处检查取消；锁等待用try_lock+有限等待与cancel检查，不在持锁的原子write/sync/rename序列中插断。把本地Wait等轮询从blocking睡眠拆出可取消叶。A2A借用现有handle_async，而非把同步handle塞进另一个blocking task；网络等待可select取消，但提交过intent的写操作返回明确不确定结果并保留child link/request id，禁止宣称回滚。cancel_children拆成原同步读取links/提交intent必须收口+有总预算的async list/get/cancel；整个分页链与所有children共享绝对deadline，不能每页/每child重新延长。跨设备取消仍以远端terminal receipt为确认。

最小真实fixture建议（未落代码）：(1) worker持本地A2A store.lock，fake sidecar发合法task_wait MCP后Stop；证明原lock等待不可仅用runtime timeout救活，新try_lock取消可停止，另一个独立控制线程释放锁供失败清理。(2) loopback A2A先接收Get/Wait再故意不回body，Stop后取消实际reqwest等待，确认child intent不被伪造为远端Canceled。(3) loopback Send接收并计数后压住reply，Stop后只允许同request查询恢复，永不发第二次Send；故障释放后仍可查原receipt。(4)持ledger/connect锁时Stop取消等待不提交；已经进入受控原子提交时Stop必须等待提交结束并保留真实记录，不允许晚写跨新run。任何模拟阻塞都用scratch、独立worker、父进程deadline/释放闸门，禁止真实用户锁/SSH/API。


## 当前候选复审：55d72f50加现场diff（2026-10-25）

root要求只读复审，实际HEAD `55d72f502219b659e23ace93482f236194d0d275`；Cargo运行中，未修改实现/测试、未跑第二个Cargo。图谱仍main，未覆盖新叶，已优先查询后转实际源。当前SHA256：khor_agent `a415c18e1ce42cdb582f3503d2a721ae004d46ae17610659623d271a63ebcd85`；input `f79f85da9c869854163f6f3f12bf4a3ae9f451891382a22e88a25e8e1af10e9c`；process common `49803bc82307e0228c0b7113b5c85788a4c6c250edbfceec547be0dd2b200424`；Unix `874b04751d841cdb5066251eb4da56d6e52f41ac0ea30204e96202fe8ea5c758`；Windows `9a84ec14364e14a88b592a113797d56bc695ed1bfe7f8b3a561f004e383658b7`；process tests `54acd2e7f4550772901bd04a46c520e6c1c4bd895e6e98aec1d226242b1de779`；Windows harness `d626b1570fc222e92cce0e3c3d88aabf4004e69e9d49dc08faedc257b958f83c`。

### 已收口的源级问题

- **Execution RAII堵住空writer panic遗漏**：Execution在drain/reader spawn前已持有Session/Input/writer；Drop调用finish(true)，先input.close通知Condvar，再force-finish supervisor，然后join全部已有handles。events/receiver声明晚于Execution，panic展开先drop receiver，阻塞send的reader也能退出。即使drain或reader spawn panic，已成功创建的writer不会再永久等待空队列。Input发布到active与Execution构造间没有可恢复错误/显式panic点；Input::spawn失败时尚无writer，Session自身Drop仍收进程。原发现的结构缺口已修。
- **abort前未开始首帧**：Input state.begun仅在writer取出待写帧时置true；abort在begun=false时清queue并closed，不写abort、不再接受start；begun=true只在帧边界追加abort，不截断JSON中间插消息。界限是writer已取出帧，不承诺对抢在首个OS字节前纳秒到达的Stop做回滚。
- **天然exit期限**：supervisor确认leader退出并kill残余group、wait后调用Control.stop，给尚未结束的pipe协议最多10s；已有Stop不重置更晚。reaped后仍不立即cancel_io，正常reader可消费缓冲done，execute finish会提前收口；若继承FD一直不EOF，10s后取消I/O。该机制不依赖有GUI Stop，也不扩大到MCP栈的已知阻塞。
- **Cancelled断言已补正**：cancel_tests现在每次观察到receipt.cancelled立即signal0/ESRCH，然后才另查running是否释放；不再把running移除作为reap断言的前置。blocked-input还断言Stop本身1s内返回。

### Windows只读判断与实跑界限

Windows正常Drop路径现在先SetEvent取消I/O，TerminateJobObject→wait leader→QueryInformationJobObject直到ActiveProcesses=0，再关闭唯一owned Job；相较仅close Job后返回，增加了“返回时本轮进程停止”的确认。spawn时Job-list属性保证创建即入Job，handle allowlist仅stdin/out/err，无PID重用杀错进程窗口。overlapped取消后GetOverlappedResult等待完成再释放栈上的OVERLAPPED/调用方buffer，顺序必要且保留。

错误fallback会close kill-on-close Job，再等leader；此路径不能据此声称所有descendants已被逐个确认。正常路径wait保留Job直到ActiveProcesses=0，harness新drop_tree在Drop刚返回立刻WaitForSingleObject(handle,0)，能防仅异步close然后较后才退出。该新断言尚无Windows实际运行结果，不能把窄cross编译代替运行。harness新增supervised模块目前仅纳入编译，checks::run仍执行process leaf七组，不会实际覆盖common Session十秒监督；后续Windows CI需明确测试层级。Cargo.toml仅cfg(windows)直接引入既有精确0.61.2，主lock只加该package依赖边，无升级。

### 当前非阻断的验证缺口 / 完整Q037保留边界

当前process_tests的unwinding场景只drop Session并用手写blocked writer，不经过新的Execution+Input空队列；建议补实际Execution的catch_unwind空writer/reader-spawn阶段回归，及Input.abort先于send的无字节/worker已退出回归。natural-tree测试覆盖同组后代及时终止；不覆盖已逃逸group的继承FD使common从natural exit起10s取消管道的路径。源机制合理，但这些分支仍缺直接动态断言，不能把现有process组通过当全部边界验证。

Unix现有Darwin zombie-only EPERM分支仅在WNOWAIT确认本leader已退出且proc_listpgrppids完整小列表只含该leader时放行，不笼统吞权限失败；不同权限后代、截断列表保留错误，方向合理。具体真绿结果由root运行后提供，本审查未另跑probe。

这批文件未发现新的源级阻断；源审结论以列明指纹为限，最终通过仍待root实际Cargo结果。MCP同步工具/文件锁/Runtime drop、cancel_children、ACP普通Stop/共享shutdown及Windows实际平台验收继续保留，不关闭完整Q037。


## 独立生命周期回归源码交付（等待root验证）

root仅授权新文件`crates/node/src/khor_agent_lifecycle_tests.rs`，由root注册；作者未改main/三叶/Cargo。新增真实父测试：`khor_agent::lifecycle_tests::execution_unwind_closes_idle_input_even_with_an_active_clone`、`khor_agent::lifecycle_tests::a_stop_before_input_installation_writes_no_start_or_abort`。两个ignored worker/sidecar由父实际调用，不能将ignored数算通过。

第一项真实Session启动当前test exe的独立sidecar（12s自退出fuse），Execution持有真实writer/reader/drain线程；writer使用Recorded Write的Drop witness，空队列且Running仍持Input clone。catch_unwind触发真正Execution Drop，断言3s内返回、writer已Drop、reader/drain已完成、仍存活的active clone无法继续send、直接child已即时ESRCH。第二项先实际Running.request_stop→execute，脚本若被启动会写marker，要求未启动且本机receipt为cancelled；另独立调用真实Input.abort-before-send验证无abort/start字节、writer退出，不复制生产input安装条件分支。父worker18s上限，所有store/脚本均scratch，无真实模型/用户配置。

初次冻结通知SHA360ffb30之后，作者自查父模块已移除Write导入，发出先别编提醒并于2026-10-25 23:06:53 +0800仅补新文件std::io::Write；该时刻与root已启动lifecycle-candidate-01交叉。已明确报告root，不将混合source运行算固定通过。最终源码SHA `bc1923410fd5f9447a9e970a9b554712a4579fdacd3bd55896c72e587e95a341`，作者现完全冻结，不跑Cargo。rustfmt与diff-check通过；编译/运行结果必须由root针对固定bc192另验，不在本卡预报绿。
