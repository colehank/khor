# 第二轮：当前执行状态

更新：2026-10-30。目标 **完成第二轮** active；范围 **29条反馈＋5遗留，共34项**。用户已授权实施，**本轮 main 合并/ship 未授权**。main `/Users/zgh/Desktop/working/khor` 保持 `e7c335c8`；集成 `/Volumes/disk0/Khor/worktrees/current/khor-round2`，`integrate/round2`，**5ddc574e**（本次核验 tracked clean）。旧阶段详[归档](history/STATE-before-round2-ce69-sync-20261029.md)，其中运行与待答语句均是历史。

## 恢复顺序与现场

1. 最新用户消息 → 本文 → [执行计划](ROUND-2-EXECUTION.md) → 当前任务卡/专项 → 核实际 refs、diff、agent 与进程。恢复镜像在 `/Users/zgh/Library/Caches/khor-code-quality/recovery`，核最新 receipt，不用旧摘要覆盖外盘未提交源。
2. 用户已回答 R2-028：**“codex输入区，是因为根本没有组合键的配置机制”**。唯一实施归属 **M/R2-026 自定义终端键行**，以配置/保存/发送 Shift＋Left 验收；不是待诊断的 Mac 物理快捷键拦截。原入口问题已结束，不再重复问。E 仍负责真实 Mac 文件中文 IME、草稿完整性和退出保护。34 项不增不减。
3. 当前写权：Root已在input pending_init时interrupt并临时接Wconsumer，保全部原dirty/ref；作者931813a4之后Root仅补retained active四源26fdebe7，原effect业务红→13callback/type0，独审无阻断。W现在Root统一集成/验收，作者不得并写。`round2_input`复用旧khor-round2-input，新`fix/round2-session-follow@e351`承接既定M025 source/纯检查；`round2_history`同原history树新`fix/round2-journal-hydration@82455217`（shared72527＋S331c65a4/8245）做S Core/GUI hydration source；`round2_reviewer`只读固定版本。不新建工作目录、不新增第三大实现线。
4. **native/Cargo槽空闲**：Root W cli-current-5ddc-1 exec84078 已真实终态 **0 / 182.225s**，types/currentVite/metadata/actualCLI/test producer/exact1各phase0，1 actual双Node测试含38far watch sources，通过heldwait+begin/leave/ordinaryls control；source/env稳、provenance true、remaining[]/TMPabsent、无清理错。不是38项测试或GUI/WK证明。fixedRoot5ddc，当前assets已构建；后续DOM单独固定fixture再做。无App/Codex；新native需重新准入，不按旧running.json重复。
5. 本任务没有 App 打开。focused E脚本独审＋actual私有CLI ws add/ls预检通过；已异步询问当前约3分钟真人IME可用性，**须收到现在方便才启动**，此问不是R2-028入口澄清，不重复旧拖放。旧 W Vite23837（原PID98386）保留；新浏览器与23838/23839、S23827/28/29均已由 owner 清理。恢复必须再核实际身份，不按旧 PID 直接结束进程。
6. shared Core API/lib/Tauri/native_io/gen/catalog 仍由 `round2_history` 单一 owner 串行接 W DTO。Root 当前 bindings 是 S green2 实际生成 **165 managed files（164TS＋inventory）**，逐 SHA 与文件集核准；catalog 沿用真实产物。Root types0。生成根还含既有 lock/catalog，不能把整个目录数量当 managed binding 数。W leaf 不改变接口。

## 当前包、证据与缺口

