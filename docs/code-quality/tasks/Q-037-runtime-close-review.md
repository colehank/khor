# Q037 Agent runtime close-interface01：独立源码审

Reviewer：node_test_review（非作者），2026-10-27。仅只读冻结source/delta和调用链，无Cargo、运行测试、进程操作、改源或commit。本卡只审本进程agent/cancellation runtime关闭，不把它说成全Node停止或远端任务完成。Q052 source02/plan04仍冻结待槽。

**当前02结论：01的B1/B2、冷Root目录创建和otherRoot Busy分类在02源码已闭；新增Root容量预留race需03修，六个native父的最终隔离入口及completed-token快路径专门oracle也需补。02未编译/运行，不授生产完成。以下01发现保历史，以末尾02段为当前判定。**

## 来源与既有待修

先读STATE/PLAN、Q-037-remote-r1 owner契约（final close、普通status/read/reconcile不隐式重开、原completion保真、有界共享quota、未持久retired不丢）。CRG先行仍main@8704a503，close_root只匹配旧tabs，未覆盖本片；无.codegraph，落实际源。

冻结 `Q037/remote-r1/close-interface01-20261026T233035Z`，内盘inflight-source/Q037-REMOTE-R1同名共5文件逐SHA一致；2生产源与完整2350 inputs全部=current。baseline8242b5bd。delta SHA `695d4322aaf7d32b88968aa3ef490659f6db3bcf51cfdd4f11857d853dcfdc1b`；khor_agent.rs `2d874c51226c715a5e097e1e03468afc50d595dd8cd1abbdad2f128f22f9d0d6`；remote_cancel.rs `43d07bf70d9270c70501c9c9e0a7c885c2f91202e605388a20a2dfcc2d3866a4`。没有GUI调用方/新tests的runtime证据。

root已先发现的两项及已撤回误判，单独保留不冒本审新发现：

- close的try_claim会mkdir Root/.khor/agent/runs/lock；冷/不存在Root退出应NoCreate/NoOp，现有忙claim必须仍返回Busy、不能绕过。
- other Root占closing_metadata lane应返回Busy而非把该Root标RuntimeClosing，仍保持全进程1meta配额。
- **root首审误判已撤回**：remote_cancel_tests.rs:673–697的metadata:None属于protocol::Task，并非Scheduler literal；实际没有该测试构造，不改原16父，不把此项当源码/编译阻断。01仍未编译，不能称测试source绿。

## B1：closing只覆盖close调用期间，Closed后无真正closed态

位置：remote_cancel.rs:1271–1278、1372–1374、1240–1259；khor_agent.rs:540–545；link.rs:1492真实resident TICK。

Closing Drop无条件从closing移除Root，成功又删除Home/home_order。随后旧Node普通Read→reconcile_agent→pump，或仍在的resident TICK→pump，都会把它当新Home插回，立即允许Discover/ready-step；持久pending本来按要求保留，于是可以在没有明确runtime reopen的情况下再起真实Worker。甚至fence在close_root返回时已释放，外层spawn_blocking尚未退claim/返回API结果的窗口也可有metadata再起，因此Closed本身不能证明返回时没有新writer。

这不是“没有删除磁盘intent所以关闭错了”：保磁盘正确，错在没有有界closed admission状态。owner卡明确ordinary status/read/reconcile不得隐式重开。仅让真实Tauri Exit调用这个函数不能补上库屏障契约。

最小建议：在既有有界Home/admission结构保closed态，区分closing/quiescent与明确acquire/reopen；普通pump/status/read不可重新enable它。Root冷关闭不创建无限tombstones。明确哪个真实产品入口负责重新取得runtime，不把通用Node::open（metadata/record也会调用）一律当reopen。全部旧owner已join后才可复用此Root的runtime，无须新daemon/全局Node关闭框架。

行为验证：真实durable pending+可控finite owner→close成功后继续调用旧Node普通Read/reconcile/actual ticker；要求0新worker/0Root晚写。显式acquire/reopen后才可由同持久intent恢复真实cancel，不伪称远端已完成。无固定sleep替代owner/start计数与实际PID证明。

## B2：Prompt producer没有完整close admission/claim生命周期

### 已存token快路径

khor_agent.rs:409–419在try_claim或closing检查之前对prior Saved调用self.record_command，然后返回run。它是一次真实Root写入；close持executor claim也挡不住。已完成token的同token重试可和close join/final return重叠，且close没有等待这个写者。保留同token运行中幂等很重要，不能简单把所有重复Prompt改成独占claim失败。

建议：把Prompt快/慢路径都纳入同Root的close admission，或证明并去掉快路径不必要的重复record写入；保持旧幂等结果与不重复model调用。不要把Configure/其它Node IO无界扩入本API，但必须明确本agent producer边界。

