# 当前代码治理状态

更新：2026-10-27。完整目标 **active，未完成**。先读本页、[PLAN.md](PLAN.md)，再核实际 HEAD/diff/agents/PID；任务细节以本目录任务卡及原始证据为准，不能据摘要覆盖未提交源码。此前完整检查点见[历史](history/STATE-before-terminal28-20261027.md)。

## 用户决定与交付边界

- 完成全仓功能审计、按风险修复/重构、Clean Code工程治理和最终同源App验收；多智能体协作与压缩恢复贯穿全程。任一首批或库基座不能代替完整目标。
- 未上线，不保旧Khor端fallback/迁移矩阵；统一当前协议。认证、epoch、数据完整性、实际支持平台及外部ACP声明能力仍保留。
- **本轮没有main合并/ship授权。** main `/Users/zgh/Desktop/working/khor` 为 `8704a503a27b36ad58f922690de50e766f8bd485`；治理集成树 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1` 为 `integrate/code-quality-wave1@670347d766191b2b30fbc0e91f31fbf4e87cb3c2`，两树最近核对均clean。用户既有App/服务/release不动。
- 权威协调目录是本目录；实现树同名文档为历史副本。外盘新资料放 `/Volumes/disk0/Khor` 对应分类。

## 已集成的成果

- 首批GUI键盘/最新消息与commit文件跳转已修，已有浏览器、真实Tauri iOS与MobileSafari/UIKit验证；物理iPhone/IME及最终同源App未全验。
- 编辑autosave/conflict/lock、身份与资源owner、凭据裁剪/A2A/输入预算、upload receipt/poll、SSH生命周期等已分批独审集成，范围见AUDIT与任务卡。
- Q039 DocTransfer库已集成strict apply、descriptor、init、outbound、receive、lookup/resume；库FlushedImport不是网络ACK，Node接入N1另做。
- Q037 owned MCP回收/claim释放已集成；远端可靠取消R1/R2另做。
- Q033 current GUI协议子集已集成（767aa23d），19项不同回归过；真GUI→Tauri→Rust→host→ACP上传未全验。
- Q041e bindings生成已集成（c89aa135→670347d7）：实际清理3个过期TS export、11项回归/标准生成/types通过；全Q041与P5未完成。

## 当前协作与下一步

| owner | 实现树 / baseline | 当前状态 |
| --- | --- | --- |
| root | wave1集成、plan总账 | 独审、公共调用方、集成及恢复镜像；Q052 source04已无冲突apply-check，待原2项验证后提交集成。 |
| node_test_review | `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-store-owner-recovery`，`fix/quality-terminal-current@13a9459f` | source04冻结，只测试fixture较03变化；plan10待最终冻结，仅一次host_shell_tmux构建+原2项。未自启。 |
| remote_cancel_owner | `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`，`fix/quality-remote-cancel@8242b5bd` | A8 plan04实际CLI构建过、Node libtest编译失败，已正式返槽。仅授权新close测试guard最小源码修正及冻结，未重启。 |
| node_transfer_owner | `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery`，`fix/quality-node-doc-transfer@5f2d5cf7` | source08等价预算提取/纯测试冻结，control05 root短审认可、尚未授槽。仅只读准备后续Usage/Pending/reserve修复。 |

**唯一Cargo/native槽当前空，下一顺序Q052 tmux2→N1 policy红/最小功能修复；R1修正后再协调。** 授槽/启动后须核现场，不据旧active.json重启。当前各实现批尚未提交/集成。

## Q052：终端当前协议与清屏

- source02统一strict khor-host/5、首Version5、整frame绝对deadline，删除旧Attached/降级；保snapshot/history/modes/tmux/Ready/Forget/reconnect。
- plan07真实TCP History/Ready/Forget复现清屏后旧内容重现。source03将parser/feed清理原子化，保光标/样式/模式与备用屏，保证本地清理先于可选Ctrl-L；新增4个真实IO回归，原TCP失败断言保留。
- **source03实际28项功能回归全部通过**（terminal-green09，21个执行阶段全0）；native_handoff只编译，不算运行。源/生成输入与实际依赖来源稳定。
- 原09整run仍清理红：2个停止的私有tmux socket残留。root另凭精确身份、不活跃与无持有证明只清这2个socket，t52v9已不存在；原失败收据不回写。
- source04仅 `crates/node/tests/host_shell_tmux.rs` fixture生命周期改动，其余1920输入含生产相对03字节完全相同。root独审已接受，22份双盘证据；12个变更路径，1921输入。manifest `6d20b800edca287236519eb57b2700be91bce0c4825219133efd2655a85b73a7`。
- plan10只跑原tmux2，继承同源其余26；不重复全28、npm或gen。通过后作者提交exact12路径，root集成并验证。
- 原05缺dist、06 CLI inode退休门、08旧summary depfile门等失败保留。实际rustc hashed depfile及源内容为来源证明，顶层debug/khor.d可能陈旧，fresh=true/current manifest本身不足以证明同源。

## R1 / R2：智能体取消与退出

- 原R0/receipt focused18项已实际过；旧Stop一次5.27s失败保留，buffered同用例276/213ms不能抹掉outlier。metadata启动可等5s+250ms，真实慢启动时限验证待做。
- S22 stream、GUI tools/network、GUI restart、paired真实认证Cancel四项过，原整run清理红14regular files/1752B保留，未后清；真实晚写推动owned close API。
- close03具有bounded ClosedRoot fence、显式acquire、冷Root NoCreate、config→executor admission、Arc claim、锁外有限join、Home/Closed/Closing union容量；6个新真实FIFO/claim测试为独进程ignored用例。close只指本进程agent producers/cancel owners，不是全Node IO shutdown或远端Cancel完成。
- root已接serve_as acquire与Tauri真正Exit调用；Busy/错误保durable root与既有退出行为。GUI fixture改为close→HTTP stop/join→删Root，超时/错误保Root。实际Tauri AppExit尚未编验。
- 版本化 `scripts/run-node-acceptance.py` 与Unix CI已编写并独审，逐进程精确用例计数、资源/超时/进程回收；fake控制通过仅是控制验证，0个native功能用例。
- 当前聚合26payload源/2352输入。A8 plan03 setup冗余import re失败0Cargo/0测试；plan04只删该行，实际CLI64.75s过并核normal依赖，Node libtest19.24s失败于新测试87行 `let_underscore_lock`，0业务用例。26/2352稳定、46双盘、PGID/编译器全退、TMP清。只授权guard binding窄修，原失败保留。
- 最新normalCLI `17b4d744e772b2b978966e82ed263b8d341e9a9c0ba06132db2a2a04b93bafc5`，来自A8实际run `close-a8-run-20261027T031815Z`；不能当Node测试已构建。
- 新close6+GUI2、原19回归、真实慢Stop及**R2未知首次Send lookup/lateSend fence**仍待完成。

## N1 / N2：Node文档同步

- 原N0合法25,447,954B设备历史旧4sync不收敛但Seen123/Pintrue推进；Pair04真29,224,082B回包失败烧票、同caller重试badinvite。原红保留；大Pair/Vouch恢复N2未实现。
- 当前N1生产仍candidate04：单DocTransfer、encoded1MiB/256KiB块、32MiB/30s visit、4块/1MiB公平quantum，无Loaded/Sink跨network await；raw1MiB门同用例红→绿，unit11实际过。03原N0被Busy误分类中止，04只修三处typed Busy，N0尚未绿。
- source06实际quiet1 Versions过；Busy6次真实body6,377,994B却计4,805,130B，漏6完整块；purepull Busy后first_pending=false但重开同manifest实际VV+Spec已完成。**两项真实缺陷尚未修复**。
- 原last-single及source07 few-op Chat在外/内盘TMP均未于30s达到32MiB；都是setup失败，不是越界红，不能归因外盘。保留优化构建下实际网络预算边界验证，不降低32MiB/30s限额。
- source08只语义等价提取生产预算判断并加唯一纯策略测试，14源/1084选择域输入（非全部GUI仓库），root独审通过。manifest `ebffe36de46e5f0045eaea0e2b3bc16fd62f7b9bf2126818f8955d708f542751`。
- control05 root已短审认可，一次当前libtest构建+普通exact纯策略红；未授槽。该用例不代替实际网络32MiB验证。之后才做最小Usage所有错误保量、opening双块reserve、partial Pending修正。
- 后续原Busy/purepull/unit11/N0、postflush/prereceipt真实SIGKILL重开和N2仍必需；Complete后重开不能冒中途崩溃证明。

## 其余完整范围

- Q053全仓fmt基线205个第一方Rust文件差异，尚未迁移；功能集成后独立纯格式批次。
- Q054私有IO gate typed错误边界待N1后，去除按英文文案识Busy；不建全仓错误框架。
- Q055共享target内容来源治理待实现；实际Sync/catalog反例证明fresh=true与manifest路径不足，精确refresh只是临时措施，未定性Cargo bug。
- 仍需Q033真实GUI上传、Q035真实borrow lease、Q043 OS拖放/主动退出/全屏、Q045真实SSH/master/remote lease、Q041 fresh onset/远程/Linux musl/嵌入；余下P1审计、P4规范与工具入口、P5最终同源App全流程。物理iPhone/IME/Windows runtime没有全部证明，类型检查不等于运行通过。

## 构建资源与恢复

统一target `/Volumes/disk0/Khor/caches/rust/khor-quality-target`，jobs1/incremental0/lockedoffline、短独占TMP；内外盘各至少5,368,709,120B，有界STEP/TOTAL/连续3IO、owned PGID全members/有限TERM-KILL-reap/source实际产物/双盘门。用户已允许外盘风扇坏条件下继续，不增加警告或触碰用户进程。

外盘证据 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`（其下直接Q052/Q037/Q039，无recovery前缀）；内盘镜像 `/Users/zgh/Library/Caches/khor-code-quality/recovery`。最新RESUME/文档manifest须同步。掉盘先读RESUME并核现场；不能按摘要覆盖dirty源码。精确缓存cleanup11–16旧清单不重放。原失败与历史二进制不回填成新源码绿。
