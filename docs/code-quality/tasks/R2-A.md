# A 全部ACP的统一观看

当前检查点：**A3已独审固定529d9035并由root集成540ea302；A4本地基座3e9376ee已集成5c612223，Node服务ced98d3b已固定、生产check+12定向全绿，待最终指纹独审/集成。GUI消费接线源码已实施、Core native/真实gen仍待验证，R005及第二轮未完成，main/ship未授权。** 作者树fix/round2-history HEADced98d3b，bridge-candidate-1工作diff；H没有native/Cargo进程，槽已明确交回root/W。

下一实际步骤（原授权路径继续，不重新开研究）：Core SessionRow.content_locator:string|null由Session.home/id/ObservationRef便宜派生、不probe；SessionContent.available.binding:string固定真实content-home/exact ref。沿原session_content/history_*命令名接id+locator或id+binding，begin/cancel改跨resident async，page/raw先claim ClientRead再offthread Node::open，Arc持到最终有界投影；DetailPane按id+locator代次、ChatView still携binding，内容不按metadata owner/row.source路由。metadata owner仍保alias元数据删除归属。真实producer与必要fixture适配后删旧core history_request，未完成前不能称统一owner交付。上述字段/入口已实际接线，仍未gen；App暂未发现需胶水。

固定证据：history/observation-a4-check/fixed-local 六源=3e9376ee；local2 reference5后磁盘floor停、root恢复空间后同exe direct26，总31分批绿（414输入）。fixed-transport 十一源逐git-show SHA=transport4实际417输入；transport4真实0/90.773s，check11.501s＋requests4/wire2/providers6（12），真双Nodecase3.85s。原transport1/2类型红、transport3缺sessionId的malformed ACP夹具红均保留，最后用正式typed SessionNotification取合法note，不改A3 parser/不放宽原断言。未gen/全App，不冒GUI可用。

A4契约：locator/binding版本化；source元数据16KiB、单ID4096bytes、cursor携full selector/view哈希；页/raw只binding+真实home，不依metadata owner在线。既有raw256KiB、实际rmp/IPC wire raw+16KiB、record64KiB、128事件、512KiB响应、8页/512帧/2MiB cache不放宽。payload无租约owner；owner只来自local cookie域/认证conn peer，Begin ACK、单次claim、30s pending、MAX64，IO/parse guard跨真正worker/丢弃await；Cancel不Node重开或provider扫描，也不live chat_cancel。

响应资源另有实际服务Node共享32 lane，QUIC所有conn＋IPC共用：try-admit后才释普通seat，Arc跨IO及(reply,permit)输出，外层跨encode/write/finish、payload先drop才返槽；Cancel/Begin绕lane，保持32不升64。cfgtest真双Node证明32慢IO时Cancel先ACK、32完成大回复停write前仍占lane、extraWindow即时拒、Cancel不提前返槽、显式放门后恢复；准确不冒内核背压采样。Resolve仍旧普通32与Node::open/同步IO不可强中断边界，非全协议QoS框架。A3 writer/gui_host逐SHA冻结无改。

所有source、失败/成功日志、实际count、417前后manifest、cleanup收据在observation-a4-check；fixed-transport与小bundle已内盘镜像。仅终态后核uid/dev/ino/非symlink/lsof空再rmdir本批两个empty tmux helper目录，原tmp-status/result未改；未删除共享缓存/用户数据。自有旧Lab88855核出生/cwd/端口后已SIGTERM，源码/原证据保留，history-lab-stop-20261029.json。

以下为保留的历史现场与既定契约，旧pending/运行PID/“尚未开始”等不代表当前状态；当前以本节及最新固定检查点为准。

A3相关原生31项已分批核实际数（包括4个真实合成host worker各1项）；vertical2→3仅cfg(test)中abort场景隔离root/early-exit诊断不同，生产/Python413输入同源，不伪称一次同全SHA批次。真实producer发布161 TS逐SHA=staging、catalog已核；最终20历史JS/16观察callbacks/types绿。所有native/发布句柄终态，唯一槽已明确释放。最新证据`history/observation-a3-check/fixed-vertical/{commit,source-manifest,validation-map}.json`及vertical-{2,3}/gui-{1,2}；恢复bundle round2-acp-capture-529d9035.bundle，checkpoint-a3.json。

A3已由root集成，A4实际执行如上；活GUI恢复交既定S，H全source搜索/复制仍待原要求核补。下方旧pending/capacity/未提交等语句为保留的历史现场，不代表当前状态。

证据：`history/observation-a2-check/final-1`（固定Rust源与真实producer）、`final-ui-1`（最终15 JS/types/mobile/words）、`fixed`（39文件逐git-show SHA/验证映射/commit）、`web-assets`（真实277文件Vite产物）、`closure-2`（补编译闭包四命令exit0/28.539s，输入unchanged=true）。恢复bundle `recovery/round2-observation-3ff8013c.bundle`。SessionSide.tsx已释放给G仅作band材质接线，H不并发修改。

最新目标授权：Khor SessionId型控制门拒alias，不做alias→native查找/翻译；明确(agent,home,exact nativeID)的Native API不按字符串前缀猜域，合法observe/... native字面ID不被误禁。agent终端只接其经证明的自有TUI PTY，GUI用自己的ACP Host，不能借旁边活host给foreign view授输入。明确shell资源保view/tmux交互。UI不会因自有GUI处于mux环境而误换终端；无Khor own endpoint的foreign记录仍只读。独审已实际核最新源码解除先前目标授权阻断，最终原生与生成物证据已在上述final-1/final-ui-1固定。

状态working；用户“开始吧”授权完整下一轮。root已授A1唯一paths；main/ship未授。
作者round2_history；树khor-round2-history/fix/round2-history，A起点499dbe4f。H已验Lab23836源码保持冻结；生产必要修改的SHA/证据另记，不能回填旧验收。

只读调查/矩阵及40个known launch IDs（不是40安装或实测）见evidence/round2/history/observation-survey/R005-observation-survey.md与source.json。官方当前v1及锁schema1.12.0已核load是恢复/回放并连接MCP，list是metadata，未保证只读多observer；不一概声称load必然抢占。

## A1已授最小纵向切片

新增Node session_content.rs统一已有native source选择；按打开调用typed session_content(id,machine?)，不往SessionRow全表poll放vendor扫描。能力仅Available{following}/Unavailable{reason}，不把readable等同于可交互/可恢复。Node probe和page/record共用source resolver；UI只有能力分支，没有Claude/Codex名单。

允许写：node新session_content.rs及lib内容能力/source选择；core lib/chat/api能力桥、真实gen_bindings producer；Tauri lib async command/registry；DetailPane/ChatView观察接线、catalog必要原因词条、定向测试。必要API.ts真实绑定消费属原H owner。未知原因/远端未接线显式提示，不把Unavailable当A全部完成。native当前空闲，测试/生成先向root领唯一槽。

A1只接已存在native来源与明确Unavailable；A2观察import/binding（不resume），A3全部托管ACP唯一canonical journal及replay事务去重，A4执行home只读history路由仍待后续授paths/实施。数据owner/持久身份和共同读管线见调查文档；不按vendor复制流水线，不顺势开S大范围。

## 实施契约

- source probe只在打开该观察页/显式重试时执行；不spawn、不登录、不session/load/resume、不写供应商文件。
- 本机现有Claude/Codex source由同一resolver解析真实文件；找不到来源与文件不可读分开。远端来源当前显式not-connected，A4接线后再更新能力。
- still ChatView只负责显示/取消自己的读取；观察页不启动live attachment或开放控制。结束GUI也按能力展示存盘内容，不按vendor判断。
- 新types由真实producer生成，不手改TS；read/view支持不能推断恢复或writer归属。
- 最小验证：既有两native夹具及非vendor未知来源；无全表probe；能力响应/前端路由、失败/离页迟到结果与retry；无spawn/load/control。

## A3实施前已确认的必要契约（独审补充，不扩A1）

不能只记录Event::Note：ACP live不保证回显user_message_chunk；gui_host::Say/SayWith直接prompt_with，统一journal owner还须记录已接纳prompt的文字/图片/文件与turn完成/取消/失败/结果不明边界。echo按可靠turn/message关联，不用文本hash吞掉合法重复chunk。