验证：在真实同tokenretry即将record的窄可控点持住写者，开始close；它不能Closed后允许原写者再落盘。关闭期间相同token语义与普通运行中幂等分别检验，不靠禁止所有重试过测试。

### std::thread::spawn失败分支

khor_agent.rs:421持claim，463–489把File直接move进executor closure。正常路径_claim确实覆盖execute→local_retired→Running removal（也包括executor panic后的原receipt保存），这点正确。但如果Builder.spawn返回Err，未启动closure的captures被销毁、claim已释放；490–502随后仍remove Running/read+save失败receipt，save会Node::open/record_command。close可在这段错误清理中获取claim、返回Closed；原Prompt随后继续写Root。只检查Running也不够，因为现分支先remove再save。

最小建议：父侧保同一claim存活至spawn结果和失败cleanup结束，如同一File的Arc handoff；成功executor仍持原claim到全部producer工作完成。避免重开lock或持scheduler锁等待。验证用私有test spawn失败控制+实际错误receipt final-save barrier，确保closeBusy/不出现Closed→晚写，并保正常Prompt/Stop/下一Prompt oracle。

## 已核正确部分与边界

- 正常close在scheduler锁内加Root fence、取该Root metadata/steps、清该Root ready/durable cache，随后锁外join，原owner绝对deadline不重置。没有join时持scheduler mutex；最后state guard在Closing RAII之前drop，正常返回没有自锁死锁。
- 取走步骤计入closing_steps，start按steps.len+closing_steps<4；metadata取走仍closing_metadata保1lane。不同Root正常步骤可继续用剩余席位，临时lane占用错误分类已单列，不是配额可突破。正常join完成后RAII才释放计数。
- metadata原Result（包括Err）放回原Completion，coalesced waiter持Arc仍能收真实结果，不被假NotPersisted/伪ACK替代；close不解读Status成功为远端完成。取走的Step结果不再enqueue，持久pending仍可在明确reopen恢复。
- 若已有Root retired未持久，直接Busy并保条目；取Persist owner前预留有界retired slot，只有validated DurablyQueued才移除预留；Err/坏reply/不确定结果保预留并Busy，没有看见丢retired或直接删durable文件。retired Busy本身不保证单靠重复close会有新进展：GUI无resident时若没有其它pump，调用方需明确定义允许的有限进展/重试机制，不能在超时后强删Root。当前契约允许Busy保Root，因此不将这一点单列为已确认新阻断。
- drop/abort外层async close future不能取消已开始的spawn_blocking；该blocking任务仍持claim和closing quota直至旧owner join结束。调用方失去Closed结果必须保Root，不能把future取消当关闭成功。正常路径无需新取消线程；还应验证取消future后的后续close在owner未退时Busy，释放后无quota/fence泄漏。错误返回也同样保Root。
- 新OwnedWorker Drop join在正常poll/take路径handle已None，不会额外长join。panic展开在_closing先于metadata/steps Drop的声明顺序上可能先释放quota，但01未见一条通常可达的join前panic链，不据此创造新阻断或要求通用事务框架；后续若改出早退路径，务必保持owner实际退前不释放配额。

## 02最小验收建议

先修已确认清单、独立freeze，再定向验证，不先用GUI目录删除掩盖生产边界：

1. 冷Root NoCreate；实际忙executor claim Busy且Root不变；executor terminal已保存但local_retired/Running未退时仍Busy。
2. finite metadata owner真实hold时close未成功且Root保留；其它Root仍可用剩余step配额/Busy metadata lane。释放后原completion真实送到原waiter、1meta4step不超；所有实际owner/group退后才Closed。
3. B1 post-Closed旧Read/TICK不重启、明确reopen恢复同durable intent；B2同token快路径和spawn失败producer屏障各按真实写入窗口覆盖。
4. 取消close future后blocking owner继续受原deadline约束，quota/fence不提前释放；后续明确close/恢复不永久泄漏，不把unknown当Closed。
5. 真实产品Exit caller只在Closed后释放本agent Root生命周期，Busy/error保Root并给可诊断结果；GUI fixture相同边界先真close，再HTTP stop/join与目录删除。测试普通/ignored父分清；原Scheduler literal推测已撤回，不改原16父；原18/相关/4父证据不能自动映射新production。

未运行，不关闭R1/R2/Q037；root可据本卡统一给作者02修订范围。本审不修改旧GUI/stream审卡或Q052冻结计划。

## close-interface02独立复审（当前结论；卡交root）

冻结 `Q037/remote-r1/close-interface02-20261027T000802Z`，10外内文件逐SHA齐，23源/2351全输入逐SHA=current；from-close01 delta `848bd48300d572aa82a83a7fca327a5320f4fab1b6cc75eaad5280ebb87f127d`。实际读两生产delta和568行新runtime_close_tests，未Cargo/测试/进程操作。GUI/公开link/原16父未改，真实serve/Exit caller由root另接。

