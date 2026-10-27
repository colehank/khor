# Q033：统一当前协议与最后原生上传取消验收

2026-10-27，owner `node_test_review`，只读定界完成。固定 wave1 `integrate/code-quality-wave1@686f2e42eb634ff5ba0edc34ab822fe8764f13f2`，现场clean。只写本卡，未改源码/依赖/公共文件、未Cargo/native/gen/browser、未启动fixture或清目录、未提交。

已读 STATE/PLAN、Q033后端/GUI/独审/drop卡，Q015及其review，Q025/native_io说明和Q010-native-run11真实App入口。用户最新“未上线，不考虑旧Khor端兼容”替代Q033历史mixed-version矩阵；外部ACP能力、认证、epoch、用户数据保护不随之删除。先graph minimal/semantic：图build1d2640、84 files、不匹配686f且upload查询0，随后核实际生产源；无.codegraph，不把图0当覆盖。

## 当前生产旧Khor分支与最小删除

| 位置/实际链 | 当前作用 | 最小处理 |
| --- | --- | --- |
| node/gui_host.rs:178–194 `GUI_GREETING`/Attached.upload_cancel/`why == khor-gui/1` | 成功Welcome内容仅作为bool，空/陌生why自动进入旧host处理 | GUI connect只接受当前GUI greeting；不匹配明确协议错误，不降级。保cookie、握手deadline、错误why与GUI/terminal协议区分；不要给GUI流额外发terminal ClientOp::Version。Attached可只保stream，无须为删bool扩大接口重构。 |
| gui-core/chat.rs:275/447/493 | attachment存upload_cancel bool，far rejoin时重算 | 删该字段、初始化与重连赋值；初连/rejoin均走同一严格current GUI connect。正常far重连本身保留，不因“不兼容旧端”删除断网恢复。 |
| chat.rs:854–875 `legacy_upload` | 上传中遇旧host，50ms尽力Stop、通知reconnect=true、left/gone/关队列/独立close | 删这个能力条件专支；所有正常current host上传Stop均写同一Stop→discard→Parts路径。不要无条件close使“取消成功”依赖EOF。 |
| chat_send.rs:79 `StopAction::Disconnected`；chat.rs:765 | 仅上述legacy专支产生的内部结果 | 删此variant/分支。Local/Written和StopWriter完成/Drop错误继续保留。 |
| core/tests/chat_upload_cancel.rs 的legacy flag、Welcome why.clear、显式重连专case及committed中legacy loop | 人为混版本测试 | 改为current-only fixture；撤混旧host测试，不计它“由红转绿”。保current大上传取消、同socket下一turn、已commit正常agent Stop/不退attachment、settings/权限。 |
| 同文件old_client_upload_stop…/host Reassembly残Part测试 | 名字带old，但验证Stop后残Part/Parts不能复活、原socket可用 | 保可观察oracle并改名为残余/重复片段防复活测试；合法当前驱动/迟到残片仍需要该防线，不因名称批量删。 |
| GUI ChatView:981/1002 的reconnect notice、api.ts/token可选与nullable帧 | 不全是旧Khor兼容 | **保持。** current host上Stop writer或末Parts失败仍可能退役，需要结构化reconnect=true；不解析Err字符串。null token无可靠草稿归属，安全展示但不覆盖当前稿的规则也保持，不把本地optional API当旧网络协议。历史“legacy”文字可改准确描述，不为了删分支改tokenDTO。 |

Shared terminal协议另仍有真实旧Khor路径：`host.rs:161` advertised空/未知why→1；`:756` min(peer,HOST_VERSION)及仅>=2才发Version；`:2187` 非Version首帧→1并把旧Input保留；按agreed选择ring/snapshot与Forget/Scrollback/Ready；gui-core/term.rs:201/660/1492的version4/5/2降级。它们是相邻current-only清理候选，但不应塞进Q033上传小片。后续独立PTY片可要求当前greeting+当前Version并删只为旧Khor的fallback，保当前ring/scrollback/Ready本身用途；不能只删版本判断误删tmux redraw/正常历史或放松认证。`host::connect_welcomed`仍供两协议握手，不整体删。