start_resume初始session/load的Note在Event::Ready前排队，尚不在GuiOp::Replay状态。startup load与显式Replay均须独立snapshot事务/generation，Ready/fence后提交、失败保旧；不能只以replay.is_some分流，避免每次重开重复append历史。A3定向用例含两次resume/bootstrap、重复显式replay、合法相同live chunk。schema未提供通用去重ID保证。

## 独审修正与最小binding前置（root已授）

A1初稿只按leaf先Claude后Codex会串source；同id换home首render可复用旧available，均为固定前必须修。首次native3项为2过1失败，exit101/76.679s，实际是Codex fixture用了parser不接的response_item message；原日志observation-a1/node.log保留，不是产品行为失败。gen/catalog未运行未发布，Rust源unchanged。曾误用既有check-session-content.mjs名字，已逐HEAD原字节恢复，新检查独立check-session-observation.mjs（不删除/不追跑旧native门）。

root拒绝短命全局FACTS缓存：完整移除实现及every_session副作用，不在list全表poll放source判断。现获A1前置A2最小ObservationBinding，私有version/home/session/provider/完整native ID，存Khor自身元数据、原子发布；session_content语义为按打开解析/登记来源，不能仍称纯probe。Meta可信来源优先，无Meta既有binding优先，否则本次唯一local row/namespace解析；未知/歧义拒绝。page/raw只读Meta/binding，不再猜leaf/重扫全表。binding来源变更进入游标source身份，page reset、raw旧引用拒收。完整A2导入UI后续复用，不混同包。

必要小只读helper已授精确paths：crates/node/src/adaptor/claude.rs、crates/node/src/adaptor/codex.rs。只解析唯一完整原生ID，同ID多备份维持mtime规则、不同完整ID同clean_leaf拒绝；不复制分页/事件解析或写vendor文件。公共S1 Cursor必要source比较仍由H owner维护。跨home前端状态带id+machine query key，不能只等effect清理。

## 最新实现与验证现场

现源码已改为持久binding版，不再包含FACTS全局缓存或every_session list副作用。ObservationBinding version1/home/session/provider/full native ID，私有`.khor/observations/<id hash>.json`，元数据读取上限16KiB，复用files::write_whole的CAS/sync/atomic-rename发布；重复绑定不重写。打开解析可登记Khor自身元数据（不能继续称纯probe），纯page/raw只读Meta/binding，绝不为每一页调用全表发现。

Meta可信namespace优先；无Meta优先既有binding，否则仅本次打开从唯一local row实际category选namespace。两vendor helper拒不同完整ID同clean_leaf，同完整ID备份保mtime；绑定后按exact full ID读，避免另一个同prefix文件抢走原观察身份。统一source ID为version/home/row/provider/fullID，page来源改变reset，raw旧cursor拒收；必要只读身份桥修改adaptor/transcript_page.rs::Cursor::is_for_source。

前端ObservedSession的answer/fault都带id+machine queryKey，同id切换home首render不复用旧available；watch=false首次仍读tail，但已加载快照不再安排700ms timer。源状态查询不在全表poll挂接，失败GUI与外部TUI均按能力路由，没有UI vendor白名单。A1当前仅本机已存在native来源，A2导入/A3通用journal/A4 home网络源仍待后续，不冒全部ACP交付。

JS现场19项历史模块及7项真实ObservedSession函数回调检查通过（是实际函数提取/模拟hooks，不冒浏览器原生）。原160行check-session-content.mjs已恢复，新检查独立check-session-observation.mjs。6项native小夹具已写，含namespace/短leaf碰撞、exact绑定与mtime备份、unknown metadata、Node hosts=false跨重开binding与source改变page/reset和raw拒收。

root已重新授唯一native槽，exec37393，observation-a1-retry/running.json记实际PID/阶段；序列6tests→真实gen空staging→catalog临时stdout成功再原子发布。源manifest含所有新Rust文件与catalog，过程中不改这些输入。旧observation-a1/node.log exit101保留；本次结果与槽释放尚待现场确认，不得恢复时重复启动仍live进程。

## A1固定检查点631c0d0a

精确提交631c0d0a647bb0ed8f7b990ce40119dbcfb9b3cf，17文件，工作树clean，不带Lab。节点6项真实通过（68.323s构建/0.72s执行）；真实gen64.649s/catalog1.224s均exit0，source前后不变；19历史JS、7真实观察函数回调、全tsc、mobile-static[]均exit0/unchanged=true。产物快照observation-a1-retry/generated（含SessionContent/ContentUnavailable与inventory）、catalog.ts、generated-sha.json；root可按固定源/明确快照同步ignored输出。

随后独审指出raw拒收断言原EOF newer cursor本来就invalid range；已只改cfg(test)为有效record cursor先成功读，再换binding断言准确changed错误。root独立核生产prefix逐字节相同，授只跑此项而不重复gen/19+7/types；真实1项exit0/74.137s构建/0.12s执行、unchanged=true。observation-a1-record-cursor/source-proof.json保存前后全SHA与相同生产prefixSHA，test.log/result.json保留。新session_content.rs全SHA为3e27bcda21d811665e7617a512c4b9789938ed0a1a74c9f0c464f4815134783a，生产prefixSHA为4f7eceaa425a501a63c390362a88a9b5246ed360a24595f0c78928d1fb7f4218。旧6绿不冒覆盖修正断言。

所有native句柄已终态并向root释放唯一槽，无全App构建/用户会话操作。独审当前身份/namespace/queryKey主体无阻断，精确ref/指纹最终审待回传。root要求下一A2/3必要UI接线把gui.chat_readonly的“它正在自己的终端中运行”改中性“只读会话/历史”，因为following不能证明writer当前运行；不为此中断A1或重gen。另exact绑定helper可少做目录遍历为非阻断观察，当前只记录不追加优化专项。

A1仍不等于R005全部完成：A2观察导入与控制边界、A3全部托管ACP journal、A4 home路由继续按既定顺序。

## A2已授权实施契约与依赖轴

root已授A2完整观察导入及必要Option执行轴；不使用短命缓存，不借thread_unbound/Idle伪造未知。观察Khor ID为observe/hash(JSON tuple(version, content-home DeviceId, registered-agent key, exact native ID))，和原tui/gui交互ID分离；记录角色为observation，执行状态真None，普通producer Some。记录更新时间/导入时间独立真实元数据，不装StateStamp。统一KindSurface/sessions/list/peer路径承接，不做仅GUI旁路。

既有tmux/shell授权托管导入不被全局改坏。ACP list导入只登记Khor own metadata/binding/roster，不resume/reopen/spawn/load。显式恢复独立动作；任何attach/prompt/cancel/load/takeover/set-option/permission response不得把observer alias翻译成native控制ID。删除只移除本地观察记录/roster，不kill真实writer。content-home从binding核对，不能用row.source(peer报告轴)省略machine而猜本机。

执行轴最小改变core::Session.state: Option<StateStamp>；ordinary Some、observer None，不增第七State。SessionRow.word同义nullable；UI无运行光环、不计Idle；state分组/stale/quiet/dead/unread/seen、CLI/MCP与peer均不得unwrap_or(Idle)。需要记录时间的地方读观察元数据，普通执行时间仍读Some stamp。此为A2领域边界必要前置，不重写S运行代次。

已定位需适配Rust：crates/core/src/lib.rs；crates/node/src/{lib,session_content,chat,mcp,live,list,tests}.rs；crates/node/src/adaptor/{claude,codex}.rs；crates/commands/src/lib.rs；crates/node/tests/{live,tunnel_wire,broadcast}.rs；apps/gui/core/src/{lib,chat}.rs、tests/seen_snapshot.rs；wide等编译器发现的真实构造点同属已授必要适配。新的ObserverKind/KindSurface在session_content owner内，权限不默认继承runtime control。

已授UI接点ImportDialog/SessionsList、core/import桥；必要nullable接线由H唯一写StatusWord/SessionFace/Face/TabStrip/use-status-bar/session-filter/words/SessionSide/SessionContent/DetailPane/api/types。G独占app.css/scroll-edge/CommitPage/MachineCard/check-face；不得并发写，若nullable诊断涉及这些文件交root/G最小补丁。App必要胶水另报root。H的23836 Lab源码继续冻结，不操作用户App。