### 01已闭与正确保留

- bounded closed set在成功后保留；普通pump、enqueue、metadata、next_home、Status均拒绝Closed，不再将旧Read/TICK当新Home。显式Node::acquire_agent_runtime、Prompt admission acquire与真实Retry才重开，普通Node::open没有隐式acquire。closing仍跨claim与join，取消waiter不早释。
- Prompt先持原config.lock短admission至快/慢路径结束；重复token的record_command也在guard内。父/子Arc持同一个原executor File，spawn Err后父claim仍覆盖read/save failed receipt；正常子claim覆盖execute/local_retired/Running removal。未用第二次重开lock假装同一claim。
- cold根close先登记有界closing fence再探现有目录/锁；不存在Root/agent目录不会mkdir；已有agent目录可取得/创建同一config admission lock，executor lock只existing-open，忙claim仍Busy。reconcile只existing-directory guard且双重closed检查，不在关闭后晚建目录。
- closing_metadata由其它Root占用返回Busy；本RootClosing/Closed分别保持RuntimeClosing/RuntimeClosed，不互相谎报。锁外join+closing1meta4step配额、原completion回复和未持久retired保Busy逻辑保留；没有发现02把未知持久化判Closed。

### C1 新确认：新cold closing的容量预留未被所有Home入口计算

`remote_cancel.rs:1324–1330/1385左右`的acquire/begin_close为一个不在homes/closed的新closing Root计入closing.len；但pump新Home约1303、request新Home1588只看homes.len+closed.len。可精确交错：已有255Home；begin_close冷C登记closing slot；另一新D的pump/request仍插入第256Home；C.finish再插closed C，总logical Root达257。可并行多次扩大，违反hard256容量。

最小修：所有新Home入口使用同一保留容量规则（保守算closing.len即可，不要求新框架）；加入真实CloseGuard尚未finish时的新Root admission回归，使用隔离进程，不重置有活owner的全局Scheduler。root已接受本项统一03。

### 六个新父与真实故障/owner验证

实际FIFO HeldRun替换真实Saved读路径（File::open/take/read_to_end），非伪造Worker结果；native读到FIFO建立writer后仍等真实EOF。release先恢复原Saved路径，再把原真实bytes交给已打开FIFO reader，原metadata/Step正常继续；owner_started只记真实spawn PID，最终ESRCH断言。producer point仅cfg(test)有界4秒真实阻塞；工厂失败明确标controlled，绝不当生产RPC失败；3秒condition等待未达到/4秒超时均应非绿，不能以release补出假前置。

- cold/no-create+256closed容量、真实metadata join/otherRoot进展/retired不丢、取消close waiter、四step含closing席位及第五替补，均覆盖实际本地生命周期，不冒远端child Cancel完成。
- spawn-failure父除了closeBusy，还直接assert executor claim仍Busy，能将Arc handoff与config guard区分；正常失败Saved确来自真实Prompt错误清理。
- **running-token-retry父目前只证明运行中幂等，未隔离completed-token writer屏障**：它在retry-record gate时仍持executor-start gate，旧executor claim本就Busy，去掉新config admission也能让while_recording Busy成立。须保原running幂等段，再等该真实executor完成/claimfree，以同一真实Saved重复token停在record点，确认executor claim可获得但close仍Busy（只能来自admission），释放后Closed。不要伪造Saved/RPC来凑这个反例。此项为必须补的测试oracle缺口，不是声称02 guard生产代码仍失效。

### 最终入口必须修，当前exact与默认入口不能混说

六父当前全普通tokio::test，在同一process共享Scheduler。cold父永久填256 closed；即使--test-threads1模块串跑，也会污染后续父和原tests，或因之前已有Home导致cold256断言提前失败。默认并发还会争4+1资源；单靠卡写exact不足最终集成。

采用root认可的最小方案：六父显式ignored native acceptance，并提交版本化真实验证入口；构建实际normal worker/Node libtest once，六次分别新的actual exe进程 `--ignored --exact NAME`，每次精确1passed/0failed/0ignored，worker/辅助进程不另计。list或默认skip不算绿；selector未知/0tests必须失败。不得为隔离清空有活owner的state。等价真正parent/child隔离也可，但无需为这六父扩通用框架。当前每父exact独立进程的试验方式本身没有上述跨父状态污染；C1源码仍需先修再绿验。

N1四个新重flow Probe/全局gate也须最终同类专用入口闭环；保原N0和已有11父，不把它们改ignored来减少默认覆盖。

本卡02内容已完成，**写权现交root**，reviewer后续不再改此卡。03作者按统一清单先冻，root可非作者复审；本轮没有任何新runtime结果。接下来仅按root明确授槽运行既定Q052 plan05，不自审该实现、不混改R1源。