**外部ACP适配不动**：`crates/acp`的image/embeddedContext、load_session、list/resume/close与MCP transport能力，gui_host Settings/Capabilities、agent permission Ask/Answer、mode/options，以及适配器真实拒绝均是当前功能。尤其现upload-agent.py `initialize.agentCapabilities={}`，core测试绕过GUI直接传image所以能运行；真正ChatView验收要让专用ACP fixture正确声明image/embeddedContext能力，不能伪造Agent frame或删GUI能力门来露出附件按钮。

## 必须保持的当前发送/停止/收据边界

实际native链为 production api.ts chatSay/chatStop → Tauri `native_io::dispatch`（原invoke key/origin/ACL + CommandItem解析）→同步submit入attachment controls/turn双FIFO → `Sending::before`在每帧writer锁内判Queued/Uploading/Committed → current gui_host Reassembly Stop/discard-until-Parts → ACP。native完成Future可等writer；HTTP/bridge的chat_say admission后即可null，两者均靠结构化SendCommitted确定本地写commit，不按Promise resolve清稿。

保首帧前本地取消、Uploading Stop写线后才发空Parts终结discard、已commit输入走原agent Stop/Turn；SendCommitted仅Whole/最终Parts完整写成后一次，不等于host/agent已接收/执行。取消/写失败/queue drop不能发commit；SendCancelled/Unsent精确token关联，token不进入GuiOp/ACP线。Stop的Ok不等于远端已完成取消，结束仍看host Turn。旧attachment closure握原Arc/owner，close/reopen同id或同文新token不能串；StopSignal与队列drop完成错误不能删。

### 适用定向验证

当前协议删除属明确用户决定，不造mixed-version兼容绿。root授槽后仅：

1. 真实GUI connect当前greeting可用、cookie错误/陌生或空greeting/错误terminal greeting明确拒绝且不额外发terminal Version；沿原handshake deadline拒绝silent peer。
2. core io_tests按实际普通父去重：同步首帧前取消、真实分片Stop控制插入、写失败/queue-close原token singleton收据、HTTP admission先null后真正commit、close/reopen旧control不落新attachment。改字段不能弱化这些断言。
3. 真current host+独立ACP log：上传中取消prompt=0、残Part/Parts不复活、原socket下一turn恰1prompt；已commit小turnagent Cancelled且同socket/settings/权限仍可用。撤旧host降级矩阵不当新通过数。
4. 原native_io两参数/ACL/admission测试：optional/null/有效/无效token、unknown invoke fallback与remote ACL保留；GUI current草稿token、同文新token/stale receipt与Stop真错误场景，标准生成后types一次。不能只Promise mock宣称native全链。

## 最后GUI→Rust真实验收的最短可执行计划

**现有入口的边界**：`check-chat-cancel.mjs` + `chat-cancel-wire.mjs`安装假的 `__TAURI_INTERNALS__` 并注入synthetic帧，只证明React/registry。`native_io.rs`两个tauri::test mock_builder测试执行真正参数/同步admission但无实际webview。`core/tests/chat_upload_cancel.rs`真host+Python ACP+TCP首Part gate执行core API，没有ChatView或Tauri invoke。Q015是notification只读ACL任务；run11已用真实App验证nullable query，不能据此称上传经过Rust。`check-native.mjs`是AppKit gate且默认mtime挑binary，不是现成上传runner；本任务必须显式same-source binaries，不能直接默认跑它。

**优先一次macOS quality App同store链**，最省去iOS跨sandbox身份准备：