## A2执行中检查点（未提交）

canonical观察记录新增`node/src/session_content/observations.rs`，唯一身份事实为ObservationRef（metadata owner、content home、namespace、agent、完整native ID），普通Native绑定仅为既有tui来源投影，observe不再另写一份binding。alias为version/home/(namespace,agent)/exact ID的JSON tuple hash；kind observe、execution None；统一KindSurface/list/peer承接。原生Claude/Codex与ACP list默认导入均转观察；明确shell/tmux资源保原输入能力。别把A1 Available.following当writer运行断言，通用只读文案已改中性（本片段真实catalog生成尚未执行）。

SEEN来源前置已授并实施：sync Answer.home保存可空32字节设备公钥，边界Node/GUI core显式包装DeviceId；sync不新增core层依赖。新查询记录实际执行机器；旧None不可猜本机/导入，要求刷新。每份候选opaque payload固定home/agent/exact ID，导入核当前答来源一致；持久ObservationRef独立，不随后续agent列表覆盖换源。手机纯登记在hosts gate之前，查询启动agent仍只在host执行。

共享执行轴已改core Session.state Option<StateStamp>，普通producer Some；activity_at读真实状态时间或观察metadata时间，不伪stamp。SessionRow.word可空，observation_owner独立于content home（为metadata读取归属），必要nullable UI正在接线。foreign agent已发现旧mux/attachable先行绕过风险；hooks Meta/on_record不等于Khor持有进程，最终须由真实host/owner事实限定输入能力，不能只隐藏UI。

root授同一唯一native诊断槽3crate `cargo check -p khor-node -p khor-commands -p khor-gui-core --lib --locked`，sharedtarget/jobs1/incremental0/TMP外盘。check-1..5诊断红与源清单均保留（sync跨层类型/缺构造/partial move）；check-6真实exit0/1.755s、source unchanged=true。证据`history/observation-a2-check/check-{1..6}`，manifest覆盖实际core/sync/node/commands/gui-core全部Rust及catalog，含新文件全文；无行为测试/gen/全App构建。槽已向root释放。此仅生产lib类型绿，A2未完成，不能冒测试或UI验收。

当前A2部分diff与新源恢复快照`recovery/round2-history-current/a2-partial-20261029`；A1 bundle631c0d0a不包含A2。下一步Option测试构造、观察导入/删除/拒控制与home来源定向回归、真实producer与UI接线；A3 journal/A4网络home仍排队。

### A2归属轴与验证进展补充

唯一输入能力轴命名`Session.can_control`，home的live既有seated/sweep同一Procs快照产出；观察false、外部agent即使mux也false。hook Meta、view lens、仅存在/死host marker均不授agent控制。自有host须Meta.pid匹配当前hostPID、进程不得在marker写完后才启动（拒旧PID回收）、host-child归属成立；GUI约定child_pid=self是host自己持有协议连接。显示Shell表面不够，明确shell资源按原Meta.kind或真实shell资源ID承接，包括其原ID被seating折叠。现cookie/已认证handle仍承载每键/Stop快路，仅新绑定重新核当前owner，无新增逐键sysinfo。

GUI旧term::attach的`Err(_) if is_hosted=>Host`能吞新归属拒绝，已删，统一Node→Live terminal_address先正向owner集合再同一私有已授权host/view解析，继续原cookie握手；unknown无事实拒绝。alias的O(1)角色拒绝放host gate前，手机也准确说readonly。新增core真实attach例构造弱is_hosted=true但child无归属，并要求准确ownership错误及无TCP连接，不能拿socket失败当授权绿。

observe ID有意不含metadata owner：同home/namespace/agent/exact ID多机登记是同一观察身份，local记录优先，peer重复按既有row去重。A4凭随行canonical ref直达content home，不能让旧metadata owner在线成为历史可读前提；同源不同owner稳定ID例已写，离线owner链路留A4实测。

SEEN负答Cannot同样带可空home，不能让B的负答阻A；GUI按当前执行home核答案后显示已答，旧来源未知显示需刷新。AgentFacts没有作为此导入能力的前置gate；不新增多home聚合。

编译驱动新增必要测试构造路径`crates/node/src/link.rs`（仅row fixture字段/Some），原生集成tests/{live,tunnel_wire,broadcast}.rs及core/tests/seen_snapshot.rs已作Option构造/断言适配，尚未全target编译或执行，不冒通过。普通测试unwrap其Some事实，突变用as_mut，不改变原执行含义。core wire新增尾字段后Serde tuple超过16上限，独立固定次序tuple-struct承接17字段，保位置检测。

真实证据：check-7、8、9生产3lib exit0且源unchanged。node-observations-2为4/4绿（62.333s）；ownership-1为3/4，手机alias角色拒绝在hosts gate后导致错误语义不符，原红保留、修生产顺序不改断言。ownership-2 owner1/foreign-mux1/observations4绿；随后wire编译红为tuple17类型上限，None/attach未执行。现ownership-3 exec56103继续固定源剩余用例，终态未回传，native槽仍H占用。上述是有界夹具，未spawn真实agent/未碰用户tmux/App。

### A2当前最终候选（等待协调恢复后复验，不是已交付）

作者树仍HEAD `631c0d0a647bb0ed8f7b990ce40119dbcfb9b3cf`，A2约36路径未提交（含新observations.rs），未main/ship。`observation-a2-check/final-candidate-1`保存当前完整changed-source SHA manifest、binary diff、新文件全文、receipt；内盘恢复副本`recovery/round2-history-current/a2-final-candidate-1`，不要用早期partial覆盖。rustfmt只检查23个本包改动Rust路径exit0，这是语法/格式，不冒类型或行为通过。

ownership-3的9项在其固定源真实exit0/138.742s/source unchanged=true，exec56103及全部Cargo句柄已终态，槽已释放。其后必要职责整理：观察导入/登记集中到canonical模块、metadata扫描不再重复读、alias拒run/resume/reopen参数；观察例新增真实已登记peer同源不同owner/local优先输入与native alias实际S1正文断言。**旧9项不外推覆盖这些新源码**。当前最终native/producer批已向root申请，尚未授：3lib check→原9项定向→gen_bindings空staging→catalog临时stdout成功再发布。root/G会话目前capacity error；最后现场ps native_processes=[]，没有挂着编译。恢复后不要重启旧已终态handle。

地址选择保留两种真实调用语义：原Live::terminal_address由远端通用Reach/GUI协议使用，Host优先；GUI本地终端Node::terminal_address则Seat优先（已有view不能错接GUI host协议）。两入口共享同一个positive归属核验/私有解析，未加RPC或宿主框架。现owner既有用例加同host/view的真实方法端口断言，待最后复验。GUI旧弱is_hosted fallback已删，core实际拒绝例在ownership-3通过；最终源需重验。

最新独审源码反馈已解除先前fallback/否定式未知放行：正向input_owners在fold前保真实shell资源、unknown拒，每键/Stop无sysinfo；原native/raw ID的显式takeover/resume独立动作保留，不能用can_control=false禁止明确接管，observe alias一律拒并不自动翻译。外部agent即使display Shell也不由表面kind授权，只认原Meta Shell/真实shell资源ID；默认agent导入仍是观察。

GUI当前实际函数提取/模拟hooks检查13/13，`final-candidate-1/gui-result.json` source unchanged=true（不是浏览器/App验收）：metadata owner/home分离、跨owner迟到能力、foreign mux/display Shell不进终端、alias先readonly、正负SEEN答案只用于所查询home、None只读角色无状态词。StatusBar只承接有执行事实的过程条目，不扩大其word为nullable；TabStrip/SessionFace无halo表达None。原160行check-session-content.mjs未修改。

真实TS检查在旧A1 bindings/catalog上exit2，原日志`gui-pre-producer/types-before-producer{,-2}.log`保留；第二次仅剩预期新字段/词条未生成诊断，无其它已知nullable错误，但**仍不是A2类型绿**，新实际producer后可能发现更多调用方。未手改任何生成TS。App/G独占路径未碰，producer后如需胶水交root协调。

