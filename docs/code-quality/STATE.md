# 当前代码治理状态

更新：2026-10-27。完整目标 **active，未完成**。先读本页、[PLAN.md](PLAN.md)，再核实际 HEAD/diff/agents/PID；任务细节以本目录任务卡及原始证据为准，不能据摘要覆盖未提交源码。此前完整检查点见[历史](history/STATE-before-terminal28-20261027.md)。

## 用户决定与交付边界

- 完成全仓功能审计、按风险修复/重构、Clean Code工程治理和最终同源App验收；多智能体协作与压缩恢复贯穿全程。任一首批或库基座不能代替完整目标。
- 未上线，不保旧Khor端fallback/迁移矩阵；统一当前协议。认证、epoch、数据完整性、实际支持平台及外部ACP声明能力仍保留。
- **本轮没有main合并/ship授权。** main `/Users/zgh/Desktop/working/khor` 为 `8704a503a27b36ad58f922690de50e766f8bd485`；治理集成树 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1` 为 `integrate/code-quality-wave1@b196992a09499d384afb97f321820a2bd89d6c51`，两树最近核对均clean。用户既有App/服务/release不动。
- 权威协调目录是本目录；实现树同名文档为历史副本。外盘新资料放 `/Volumes/disk0/Khor` 对应分类。

## 已集成的成果

- 首批GUI键盘/最新消息与commit文件跳转已修，已有浏览器、真实Tauri iOS与MobileSafari/UIKit验证；物理iPhone/IME及最终同源App未全验。
- 编辑autosave/conflict/lock、身份与资源owner、凭据裁剪/A2A/输入预算、upload receipt/poll、SSH生命周期等已分批独审集成，范围见AUDIT与任务卡。
- Q039 DocTransfer库已集成strict apply、descriptor、init、outbound、receive、lookup/resume；库FlushedImport不是网络ACK，Node接入N1另做。
- Q037 owned MCP回收/claim释放已集成；R1可靠取消/owned runtime close检查点亦已e5e913ff/47d509e2/dd9c45ca集成，dd9时26 payload与Q05212路径逐SHA保留；现c13加入N1后，R1非共同23/N1非共同9/全部Q05212精确保留，catalog键值并集核齐，三个共同源等待组合native验证。A8新增8、原B19 19与正常Tauri caller check过；组合native/App、慢Stop与R2另待。
- Q033 current GUI协议子集已集成（767aa23d），19项不同回归过；真GUI→Tauri→Rust→host→ACP上传未全验。
- Q041e bindings生成已集成（c89aa135→670347d7）：实际清理3个过期TS export、11项回归/标准生成/types通过；全Q041与P5未完成。

## 当前协作与下一步

| owner | 实现树 / baseline | 当前状态 |
| --- | --- | --- |
| root | wave1集成、plan总账 | 独审、公共调用方、集成及恢复镜像；Q052/R1/N1已集成至b196992a（含取消31回归、Q055闭包输入修复、trust18提取）；共同lib/link/proto组合native及App待。 |
| node_test_review | `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-store-owner-recovery`，`fix/quality-native-provenance@28ed5bee` | 复用原terminal树做Q055，Q05505已fc592fdb→0a888f24集成，20纯控、real03五对照及真实R1 CLI/Node来源接线过；容量错误已停；R1 driver与fixture07已8292→8395提交集成，31distinct全过；Q055 closure06已28ed→bdb689c2提交集成，22纯控及真实无dist Node-only构建/来源接线过，当前clean。 |
| remote_cancel_owner | `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`，`fix/quality-remote-cancel@c81d0a18` | 已c81→dd9检查点集成；真实2s metadata阻塞Stop已红，原owner额度错误停；现8292e654 clean，source07实测31父全过并8395集成；旧04/06/各失败均保留。正常Tauri/组合App仍待，不把R1全部/R2标done。 |
| node_transfer_owner | `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery`，`fix/quality-node-doc-transfer@fdfb6d0b` | N1 crash与source11原17/N0已过；trust提取candidate15真实新3+原15全部过，fdfb→b196提交集成，树clean。Q055 closure06也已提交集成。现仅准备b196组合构建入口，尚无新native授权；大Pair04/claim/wire及优化构建32MiB边界仍待。 |

**当前唯一Cargo/native槽空。N2 trust15 run01/session6954已经退出：metadata/无dist prepare/唯一Node构建及18父全绿，109.281s，34dual/所有owned组和TMP清；root独核。当前仅授权准备b196组合构建，未授执行。** 授槽/启动后须核现场，不据旧active.json重启。Q052/R1/N1三个检查点均已集成，组合native/App尚待；R1调用方关闭/R2、N1实际32MiB边界/N2等仍未完成。

## Q052：终端当前协议与清屏

- source02统一strict khor-host/5、首Version5、整frame绝对deadline，删除旧Attached/降级；保snapshot/history/modes/tmux/Ready/Forget/reconnect。
- plan07真实TCP History/Ready/Forget复现清屏后旧内容重现。source03将parser/feed清理原子化，保光标/样式/模式与备用屏，保证本地清理先于可选Ctrl-L；新增4个真实IO回归，原TCP失败断言保留。
- **source03实际28项功能回归全部通过**（terminal-green09，21个执行阶段全0）；native_handoff只编译，不算运行。源/生成输入与实际依赖来源稳定。
- 原09整run仍清理红：2个停止的私有tmux socket残留。root另凭精确身份、不活跃与无持有证明只清这2个socket，t52v9已不存在；原失败收据不回写。
- source04仅 `crates/node/tests/host_shell_tmux.rs` fixture生命周期改动，其余1920输入含生产相对03字节完全相同。root独审已接受，22份双盘证据；12个变更路径，1921输入。manifest `6d20b800edca287236519eb57b2700be91bce0c4825219133efd2655a85b73a7`。
- plan10实际build0/74.58s，原tmux2为2/0/0、运行6.42s，整run83.08s/exit0；1921稳定、normal/actualexe稳定、所有owned组退、TMP/socket清，无guard/forced。root独读raw/final/results并核42dual。95428b25→8d1033bb已集成，12路径完整SHA精确source04，集成定向native尚待。
- 原05缺dist、06 CLI inode退休门、08旧summary depfile门等失败保留。实际rustc hashed depfile及源内容为来源证明，顶层debug/khor.d可能陈旧，fresh=true/current manifest本身不足以证明同源。

## R1 / R2：智能体取消与退出

- 原R0/receipt focused18项已实际过；旧Stop一次5.27s失败保留，buffered同用例276/213ms不能抹掉outlier。metadata启动可等5s+250ms，真实慢启动时限验证待做。
- S22 stream、GUI tools/network、GUI restart、paired真实认证Cancel四项过，原整run清理红14regular files/1752B保留，未后清；真实晚写推动owned close API。
- close03具有bounded ClosedRoot fence、显式acquire、冷Root NoCreate、config→executor admission、Arc claim、锁外有限join、Home/Closed/Closing union容量；6个新真实FIFO/claim测试为独进程ignored用例。close只指本进程agent producers/cancel owners，不是全Node IO shutdown或远端Cancel完成。
- root已接serve_as acquire与Tauri真正Exit调用；Busy/错误保durable root与既有退出行为。GUI fixture改为close→HTTP stop/join→删Root，超时/错误保Root。实际Tauri AppExit尚未编验。
- 版本化 `scripts/run-node-acceptance.py` 与Unix CI已编写并独审，逐进程精确用例计数、资源/超时/进程回收；fake控制通过仅是控制验证，0个native功能用例。
- 当前聚合26payload源/2352输入。A8 plan03 setup冗余import re失败0Cargo/0测试；plan04只删该行，实际CLI64.75s过并核normal依赖，Node libtest19.24s失败于新测试87行 `let_underscore_lock`，0业务用例。26/2352稳定、46双盘、PGID/编译器全退、TMP清。只授权guard binding窄修，原失败保留。
- 最新normalCLI `17b4d744e772b2b978966e82ed263b8d341e9a9c0ba06132db2a2a04b93bafc5`，来自A8实际run `close-a8-run-20261027T031815Z`；不能当Node测试已构建。

- **resume06实际A8全绿** `close-a8-run-20261027T034134Z`：Node67.753s+GUI60.667s正常构建，原6个close测试与GUI2各1/0/0，整体180.977s/exit0。root独读raw/result并核80dual/source稳定/cleanup空；66PID/32PGID全退，TMP清，无guard/forced/fallback。当前槽已返。原Node actualtest已内盘保全 `runtime-artifacts/R1-close-a8`，SHA74618b30999017c372519cd392f866d2eca0f4f6ace4f1b983115d81b2c8e071。
- B19实际paired target52.566s编译过，原R0业务父通过六Prompt/六Cancel/四held与后5/6投递/清理；控制误将worker+外父两次成功summary当异常，原41dual/失败保留，不计成2业务父。root已读完整原raw承认1distinct通过；源码/4binary/8normal稳，组与TMP清。
- 原tail18实际在 `close-b18-runtime-20261027T035817Z` 全过，各raw唯一1/0/0，0Cargo/0mtime，60.947s/exit0。root逐raw并核91dual；163PID/76PGID全退，TMP清，源/actual4binary/normal8稳。组合原R0恰为B19 19 distinct，原错误收据不回填。正常Tauri lib check实际0/35.297s，当前四critical metadata fresh=false/当前ROOT，root核26dual/源与TMP/组清；作者已提交c81d0a18，root集成e5e913ff/47d509e2/dd9c45ca且26 payload与Q05212路径逐SHA全部保留。App/真实Exit/慢Stop与**R2未知首次Send lookup/lateSend fence**仍待。

### 真实慢metadata Stop红（尚未修复）

source02只两fixture路径27payload/2353输入；原source01的DeviceId.id编译错误已最小删.id，旧compile101/0父20dual保留。`metadata-startup-red-run-20261027T050904Z` 实际正常libtest63.730s过；zero内外父各1/0/0，16Stop567ms；真实2s启动下slow内外父各0/1/0、first2123ms/16calls2124ms，唯一primary Stop admission blocked，真实gate→exec2.0097135s/同PID/原CLI最终DurablyQueued。实际intent身份、正常close、Pi/metadata ESRCH均过，50dual/10PID/9PGID/TMP清。root已独读result，明确是本次产品红，不倒推旧5.27。

已授权最小生产fix：仅persist&&!retry的Stop登记准入owned metadata后不再等启动/落盘，已完成照实decode、未完成诚实PersistenceUnknown，owner/claim/quota持续持有；Status/显式Retry语义保留、原1s门/exercise不改，补真实Status观察持久化完成。source04已交（27/2353，生产仅5行Stop fast分支，慢fixture真实Status及原R0/两close等待者最小适配），root及node_test_review非作者独审通过；R0真实Status+intent在原六/four-held/5,6 delivery之后，close以同owner显式Retry实际Pending，原owner/claim/quota/fence oracle保留。run04已实际新CLI/Node构建及来源验证过，zero/2s+close6共8父绿，slow Stop由原2123ms降至1ms且最后真实durable。R0第0run原2秒内未到实际Cancel，定位metadata完成结果无自主pump；整个run非绿。原source04/28dual保留，source06单一process-owned driver双审过，run05实际两build+原9父全部绿，原R0六/fourheld/5,6delivery不改且通过。随后同二进制tail01策略3+旧receipt2过，旧锁fixture初始Unknown≠NotPersisted断言红：其过早drop锁未等实际producer尝试，源06生产冻结，正source07仅3处旧fixture适配真实故障观察后，新两build+原9父run06再次全过、同新binary的21项tail02全过、必要新stub+paired父run09过，共31distinct；8292→8395 exact6已提交集成，2353源稳/无forced/owned组与TMP清。不能给R0加Read/Status驱动，R2不混入。详见Q-037-cancel-driver。

## N1 / N2：Node文档同步

- 原N0合法25,447,954B设备历史旧4sync不收敛但Seen123/Pintrue推进；Pair04真29,224,082B回包失败烧票、同caller重试badinvite。原红保留；大Pair/Vouch恢复N2未实现。
- N1初始candidate04：单DocTransfer、encoded1MiB/256KiB块、32MiB/30s visit、4块/1MiB公平quantum，无Loaded/Sink跨network await；raw1MiB门同用例红→绿，unit11实际过。03原N0被Busy误分类中止，04只修三处typed Busy，N0尚未绿。
- source06实际quiet1 Versions过；Busy6次真实body6,377,994B却计4,805,130B，漏6完整块；purepull Busy后first_pending=false但重开同manifest实际VV+Spec已完成。**两项真实缺陷已由后续source11实际回归闭环**。
- 原last-single及source07 few-op Chat在外/内盘TMP均未于30s达到32MiB；都是setup失败，不是越界红，不能归因外盘。保留优化构建下实际网络预算边界验证，不降低32MiB/30s限额。
- source08只语义等价提取生产预算判断并加唯一纯策略测试，14源/1084选择域输入（非全部GUI仓库），root独审通过。manifest `ebffe36de46e5f0045eaea0e2b3bc16fd62f7b9bf2126818f8955d708f542751`。
- control05实际当前libtest构建0/67.885s；唯一普通exact纯策略用例0/1/0，在原第1057行headroom断言真实失败，root已读raw/final。1084/binary/deps稳、PGID/TMP清，槽已还。该用例不代替网络32MiB验证。source09已完成最小link_doc_transfer/link/proto三路径修复并冻结：23dual/14源1084、delta881ee330…126b6f。root与node_test_review独审无阻断。实际A15项绿，Busy在全visit交替断言失败，最终accounting未达；原238dual保留。fixture10只改flow，以真实AfterRead暂停+两端target VV完成证明界定公平窗口，其他1083含生产及原计量/quota断言不变，双审通过；resume08实际Busy绿/purepull控制失败后root定位失败流量被当advanced致只Busy空转；source11仅删OR，保计量、Ok驱动及原purepull。完整17重新实际各1/0/0，root逐raw核，250dual/18组/TMP清，Busy6次6,377,994B记量精确、purepull首Pending和重开完整都过。B原N0已实际过：25,447,950B在原4visit内固定VV+内容收敛，首visit Seen123/Pintrue，cap拒绝0；root独读raw/final与49dual。作者d7a与原dd60回归已695fa455/c13a5c44集成。详见Q-039-usage09-root-review。
- 原Busy/purepull/unit11/N0已由source11实际闭环。crash14/control12又实际完成checked-flush之后、Complete/回执之前SIGKILL重开：build0/71.375s，单ignored业务父0/19.310s，fresh verifier与parent各1/0/0、receiver实际signal9；首请求未Applied、Importing重开/Equivalent稳定、并发Pin与epoch、错误ACK保真实stage、正确ACK exactGC/new-id/lateWriteMissing均过。47dual由root逐SHA核，1085输入/实际binary/deps稳定，92917/94169组全退，无forced drain，TMP真清，整run97.513s。8f2f1d1b→3a2bb9bf exact2集成，组合native待。前半真实认证网络，后半fresh State API，非二次网络pump或整机断电证明。N2与优化构建真实32MiB边界仍必需。

## 其余完整范围

- Q053全仓fmt基线205个第一方Rust文件差异，尚未迁移；功能集成后独立纯格式批次。
- Q054私有IO gate typed错误边界待N1后，去除按英文文案识Busy；不建全仓错误框架。
- Q055 candidate04已859b9f7d→d49d88c3提交集成。root独审及真实小型编译先后抓到包外输入未入producer key、hyphen bin名配对、回A漏刷新生成器三处缺口，均保原失败后最小修复。19纯控及real03真实A→B→同源B→A→relocation的5build/5native全过；同源3artifact全fresh/empty refresh，变化正确重建、生成内容与嵌入root正确，66dual/owned组全退/TMP清。R1实际host metadata发现离线全平台依赖问题后CI/doc显式host过滤（9229→16a86），actual CLI rustc.d又确认Node包含providers.json/install.sh，05显式纳key而不放开未知输入（fc592→0a888）。20纯控及真实R1 normalCLI/Node完整来源验证已过；最终整合App与CI远端运行仍待；现又确认Node-only树无dist却被未选web输入阻挡，closure06按request闭包读取states/keys/extra（22纯控过），已在真实N1无dist树成功构建并完整证明Node产物，28ed→bdb689c2 exact2集成，后续复用入口不复制runner。实际Sync/catalog反例证明fresh=true与manifest路径不足，精确refresh只是临时措施，未定性Cargo bug。
- 仍需Q033真实GUI上传、Q035真实borrow lease、Q043 OS拖放/主动退出/全屏、Q045真实SSH/master/remote lease、Q041 fresh onset/远程/Linux musl/嵌入；余下P1审计、P4规范与工具入口、P5最终同源App全流程。物理iPhone/IME/Windows runtime没有全部证明，类型检查不等于运行通过。

## 构建资源与恢复

统一target `/Volumes/disk0/Khor/caches/rust/khor-quality-target`，jobs1/incremental0/lockedoffline、短独占TMP；内外盘各至少5,368,709,120B，有界STEP/TOTAL/连续3IO、owned PGID全members/有限TERM-KILL-reap/source实际产物/双盘门。用户已允许外盘风扇坏条件下继续，不增加警告或触碰用户进程。

外盘证据 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`（其下直接Q052/Q037/Q039，无recovery前缀）；内盘镜像 `/Users/zgh/Library/Caches/khor-code-quality/recovery`。最新RESUME/文档manifest须同步。掉盘先读RESUME并核现场；不能按摘要覆盖dirty源码。精确缓存cleanup11–16旧清单不重放。原失败与历史二进制不回填成新源码绿。

调用方新明确风险：[Q037 shutdown callers](tasks/Q-037-shutdown-callers.md)。实际无单数agent CLI命令，GUI/Node API可local fallback；serve_as仅acquire，Tauri Exit已有close；强制exit只有child5s fuse不等于join。完整App/serve关闭仍待，不能据底层31回归声称全App退出已清理。

N2仅trust分层第一步完成：candidate15新3＋原15 trust父实际全绿（18distinct），fdfb6d0b→b196992a exact1集成；原29,224,082B Pair04烧票、可恢复claim/intents/fence/wire/bootstrap仍未实现。后续优先组合当前N1/R1/trust并完成bindings/frontend/App验证，不把各作者树结果冒整合App。
