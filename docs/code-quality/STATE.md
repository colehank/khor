# 当前代码治理状态

更新：2026-10-27，close03 / N1 flow06实际红 / Q052首构建前提失败检查点。完整目标 active，未完成。先读本页、PLAN与任务卡，再核实际 HEAD/diff/agents/PID/SHA；不得按摘要覆盖未提交源。上一完整检查点见 [历史](history/STATE-before-native-close03-20261027.md)，旧阶段证据继续保留，当前事实以本页为准。

## 用户决定与交付边界

- 全仓功能审计、按风险修复/重构、Clean Code工程治理、最终同源App验收；多智能体与压缩恢复贯穿全程，不能以任一首批或库基座代替整体完成。误发AEC撤回。
- 未上线，不保旧Khor端fallback/迁移矩阵，统一当前协议；认证/epoch/数据保护、平台与外部ACP声明能力保留。
- 本轮没有main合并/ship授权。main `/Users/zgh/Desktop/working/khor` **8704a503a27b36ad58f922690de50e766f8bd485**，当前clean；既有用户App/服务/release不动。
- 权威协调目录为本目录plan/code-quality；实现树同名文档仅历史副本。外盘新资料均在 `/Volumes/disk0/Khor` 分类内，不新建散落树/target。

## 已集成