CleanCode本片段核对：执行状态/记录角色/输入归属三轴分清，不以Idle或thread_unbound充观察状态；ObservationRef是唯一身份owner，路径/provider为可重建来源投影；ID纯元组hash、元数据预算/原子写归canonical模块，Node.import保编排；正向新绑定与已认证handle快路分开，错误不按文案分支，也不容许弱fallback吞归属拒绝；同源重复元数据读取已去除。未扩大为全局格式/注释/框架治理。

### final-candidate-2：按实际输入目标收口（新增独审阻断已修源码，待复验）

final-candidate-1的“双cookie各选正确”不足以证明目标归属：一个活GUI host不能把自身授权借给旁边foreign tmux view。独审明确指出后，当前源码收紧为：非明确shell资源的agent terminal只允许已证明host/child归属的自有TUI PTY；Meta GUI的Seat/view请求准确拒绝，GUI继续用其自身ACP/Host。真正shell资源才进入view/tmux解析。仍只有can_control这一输入能力轴，不另造readonly真假值。

不采用“GUI+任意mux=>foreign”的启发式，因为自有GUI也可能从外层tmux环境启动。UI因此isTerminal排除GUI，自有GUI按当前可信host走对话，foreign无owner走Observed；旧mux抢先空分支删除。这样不会借host控制foreign view，也保留真实Khor interactive owner自己的协议。现有owner例改验证GUI Host可用而Seat拒绝、TUI+旁view只选自己的已证明PTY；这是新源码断言，**尚未native执行**。

最新完整候选固定在`history/observation-a2-check/final-candidate-2`，36路径SHA、binary diff、新源全文、receipt均保存；内盘恢复`recovery/round2-history-current/a2-final-candidate-2`。23个改动Rust路径rustfmt check0；实际TSX parse/函数提取模拟回调13/13，source unchanged=true。它们不替代最终native/真实producer/TS/App门。原final-candidate-1/ownership红绿证据全部保留，不回填。

root当前capacity error，native新批申请尚未回；所有旧句柄终态、native槽已释放，没有后台编译。独审已恢复同owner只读审当前收口；不新建worker、不擅自生成或启动真实agent。后续一恢复直接以final-candidate-2（先核实际diff无变化）跑已报3lib+9定向，再真实gen/catalog与types、固定提交后独审。A2未提交/未done，A3/A4未启动实施，main/ship未授。


### final-candidate-3固定源码与独审结果

最终记录改为`observation-a2-check/final-candidate-3`，36路径SHA、binary diff、新文件全文、receipt/恢复副本齐全；相同源rustfmt0、真实TSX parse及函数提取模拟回调14/14、source unchanged=true。状态/记录分组缺执行值时也改中性unknown，不误写Idle或thread_unbound（GUI words与CLI group同改）。

独审实际读最新目标收口，确认agent在读取view前即拒绝非自有PTY；GUI Host仍可用于其自己的ACP，而Seat/view拒；TUI旁有view仍只允许自己的PTY。DetailPane不因GUI有mux而换成终端，没有“GUI+mux就foreign”启发式；真实shell资源保其输入行为。此前host借权view源码阻断解除，但不替代最后native/producer。

同时纠正新增Native参数前缀判断：ACP native ID是opaque，observe/...也可能合法；Native resume/reopen仍是明确agent/native域、已记录home/名单验证，不做Khor alias翻译，不能盲用Khor前缀禁供应商字符串。删除两条尚未执行、会混淆两域的新断言，保全部Khor SessionId型控制门拒alias与纯import无spawn/load例；加opaque candidate encode/parse完整保留。原始native ID的显式takeover/resume作为独立动作保留，不被can_control=false机械禁止。

最终native批/真实producer/全types/固定commit仍未运行；不手改绑定、不发布App。所需命令/槽申请已交root，root容量恢复后继续同一owner即可。当前没有cargo/rustc或测试进程挂着。


## A2固定后的真实编译闭包（2026-10-29）

原`closure-1`CLI编译setup exit101/2.871s因作者树无apps/gui/dist/index.html，未到consumer诊断；原结果未删除。root授权直接已装Vite真实构建（绕过npm prebuild隐式gen），`web-assets`exit0/7.689s、源码unchanged=true，277文件/15,154,979字节，未复制其它树产物、未建原生App。

`closure-2`四条串行compile-only全部exit0、总28.539s：CLI `--bin khor`含khor-web 3.977s；Node live/broadcast/tunnel_wire集成test目标19.589s；gui-core seen_snapshot目标1.396s；sync+commands `--tests`3.398s。完整native源码+Cargo+277 dist输入前后SHA不变，无构造补丁，无重复9项行为测/producer，全批终态exec86522 exit0。该证据仅证明这些consumer编译闭包，不冒行为执行或正式Tauri/App验收。native槽已向root释放。

root集成`6f103d9b`时producer输入328项与作者固定快照327项相同，唯一差异为root既有C1 web.rs system_target/open_link体及测试，先停复制留差异证据，由独审判断导出结构/签名；H未为此重复gen或改源。


## A3下一片段只读方案（待root派发，尚未改源）

图查询`gui_host_main_with_timeouts`未命中；图构建631c0d0a与HEAD3ff不匹配，以下按实际精确文件回退读取。当前调用链：`gui_host_main_with_timeouts`启动`start_configured_on`，ACP把opening Notes放在Ready之前；host单conversation loop在Note分支fanout，Say/SayWith与Delegation分别调prompt_with/prompt；`finish_host_operation`先Resolved再fence，loop等finite tail清空才处理Turn或Replay，Replay::publish再向GUI发送History。GUI只读history_page当前Node source→transcript_page的Native Utterance→core replayed_update；富ACP更新不能转成这个有损Native子集。

最小paths（拟授而非已修改）：新增`crates/node/src/session_content/journal.rs`，现`session_content.rs`/`lib.rs`统一source派发；`gui_host.rs`单loop唯一writer；`adaptor/transcript_page.rs`payload泛化为Native/ACP（沿用同一页/引用/游标算法）；`apps/gui/core/src/chat.rs`payload到现ChatFrame。若wire字段与根类型不变不重复producer；覆盖事实必须可见的最小类型/消费者先报root，不造多版本source或新reader。A4网络路由仍后排。

唯一journal持有者为该home的GUI host conversation loop，不在GUI、reader或每vendor里写。source身份version/home/registered-agent-key/完整ACP session ID，与Khor clean_leaf row ID分域；从现已注册上下文固定，缺注册key时需保存独立opaque launcher identity（绝不猜vendor）。持久source引用绑定完整identity，观察从canonical ObservationRef派生/验证；文件path投影可重建。写者需独占lease/锁防两个host交叉append，不以PID marker存在作权威；读者不取写锁。

记录保留原ACP通知、host已接纳prompt的原文/图片/文件块与turn token/结局，不把host接纳冒成agent已接纳。Say/SayWith与Delegation均在通过单turn门并成功写入Accepted后才dispatch；失败拒绝本次发送。dispatch未完成/crash只有Unknown，不伪造未发送/已完成；fence后记Ended/Cancelled/Failed，transport错误是否已执行未知明确保留。payload写入流式序列化，不为计量先复制全附件；读取继续256KiB扫描/64KiB正文引用/128帧/512KiB响应及既有GUI三预算。

bootstrap与显式Replay各自一个snapshot事务（代次、source、临时文件、提交/中止），startup以匹配Ready且打开成功为边界，Replay以匹配operation成功+event fence清空、未overflow为提交边界；失败/closed/取消只回收本事务临时文件，旧已发布快照不变。首次无committed内容（不能只判path.exists）可以建立snapshot base。已有本地accepted turns时，ACP没有通用可靠message身份，不能文本hash拼接、不能无条件替换吞掉agent未echo的用户prompt。root和独审已收口：既有journal为唯一append truth，后续load/replay仅既有runtime回放、不更改canonical；不建立多版本source/UI或每次resume保全历史。snapshot最多current+一个pending，失败/崩溃保旧。较新外部内容无法可靠关联时，以覆盖范围/gap事实明确未自动合并，不伪完整外部历史。bootstrap期间不接新prompt。具有可靠turn/message ID才可关联echo；无ID保原protocol事件/provenance，不按内容吞合法同字chunk。