| 包 | 已实现与证据 | 剩余工作 |
| --- | --- | --- |
| E 输入/草稿 | 单 accepted buffer owner、同步草稿登记、迟到读取/merge/mine拒收、standalone discard；最后 E refresh `aba5ac53/64a70347/c4243305` → Root `dbba8321/6bd7023b/4e63a4b4`：仅保 fresh-read intent，composition end 读新版本，单物理 stat/text 请求并合并后续 intent。原红保留；最终 refresh18、accepted20、sync9、merge9、types0，独审通过 | 真实 Mac 文件中文 IME、最后一键→正常 OS Quit；不得拿合成事件冒物理证据 |
| H 长历史 | 有界 IO/页/记录/缓存/取消，实际 ChatView 大历史定向证据 | source 全文搜索/复制、最终原生组合；loaded DOM 不代表完整源 |
| C 内容打开/导航 | 普通内部/Cmd系统直接分流；终端软折行/CJK路径；返回最新cwd保草稿 | 原生组合验收 |
| G 材质 | 共用正文渐隐、窗口底材质归属和菜单厚底，消费者已封口 | 三种窗口材质、深浅色、原生/触屏组合 |
| W 工作区 | 正式布局/独立commit/shared preview/pin/dirty Undo；完整分支 all-parent 固定HEAD分页后端及 UI 已集成。UI `31d1336d→0478df63`，pure9/tabs6/memory5/types0，宽8/窄前5＋窄修后1＋余2分阶段真实证据。最新 watch leaf `d979ffaa→ce69e0bd`，Git真实控制路径、私有有界owner规则、过滤Access；实际15例绿，独审通过 | watch runtime/DTO/前端订阅与真实kernel证据、Face关联及原生组合；first leaf不是实时更新已完成 |
| A 外部观看 | A1/A2能力身份、A3 journal capture、A4真实双Node含取消/断线/响应预算及 Core/GUI实际生成；DetailPane→reader浏览器5组绿 | 全能力矩阵及最终组合；live GUI hydration 尚不由 A3 证明 |
| S 恢复/生命周期 | 首 slice `94131dc8→9112f1b7` 精确 opaque view lease贯穿begin/open/read/write/control/leave，旧A不可重查id操作新B，双视图非破坏性读，pending有界/取消owner明确；实际Registry9＋后批43共52行为，三lib check0，真实gen。test-only `7246b665→4b1c64bc` 真实ChatView lateACK/retry/cancel/replay5绿 | liveGUI NoLoad/journal hydration、startup forwarding字节有界、完整GUI/terminal宿主丢失/重启恢复；L01原040 deadline、L02原page_steps竞态仍独立未解；M后继续，不丢范围 |
| B 浏览器 | 只读调查 | 五项正式功能与真实WK/mobile验收均未完；CLI200不能证明WK拒绝已修 |
| M 移动输入/切换 | 键行pure9f＋fb4、UI2423已独审/Root e409集成；实际catalog、12模型、旧33×9、types/words、9Chrome含真双window/320px/隔离试按绿 | M025正式跟手、真实Codex行为与手机触屏/系统软键盘仍未验；不标M完成 |
| D/N | 旧卡死未知；Busy多入口已定位 | 限定调查与并发夹具、明确结果；H绿不关闭D |
| Q/R2-029 | 共享括号公式转换，保护code/escape；MD门/9文件/7合同/types绿、独审 | 原生文件渲染组合 |
| V | **L04/L05 done**：真实来源/严格产物绿，focused L05正常CmdQ与宿主认证replay、私有清理独审绿 | 不替代E/S全轮验收；原270s失败保留 |

## 当前工作副本与下一检查点

- **Root**：`khor-round2` / `integrate/round2` / `5ddc574e`。E三片、S两片、W leaf顺序集成，Root受影响JS/types绿。Lab测试文件不自动进入产品。
- **shared owner**：`khor-round2-history` / `fix/round2-watch-shared` / **6c20c13b**＋8源候选；保留S `fix/round2-history@b14f73a8`。shared接W API/DTO/Tauri/真实gen/catalog，唯一native见上；S Captured Core桥尚未接，不能直接整链集成。
- **W consumer**：`khor-round2-watch` / `fix/round2-watch-ui` / **6c20c13b**＋前端dirty；backend `fix/round2-workspace-watch@0f911558`已保留。controller/hook/Tree/Store/Pane/Side与FileView非text按精确写权实现，等待actual pubgen与组合验收。cached entry/visibility return必须即时fresh；quietpoll不得放大读取。M025、B依次在其后。
- **旧 W pager**：`khor-round2-git` / `fix/round2-workspace-git`，UI `31d1336d`、Lab `74028700`；已完成源码/浏览器交接，不复跑无变更绿批。浏览器宽/窄各阶段的SHA范围保留，不冒同一最终SHA整批通过。
- **root E tree**：`khor-round2-math` / `fix/round2-draft-sync` / `c4243305`，已全部集成且clean。Root三次collection、standalone、accepted/刷新反例详 input/draft-sync，不再扩工具专项。

## 实际原生与来源边界

`/Volumes/disk0/Khor/evidence/round2/validation/native-editor-0478df63/result.json`：exec97920终态 **0 / 253.027s**，实际metadata/catalog/bindings/types/Vite/CLI/corefixture/GUI完成；source/env稳定、provenance完整、remaining[]、cleanup无错/TMP不存在。最低双盘约7.65GB/11.85GB。**仅build，没有App/IME/Quit证据。** 该产物早于E新刷新和S lease API，不能用其旧二进制运行当前Root前端并声称当前完整证明。新E真人实例须先准备同源产物、私有工作区/直接编辑入口，再询问用户可用性，不能让270s时限消耗在准备上。

