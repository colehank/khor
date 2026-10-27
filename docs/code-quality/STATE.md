# 当前代码治理状态

更新：2026-10-27，N2 source22 local13 原Pair04通过并归槽。完整目标 **active、未完成**。继续先读本页与[PLAN](PLAN.md)，核实际refs/diff/agents/PID；旧长状态完整保留在[历史快照](history/STATE-before-n2-read22-20261027.md)，任务细节看原任务卡/原始收据，不能按摘要覆盖WIP或重启旧进程。

## 用户决定

- 完成全仓功能审计、风险修复/重构、Clean Code工程治理和最终同源App验收；不把任何局部里程碑当全目标。
- **“不用冻结，开始了的做完，后面只记录不新增，再有问题给下一个轮次。”** 当前R2/N2/退出治理与既定验收继续；此后独立问题只记下一轮，不扩实现/矩阵/工具专项。先定向、再一次组合，无变化不重复跑。
- 未上线，统一当前Khor协议；不做旧端fallback/迁移矩阵。认证、epoch、数据完整性、实际平台与外部ACP声明能力保留。
- **无本轮main合并/ship授权。** main `/Users/zgh/Desktop/working/khor@8704a503a27b36ad58f922690de50e766f8bd485` clean；用户App/服务/release不动。

## 当前实际代码与验证