最小验证拆两提交：A3a journal owner/格式/Accepted+边界及统一page实际消费；A3b host bootstrap/replay事务接线。定向用例：无user echo仍见原prompt与附件；两个完全相同live delta均保留；两次bootstrap与重复replay不重复canonical正文；中途失败/取消不替换旧snapshot；accepted后断开显示Unknown；超大附件/通知走既有refs分块且不全量读；不同agent同native ID/双writer拒绝/重开source身份。使用仓库隔离ACP stub与临时小夹具，不运行真实agent或用户session。不能把pure writer unit绿冒host调用链验收。

能力边界：全部Khor托管ACP不按vendor名单统一写journal并可观；foreign外部会话若既无Khor journal又无真实Native只读source，仍如实Unavailable。list metadata不是全文支持，不暗中session/load/resume；A3完整交付不等于每个外部工具都会提供历史。


### A3只读方案独审收口（尚未开写）

root/独审确认以上唯一append truth、首次pending原子bootstrap、后续runtime-only replay/no canonical mutation，不做多版本source/版本UI。必要新增叶路径拟`crates/acp/src/lib.rs`保存实际opened_by（New/Load/Resume）只读事实：当前open_one在cap resume可用时走ResumeSessionRequest直接return，故不能从Ready或load能力推定实际回放历史。首次Resume无历史回放只记partial/captured-since；不为只读观测额外发load。New与成功Load覆盖同样有实际边界，不能称通用外部全历史保证。

Ready证明opening队列排在其前；`fence_events`仅把调用前有限source队列转发到host receiver，必须继续复用`pending_completion.remaining`消费到0后才提交snapshot，不能仅await fence就rename。重复bootstrap/Replay不复制canonical正文；同字live chunk保留。覆盖字段有界存latest/count或持久gap事件，不新建随poll增长的内存数组。读取首屏继续既有tail预算，不能为了判断已有committed内容先扫描整个journal。


### A3a writer首diff进行中

已获root正式A3a写权，并实际新增`node/src/session_content/journal.rs`+session_content模块入口，尚未接host/reader、不算用户路径完成。单标准File独占写锁，version/home/agent/full native ID哈希目录；原子publish整个`pending`目录为`current`，其中manifest+records一起可见。manifest16KiB上限，pending流式32MiB硬预算（匹配既有runtime Replay阈值），记录BufWriter64KiB，超限/IO失败writer失败封闭，不truncate current。accepted prompt原文/图片/文件写入后同步并持久active turn，成功返回才允许调用方dispatch；完成记录准确turn token和outcome。恢复保留半行原字节（补换行，不删字节），新gap记录含interrupted turn；reader下一提交须将无效半行变raw引用，不静默吞。

6项小IO测试草案覆盖原子首次base/失败pending清理、无user echo原prompt+150KB附件、相同delta保留、独占锁+重开未决turn/半行字节、namespace与伪current目录拒绝、accepted持久失败禁止继续与pending预算。最后两项实际是独立用例，共6项；尚未运行。已向root申请仅`cargo test -p khor-node --lib session_content::journal::tests --locked`，不gen/全App。

限制保持：ACP start_configured在Handle返回前仍有opening_notes32768条上限且无byte上限。host pending有界不能证明整条ACP startup内存有界；本片段不冒已修该既有队列，不暗中扩大队列改造。


A3a固定前修正：root指出32MiB总snapshot存储cap错误挪用旧runtime限制，确会让合法长历史首次不可观；尚无Cargo启动，现已完整移除该cap/LimitedWriter及其测试。只保64KiB流式写buffer/16KiBmanifest，完整合法原始记录不因总历史量被截断；真实磁盘/IO失败明确错误且pending不发布、旧current保留。第6项测试改为metadata16KiB读取上限，不以人为总历史上限追绿。上段提议cap已撤销，非当前实现。


A3a首次writer编译：writer-1终态exit101/24.952s，source unchanged，Record三个非泛型payload variant构造漏生命周期；writer-2终态exit101/11.615s，显式variant turbofish仍E0109。两批均编译诊断、未执行测试，日志/完整新文件与SHA保留。root授权同target纯编译语法/类型必要修正自主迭代，现仅改局部`Record<'_,()>`类型标注后构造，无别名/断言变更，writer-3 exec91592同6IO运行；不得恢复时重复启动live handle。commit_base全部IO错误已按独审统一poison（包括rename成功但目录sync失败），下一host错误即退。Prompt已durable而active manifest未更新的crash窗，reader须把无completion当Unknown，不能只凭active_turn判完整或自动补发。


### A3a writer真实绿（待固定）

writer-3 exec91592/Cargo63370终态exit0/59.646s，6项IO全通过（执行0.23s），完整相关Rust/Cargo前后manifest不变。仅前三处Record构造用局部`Record<'_,()>`类型注解正确表达，无别名、无测试断言放宽。journal SHA`3a0629ae81bea1a5e8a4665fe7a60a22aa82100c768360231e7f48138645f1ad`；module入口SHA`6ee6d129bd6f3f9ed5e98dd8771848369a4894ac8f22f2a9e048993b1b171089`。source-before/new-files保全新文件，fixed-writer逐文件快照/manifest；rustfmt --check通过。root与独审已收到结果并明确释放native唯一槽。首两编译101均保留，不叫行为red；没有测试旧总量cap。

此首片段只有writer/磁盘事务，不含host/reader调用、未gen/改GUI/操作真实agent，不能将6IO结果冒端到端可观。下一片段需要实际New/Load/Resume fact、bootstrap/Replay fenced有限消费、Accepted-before-dispatch与所有outcome接现loop；reader原样ACP/原prompt块及gap、畸形半行rawref接现预算。


### A3a首固定检查点8a7ae1aa

独审核精确journal SHA/6项真实日志，无源码阻断后提交`8a7ae1aa5c4490d54b5a687f724e4bdfebdc9cd7`，仅2路径/546行（含6项IO夹具），clean。fixed-writer/commit.json逐git-show两SHA与测试源一致，source.patch保存完整新增文件，不依赖未跟踪文件的diff header。writer-3新文件全文已被source-before/new-files捕获。恢复bundle`recovery/round2-journal-8a7ae1aa.bundle`。native槽已释放。此提交不含host/bootstrap时序、硬崩溃注入或最终UI消费；无completion=>Unknown、malformed=>raw引用在下一真实接线验，不把基础单测冒全部R005完成。


## A3纵向片段进行中（8a7ae1aa之后，未提交）

root已授实际opened_by叶与coverage窄接线；现15路径diff：ACP Handle真实New/Load/Resume贯穿而非猜cap；gui_host以Bootstrap enum AwaitReady→Fencing→有限Draining→Live，Ready启动fence、消费完才提交首次base并写原ready文件，避免首条Say被初始化门误拒。失败写原refusal；Close取消pending显式abort并报告清理失败，Drop仅兜底。已有canonical后startup/replay不重复写正文，Replay fenced后的真实live事件仍捕获。Say/SayWith/Delegation均先durable接受再dispatch；fenced完结保原stop（MaxTokens等Ok为Ended而非Failed），transport/退出未决记Unknown。

Node Source统一Native/Journal，观察Acp按canonical完整home/agent/native tuple找source，GUI host写只读source ref，ref冲突拒绝；读者只读current manifest/file，不抢writer锁、不目录扫历史、不打开ACP。transcript_page仅payload泛型+字段payload重命名，游标/窗口算法未改；原ACP富内容/accepted prompt块保真展开，malformed或超大记录显式ref，core同响应预算。capture元数据为实际首次opening与无法确认连续捕获的间隔数，不是缺失消息数；Native为null。DetailPane初始范围说明，最新gap从已验证journal记录转typed ChatFrame::CaptureGap→history-paint既有sys投影，原ACP同名字段不是系统事实；不改ChatView、不新增RPC/版本选择UI。仅两catalog keys history_capture_scope/history_capture_gap。

新增真实测试复用现45s隔离worker/55s合成ACP，无新框架/真agent：Load首base后ready首SayWith无echo、150KB图片与文件先落盘、两个相同delta、rich diff通过未知vendor观察aliasreader、MaxTokens原值；两次runtime Replay字节不改current、二次Load不重复、后续Resume的真实gap；source文件inode更替后原raw先成功再准确changed拒；首次Resume不发load且capture.resume；invalid/Close-bootstrap不留ready/current/pending，确认reap与锁释放。source/parser两新IO与core富payload/gap/raw映射、JS同名ACP字段不变system均已写，尚未说通过。