S证据：`evidence/round2/history/session-recovery/green-2`，exec95761终态0/246.613s；1479源前后同，43行为绿，加green1 Registry9。ignored app worker明确未跑。原green1两个HTTP夹具缺lease失败保留，修的是3个fixture JSON，不冒生产源变化。2个stage自有小TMP后续按身份清理，原记录未改。S浏览器5/9.537s，193源/gen前后同、ownChrome/端口/profile已清。Root首次脚本路径猜错与source-scope字典比较错误是记录/命令setup，正确入口补验绿，不冒产品红。

Clean Code按每片段的领域命名、状态/资源单owner、纯规则与效果边界、明确取消失败和移除替代旧路审查；平台/协议/并发必要注释保留。**新独立问题只登记下一轮；既有34缺口继续。** 先前剩余8–12小时只是当时估计，当前尚非收尾，真实设备时间另计，不按提交数报完成比例。

本次实际当前源产物：`validation/native-editor-ce69e0bd`（Root ce69），GUI SHA `06e3809ca745ecacb8e3e6d947ea70cda83feb30a63d54bd9320ba1f7c79e9d7`，CLI SHA `531221ce20337204d2607324f9c27b0efe9332a0064433ba21e02ba10f571039`。已逐复制二进制SHA再核，仅build，尚未启动file-only App。历史0478来源边界仍保留。

M首pure `9f034973` 固定，new11/旧keys33×9/types绿，尚未集成。独审发现preencoded focus-report过滤及坏配置可被toggle/move覆raw两处阻断，作者按原spec收口，原失败保留；UI仍待后片，不能以pure绿标M完成。

M两阻断已由fb4db8bc消除并独审核准；9f＋fb4现集成Root3a78cfe2/644806ff。Root实际new12/旧33keys×9states/links21/attachment11/types0，七源除既有C1的TerminalPane均等作者最终SHA；TerminalPane只增加预期origin/focus守卫，S发送未改。receipt integration/m-pure-6448-receipt.json。作者获授UI后片及M卡，M仍未完。

E待真人不冻结Root集成：复用原有clean `khor-round2-validation`，保原`fix/round2-validation@8bf8c424`，另建`validate/round2-editor-ce69`固定ce69。该树2167 source frame整个dict与实际native ce69输入精确相等，真实gen167逐SHA相同，旧gen164按uid/regular/lsof/devino/SHA可逆保留在Eout；未新建工作副本/缓存。E contract仅root/entry/module切该固定树，helper/bin/observer内容未变，独审准入通过。故此后Root6448的M变更不会污染受验ce69文件编辑源；E仍无App、等待用户当前可用性，不能拿这一片覆盖M终端或最终整轮。

W runtime候选bf11a6ea/dae8b19c已固定但未执行。独审发现source running/queued吞replan intent，可能漏初始化期间新目录；作者先按既有generation/fence收口单pending intent并加确定性反例，再交Node实际21准入。38远端流/HTTP强制0及真实gen等shared胶水后一批，不用stub绕依赖。

M UI2423f2b2已独审并集成Root e409a993，七源全等git-show；原子同步实际catalog4cbc1e9f…92a1前核Root/作者9个catalog源逐SHA全同（非手写产物）。Root words/types全0。实际producer ui-catalog-1三phase0/8.422s、2156源/env稳/provenance/cleanup全绿；ui-js新12/旧33×9/words/types0。browser-1实际9组0/10.6776s，2156源/167gen前后同、ownChrome/23840/fixtures全清；source映射与catch截图仅script差异独审通过。receipt integration/m-ui-e409-receipt.json。M026/028实现已在集成，但真实Codex/触屏/系统软键盘仍未验；M025正式跟手没有因此关闭。

当前两实现线：history收到S NoLoad首Node闭包正式写权（精确hostloop只读Capture token/fence、journal完整record EOF/同File身份、可信cursor；先NoLoad实际private host+两个token，不提前Core/GUI/gen）；W稳定DTO到来时仍由其shared单owner优先短串行。input的W v2固定e65e8a92及23正向＋两受控负变体准入待尾核，尚未native。随后input在W稳定后承接既定M025，B在其后按R2-REVIEW已存交接，保持两实现线，所有34仍在范围。S startup/L01/L02及H全文搜索/复制均不销账。