治理集成树 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`：`integrate/code-quality-wave1@3a0104a9` clean。权威协调目录是本目录，作者树同名文档只是历史副本。

| 工作 | 当前事实 | 尚缺 |
| --- | --- | --- |
| R2精确input取消 | 作者828fbd3167732c27fc4bb1718474a41961882279→集成aa9f80ca；12payload已核。run05五必要产物正常来源+59 distinct实际全过，311.919s/94dual，2354输入稳、288组与TMP清。sticky scope/late Send/坏丢receipt不退task-wide已验。 | 当前集成组合/App；HTTP无能力仍Unknown，不能宣称所有远端停止。详[任务卡](tasks/Q-037-input-cancellation.md)。 |
| 正常serve/GUI agent退出 | source03 exact10已3707c1fdeadc1cb6ef2c6285d51a843577d97267→dc98d327；作者树3707 clean。CLI/Node正常来源+37 distinct实际全过，normalTauri Cargo check0且currentROOT/freshfalse。61dual/2378输入稳、239组与TMP清。 | 自动GUI producer proof因TAURI_CONFIG/REMOVE_UNUSED_COMMANDS未覆盖保原非绿，独立工具问题仅记下一轮；不重跑37/改Q055。组合及真正App Exit仍待；不宣称一般NodeIO/durable hosts全部join。[任务卡](tasks/Q-037-shutdown-callers.md)。 |
| N2 Pair/Vouch | 作者 `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery`，`fix/quality-node-doc-transfer@27093f4a4ff94afcdffa36aa4cd6143e11ecaaad` dirty12。bounded claim/intent/settlement/Vouch/devices完整路径在source22；未提交。source21已实际8父过与双SIGKILL恢复幂等过，原Pair04仍90s红。source22仅read_outgoing复用已prepare canonical本地Ready，仍完整core digest/epoch/auth验证，普通resume/receiver不改。 | source22 run09真实29MiB Pair04仍90s超时；runtime-only run10重复观察同红，0父/97.262s、清理空。原47尚未整体过（source21为8绿/Pair04红/38未跑），不能给source22继承全绿。 |

## native槽与立即下一步

**当前无Cargo/native活动槽。** node_transfer_owner 的 source22/local13 已完成并明确归槽，session51036/controller69298/Cargo69367均已结束；禁止重启旧handle。原Pair04唯一业务父通过，实际优化构建976.241s、父21.416s、总1002.443s，原90s/数据/全部断言保留。root已核原raw、20双盘SHA及manifest、来源profile/正常Node、before=after、6个进程组消失与TMP清。剩余既定47/N0/优化网络边界先出最小复用计划再执行，不自动追加target。

local13 receipt外盘`Q039/node-pair-vouch22-local-run13`，内盘`recovery/Q039`同名；source1932/12payload冻结。actual Pair04 exe SHA bb0dc0b4…a47e5，provenance b450504e…62eb0，normalNode和selected均opt3/freshfalse。只计1父，helper不充数；source22 dev90s红保留，不以优化绿回填。后续real RUSTC/RUSTDOC/PATH固定1.95，绝对RUSTUP_TOOLCHAIN/AUTO_INSTALL0、有限owned历史8192冷编门、双盘5GiB不变。

- run11因未pin编译器代理触发proc-macro2依赖cwd要求rust-src，rustup开始升级卸载原stable；已停机/0父。原642份事务备份按14组件reverse manifest独审映射630项，root已恢复628原文件硬链接+原docs目录+初始components字节。Cargo/Rustc原SHA精确相同，仍1.95.0/LLVM22.1.2，真实版本及依赖cwd探针通过，未升级/加组件/改settings/触1.89。恢复receipt `Q039/rustup-incident-recovery-20261027/restore-result.json` complete=true；不得重复恢复或运行rustup。
- run12因监视器累计记录512个历史进程耗尽，第二次observe异常跳过cleanup；root/owner已只对已知组有限TERM排空，原非绿与late raw/postcleanup分开保留。当前必要边界修仅entry默认512改可有限设置、末次observe异常仍走原有drain；2真实私有进程树检查/独审过，已提交3a0104a9，0新增产品父、不改provenance模块/GUI env。冻结entry f7ff0426…085e1，冷编显式8192。

- root已正核exact owned helper并仅一次5s sample source21、一次5s sample source22；sibling evidence `Q039/node-pair-vouch21-pairdiag-root-observation08`（6dual已闭合）与 `node-pair-vouch22-pairdiag-root-observation10`（6dual已闭合）。不读payload、票或签名。
- source21热点resume→validate_body→Loro整body解码已在source22移除；新sample剩read_outgoing→open_existing→recover/BLAKE3(681)、Context.version/load_pile(252)，嵌套计数不是CPU百分比。caller约6.0→8.1MiB推进，仍未满足90s。不能为赶门删完整性验证或猜造缓存。
- 原合同未额外限定dev profile90s；实际本机安装脚本用既有`profile.local`，继承release/CGU16/保symbols，dev实际opt0。source22 dev run09/run10仍红且不回填；local13使用同原父/原数据/90s、actual opt3正常Node与exe路径来源需验证。此前无warm local，只自然复用本任务部分缓存，不删fingerprints/不造旧producer。
- N2既定收口：原Pair04数据12288×64、>24MiB/<=64MiB/90s、samecaller/thirdcaller/VV/root不变；过后完成原47（当前同源已过父无变化可复用既有provenance）、原N0与已登记优化网络边界，提交整合，再一次受影响组合。

## 已有里程碑及原目标余项

- GUI键盘/最新消息、commit文件跳转已修，有浏览器、真实Tauri iOS、MobileSafari/UIKit验证；最终全部同源App仍待，物理iPhone/IME未全验。
- autosave/冲突/锁、身份/资源owner、脱敏/A2A/预算、上传receipt/poll、SSH生命周期等已分批独审集成，详[AUDIT](AUDIT.md)与各卡。
- Q052 current khor-host/5与清屏：28实际功能回归、tmux专门2过，95428→8d1033bb集成；当前组合native待。
- R1正常取消/owned close/自主driver已集成；真实2s慢metadata Stop由2123ms降至约1ms且最终durable，source07原31过。旧5.27s outlier原证据保留，不倒推单根因。
- N1 source11原17与25,447,950B/max4固定VV+内容收敛、crash14真实postflush/prereceipt SIGKILL fresh恢复均已过并集成；Q054 typed gate与trust18也已集成。ec404组合CLI/Node+61 distinct实际过333.684s/81dual，不能冒dc98/N2/App绿。
- Q055既有22纯控与real A→B→B→A→relocation实际5build/5native、closure06选择输入已过并集成；复用现入口，不另建framework。新GUI环境覆盖见下一轮记录。
- **原完整目标余项仍保留：** Q033真GUI→Tauri→Rust→host→ACP上传、Q035 borrow lease、Q043 OS拖放/主动退出/全屏、Q045真SSH/master/远端lease、Q041 fresh onset/远程/Linux musl/真实嵌入、余下P1覆盖与P4规范/职责/工具入口、P5全部同源App流程。Q053原205第一方Rust fmt差异尚未迁移，功能集成后单独纯格式批。类型/fixture不代替实际平台；Windows等未实测不冒已完成。

## 恢复与资源纪律

统一Cargo target `/Volumes/disk0/Khor/caches/rust/khor-quality-target`；jobs1/incremental0/lockedoffline/短独占TMP；**唯一槽、内外盘各>=5368709120B**，有限STEP300/TOTAL720与owned PID/PGID全成员、actual source/producer/artifact、dual、有限TERM/KILL/reap。仅精确任务资源可清理，用户进程/App/release不动，不重放旧cleanup清单。

权威plan在本目录；外盘证据 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`，内盘 `/Users/zgh/Library/Caches/khor-code-quality/recovery`。掉盘先读RESUME及镜像，再核真实WIP/refs/PID，禁止按摘要覆盖源码。runtime-artifacts历史14大Rust副本已可逆归档，旧路径不能直接复用，见[STORAGE](STORAGE.md)/ARCHIVE.json。

本轮后续新独立问题仅见[AUDIT下一轮](AUDIT.md)：Q055 GUI env输入覆盖、尚未复现的normal devices plan协调候选。它们不扩当前实现或矩阵。