root已授唯一native vertical-1：exec36684/Cargo初PID87383，413相关输入SHA包含所有Rust/Cargo/catalog及Python fixtures，整批冻结。check node+gui-core lib→journal8→host三个各1+旧handshake/reap1→page10→core5→ACP fence4（预期31项）→空staging真实bindings/catalog临时stdout。每阶段失败停，不发布，纯编译必要接线修正每批新源manifest；实际测试数必须非零且核对，包括隔离ignored worker确执行1。证据vertical-candidate-1完整15源+patchSHA8bf8401103a2993742de46e19ddc50ffb47c8f65cfcd1bcc4365ddb7859d3a14；vertical-1保存本次实际源/进程/输出。当前不能恢复时重复启动live句柄，结果/槽状态待现场终态。

保留边界：ACP Handle返回前旧opening_notes仍32768条/无byte预算，host64KiB buf不冒startup全链内存已界定。A3只读统一journal与观察/结束GUI消费者，活GUI原chatReplay依旧向agent请求runtime load，无load能力的活GUI重新挂载历史消费留既定S恢复路径核，不将基础存储冒所有GUI恢复完成。A4网络home路由仍未实施。


### A3纵向最终候选：31相关原生＋20/16 GUI检查绿，未提交

vertical-1两lib check编译101/14.265s、unchanged：ContentSource不该假装NativeTranscript的方法；新增统一observation_identity按真实Native/Journal来源给canonical namespace/agent/exact ID，观察导入用它。未知gap opening返回Reference，保原record而不默认new/full。原诊断保存。

vertical-2终态101/92.999s、unchanged：两lib0，journal8＋真实host-journal1＋首Resume1绿，abort case第二mode等endpoint5s失败。夹具两个session/new复用同store+同固定nativeID，第一invalid留下Meta，第二register源码必拒；两peer90935/90936实际已退出。root确认该复用根因；只改cfg(test)中每mode独立store子根+host提前退出实际result诊断，保5s/10s/NoReady/current/pending/锁/reap断言。生产Rust/Python/Cargo/catalog413输入仅此test文件diff，vertical-3-source-difference.json明示两SHA；不把新fixture全SHA冒旧绿同源。

vertical-3 exec83152终态exit0/143.847s/unchanged，先单abort1过，再旧handshake/reap1、page10、core5、ACP fence4，合计21；加vertical2原10＝31。四个隔离worker.stdout均实际1 passed，不是zero filters；失败worker另保。未重跑已绿无关项。真实gen空staging11.448s/catalog0.976s成功，原编译源413逐SHA仍不变后实际gen binary发布，161 TS逐SHA=staging，catalog SHAa30639adb2aef712475c86e6e6088f9abf55a5b7d77e7e79b84067c95e0ee45f。test-counts/generated-sha/publish-result在vertical-3。native唯一槽已明确释放。

GUI-1真实history20/observation16（新增capture注入）/全tsc0、source unchanged。最后依root要求头部只说明初始捕获范围，删一次probe累计数字，避免新gap发生后误读成当前总数；最新typed CaptureGap仍随现有page显示sys，每个interval事实不等消息丢失数。仅改DetailPane+新增callback断言，GUI-2 observation16/types0、source unchanged/Native413 unchanged；history-paint和其20检查源码未变，沿gui1不无故重跑。原ACP同名扩展不变system已测；未改ChatView/新增RPC/版本UI。

最终16路径完整源码/差异/SHA/验证适用映射在fixed-vertical。独审待最终确认后commit；仍不计A4/活GUI恢复/整个H完成。所有source/gate失败均保原记录，测试是合成真实host与真实IO、JS是实际函数/模拟hooks，不冒浏览器/正式Tauri验收或硬崩溃注入。


### A3纵向固定529d9035（本机capture/只读source切片）

独审核fixed-vertical16路径SHA、31项分批适用/四worker实执行、161 TS及catalog真实产物、最终GUI证据均无源码阻断后固定提交`529d903527532df9d2b069fea00a5c2a909be022`。16文件962新增/81删除（包含实际host/reader/GUI定向夹具），git-show逐16 SHA与已验源完全相等、树clean；fixed-vertical/commit.json、source.patch、validation-map保存。最终Native413未变，GUI最终16callbacks/types沿gui2，history20按gui1同源复用。内盘恢复bundle `round2-acp-capture-529d9035.bundle` SHA`f54d8f6039bff669455ddfaeafb7d579ef9484a378b0ef3e3e44200f943832eb`，checkpoint-a3.json标当前ref/证据/已释放槽。

不计全部R005/第二轮完成：A4内容home路由未接，活GUI现有runtime replay与无load恢复留S，H全source搜索/复制按原需求待核补；ACP初始化旧opening_notes无byte预算未冒已修。main/ship未授权且未操作用户App。旧语法/生产类型/夹具root复用失败全部保留并区分，未把31项伪成一次相同全源码批次。


## A4 transport working（首基座后，未编译）

首六源3e9376ee已由root集成5c612223；local2实际编译输入414（新增reference），与direct26已有输入逐SHA同。fixed-local六源精确指纹/31分批收据独审无阻断。旧本轮自有历史Lab PID88855经PID/出生/cwd/端口及原history-lab.json同核后SIGTERM，已退出，source/evidence保留，stop收据history-lab-stop-20261029.json；无用户App/agent操作。

当前下一源在author HEAD3e9376ee后working：新增session_content/requests.rs、remote.rs；必要session_content.rs模块/Unavailable serde；transcript_page/window DTO Clone/RecordChunk serde；proto.rs、ipc.rs、link.rs typed ContentRead route。Node.content_read→既有resident/paired connection→action.home真实DeviceId；Target Reference不访问alias owner，普通TargetHome在actualhome解析一次；远端worker只resolve真实source/prepare window/raw，caller后续同parser。Begin/Cancel只validate metadata+registry，不Node::open/provider；Resolve显式probe仍有Node::open不可中断边界，Window/Record source lookup与同步单次读只在前后flag检查。A3 writer/gui_host完全不改。

租约Reader仅内部构造（真实local root/cookie域或conn.remote_id经admit的peer）；payload无owner。key=home/reader/token，source hash绑定，32 hex token；MAX64/pending TTL30s/claim once，claimed cancel持到实际worker Drop；scope外cancel不影响他人，已写4个未运行私有registry例。现core旧history_request尚由旧GUI消费，待最终新API接线后移除，不能将两套并存当最终交付。proto与IPC真实envelope量encoded bytes（metadata32KiB，window/raw256KiB+16KiB；record本体64KiB），source/view/full ref hash与新Reply.validate_for核request归属；不会手改TS。

已核旧connection 32 stream seats可能挡history Cancel的风险，正按只作用ContentRead数据IO的窄路径收口：正向lease claim无await后移guard进blocking worker，数据IO另受实际64请求槽约束后释放普通stream seat，Resolve/其余请求保原32。不得放开成无界IO/全局协议重写。真实两Node transport/断线/取消/source换代测试还未写/运行，不冒远端可用。

SessionRow.content_locator/GUI/core API/Tauri/producer尚未动，root现严格native pipeline以集成源独立冻结，H不并发Cargo。下一阶段先收窄service/wire源码＋测试再申请专用native批；R005/S/H欠项仍照前述边界，不按本地31称整包完成。


### A4 服务候选已送独审，下一native尚未启动

transport-candidate-1完整10源/manifest/patch与作者现场逐SHA同，新增requests.rs/remote.rs/wire_tests.rs；module与协议/IPC/link已有实际调用方。原cfg(test)self crate alias已在Node lib存在，直接复用，不加production alias。新wire_tests.rs依root授权在库内cfg(test)并复用../../tests/util，生产无env/gate；32个真实home worker到Condvar门，Cancel必须在显式release之前通过两Node+IPC返回，claimed canceled仍占满(client+home)64，真实worker退出后才可重用，失败Drop放行自己的IO门。