Root另做原L01只读准备，未启动新native：history/session-recovery/L01-read-preparation-6448.json固定旧raw实际SHA、原8.97s强时间戳断言、当前两源SHA及下一exact baseline要求；未猜rootcause/未改产品。真实Codex只读确认本机CLI0.165.0、二进制SHAa4509915…e9a85a、--no-daemon可用；custom provider/凭据文件/analytics等选项已查官方配置文档，尚未启动Codex或模型请求，后续验收计划须隔离home/禁发送/有限owned清理。

2026-10-29 最新调度与来源补记：history已核GuiOp/GuiNote均在gui_host.rs，不撞W proto/lib；S首Node token固定[u8;16]，typed Captured结果/20KiB总预算、NotReady/ReplayInFlight/Unavailable，完整EOF同File身份，不扫描/不调用ACP load。Capture只作可见性fence，不无依据增加per-view fsync阻塞控制loop；完整前端内容预算仍需后片核，不以fence掩盖无界live累积。

真实Codex验收准备（不是运行）：mobile/codex-input-preparation/plan.json已保存本机0.165.0实际help/binarySHA及官方配置来源，固定私有home/work/no-daemon/loopback假provider/不按Send/Enter、真实Khor PTY与物理chord对照边界；尚未启动Codex/模型请求/网络服务。当前重点先完成W v3，Root不另开L01或Codex native争槽。E真人可用性问题仍待答；已答R2-028不再重复。

W v2历史终态（不回填）：exec3014=2，Cargo48670编译101，DialFailure无Debug格式化错误；没有负case执行。stopped.json实核process[]/remaining[]/TMP空并精确恢复e65正源clean后交回。新1dc只复用unreached_words+pub(crate)两行，v3 driver/owned正负字节不变，独审通过。

S NoLoad首Node source-only checkpoint ac53589d（五源）已独审，当前上述首次实际编译测试中；不要在Root全项目先集成，Core对Captured穷尽桥后片未接。新桥方向已改为canonical journal唯一正文/H三预算，live仅控制/状态/wake；已核原draft UUID在Core SayWith被丢，后片复用原token并考虑巨record raw引用的小origin，未在本片暗增。

S first capture-node-1实际终态：strict metadata/libtest/verify均0，前3各pass；第4在fence/page及A有序barrier之后新增halfclose清理期待4s超时，原NoLoad零load断言位于其后未执行，不能外推整case绿。日志actual initialize/new/prompt/prompt由作者独立核不替代原case。Reviewer准仅cfgtest明确Both清私有A、保业务/NoLoad断言、关B/reap，不改产品/加时；原halfclose未知性质单列债，未归因L01/L02。后仅4+5复验，纯3沿旧精确SHA。

W v4最终实际边界：producer compile0约77s，raw3/kernel7/wire1全过且原1s/500ms断言保；old Owned12/负2分别沿v3真实源，不声称全部23在0f同SHA重跑。readonly同bin一次nocapture复核0.55s/20reads+500ms quiet过，stdout无事件trace（未取得旧事件根因），source/strict/TMP净。backend/Coreleaf整理交接后shared/API/gen/CLI38/前端仍待，所以W未完成。

2026-10-30 S Node收口固定：cfgtest修正b14f73a8（父ac535）最终5SHA独审通过，Node1 EOF2/Journal1＋Node2 NoLoad1/原hosted-journal1分源5绿；NoLoad最终零load/resume、token barrier、H页1/1/2、富prompt/reap均到达，未冒一次final5全跑。原halfclose4s失败仍未知性质，NotReady/ReplayInFlight实际失败时序、startup/L01/L02及Core/GUI恢复未完成。该Node链不先Root并入导致Core穷尽桥未接。

共享/消费新基线：history在同树保fix/round2-history@b14后切fix/round2-watch-shared@6c20c13b（e409＋bf11/dae8/e65/1dc/0f五ref的cherry），集中CoreAPI/Tauri/roots/GUI API三动作五DTO/HTTP服务端0/native≤20k/唯一git-history-updated及私有watch_bridge_tests。候选8源已独审准入，尚未native/gen/types；准备Core/GUI libcheck→DTO1+HTTP2→tables4→实际bindings/catalog的最小批，CLI1含38streams待新consumer/真实Viteassets后。input保backend0f ref，在同watch树切fix/round2-watch-ui@同base，单controller11/hook受控2/metadata5等初源检查通过，StorePage迁移/Tree/Pane尚待实际pubgen/组合。不以控制模型绿冒ReactDOM/WK/整W。