`/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，`integrate/code-quality-wave1`，**670347d766191b2b30fbc0e91f31fbf4e87cb3c2**，当前clean。N1/R1/Q052尚未集成，Node仍旧Sync/inline Pair。

- 首批GUI：键盘/最新消息、commit文件跳转已修，真实Tauri iOS run11与后续MobileSafari/UIKit26指标、ACP/Khor尾部/历史锚点有证据。最终同源App和物理iPhone/IME仍未全验。
- 编辑/身份/资源/agent：autosave/conflict/lock、identity/purge、FIFO/connection owner、凭据裁剪/A2A/输入预算、upload receipt/poll、SSH生命周期分批独审集成；精确范围见AUDIT/各卡，局部绿不冒全用户流程。
- Q039库：strict apply92742250、descriptor d16d83df、init f721a2c3、outbound5da91f03、receive67927fcf、lookup/resume686f2e42；实际SIGKILL/有限组与来源独审齐。库FlushedImport不是网络ACK，Node接入另在N1。
- Q037 owned MCP：e0c6abec/4493ed3c已有process回收/claim释放；远端可靠取消R1/R2另做，Windows静态/类型不冒runtime。
- Q033 GUI current：13a9459f→767aa23d，5生产+3tests，严格当前greeting、删旧upload_cancel/legacy_upload/Disconnected。normalCore+greeting4+coreIO12+真实host/ACP3=19不同父独审过，真GUI→Tauri→Rust→host→ACP未全验。
- Q041e bindings生成：c89aa135→670347d7，3源；真实多3旧TS红→11父/Busy/SIGKILL/标准gen/types绿。root作者1922/190dual核齐，集成又normalbuild64.098s→gen6.729s→types5.164s，1923inputs稳定，157TS→154TS+管理marker，恰清3旧export。后验npm TMP94项另凭精确清单后清，原非空收据不改。仅本片闭环，全Q041/P5未完成。

## 当前写权、并发与构建槽

| owner | 树 / baseline | 当前授权与下一步 |
| --- | --- | --- |
| root | wave1@670347d7、plan总账 | 公共接口/集成/独审。已改R1树link.rs的serve_as显式acquire及Tauri真Exit close两产品调用方（caller01，未编）。已新增版本化native独进程入口与Unix CI接线，正在非作者审；负责N1 runtime独审、恢复镜像。 |
| node_transfer_owner | `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery`，fix/quality-node-doc-transfer@5f2d5cf77ad05bd7e8ce2e11d0b90c36485c7ef3 | 生产仍candidate04；cfgtest06已实际quiet绿、Busy及purepull红，第三quota case setup未达边界。已批准07只新增few-op合法Chat的实际32MiB quota边界父+5个heavy父显式ignore，原4父body/N0/Pair/11保持，不改限额、不假IO。无native槽；link公共写权除serve_as冻结给root外仍N1，勿整树copy旧R1。 |
| remote_cancel_owner | `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`，fix/quality-remote-cancel@8242b5bda383b456a0122fc2613ca4e274620187 | 原19生产checkpoint、S22测试入口修订实际4父过但清理失败。close03已source-only冻结24源/2351inputs/31dual，含root两产品caller；容量union、completed-token oracle、新6父ignore待独审/真实run。link/proto/lib/Tauri后续由root协调，不自改公共文件或编译。 |
| node_test_review | `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-store-owner-recovery`，fix/quality-terminal-current@13a9459f6c2dfd8d12912e6e2325b58de6e06999 | Q052 source02/12路径/1921冻结，root独审通过。plan05实际首build失败于本树dist缺失，23父0运行，已停；只读补证/归槽。获准准备plan06本树真实frontend build前提与actual assets来源，不跑gen、不改源；另独审R1 close03及rootcaller01，先freeze不Cargo。 |

**当前唯一Cargo/native槽空，未授新执行**。Q05205 tool session55941已exit1，Cargo PID/PGID97656已gone、owned_live=[]、TMP清，owner已正式返槽并23dual核齐。次轮先核ps/session，不能因旧active.json重启。plan06准备后root审再明确授槽。R1/N1均仅源码阶段。

统一target `/Volumes/disk0/Khor/caches/rust/khor-quality-target`，jobs1/incremental0/lockedoffline、短独占TMP；STEP/TOTAL/MIN真实AST及引用、源前后/实际binary/有限TERM-KILL+wait/all PGID members/双盘。两盘最低5,368,709,120B，掉盘/越线/连续3IO只停自有组。用户已允许外盘风扇坏条件下继续。最近外14.89GB/内11.68GB，逐run动态测。

缓存cleanup11–16精确清单/来源见历史与evidence，不能重放旧清单。audit15旧76integration-test exe逻辑9.064GB但实际无增；audit16再逐SHA删main debug同76副本，实际增9,063,682,048B。App/bin/lib/release/源码/当前任务制品不动。

## N1 / N2：真实进展与未完成

1. 原N0合法设备历史25,447,954B，旧4 sync不收敛但Seen123/Pintrue推进；Pair04真29,224,082B回包失败烧票、同caller retry badinvite。原fixture及红证据保持。大Pair/Vouch/恢复claim **N2未实现**。
2. N1候选已单DocTransfer、1MiB encoded/256KiB块、32MiB/30s visit、4chunk/1MiB公平quantum；固定Ready、checked flush→durable receipt→Applied/ACK，无Loaded/Sink跨network await。正常check01过，02真实raw 1MiB门已同父红→绿，root35/36dual核齐；03 unit11真过且root144dual/1083/actual exe独审。
3. 03原N0首visit约30s后Invalid `document transfer state busy`，未达收敛断言。04仅三处错误typed Busy修正，无独立04run；当前06生产仍04，不可称N0绿。
4. F1候选05真实cfgtest wire/gate 4父，首compile因cfg移动错+共享旧sync rlib21API错失败，0业务。06只两处clone；production相对04不变，14源/1084inputs/23dual。control03精确重建当前sync/catalog，实际normal fresh=false、dep-info三模块/源SHA、actual catalog.rs两Pending函数/输入/rlib/rmeta齐。
5. **06实际control03** `Q039/node-transfer-n1-flow-red-06-control03`：compile61.244s过；quiet1 Versions 1/0/0过；Busy真实6次，body6,377,994B却计4,805,130B，漏1,572,864B=6完整块，SeenPin/反向实际块/公平与清理先通过后目标断言红。第三remaining1chunk父仅7,602,386B/31frames，30s内未达32MiB前提，**是setup失败，不是预算overshoot红**；第四未跑。83dual/1084前后/actual deps/binary/4PGID/TMP核齐。
6. **purepull实际runtime-only** `Q039/node-transfer-n1-purepull-06-runtime01`：原第四父0Cargo/0mtime，实际receipt gate Busy后first_pending=false、target_absent=true；fresh reopen同manifest82ca4d…/epoch1/545946B完成VV+真实Spec且不Pending后，再于目标断言红。42dual/同1084/actual deps/binary/TMP/单PGID核齐。runner0仅成功采到预期红，非产品绿。
7. root已独立逐raw/4实际summary/83+42dual/历史源前后/实际copy审计：`Q039/root-flow06-runtime-audit`。实际libtest **99b3c42b8404f6a30aa8057489e89e297e5e4ee9d262ea5c3763812ca92b6ab7 /259959096B** 已在Q052覆盖前copy内盘 `runtime-artifacts/N1-flow06`，receipt `Q039/root-n1-flow06-pre-q052`。旧结果只证06，不映射07。
8. 已授权07独立few-op Chat真实quota red fixture，不把原Devices/Agents海量历史吞吐失败改成绿、不放宽32MiB/30s/256KiB。之后才最小Step计量/partial Pending/双向reserve功能修复。postflush/prereceipt真实SIGKILL重开仍必需，旧11的Complete后重开不能代替。

## R1 / R2：真实进展与关闭机制

- 原R0六Prompt/Stop、4heldCancel后第五六不达真红→绿；03两父+05新16共18不同focused过、root2350/19源/两binary/161PID独审。**R2未知首次Send lookup/lateSend fence仍未实现**，契约在owner卡，HTTP无能力诚实Unknown、不重放Send/Answer或无限扫List。
- 首原MCP Stop有5.27s失败；diagnostic01 critical-window打印又1.356s，不能纯生产归因。buffered02原16calls/1s门不改，同父外/内两attempt276/213ms过，只1distinct父，不能抹掉旧outlier。`remote_cancel::request`仍可等metadata5s+250ms，真实2s startup latency复现仅提议未做。
- 剩24先15绿/旧output fixture失败；机械真实owner/去旧projection/有限drain后last9前6绿，但GUI tools因默认libtest缺真实MCP reexec入口失败。原证据保留。GUI测试target现harness=false/custom main真实worker入口，stream8子oracle原样+实际owner/drain，S22/2350inputs独审齐。
- **最新4父** `gui-entry-stream-four-20261026T230149Z`：stream、GUI tools/network、GUI restart、paired真实两Node/relay/ACP认证绑定与Cancel均1/0/0通过，编GUI63.632s/Node51.458s，实际5binary稳定。整run仍exit1：后台metadata在GUI fixture删除root后重建**14regular files/1752B**，不是清理绿。17PGID/22PID死、0forced、lsof0、60dual、原22/2350稳定；root `root-gui-stream03-runtime-audit`独审。原r1h残留保留待精确后清，不改原失败收据。
- 该真实晚写红推动owned agent close API，不拿Saved terminal当producer退出屏障。close只指本进程agent producers+有限cancel owners退休，不是远端cancel完成/全Node IO shutdown。deafen/suspend原义保留。
- close01独审确认冷Root Create、otherRoot错误分类、无持久closed fence、Prompt重放record及spawn失败writer缺口；原误猜Scheduler testliteral已撤回，不改旧16。
- close02补显式acquire/有界closed、NoCreate、config→executor admission、Arc claim、join锁外且保1meta4network实际occupancy/原completion；6新native FIFO/真实claim父未运行。独审又确认closing冷Root预留未被pump/request计数，可达257；completed-token oracle被尚活executor遮蔽，默认整模块串行也会closed256污染。
- **close03** `close-interface03-20261027T004746Z`：24源/2351inputs/31dual，包含rootcaller01。统一Home/Closed/Closing union容量；cold父255实际Home+publicclose预留下pump/Status/acquire新Root均拒绝且count256；completed-token真实executor退出后第二retry gate先证executor claim free而close Busy。6新父显式ignored，需版本化专用逐exe --ignored --exact，各1/0/0才计绿；原16/18不改。仅parser/diffcheck，无compile/runtime。当前树在该冻结之外另有root的scripts/run-node-acceptance.py及quality.yml CI WIP，最终运行前另聚合当前输入，旧03原件不覆盖。
- root产品caller01：common serve_as显式acquire，Tauri真RunEvent::Exit block_on close typed日志，捕获同Root给serve与close；Busy/error保durable work和既有exit行为，不称active Quit全闭环。两源parser/diffcheck过，独审/实际App待。新GUI teardown须真实close屏障、stopHTTP/join后再删Root，不用任意sleep吞晚写。

## Q052：统一当前终端协议

source02仍12路径/1921，host strict khor-host/5+firstVersion5、共享整frame绝对deadline，删旧Attached/version降级；保snapshot/history/modes/tmux/Ready/Forget/reconnect。3生产/9新父与必要机械调用；原IO3保持，旧ignored旧binary删除非新绿。root source独审无确认阻断，02仅tmux helper -f/dev/null不覆HOME。

plan05已root审12dual/AST300/900/5GiB/11必要target build→对应run→精确exe退休，23不同父，native_handoff仅compile。catalog与N1仅2TOML不同，plan05只刷新2输入、要求actual normal fresh=false/package匹配out_dir/catalog.rs来源及全部10section函数+msg/cli keysets精确，HostGUI存在/Pending缺失，避免旧依赖冒同源。

**实际terminal-green05失败在首Cargo build**：52.12s、exit101，khor-web build.rs:50发现本树apps/gui/dist/index.html缺失；不是Rust类型/业务红，23父均未跑。资源正常、guard null、1921前后稳、97656PGID gone、owned_live=[]、TMP已清，原dual回执保留。catalog/sync/net/Node/commands已partial actual fresh=false，CLI/test无成品。plan06只准备本树npm --ignore-scripts run build前提（tsc+vite，既有tracked gen不改，不启动旧gen/额外Cargo），保存dist与实际assets.rs/UI_REVISION来源；需root短审及新槽授权，不copy别树dist/伪造index，不把frontend前提冒最终App。

## 工程治理与完整剩余

- Q053已真实cargo fmt --all -- --check exit1，205第一方Rust格式差异、1923source未改。功能集成后单批格式迁移，不205ignore。
- Q054待N1后：私有IO gate改typed本地边界，保Storage/Invalid区分，不全仓Error框架。
- [Q055](tasks/Q-055-shared-artifact-provenance.md) ready：共享target路径/current manifest/fresh=true不等内容同源；需实际源/生成/嵌入输入来源与可复用runner。此次N1 sync旧23depinfo和catalog差异已有真实反例，精确依赖refresh只是短期措施，未将原因定性Cargo bug。
- 完整剩余N1/N2、R1收尾/集成及R2、Q052真run、Q033真GUI上传、Q035真borrow lease、Q043 OS拖放/关闭/全屏、Q045真SSH/master/远端lease、Q041 fresh onset/远程/Linux musl/嵌入、余下P1深审/P4规范/测试与工具入口/P5最终同源App全流程。物理iPhone/IME/Windowsruntime未知，不把types/metadata当runtime，不关闭完整目标。

## 恢复与证据

外盘 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`；内盘 `/Users/zgh/Library/Caches/khor-code-quality/recovery`。RESUME/current-code-quality-docs/current-worktree-refs.json须与本次状态同步，掉盘先读镜像并核实现场。R1原03CLI/05lib/stopdiag02、S22 GUI+Node在runtime-artifacts各manifest保SHA/path/mtime；最新N1flow06另存。缓存后续覆盖/退休不回填旧收据，旧binary不冒新source绿。