同一真实双Node case还核Native+非vendor名单ACP journal传输、离线metadata owner/无alias文件/本机bait、wrong binding与未配对caller拒、earlyCancel阻IO、raw64KiB逐段原字节、旧raw先读成功再换inode准确changed拒、home物理pause断线不回退本机与恢复。另1 codec/retry例核只Cancel可重试、Window/Begin不自动重试、0RTT未放开、真实rmp二进制字节和过budget拒。新registry4例，因此新增6；拟同时跑既有Native source6（抽共用parser，保原断言），合计12。

Node ClientRead为Arc guard，caller/core可从Node::open前claim，穿过异步IO、blocking parse到最终512KiB投影；free begin/cancel仅既有endpoint cookie handoff，cancel同时先置本机client/local IO flag，绝不Node重开/发现provider。普通Source解析可阻塞Node::open与单次IO边界继续明记，无法强中断同步调用；一条request不意味可取消live agent。旧GUI core lease等最终consumer切换才删，现UI还没接新RPC签名，不冒已统一owner交付。

root已条件授下一唯一native（check→requests4→wire2→existing source6），须独审无阻断、现场无其它Cargo且内盘≥5GiB后才启动；当前只rustfmt语法/差异检查，未编译/未gen/未启动新App。SessionRow/content_locator/GUI/Tauri/API未改，不影响root当前真实pipeline；后续共享fields/gen/fixture仍先协调。


### A4 response owner独审纠偏（尚未native）

candidate-1未编译：独审指出IO Request Drop后大reply还在write背压等待会失去资源上界，阻止进入native。现candidate-2实际11源与完整内盘mirror：每个实际服务Node字段content_responses只初始化一次32 semaphore，所有QUIC conn＋本地IPC共用；typed/cookie/peer验证后try-acquire，拒绝立即返回，不排IO。拿到后才早释普通seat，Cancel/Begin不拿此lane。outer声明permit在payload/frame之前，持到encode/write/finish与payload Drop；同Arc随真正blocking worker和(reply,permit) tuple同行，await丢弃也先释放payload再许可重用。IO64和响应32是两个明确资源生命周期，未静默把原32改64；Resolve仍旧普通32，非全协议QoS重构。

只cfg(test)两Stage门：原32真实慢IO→Cancel ACK＋实际claimed槽不早返；新32完成并编码、阻在write前的大回复→Node A IPC与B home实际lane均0剩余→新Window即时buffered拒绝、不排更多IO→Cancel仍ACK且不早放响应槽→放门/任务真实完成才恢复32。此为确定response阶段owner证据，不冒实际内核背压采样。fixture Server在serve消耗Node前clone同一个lane观测，未每请求open新lane。Root复用已有test self crate alias/既有tests util，无生产env/gate或新服务。

Root当前持唯一native/App槽做用户真实Mac验收；H从未运行transport Cargo。下一check＋requests4/wire2/provider6仍待candidate-2源码独审解除＋root交槽，不重复已过31、不gen/全App。后续GUI binding/locator和旧core lease清理尚未开始，不能标A4完成。


### A4 Node服务固定检查点 ced98d3b（GUI仍未接）

精确ref ced98d3b85432b4cb32c0a6ae38d6ecc23aa4435，11路径1855+/59-，作者树clean。fixed-transport 11源逐git-show SHA=transport4实际编译manifest（417输入），前后完全相同；journal.rs/gui_host.rs逐SHA仍529d9035冻结源。最终transport4真实0/90.773s：check11.501s，requests4、wire2、既有source/providers6全绿（12），真实双Node整链case3.85s。未gen/全App，不冒GUI或全轮完成。唯一native已明确交回root给W browser，旧进程全终态。

失败逐批不回填：transport1 Cursor私有字段3处E0616；transport2 check绿但fixture codec少RecordChunk import；transport3 requests4与codec1过、真实case因synthetic ACP note漏required sessionId被既有A3 parser正确保为Reference，在后续32门前失败。已用正式typed SessionNotification验证/保存合法note，原深等/Gap/accepted prompt断言保留；删除首片段遗留未调用PreparedRead.reset私有mutator，最后同check+12源再跑取一份完整终态，不扩矩阵或重31。

transport3/4剩余仅本批TMP中的empty khor-test-tmux/tmux-501目录；终态、uid/dev/ino、非symlink与lsof空核后仅rmdir这些owned空目录，cleanup-after-terminal.json另记，原tmp-status/result不改。fixed-transport/source-manifest/commit/validation-map与小源码内盘镜像齐。后续按已授Core SessionRow+Available.binding/API/Tauri/DetailPane/ChatView接真实消费者、producer生成、删除旧core history_request；完成前不把Node可用当GUI已可用。


### A4 GUI bridge working candidate-1

当前11路径（含删除旧core/chat/history_request.rs、新增chat/history_bridge_tests.rs）已源码接线，bridge-candidate-1逐SHA/patch/内盘镜像齐；Node服务ced98d3b未再改。SessionRow.content_locator仅lib唯一字面构造encode Session事实，Available.binding由actual home Resolution返回；public5条Core/API/Tauri入口全async，id校locator/binding，ClientRead在Node open前claim，open/parse/512KiB转换或raw base64各blocking worker与结果tuple持同Arc，终态不保旧GUI Registry双owner。Native同步/GUI类型尚未验。

UI API闭包携binding；DetailPane key=id+locator，与metadata owner/row.source解耦；owner变化不断现有正确source，home真实locator变化即时清旧cap，迟到reply拒收；ChatView still使用强props union、binding纳effect代次，无binding不猜本机、不调用旧RPC，live ACP分支未改。旧ended注释误称只Claude vendor已窄清，未扩全局注释治理。Frozen原H Lab不动/已停，只保历史结果，不能拿旧未传binding入口冒当前可用。

实测bridge-ui-1：18观察callback＋20历史模型＋mobile-static均exit0/源码不变，tsc明确仅旧真实producer未跑的2字段缺失，exit2保留，绝无手改生成类型。原脚本旧row fixture无locator导致f.requests[0] undefined的开发夹具不匹配单记，不冒产品red；新facts显式给locator/binding，原metadata-owner内容路由预期按A4已授改为owner不参与，非删测试追绿。

新增Core bridge2例计划经真实api::answer调用locator→binding→actual Node IO/页预算/raw base64/earlycancel，transport只stub旧cookie admission Begin/Cancel，未知拒绝，真正2home/IO背压在Node12已验；另拒crossview/malformed且不创建Node。未新增依赖或生产服务。拟最小native Core --lib check＋bounded_history_response_tests5＋history_bridge_tests2，待root单槽授权。

W Git下层两leaf正由W推进；shared api/lib/Tauri/producer仍H唯一writer，待W精确DTO/签名接线后Root合并稳定接口真gen一次。当前producer已export SessionRow/SessionContent，无新增DTO，gen命令未运行；旧types2红不冒最终tsc绿。main/ship未授、A4/全轮仍未done。


### A4 bridge固定前原生闭包已绿

bridge-1真实终态exit0/83.098s、相关Rust/Cargo/fixture源前后逐SHA相同：Core --lib check0，bounded_history_response_tests实际5/5、history_bridge_tests实际2/2，分别16.50/65.26/1.21s。新增两例沿真实api::answer分派和Node读取/512KiB页转换/64KiB raw base64/提前取消，admission transport仅Begin/Cancel owned fixture，不冒实际浏览器或正式Tauri端到端。fixed-bridge保存11路径全文（含删除标记和新增文件）、source manifest/patch/validation-map；JS18+20/mobile静态0沿bridge-ui-1，旧生成类型缺2字段的tsc2仍保留。真实producer等待W Git稳定DTO同批，不手改生成物。

预检编排失误独立记录bridge-1/preflight-orchestration.json：按通用khor-gui basename误匹配7个用户release进程导致预检assert，而调用者未先核exec.exit_code仍启动已授权Cargo批。无其它Cargo；没有启动App、也没有终止/修改或读取用户进程参数。root确认无App构建不受用户App影响，后续预检改用owned产物/fixture身份且失败结果必须gate，不能将原assert冒通过。native批源冻结、真实结果保留。终态lsof与uid/inode/非symlink/空目录核后，仅rmdir本批owned空TMP子目录，cleanup-after-terminal另记不改原结果。唯一native已明确释放由root转W；下一shared注册可源码准备，不gen/native。

