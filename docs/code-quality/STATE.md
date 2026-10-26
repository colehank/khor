# 当前代码治理状态

更新：2026-10-27。完整目标仍 active，未完成。先读本文件、PLAN与任务卡，再核现场 HEAD/diff/agents/PID/SHA；不能按摘要覆盖未提交代码。前一检查点完整保留在 [历史](history/STATE-before-670347d7-20261027.md)。

## 用户决定与交付边界

- 目标是全仓功能审计、按风险修复/重构、Clean Code工程治理和最终App验收，含多智能体协作及压缩恢复；不能以首批或库基座代替完整目标。误发AEC消息已撤回。
- 用户已明确未上线，不保旧Khor端fallback/迁移矩阵，统一当前协议；外部ACP声明能力、平台支持、认证/epoch和数据保护保留。
- 本轮没有main合并/ship授权。main `/Users/zgh/Desktop/working/khor` 仍 **8704a503a27b36ad58f922690de50e766f8bd485**，最后现场clean；不动既有用户App/服务/release。
- 权威协调目录就是本目录的plan/code-quality分支；实现树中的同名文档只是历史副本。外盘资料统一放 `/Volumes/disk0/Khor`，不再新建散落目录/worktree/target。

## 当前集成

`/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，`integrate/code-quality-wave1`，**670347d766191b2b30fbc0e91f31fbf4e87cb3c2**，最后tracked clean。Node正常网络仍旧Sync/inline Pair路径，N1/R1尚未集成。

| 已集成范围 | 当前证据与边界 |
| --- | --- |
| 首批GUI | 键盘/最新消息和commit文件跳转已修；真实Tauri iOS run11及后续MobileSafari/UIKit26指标、ACP/Khor尾部与历史锚点回归有证据。最终整合App、物理iPhone/IME尚未全验。 |
| 编辑/身份/资源/agent | autosave/conflict/lock、identity/purge、FIFO/connection owner、凭据裁剪/A2A/输入预算、上传receipt/poll、SSH生命周期已逐批集成并独审；确切范围看AUDIT与任务卡，局部绿不等于全用户流程。Q033完整GUI上传链、Q035真borrow lease、Q043 OS拖放/关闭/全屏仍open。 |
| Q039 Doc/blob库 | strict apply92742250、descriptor d16d83df、init f721a2c3、outbound5da91f03、receive67927fcf、lookup/resume686f2e42已集成并有限组独审。init真SIGKILL红绿，lookup root49/1076inputs/6binary/17组及kill子核齐。库FlushedImport不是网络ACK；Node尚未使用这些基座。 |
| Q037 owned MCP阶段 | e0c6abec/4493ed3c已集成，可杀回收process并释放claim；旧106相关作者组/root62有记录。远端可靠取消R1/R2另做，Windows仅类型/静态不称runtime。 |
| Q033当前GUI协议 | 作者13a9459f→767aa23d，5生产+3tests；作者/集成完整tree同87f24fd6。删旧upload_cancel/legacy_upload/Disconnected，严格当前GUI greeting；normalCore+greeting4+coreIO12+真host/ACP3=19不同父通过、独审。磁盘guard及一次fixture缺mut编译失败原记录保留，非业务红。实际GUI→Tauri→Rust→host→ACP仍未全验。 |
| Q041e bindings生成（本轮完成） | 作者c89aa135→**670347d7**，只3冻结源。先真fresh/reused多3旧TS红；后11普通父、真实Busy/SIGKILL恢复、实际runtime默认/显式路径、旧输出接管、standardgen/types通过，root核1922inputs/190双盘file/2binary。集成又实际normalbuild64.098s→gen6.729s→types5.164s绿，1923inputs稳/3组退出，旧157TS成为154TS+1管理marker，恰清PlaceRow/QuotaWindowKind/Wheel；旧原件保留。TMP94个npm缓存另凭精确清单清理，原tmp_absent=false记录不改，post-cleanup证明已清。全Q041/finalApp未因此关闭。 |

## 当前所有者与写权

| owner | 实际树 / baseline | 当前权限与行动 |
| --- | --- | --- |
| root | wave1@670347d7；plan总账 | 集成、独审、公共接口与最终验收。Gen小片完成；已独审N1 raw红绿/03 unit11，Q052仅源码审无阻断；负责R1后续证据。 |
| node_transfer_owner | `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery`，fix/quality-node-doc-transfer@**5f2d5cf7**+13源 | 当前冻结**candidate04**（typed Busy三处修正）。已有13源及link/proto/lib/Pending catalog写权；root已批准新增私有doc_transfer_flow_tests.rs及link/doc_transfer_state窄cfgtest真实wire/gate观察点，先冻结04生产加4父red候选，不作功能修复、不Cargo；postflush/prereceipt实际kill另闭环。不能按旧表排02/03或04自动跑。 |
| remote_cancel_owner | `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`，fix/quality-remote-cancel@**8242b5bda383b456a0122fc2613ca4e274620187**+paired02/Stop诊断tests WIP | R1原18 focused已过；buffered Stop诊断同父外/内两次过，旧5.27s失败未解释。剩24先过15在旧output fixture失败；适配后last9已过6、GUI真实tool completed失败，后GUI restart/paired未跑。root核真实App已有worker入口，GUI integration默认libtest无入口；现获core/Cargo.toml与tests/khor_agent.rs最小harness=false/custom main修订及GUI2+paired1 runner准备权，先freeze不Cargo。原生产仍不改；link/proto/lib未来写权已N1/root。R2仅契约未实现。 |
| node_test_review | `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-store-owner-recovery`，fix/quality-terminal-current@**13a9459f**+Q052候选01 | Q052作者，12变化路径/1921inputs/28双盘file已冻结，23父runner准备未运行；源码/实施卡已交root，不能自审。独立N1 followup审已交root，确认计量/部分Exchange两问题。无Cargo或源写活动。 |

已复用原worktree，不新建target。send_message不唤醒completed agent，动作需followup_task。作者不能自审；活动源按实际diff/hash优先，不能从摘要覆盖。

## 当前构建槽与资源

- **当前槽空**。最近R1 output-fixture-and-last9-20261026T221818Z / exec93250 / runner13546 / Cargo13559已经结束，root现场未见runner或cargo/rustc。下一仅GUI测试入口修订冻结后另授槽，不自动续旧清单。
- 最新必要Node libtest编译51.464s成功；last9中4个Node父和2个agentwire父绿，GUI network父因真实devices tool未completed失败，GUI restart与paired未运行。源2350前后稳定，记录组退出/TMP清理齐；旧15绿和Stop两次诊断通过不累加为全部完成。
- 统一target `/Volumes/disk0/Khor/caches/rust/khor-quality-target`；jobs1/incremental0/lockedoffline，短独占TMP/明确STEP-TOTAL-MIN AST与实际引用/源前后/实际binary/有限TERM-KILL group drain/双盘。内外最低5GiB=5,368,709,120B；掉盘/越线/连续3IO只停自有组。用户已允许在外盘风扇损坏条件下继续。
- R1首次25组构建阶段GUI触资源线5,304,504,320B自动停；stub与agentwire编译成功保留，paired/25父没跑。后资源恢复只续GUI Cargo0+paired Cargo0，未重编已成stub/agentwire；actual GUI fresh=true仅因同源partial fresh=false事件+完整binary hash匹配且本次Cargo0才认可，不回填历史。
- storage-audit-11：精确3已完Q033 test缓存320,048,608B；12：精确21闲置gui-core/seen_snapshot/console_receipts test缓存，Cargo depinfo/UID/nlink/inode/SHA/lsof0齐，逻辑4,240,794,944B但实际只增加约0.6GB，不能说物理回收4.24GB；13：保N103两个实际exe到内盘后移除共享副本431,509,448B。源码/证据/App/release/依赖rlib未动，不重放旧清单删新文件。
- 当前最近外5.74GB/内11.3GB，须每run动态测；正常artifact后续覆盖不回填旧SHA，缓存清理不否认历史结果。

## 已保留的实际运行制品（避免共享target覆盖迫使重复编译）

内盘 `/Users/zgh/Library/Caches/khor-code-quality/recovery/runtime-artifacts/`：

- `R1-8242b5bd/manifest.json`：03 CLI、05 libtest、实际stub/agentwire/GUIagent/paired六个origin→copy SHA/size/mtime核齐，debug/deps布局保留；旧05原件不覆盖。
- `R1-stopdiag02/manifest.json`：新增buffered诊断02 actual libtest **759961a3195e639ca515650493ad823da536c534d0d3bde414d9ebee088d0cd3** /236,377,752B，另五copy引用前一manifest。剩24已实际按这些copy env/path运行，不假指sharedtarget。
- `N1-candidate03/manifest.json`：实际unit11 libtest356acc6b…/234,023,624B及N0 exe两份，旧source/run receipt不改。下一源码不能直接冒旧binary绿。

各manifest在外盘同任务证据有副本；复制件的运行path单记，不篡改原制品历史path/mtime。产物只作确切源码的有限验证，不能当最终App。

## N1 / N2 当前事实

- 原N0真红：合法4096×64设备地址历史，25,447,954B差集，旧4次public sync cap不收敛但Seen123/Pintrue推进；fixture commit dd60fc5d与audit ref保留。原Pair04真29,224,082B回包失败并烧票，同caller retry badinvite，1普通父红；之前Pair02/03仅准备失败。**巨大Pair/Vouch与可恢复claim N2未实现。**
- N1单DocTransfer1MiB encoded/256KiB块，Doc registry统一、准备/固定Ready、checked flush→durable receipt→Applied/ACK已有实现；normal check01过15.307s，除后续列出问题不声称已收敛。旧正常Sync/Synced/base64已在候选中删，尚未集成。
- 01独审raw入站1MiB漏门与最大request proof方向已02修。实际B1同一真实Node父先红（padded Write/Confirm被接受并创建state）再绿（exact cap Refused且state未创建），两段正常256KiB Stored对照均过；root核35/36双盘file、1083source、actual normal/test fresh=false、PGID/TMP齐，收据Q039/node-n1-b1-root-audit。
- 02 unit构建暴露旧cfgtest一个str→ByteBuf期望漏改，原编译失败保留；03仅同内容byte vec一行修订。**03 compile+11不同exact父通过**，root逐raw/144双盘file/1083前后稳/真实exe保留独审。编码实测最大request590,328B/response721,546B，canonical chunk262,144B。
- 03原N0实际Pair与25,447,938B准备成功，首visit约30s后硬 `document transfer state busy`，停在不接纳无关错误的前置断言，未读首Seen/Pin/最终VV，不能称旧cap红或收敛绿。root确认三处BUSY(&str).into误成Invalid，04仅3×Error::busy()+删常量，未编/未运行04。
- 独立followup另确认两生产缺口（见Q-039-node-n1-followup-review）：已收发body后的Busy/本地错误会丢bytes/chunks预算，初Exchange可能在剩1块时发双向2块；初Exchange提前started=true后receipt Busy，纯pull尚未记录就可能Work.done且缺Pending。下一次Versions可恢复，不声称永久丢数据；仍须修复本visit真假完成。
- root已批准N1先实现04生产加窄cfgtest真实wire观察和4个red父候选，独审及真实红后再最小Step计量/typed结果与部分Exchange Pending修复；真实quiet1Versions、双向多Doc实际wire trace含Busy和剩1块边界、receipt gate Busy→Pending→释放重开public sync完成的红绿；限定新doc_transfer_flow_tests.rs与link/doc_transfer_state中的私有cfgtest观察点，生产无env故障后门/新依赖/通用框架。实际wire tag排除内部直接dispatch，真实gate/IO制造Busy，按root有界记录并RAII释放。
- postflush/prereceipt实际kill/reopen也须闭合：旧11只在Complete已持久后重开，不能代替该窗口。原N0 max4/25MB/VV/content/SeenPin、visit30s/32MiB、chunk256KiB/body64MiB均不放宽；05修闭再重冻后排适用验证，04 runner保历史不自发运行。

## R1 / R2 当前事实

- 已知child投递R0原六Prompt/Stop、四held Cancel释放后第五六不达真红→修绿。03原R0+auth两父、05新16父共18不同focused父通过，root逐2350源/19冻结源/raw/双盘/两binary/161PID独审；checkpoint8242b5bd=受测19源，未集成。
- paired02新真实接受父源码已root审：ResolveBinding→BoundSend/Get→actualACP permission，错caller/home/反向绑定走真实认证transport，原Task不变；有效Cancel→freshGet Canceled，原parent/单Task保持，cleanup先断言。Unix helper guard已修，非Unix明确unsupported不假pass。**目前该父只编译，仍未运行。**
- 第一次相关runtime首原MCP Stop父出现5.27s `Stop admission blocked`，0/25；原raw没有逐call数据，不定性fixture或生产根因。诊断01在critical window打印，16 calls实际约161ms但日志/调度间隙总1.356s又触门，该诊断不能当纯生产延迟。
- 诊断02改为32条有界内存、原cleanup完成后才输出，原16calls与1s门/全部后续期限不动；同新binary同父外盘原CLI→内盘copy两attempt均过，分别276ms/213ms、Persist owner各1、Pi/claim/map/nextPrompt/lateReply全部原oracle齐。distinct只1父，旧5.27s仍未解释，不能以两次pass抹掉。source2350/64日志/10PID/两CLI+新binary/双盘齐。
- 剩24 runtime-only **15父绿、case16输出fixture失败、后8未跑**。root实读确认旧测试强求Saved.data_json.remoteCancellation pending/confirmed，而R1已按契约移至独立intent/receipt；fixtureSaved.owner也未设真实Node。主4组脱敏/空key/Unicode/redact-once及已有metadata保留oracle必须保；root只授权该test源机械适配真实owner、验证无旧竞争projection、bounded回收local_retired元数据owner后退出，若读cancel状态须走真实authority，不伪造。先delta/freeze，不自动编或续。
- output fixture机械适配已冻结并编译：Saved.owner设真实Node，旧竞争projection确认不存在，bounded回收metadata owner，保4组原脱敏与oversize oracles。last9前6父通过；GUI network失败于devices-1 tool completed断言（模型仍回network ready）。只读根因：Node normal依赖self-exec GUI integration默认libtest，而其缺_mcp_worker入口；真实CLI/App已有console→mcp worker分支。因此授权仅core/Cargo.toml的khor_agent harness=false与该test自定义main先调真实worker入口、严格两父选择，原body/断言不放宽。新候选/GUI2+paired1验证待审待跑，不能称GUI或paired绿。
- **R2未知首次Send task发现/lateSend fence未实现**。已有只读契约：native按Binding/caller/agent/message及原parent直接lookup+与Store.accept原子fence；HTTP无远端能力时诚实Unknown，不重放Send/Answer或从List起点无限扫描。实现待R1收尾另授。
- 集成R1须先原R0 seed f0104d73，再固定生产8242及后续必要test；N1三公共源另由root合并意图，不copy整棵旧树。main/ship仍无授权。

## Q052 与剩余完整目标

Q052 terminal current candidate01：host精确khor-host/5+首Version5，绝对整frame deadline，删Attached/versioned wrapper/Term版本降级；保当前snapshot/nomodelring/tmux/history/rendererReady/Forget/reconnect。3生产+tests 12路径，1921inputs/28双盘file，root独立源码审无确认阻断；计划23不同普通父，native_handoff仅compile，tmux无skip才记绿。runner准备但**未编译/运行/集成**，不把旧兼容测试删除当新绿。实施卡已交root，validation-plan卡另有冻结准备。

完整剩余N1/05真实同步及N2、R1相关收尾/集成及R2、Q052运行、Q033真GUI→Tauri→Rust上传、Q035真实borrow lease、P1余下模块深审、P4命名/职责/错误/失效历史/工具链治理（Q053已实测205第一方Rust格式差异且未改源；Q054本地typed IO边界待实施）、P5最终同源App全流程；Q041剩fresh onset/远程/Linux/musl/Rust嵌入，Q045 SSH/master/远端lease依任务卡继续。物理iPhone/IME/Windowsruntime未知，不以类型/元数据冒runtime，不关闭完整目标。

## 恢复入口

外盘证据 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`；内盘 `/Users/zgh/Library/Caches/khor-code-quality/recovery`。掉盘先读RESUME/current-code-quality-docs/current-worktree-refs.json，再核现场；源WIP不能按摘要覆盖。详细来源见对应任务卡与历史，不累加局部test count当全仓通过数。