1. 冻结最终current-only源及frontend生成结果，由root唯一槽正常构建同源quality GUI/CLI与一个test-only驻留host worker。复用现gui_host_main/first-Part中继/独立upload-agent，不写生产新服务或debug权限。Mac App和fixture用同一个本次私有KHOR_HOME，由Node正常建立owner/session/host文件；禁用mDNS、只私有relay、私有HOME/tmux。先验证App自己的 `Node::far_machine=None`、session_dir真实存在、host-file路径命中gate，不能复制用户store/伪造设备owner。现core fixture随单test Drop退出，需最小“ready后保持至stop”的测试worker入口，否则并无可供App连接的host。
2. 该worker从真实host ready导出本次session id和gate控制文件到私有证据；cookie只私有文件不日志。TCP relay保原Welcome，记录连接编号与帧种类/长度、首Part已实际转发、Stop/Parts次序；不记录图片/base64/token到host wire。独立ACP进程保JSONL request count/text标记；声明真实可接受的image/embeddedContext，提供mode/option/permission以沿用现fixture断言。gate有短总deadline及独立进程cleanup，不能300s无限驻留。
3. 在真正quality WKWebView加载当前ChatView和production api.ts。优先无需bridge，令所有chat API直接实际Tauri；如复用run11纯origin+测试Vite，api测试transform只把明确白名单强制真实`@tauri-apps/api/core.invoke`，其余页面数据可fixture。白名单至少 `chat_open/poll/wait/say/stop/leave/replay/set_mode/set_option/answer`，使用chat_put附件路径时也必须真实invoke；禁止覆盖 `__TAURI_INTERNALS__`、合成SendCancelled/Committed或假chat_open/session分支。观察层只旁录真实call/token/returned frames（附件仅长度/hash），不替换promise完成或帧。run11 retention-server的enforce:pre白名单模式可复用思路与sourcehash证据，不能把旧App/Rust库当新686f验收。
4. 真实UI打开该session，确认来自host的Agent/Settings，选择本次合法大附件（例如本地生成的有效PNG，使SayWith encoded总量明确超过单frame且<=64MiB）并输入带空白正文。可以测试driver向实际file input投递File/真实change事件，文件选择接缝需注明；附件处理、token生成、生产say/stop与Rust传输不mock。点击Send后等待**真实relay已收到且转发首Part**并记录总未commit状态，再点真实Stop按钮，释放受控gate；不以固定sleep猜上传正在进行。若门控/时序没达到则SETUP-BLOCKED，不算取消绿。
5. 同时核三方证据：native调用有token A/Stop；真实socket先Part、后Stop、最后terminator Parts，且没有原payload被commit到ACP；ACP `session/prompt`恰0，UI只收到A的SendCancelled(reconnect=false)，没有A SendCommitted/重复Unsent，batch.gone=false，草稿/附件恢复且原optimistic行只撤A。日志保存完整性/hash，不长期保存大附件或凭据。此处才是“GUI→Rust实际上传取消”。
6. 不重连/不重新chat_open，使用**同一文本**再次实际Send，生产生成token B且B!=A。保持原连接编号（accepted=1），只B SendCommitted，ACP恰1prompt且正文/附件hash对应B；完成真实Ask/Answer→EndTurn。再做一次已commit小turn Stop，ACP实际session/cancel→Turn(Cancelled)，连接/settings仍可用。旧A收据若仍在poll/replay缓存，不能撤/恢复B；若要控制A/B迟到顺序，只能在测试观察/返回交付层延迟**真实A receipt**，不能注入synthetic帧或延迟原Stop请求去影响B。最短主流程先覆盖真实A→B，既有组件stale-token矩阵提供额外UI时序覆盖，二者分别记。
7. 每项独立PASS/FAIL/SETUP-BLOCKED；保same-source manifest、实际GUI/CLI/helper/ACP hash、generated inventory、invoke/relay/ACP时间线、截图/必要xcresult（如选iOS）、ownedPID/PGID与cleanup receipts。只停本次quality App/worker/relay/ACP、wait/join核ESRCH；无main/ship/用户服务动作。

如最终要求iOS App也实跑，再复用run11专用quality bundle/simulator/Swift driver和纯origin devUrl。它的store必须在本App container正常创建，Mac GUI host不能靠拷owner stamp伪装local；可走真正隔离paired far session + reach或先证实合法attach链。该跨sandbox前置尚未具备，不能把macOS验收冒充iOS/物理设备。iOS Q015现成native query可作“真实invoke活着”正对照，只是一项前置，不替代上述chat调用/ACP独立oracle。

## 固定源指纹与交接