独审核11路径无源码阻断后固定`41ed240b45ca9b0de564b673e0761bc5c1185279`，499+/300-（新增280行真实Core夹具、删除旧149行lease）。逐11 git-show SHA与fixed-bridge/bridge-1已验源完全相同、作者树clean。fixed-bridge/commit.json逐路径保存；内盘bundle `round2-content-bridge-41ed240b.bundle` SHA`f0bd0f46c5e22edf1dfb07259dd53df3b37eafbbeb27cefbbbcbecc5ad0bc650`、checkpoint-a4-bridge.json与小源码镜像已保存。下一阶段只准备W Git共享入口；生成、组合types、实际新binding浏览器验收仍待，不冒全A4完成。


### 共享Git公开入口源码准备（W依赖，未编译）

41ed240b后仅4路径source diff：core api.rs新增git_history整型u32严格转换，Tauri async command＋registry，真实gen_bindings清单新增GitHistoryPage/GitCommitSummary，GUI api.ts import/export＋fetchGitHistory(machine,dir,cursor?,limit=0)。W确认0默认40/max100、cursor缺省/null＝None；旧git_log仍文件历史。未碰W两个git.rs叶、未运行gen/native，等待W固定ref后合并一次producer。git-shared-candidate-1保存完整4源/patch/contract。当前此working源码故意依赖未取的W leaf，不能把旧bridge绿套到shared新入口。

A4 parent mount必要窄修固定`cde0bf011b1ff0ed33f01e898563fabc481194fa`（两路径19+/1-）：自查实际DetailPane observe分支key仍含metadata owner，与child queryKey不依owner合同冲突。新增AST执行两真实parent JSX key表达式，owner-only变化先red exit1；一行key=row.id后19/19 green/源不变，source变化仍由child queryKey同步render防旧cap。owner-key-red/green保真实结果、commit.json逐2 git-show SHA与green源同；非浏览器reconciliation证据，不重Cargo或无关模型。原41ed证据不回填。仅W共享入口4源仍working、未gen/native。


### shared-1真实编译绿、旧命令finder假红止步

shared-1 exit101/96.765s，892相关输入前后SHA同：Core+Tauri --lib check真实0/28.533s；既有command_tables实际1过1失败。严格停在gate，不生成/发布，TMP实际空，唯一native释放。差集7个native page名字实际在main和ced的同一个多行API arm注册；不是缺HTTP arm，也不是本轮C/G增加。e251(9/9)原5命令、1eae(10/6)page_ground，d26e(10/28、main之前)格式把or-pattern拆两行，旧finder逐行要求quote开头且同一行=>，故两行全漏。command-table-root-cause.json保存真实arm、逐源SHA与历史；较早baseline诊断regex不能识别整多行arm，明确被此准确记录替代，不能凭其api_match false声称未注册。

root批准仅测试finder正确识别跨行literal or-pattern和直接反例，不加ONE_SIDED、不削弱差集/数量断言。当前test-only候选定位实际match cmd范围/自身括号深度，严格pattern识别并排除body/nested/outside/comment字符串；新增2定向例，下一批实际4项（旧2+新2）待独审/授权。生产Rust共享入口和W叶无再改，故check0适用范围保留，不冒test绿。

另外root明确3个Git专用catalog label：gui.git-history-newer/older/search-page→gui.git_history_newer/older/search_page，较新/更早/搜索当前页提交与Newer/Older/Search commits on this page。现H history_newer/older两动作词值保持，不造归一化撞名。两个catalog源已准备，下批须真实catalog producer；此前shared-1未执行任何gen。


### A4＋W共享公开入口固定三小提交

shared-2真实exit0/71.292s/1526相关输入逐SHA前后同；test-only finder受审后tables实际4/4绿63.934s，空staging真实gen_bindings4.676s、真实catalog gen_ts1.097s、同producer binary SHA26b43b0d81839f21c4fc1ffb71e71fe8562d4e6126bc84755ae2858404e039ba默认managed发布0。生成163个TS＋1inventory JSON（164总文件），staging/真实发布逐SHA同；catalog真实stdout原子发布SHA5485711d263dc9b3c19db614fd5a68dcf9a58f3f078d1db36530fdc61dd9d18e。全部阶段floor/exit gate，TMP空，未App/native槽已释放；shared-1原101保留，不回填。其Core＋Tauri lib check0在4共享生产入口字节完全相同范围内复用，retry只改test finder和catalog3标签，不冒一次全同源码批。

shared-ui-2真实19观察callback＋20history＋mobile静态＋全tsc均0，源前后同；旧真实gen缺字段tsc2现已由producer解决，不手改类型。3小提交：test-only `269d2a14`，catalog3label `e2eeff21`，shared API/Tauri/producer/GUI `fa3b57c6`。最终head fa3b57c680c1096bb806b47e33f9a15d96f2e0c9、树clean，fixed-shared逐7 git-show SHA＝shared-2实际输入；W叶c6已精确取为5e4ac206且未改，原Node9沿W证据不重跑。bindings/catalog快照和双盘恢复bundle/manifest齐，仍待新binding生产DetailPane→ChatView实际浏览器验，不能按types绿冒全A4完成。


### A4新binding真实GUI验收5组通过，资源已释放

独立Lab固定45e8651dd8d9d04982186956b9e8d9d5d21c29bf，4新文件（176行），原H Lab源码冻结未碰；真实TS program包含新Lab诊断0。只stub transport，实际生产DetailPane→ObservedSession→ChatView→source/reader/cache，严格id/locator/binding/lease、未知/旧RPC显式拒绝；只UI接线证据，不冒实际网络（网络沿ced双Node真实用例）。

root授唯一短browser后Playwright新独立visible Chrome profile actual5业务组全过、finally关闭：初始只读128帧/108282B/DOM128、无textarea；owner-only后实际DOM同对象、probe仍1/tail-read仍1；延迟A页claim后换B，真实cancel A、A迟到结果不混B；B raw按需单块≤64KiB/无UTF8替换字符，换源文件generation后清raw/旧cursor；unmount请求取消、leases归0、1100ms超过poll间隔无新增页。unknown[]/invalid0/pageerror[]，content-browser-1完整script/network/result/截图与production source＋163TS/inventory/catalog before/after逐SHA同。未触用户App/用户Chrome数据，独立profile仅localhost fixture。

browser槽已明确交root/W；own Vite14072按原server.json出生/cwd/监听端口/独立PGID核，组成员仅node14072与own esbuild14073，SIGTERM后父退出/23838空，stop.json保存。未停W或用户进程，不删源/缓存/失败记录。双盘mirror、bundle45e与checkpoint-a4-browser.json齐；首bundle用raw SHA endpoint致无advertised ref空包128，未产生文件，核HEAD同45e后用具名HEAD range重做并verify0，单独记录而不影响产品证据。A4服务＋Core/GUI只读接线已具实际分层证据；Root集成组合type/source核仍独立，不能用author types0冒Root组合绿。下一转既定S，H源全文搜索/复制欠项不与S同时扩框架；无main/ship。

## 40预置source能力矩阵（Root只读5c，2026-10-30）

`history/capability-matrix-5c/matrix.json` SHA `b2e1565910757296e77339d849204725d0bcd8f52935301143c83bd695203945`：40预置、22自动argv、18需手动安装/argv，与既有survey逐ID对账。仅repo-pinned登记表和源码策略映射，zero新安装/spawn/probe/厂商runtime测试；不得叫40厂商实测。所有托管ACP统一journal可观，外部来源仅真实声明native reader；预置ID/名字不推断来源/能力，自定义argv同规则。观察不授控制，resume仍独立显式能力动作。当前S/B新source/native组合未验，不能用本source表替代其NoLoad/WK/GUI正式证据。


## 2026-11-01 发布决定与历史记录边界

用户最新明确授权“先直接commit到main并ship”。本卡以上实施、失败及阶段权限记录原样保留；本次授权及发布状态以[本轮发布记录](../ROUND-2-RELEASE.md)为准。原 Mac 真人中文 IME 与真实 iPhone 操作仍未验，按用户决定移到发布后，不登记为通过；本卡记录提交不代表已合并 main、完成任一 SHIP 目标或已发布。