**最新FileView写权**：Root已graph-first核现readAt正好通text fresh-read intent/单physical/accepted/composition，不需新prop或重挂。现非text preview只初读/reread不会跟readAt，属于W023可见文件内容缺口。Root已精确移交input仅FileView非text preview effect＋Away元数据私有请求槽（单IO+合并dirty、identity/generation/离开不followup、失败不自旋/保图、同owner total），text accepted/edit/composition/保存/refresh严禁重写；Root停止该文件写入，源在input branch闭包后独审。只更新Khor自身图片/SVG/PDF/文档fallback/metadata，不扩自动系统QuickLook或外部app。SideWorkspace已授隐藏树watchActive一行及可见file父hook/readAt最小胶水；旧无父owner的事实已登记，不假称side实时。root固定validation E树/已问的物理IME可用性仍不动。

2026-10-30 现场补记：W消费者作者误把现有 `check-editor-overwrite.mjs` browser runner 当纯脚本并发启动；exec82774实际exit1，Node97929与正证9后代已按出生/cwd清理gone，原失败及fixture按inode保留在 `frontend/overwrite-unapproved-start`，不计E任何case。refresh/sync/merge三个纯检查各0；后续检查先核入口、browser/native须准入。

2026-10-30 Root将独审 `fa62a6ac` 两源单独集成为 **e351ad70**，source逐字节等author；非text单physical/latestintent、samefile刷新保图、失败可关/明确retry、原生office同owner stat，text accepted/edit/composition/save/refresh owner未改。Root actual preview8/types0，receipt `integration/nontext-e351-receipt.json`。完整W订阅/真实界面仍未验，不能以这片关闭W023。shared-1最终证据已独审核准，消费者获准接57de＋72527及真实gen后先types/纯检查，再固定版本申请browser；无handwritten bindings。

2026-10-30 最新W组合checkpoint：Root **5ddc574e** 已顺序接五backend链＋shared57de/72527＋consumer9318/26fd，集成映射 `integration/watch-source-1.json`；36变更源逐字节等author26fd。实际shared生成来源885个Rust/Toml/lock逐SHA同，managed169＋catalog4b4b从immutable staging复制（旧165含DirWatch.ts已可逆保留，未手改DTO），receipt `integration/watch-generated-5ddc/receipt.json`。source-only集成不冒完整W验收。下一最小批 `workspace/watch/cli-current-5ddc-1/run.py` SHA18ee4a49…45fc准备独审，types→currentViteassets→actualCLI/test→一exact含38far sources，600总/300步/5GiB/strict，尚未启动；native仍FREE，须登记实际handle。

最新WDOM实际运行：exec12182，driver37372 / Node37386，Root5ddc/source+171gen+fixture固定；runner f6025ffb…b7f6独审准入，23841/全API内存fixture/独立Chrome profile/180s/owned+TMPuid-devino-lsof。恢复核同handle/result，不重复启动。无App/物理验收；Cargo槽FREE，作者仍仅source不新native。

WDOM v1 exec12182已terminal1/0.731s，配置CJS互操作setup失败，零DOMcase，source/gen/fixture稳定、owned[]/portfree/TMPabsent；原失败保留。v2仅fixture添type=module包边界与路径/manifest，runner418874a3，Root产品未改。当前实际exec40871/driver38072/Node38093/23841，180s同handle，不重复；Cargo仍FREE，无App。

WDOM v2 exec40871已terminal1/14.413s，React Refresh HTML preamble缺失setup失败/零DOMcase；strictclean。v3只把fixture HTML送Vite transformIndexHtml（断言/产品均未改），当前实际exec85324/driver38961/Node38976/23841，180s同handle，不重复；native/CargoFREE，无App。

最新runtime以 recovery/current-runtime.json 与实际handle为准：WDOMv3 exec85324已终态1/12.205s/strictclean，原treepulse1绿、dirty-case期待与既定E defer合同不符，未改产品；v4换实际treeLS+pendedread/undo clean验证，exec91529/driver39850/Node39864/23841实际运行。CargoFREE，无App。

当前WDOMv5 exec85775 driver40720 Node40735仅补Store＋原未跑latebegin两组，23841/180s；v4 exec91529已terminal1/strictclean，前四实际DOM过，Store fixture设备me修true（此前无本机/dir=null未读为预期），非产品改动。原四绿不重跑、不冒一次final6全绿。current-runtime.json实时，CargoFREE，无App。