| Path | SHA256 |
| --- | --- |
| node/gui_host.rs | `d2692b1bf8806d5edcb92df3dc310781275c4f44f3fc037d13806af2e9efcb2b` |
| node/host.rs | `1be8b7ff95558b9dddb370f7766cac5440110548fd627a86797b19c9cc2b9093` |
| gui/core/src/chat.rs | `7c503a161da3a2f7d6ed3b6887e815eeb67a17938da893880530582a7d304618` |
| gui/core/src/chat_send.rs | `5017cb830a61665985aa6f04572b556637f713eeadb61a21c3d6890906540ab4` |
| gui/core/src/chat_io_tests.rs | `1622b571b2d32ca68654a10c0ad339bada43f151680fc0382fa807da4838376a` |
| gui/core/tests/chat_upload_cancel.rs | `a04c57baa19817d8684734e5cb495d79ab0ee2d8db5fa5ce4f76dc25e8f05596` |
| gui/core/tests/fixtures/upload-agent.py | `88bea447c9b916b8bed8a505a7348a8402793b1ccf7151b584cb6baf0aa781a3` |
| gui/src-tauri/src/native_io.rs | `d8f0e291b7123045df4e90abec37f860dc30aeeeeeb314127ec3c350ff3ff403` |
| gui/src-tauri/src/lib.rs | `c8b5ae2f455f7b9058dc92d20693633983dd1b7f28cc7512546ba195321b9dfb` |
| gui/src/views/ChatView.tsx | `90830b697861af48b7f962bd59e75e078f7335e15d588817d35b1d653a56843d` |
| gui/src/lib/chat-drafts.ts | `6d9167492a1eaa5acadc862978600856bfdb5c1d2c1a6f4a41f7551aabbadc2b` |
| gui/src/api.ts | `477e50591996a6795fc173b81369494c270382ab2e2eb2252c62ae1b51658c43` |
| Cargo.lock | `4436a386ad74fdceec35dbfcb67ecbb32d593bfff56badfc92e9d5f5437bf9e6` |

表中node前缀为crates/node/src、gui前缀为apps/gui；指纹属于686f现场，不复制旧Q033作者binary来源。实施候选allowlist先限gui_host/chat/chat_send及定向tests，GUI只改已失效说明/必要断言，catalog/schema尽量不动；生成器Q041e、onset Q041d和terminal current-only后续分别由root安排。当前仅方案，无新增运行红/绿或完整Q033关闭结论。


## 既定P5驻留接线source01（root协调记录）

node_test_review已在integration3a0104a9只改原chat_upload_cancel.rs/upload-agent.py两test文件，未提交/编译/启动。普通cancel_upload与三ordinary父尾部逐字保留；新增ignored app_upload_host仅驻留helper不计业务父，复用正常Node/gui_host/现首Part gate、私有0700控制目录与0600 ready/wire/ACP元数据文件，真实App自行第1连接、同Root本机session。helper270s驻留+有限close/join，总<=300s；实际App外层由现owned-entry记录/控制全部组，setup/未到gate不计绿。

双盘Q033/app-upload-helper-source01四文件（2源/complete.patch/receipt），patch f7dbbf6060a894db3fe94a4150d4751758a805e5fb119be421e4444dc6384b11，Rust1c9deb4f899eef7203b9f7b9c517e80e0559082679cf5c678e54a8983ef32cc9，Python1b7fd357638c400d7a19d0db51cbba555f051a0e442a05200f6f95292f901f48。root/shutdown_owner源审中：新capabilities须仅驻留redacted分支，不改默认ordinary初始化；新增bounded cleanup未确认不能仅打印而默认父成功。其余原setup构造前owner gap先比HEAD，若原已有独立缺口只记下一轮，不借验收新建框架。无生产权限/API/服务或新普通测试变化，尚无实际运行证据。


### source03静态独审并提交，尚未native/App验收

source01三项新增fixture问题已按root/非作者窄审最小纠正：capabilities仅resident(redacted)模式、Drop正常时cleanup Err使test失败/已有panic仅诊断、Close发送失败不盖过已确认host/relay正常join（只有未结束/join panic/host Err判cleanup失败）。原Self构造前setup gap确属HEAD既有，下一轮记录；当前必须外层owned-entry300s/实际PGID控制，未到firstPart或auto-release explicit=false不可计业务绿。root已核source03五文件双盘与current两源精确，原ordinary父/断言跨delta完整保留，0编译/运行。

source03 Q033/app-upload-helper-source03，delta7881c46bdd5fe8707741ac5745c183e5d9617d9dbcb595922fc0a0486a635139、fullpatch41d28a171b30b3cb6ebac2af4b70d639e86a32ce96da2122a839e4299d4fbeda；Rustdc8bfd77f54448b7aa7b71767c110f60a79fc8ff5605d491d71e1d7eebeda08a、Python38e21d14c67a8df86fac9ae5bdc6d65d36562454c7375f3cfc449f63ffb1cb23。root只将两授权test文件提交a3a7cbdebd125fa1145c73f399ad5992e42601cd，integration clean；source01/02原件保留，不冒真实上传取消或App Exit绿。最终需当前同源GUI/CLI/testexe+真实UI/host/独立ACP oracle及正常退出。
