# H 长历史有界加载

状态 working；2026-10-29用户“开始吧”授权下一轮实现。main/ship未授权。
owner: round2_history；工作树 /Volumes/disk0/Khor/worktrees/current/khor-round2-history，fix/round2-history，基线e7c335c8。
范围R2-006/S1，先固定读取/传输/渲染基线与预算，提出稳定分页/跟随/取消接口，再实现一个可独立验证的有界片段。不得先接Codex全量观看或扩成通用分页框架。
允许写：原生历史提供者/GUI历史消费与定向检查；禁止CodeEditor/FileView、App.tsx/TerminalPane和全局样式。图谱定位后先报root实际修改路径/接口再改共享协议和生成器；生成结果不得手改。
读SESSION-CONTINUITY、ROUND-2-EXECUTION、ENGINEERING。证据 /Volumes/disk0/Khor/evidence/round2/history。只本任务卡可写中央文档。
首次返回不能扫描整份历史；单条/批量有预算；原记录不截断。跨页顺序/工具关联/迟到响应与取消要明确。native/Cargo唯一槽须root协调，target按PLAN。不要构建第三套缓存或运行用户会话。

## 首片段检查点（2026-10-29）

已确认读取链：Claude/Codex transcript_since从from到EOF全读；gui-core history_follow整批转帧；ChatView首读from0且全量创建items。首次全量读取和巨型单行确实无预算，但未复现用户App冻结现场。

新增crates/node/src/adaptor/transcript_window.rs及mod登记；Claude/Codex的transcript_window调用共享读取原语。256KiB总读预算（含左边界1字节）、64KiB内联记录预算。before为exclusive、超EOF截到当前观察长度、到0无previous。每页绝对范围；未完成/切开的行返回Fragment引用，完整大行返回Oversized引用，原文件不改。并发变化按实际读量标range。

验证：root授权standalone rustc --test，1.95.0；纯std原语8项通过，包含尾读预算、0/EOF、短读、巨大行跨页前进、半行追加、UTF8边缘、并发截断/追加。legacy-red使用现有read_to_end IO序列的独立algorithm probe，3MiB读取超256KiB预算，exit101；不是链接原provider执行，不能冒完整产品red/green。证据 /Volumes/disk0/Khor/evidence/round2/history/window-check.json、legacy-red.log、window-green.log，产物约1.1MiB。编译已结束并向root释放唯一槽。

未完成：Node crate/vendor wrapper尚未完整编译；新入口尚无Node/IPC/UI实际调用，旧history/follow仍全量。文件身份（等长替换）、片段拼接/按需大记录、工具关联、取消、界面分页/有界渲染均属后续。当前仅基础片段，不计S1或用户卡死已修。

下一步：固定首片段提交供非作者审查；向root报告history_page稳定契约、Node/IPC/生成器/GUI最小修改清单，获共享路径所有权后继续实际消费。完整crate编译须另行协调槽，不能由standalone结果推断已编译。

## history_page 纵向契约 v0（待实现，root已授共享路径owner）

精确基础片段提交8bb3c459，非作者round2_reviewer已读。root允许H唯一写Node lib.rs、adaptor历史、gui-core chat.rs/api.rs、Tauri lib.rs、gen_bindings producer、GUI api.ts/ChatView和局部history模型/样式。生成器准确位置apps/gui/core/src/bin/gen_bindings.rs，roots宏注册新HistoryPage/HistoryRecordChunk类型后由既有生成器生成TS，禁止手抄接口。

拟接口：history_page(id, cursor?, direction: tail|older|newer) -> HistoryPage。固定预算：每次最大读取256KiB（含边界），单条内联64KiB，最多128帧；预算不交由前端任意放大。回应含frames、older/newer游标、明确记录引用、reset、following。游标绑定session/source/file身份、绝对记录offset及record内frame位置；一个JSONL记录产生>128帧时必须续该记录或给明确按需引用，不能游标跳到记录后丢第129帧。

文件身份不能用单独mtime：append会改变mtime但不应reset。应使用已打开文件的平台身份，并对截断/同长替换/边界变化分开核对；保持snapshot位置和必要边界指纹，不能声称仅len<offset覆盖等长替换。身份与游标须同本次文件句柄读取，不能先stat旧路径后open新文件。

首屏由尾部开始，向前页只取有限字节。左边缘小记录应在预算内向前页重叠恢复，确保不会永远变成两块被忽略的片段；超大记录/未完成尾行保持引用，不能当JSON错误静默忽略。大记录仅由用户按需分块读取，禁止自动递归重组整条/整文件。完整行才能进入vendor utterances_of；工具call/result跨页用稳定call ID关联，缺call不得启动全文件扫描补齐。

GUI只读路径使用有界页面缓存，初拟最多8页/2MiB原始载荷（帧展开量还需单独预算），超出淘汰可重载。预取有限，generation隔离快速切换/迟到回应；older prepend保留锚点，跟随只在接近底部时移动。不能仅限制IPC而继续无限items.map。搜索/复制不依赖当前DOM的全部历史，缺后端来源时如实说明而非假搜索全量。

实际生成/桥接路径：Node::transcript_page → gui-core chat::history_page → core/api.rs dev桥及src-tauri/lib.rs off_main命令 → gen_bindings roots导出HistoryPage → GUI api.ts使用导出类型 → ChatView still分支与局部history模型。新链接完后删除被替代全量只读消费，不为了旧端并存两套可写真相。live ACP replay路径不顺便改造。

## 编译槽与setup失败

首次cargo check -p khor-node --lib --locked在创建debug/incremental时EEXIST，exit101，未进入编译。root核为旧退役缓存断链，按可逆方式改名并建空目录；H未清理缓存。原setup失败记录history/cargo-check-setup-failure.log保留。

root重新授相同Cargo check槽，CARGO_TARGET_DIR=/Volumes/disk0/Khor/caches/rust/khor-quality-target、CARGO_BUILD_JOBS=1、CARGO_INCREMENTAL=0；第二次独立日志cargo-check-retry.log/result.json。开始磁盘外盘20GiB、内盘6.6GiB。只check Node lib，不test整库、不隐式gen或native App；结束报告释放。运行现场以证据cargo-check-running.json PID和实际进程为准，恢复不能重复启动。

## 最新编译与纵向预算（2026-10-29）

修复target断链后cargo check -p khor-node --lib --locked重试exit0，21.8s，精确源8bb3c459；vendor wrapper已由真实Node crate编译。仅现有loro/link/trust warning。首次“尚未编译”是历史检查点，不覆盖本条；原setup exit101保留。唯一槽已向root报告释放，无cargo进程。来源与日志见history/cargo-check-result.json及cargo-check-retry.log。

纵向slice追加预算（实现前固定）：最多128帧，返回序列化frames不超过512KiB；单帧超过预算以有源范围的引用替代，不先复制大正文后才计量。载荷生成过程逐记录限额，所有128+输出必须由record内游标续接或引用承担。GUI同时限制8页、512帧、2MiB已接收序列化载荷；超出从远离当前阅读位置的一端淘汰并保留重载游标，先核页元数据预算再合并，禁止unbounded items.map累计。必要大记录引用由用户按需读取每块不超过64KiB，不自动递归加载。固定数值对应本轮首个性能夹具后可按实测解释调整，不为追绿事后放大。

取消契约：每次打开/切换生成独立generation；离开或切换立即停止旧poll/预取调度并拒收旧结果。同步文件读取单次256KiB不具备途中强制撤销，在接口中据实声明，其有界步骤后检查取消再解析/发送；不能声称JavaScript忽略结果等于取消原生工作。native worker取消与transport token要在实际接线时明确owner，不能后台持续重读。

## 实际纵向slice检查点（8bb3c459之后，未提交）

已写Node adaptor/transcript_page.rs及Node::transcript_page，直接解析真实Claude/Codex来源的有限窗口，无全量fallback。Cursor绑定source与同一File句柄metadata的file身份、观察len/mtime、record范围和record内item。append不reset；替换、缩短与同长改写标reset；增长同时原地改写还需provider generation，不能声称通用检测已完成。小行跨窗口通过older边界重叠恢复，巨行纯fragment页严格向左。

Node真实lib test已执行5项通过，1m35s构建/0.08s执行：300事件单记录128/128/44无漏重、边界小行恰好恢复一次、巨行引用与进度、append/rename、source/range拒绝。证据page-test.log/result/source.patch（含实际源diff SHA）。随后按独审补了原inode set_len截断断言，尚未重跑，不冒旧5项覆盖新断言。

gui-core chat::history_page/response已实现HistoryPage、HistoryEntry稳定record:item键、HistoryRecordRef身份绑定cursor，逐entry无额外序列化缓冲计量；128条/512KiB上限，wire budget不接纳的旧事件通过older准确续接，过大expanded event显式引用。为响应encoded_bytes固定点计算避免字数位数跨界误差。消费的是已由provider限制的内联record，计量前的结构转换仍有其固定上界，不冒任意大输入零分配。

实际接线已写core/api.rs dispatcher、Tauri history_page off_main+命令注册、gen_bindings.rs roots注册真实HistoryPage依赖；GUI api.ts使用真实生成HistoryPage类型，无手抄结构。新增3项core wire预算/refs定向测试尚待运行。当前服务只tail/older，newer、chunk按需、取消贯通、GUI实际只读消费/缓存未完成；旧UI仍全量follow，不能算S1已修。

root当前turn已交付，作者核无其他Cargo进程且E完成、审查者只读，已向root登记按其“报调用后独占”条款占唯一生成槽。命令cargo run -p khor-gui-core --bin gen_bindings --locked -- <本树apps/gui/src/gen/bindings绝对路径>，sharedtarget/jobs1/incremental0/TMP外盘，证据page-gen-running/result/log/source.patch。生成运行中不得恢复重复启动；完成立即释放，若失败保留原日志按明确失败修接线。

## 纵向服务片段验证完成（2026-10-29）

真实gen首次已编译Node+gui-core成功（1m05），但发布拒绝inventory154 vs实际157；准确extras是本作者树既有ts-rs生成PlaceRow/QuotaWindowKind/Wheel，无业务API引用。逐文件生成头校验后可逆移至history/preexisting-extra-bindings，SHA收据保留，未删除目录或改producer规则。重跑同一gen成功（0.91s），实际生成HistoryPage/HistoryEntry/HistoryRecordRef与inventory；原拒绝exit1完整保留。输出快照/哈希generated-history-types(.json)。

core真测试3项通过（14.31s构建/0.04s执行），覆盖序列化预算截断后older续接、expanded引用、giant raw不调用parser。Node真测试5项重跑通过（1m05s构建/0.10s执行），已包含新增原inode set_len断言。两者源diff SHA与日志在core-page-test*/node-page-test-final*；没有增加其他features/矩阵。native/Cargo唯一槽已明确释放。

全GUI TypeScript直接命令tsc --noEmit --pretty false exit0，API使用真实生成类型；无手写TS结构。Tauri命令注册源码尚未独立编译完整App，不把core compile当整个native通过。当前工作树将提交纵向服务片段，下一步仍是newer、记录chunk、取消贯通、GUI有界缓存与ChatView实际消费，旧全量follow尚未替换。

纵向服务片段提交：2950ddb2d581860c4cef367e373b7cb514c49191，作者树clean。内盘任务独立恢复bundle round2-history-2950ddb2.bundle，收据round2-history-checkpoint.json；不修改root的全局refs/manifest，待root下一检查点整合。

## eca83a8b newer/chunk/lease 稳定服务检查点

已提交eca83a8b，位于C接口依赖bb21b8d3/f6d9819及ChatView独立C1胶水efeaa12b之后；root只需各自cherry-pick对应胶水/服务提交，不把C依赖副本重复整合。C1普通path/URL进入会话内容，无cwd也保留URL，去launch及旧preview分支。

服务实际实现read_next（正向record内item续接、小半行等待、巨大pending引用并前进）、64KiB显式raw record chunk（base64 bytes，无lossy文本转码）、begin→单次claim→worker check→RAII释放及cancel。先begin握手多一次轻量RTT，客户端离页时未完成begin在应答后cancel而不派IO。unclaimed lease有30s admission期限并cap64；claimed cancel只标stop并保留容量直到worker drop，避免取消后仍不可中断的worker逃出上限。sync256KiB读、Node::open/目录定位途中不可强制中断的边界保留。

真实gen→Node10→core预算4→request生命周期3全部exit0；newer-worker/source-manifest.json对每个新/改文件逐路径存全文，包含新history_request.rs，运行后Rust8文件SHA逐一相同（verified-rust-source.json）。原始日志/results.json不变。Node覆盖正反300事件/半行追加/巨大pending/原字节chunk/取消与旧5项；core正向wire budget后newer续接；请求3项含cancel-before-claim、claimed容量占用/释放和过期。Tauri源码仅语法/接线，未完整native App编译，GUI仍未真实消费。

eca真实生成产物已在任何catalog改动前整体快照到newer-worker/generated-eca83a8b，SHA256 manifest为generated-eca83a8b.json（158 TS+inventory=159文件）；producer与git show eca完全一致。root可按明确快照同步ignored绑定，无需重编同源。native槽已释放。

现正写GUI有界cache和lease source模型（尚未提交/验证）；独审指出并正在修：空页推进游标不能被另一方向非空页回退、输运失败须cancel未claim lease、dispose须追踪in-flight取消与begin、清理错误摘要有界。下一步接ChatView取代旧全量follow，并以源文本稳定key/锚点验证；随后按需raw chunk viewer必须streaming UTF8解码且显式逐段读取、不自动拼全记录。catalog zh/en仅history_*新键已获root单独写权，需真实producer生成。

## GUI实际消费纵向片段（待固定提交）

ChatView只读still分支已移除from=0的followHistory消费，使用historySource（独立lease owner）、historyReader（单页操作/可见性/自动有界追赶）、history-window（8页/512entries/2MiB响应预算取先到者）。旧RPC目前仍在源码，已确认无GUI消费者；下一个小片段删除，live replay不变。

首读tail、older/newer显式加载、离页/隐藏取消在途请求；未完成begin应答后cancel且不派IO，异步工作输运失败先cancel再释放owner，dispose追踪begin/work/cancel并给有界错误摘要。响应代次隔离不称作强制中断同步IO；不可中断Node::open/路径定位/单次有界读边界仍保持。自动跟随has_more可逐批追赶；用户阅读离开尾部暂停。

GUI帧投影每entry保留稳定source key，避免prepend把相邻历史正文合成一个无界段；工具结果跨保留页按call ID关联，缺call时先显示结果本身，可继续加载更早内容，不回头全文件补索引。内容pages身份不变的cursor-only poll不重建paint/Markdown；older空页或失败完成清锚点，内容prepend按可见历史节点保留位置。reset同时清缓存、raw viewer、展开状态和旧锚点。实际DOM锚点/原生长历史观感仍需隔离Lab/App验收，不以单元测试冒实机通过。

HistoryRecords仅显式读取64KiB原字节区段，streaming TextDecoder跨段保留中文/emoji边界；仅一段正文驻留，不自动递归拼完整记录。换引用/重头读先清next，首次失败不能沿用旧offset。raw引用元数据同样计入页字节预算；当前引用入口集中位于已加载内容下方，最终视觉待Lab检查。

catalog zh/en仅新增9个history_*词条；真实cargo run -p khor-catalog --bin gen_ts --locked exit0，临时文件成功后原子发布，source/output SHA见history/gui-consumer/catalog-gen.json，native槽已释放。C1根App胶水432d98ce仅作为依赖cherry-pick到H的c57d3ae4，root无需重复集成依赖。iOS validator胶水8449f825仅两行，依赖C ec9cf06c，不冒本地完整iOS编译。

18项JS定向检查已通过（包括缓存双端/空页游标、lease失败/退出、backlog连追、cursor-only内容身份、UTF8流、真实HistoryRecords.load回调换引用失败、稳定段落key、缺call结果）。全GUI TypeScript通过；最后reset锚点/内部key修正后同命令按新源快照复验中，最终以gui-consumer/final/results.json为准。第一次gui-consumer/results.json记录的是前一固定片段，不用于冒最新diff。无用户App操作，无main/ship。

## 已固定GUI消费、旧RPC清理与验收Lab

GUI实际消费提交77168f9b6bada5563103dad1e791dccc0bf570a8；非作者逐10路径git-show SHA与gui-consumer/final/source-manifest核同，18JS与全tsc exit0/unchanged=true成立。真实catalog产物已独立快照gui-consumer/generated-catalog-77168f9b.ts（SHA b499d69dc0c94876441b97ac1a37d3bba9c377b6ea33f8cb5e1896d58c586d58），同名.json记录两语言源与git-show一致，供root无需重gen同步ignored产物。

旧全量GUI只读入口已单独删除为a3e9b6ad：GUI fetchHistory/followHistory、core history/history_follow/Followed、dev分派、Tauri命令/registry均已无入口；mobile-static的history_page/record要求async，begin/cancel纯内存admission入sync名单。Node vendor transcript_of/follow保留其他消费，live replay/resume未改。真实gen空staging成功exit0/68.993s（Rust源unchanged），mobile-static[]、tsc exit0，证据retire-full-read/source-manifest.json、result.json、gui-checks.json与generated输出逐文件SHA。native槽已释放，Tauri整App仍未编译；独审5文件diff无阻断。

独立实际ChatView still Lab为499dbe4f：lab/history-reader.{html,tsx}、scripts/history-preview.vite.ts/history-fixture.mjs四文件。实例仅127.0.0.1:23836，URL /lab/history-reader.html，PID88855（以history-lab.json与实际进程为准）。只stub原始API输运，不替换生产ChatView/source/cache/reader/Markdown。20万逻辑记录按128条惰性生成，不写大记录文件；raw396000bytes仅64KiB按需分段。GET /__history_fixture记录pages/entries/bytes/lease/cancel/chunkranges、最近64页，未知和已退役API显式400并记账。控件追加500条、下一页延迟3s、隐藏面板、卸载视图、替换来源；供root浏览器验首屏、DOM有界、prepend锚点、自动追赶、退出取消与原始字节。Vite依赖缓存目前52MiB、外盘仍约20GiB；不是新Rust缓存。最后fixture修改触发一次Vite自动重启后已冻结源码，避免验收中改变来源。

当前Lab视觉/浏览器结果待root回传，不能用模块测试或输运fixture冒真实磁盘巨历史/UI原生验收。真实提供者路径解析只枚举文件名/metadata，不先扫描待加载transcript正文；Node::open/目录定位尚非可强制中断，全请求成本不等于256KiB正文读预算。该限制继续保留，暂不扩通用索引治理。

独审补充：a3e9b6ad source-manifest含6源（含未改producer）均与git-show匹配，真实gen/mobile[]/tsc收据无阻断；499dbe4f直接消费真实ChatView且未知API拒绝的边界已核。Lab raw分支未模拟cursor generation/identity拒收，也未延迟raw读；故该Lab只用于分段展示/UTF8及page可视/退出取消，不计raw旧来源/取消已验。按既定范围不为此扩新矩阵，生产backend仍按其定向源测试与明确限制评估。

## 实际ChatView浏览器验收回传与A交接

root在冻结23836实际ChatView上完成13项浏览器检查，证据history/browser/reader-check.json：近期首屏有界只读；empty polls保DOM身份；连续5次older精确保留可见节点top且淘汰后DOM<=512；追加500条分四批连续无空隙；raw单段<=64KiB与UTF8；来源reset清raw；hide/unmount取消后lease归零且不再poll；legacy0/unknown[]。首脚本仅中文按钮标签写错导致harness超时，原记录保留；后续只跑未验阶段，未冒前8项重跑。该验收是原始API fixture上的真实GUI，不是用户供应商文件/完整原生App现场。root接收后继续安排A，main/ship不变。

按root交接，H作者正在只读调查A/R005而非空等，冻结ChatView/Lab不改。40个known launch snapshot+任意用户argv的能力/来源矩阵、当前native provider/托管ACP缺durable journal、ACP import直接resume等实际缺口已落evidence/round2/history/observation-survey/R005-observation-survey.md，source.json逐路径SHA。官方当前v1文档及本地锁SDK schema核对均表明list是元数据、load是恢复/回放且带MCP上下文；没有独立readonly observer承诺，不能推导load必然抢占或必然无副作用。计划统一native source与所有托管ACP canonical journal，复用S1读/缓存链；完整最小切片与paths待root分配后实施，不开始S大范围。

root提醒：下一次ChatView必要改动时清除该模块已失效的长段历史/tombstone注释及过时全量replay陈述，保留现行不明显的契约理由；不另开全仓注释专项。


## 收口审计待项：全source搜索/复制（2026-10-29，root补核）

SESSION-CONTINUITY §5/§3及原计划要求只读阅读/搜索/复制不能只依赖当前DOM。现有ChatView未发现消息搜索入口（settings.find等不是消息搜索），复制在子消息组件，history_loaded_only说明仅限定已加载内容，不能当全source搜索/复制能力已交付。有界page/cache/read/cancel已有证据，但整个H不得标done。当前A3固定不扩此批；后续A4/H→S交接核真实入口、source能力和原要求，按既定任务补缺，不伪称已实现全历史搜索。

## Source-backed Find首normal worker checkpoint（2026-10-30）

Root批准worker复用现JS `paintHistory` 单正文owner，HTTP/native原有bounded page/record RPC与historySource lease继续使用；不另写Core/Node search引擎。query默认不区分大小写，与现CodeMirror SearchQuery/xterm Find一致；UTF8 query≤256B，选项随worker/query实例固定，case lower按Unicode codepoint流式处理，字面搜索/不新增regex框架。worker一次仅持一个≤512KiB/128entries page，每次≤32hits、snippet≤512B，同item内保留matcher/offset续接，结果带可重取page.newer fence+entry key/part/原始UTF16偏移；不以DOM查找正文。巨raw范围未处理时明确unsearchedRanges，不能冒全complete。

正常4source fixed `531a6ee50e57c50a31b4d5e2d913f0d69e42fb0e`；`history/search/normal-worker-1`含精确全文、git patch、manifest与内盘同名mirror。实际`node scripts/check-history-find.mjs` exit0、8项：真实paint排除非正文JSON键/附件base64；Unicode/表情跨块+lower expansion原位置；32+32+6同记录续接；tool更新/正文diff现投影；UTF8 snippet预算与未检索raw；query/page硬限；production worker+实际historySource晚读cancel/dispose不复活；source reset停止。两个实际Node worker threads均exit、owned0；不是浏览器/原生后端/GUI入口或giant处理验收。source fingerprint是测试后捕获，未伪称有before frame。

重要未闭：AgentTool可见raw input的command/prettyJSON正文仍在renderer私有表达式内，需下一小片提取同helper供render/search/copy，当前normal checkpoint不能标全正文支持。尚未GUI入口、giant两遍scaffold、projection动作、完整message copy/导出或browser验收，整个H继续未done。Root允许纯projection准备动作：validated ReadBinding格式、scaffold≤64KiB、只普通body Note，128frames/512KiB、budget超限coverage=false；严禁系统事实/Origin/CaptureGap提升、不另registry。巨字符串第一遍流式skeleton/ranges，第二遍解码被原Rust规范化→现paint选中的可见text ranges；两遍IO均计每调度256KiB/块64KiB，field order/UTF8/JSON escape边界必须验证。

### Tool input单正文helper后续checkpoint

`3caa3ebe33d5c83f9ac77c38d330f5fbf2eae585`：只acp-view纯toolInputText/helper+agent renderer调用、find model/controller和同脚本fixture，六source manifest `e485b0f8e1be03d9de0a4b44c2fa7ada02b5fdfc517a493771fc59e1ef369a7b`已独审。renderer沿原diff/patch/raw条件，原command或two-space JSON正文和复制值只生成一次；有限预算search先按UTF8转义/缩进/cycle/depth128计量，超过64KiB不构造pretty而计未检索内容，不能冒complete。renderer默认Infinity保原行为，不能用本片声称既有展开renderer巨输入已限。

9实际检查/三个实际Node worker线程0/owned0，包含helper等价和oversize前JSON.stringify未调用；原错误fixture compact37212/pretty48617没有达到预算，旧exit1留在`tool-input-1-first-fixture`；改后fixture先证compact45312<64KiB、pretty70517>64KiB，原核心断言不放宽。首tsc发现新renderer children可空，明确空文本coalesce修后仅余旧S/B真实gen未出的20诊断，H/helper无诊断；不称全types绿。原9核心运行不依React renderer源码，coalesce后只跑针对tsc不无故重9。start在error/reset后不再新load；sigma形式等价回归，仍不是完整Unicodecasefold/locale normalization承诺。全文/patch/after manifest/validation与两类型log已内盘mirror `h-find-tool-input-1`，source前帧未捕获不伪称before/after。

下一按Root批准giant两遍body projection/helper+worker后接GUI search/copy出口；当前无UI/giant/backend/browse/native新运行，不影响Root固定S/B验证checkout。

### 同源scaffold projection原语已fixed，Rust/生成待验

`4b3fbf86cebdd9b622acf552ac8606ec2549fad5`，14path manifest `d89f3ebf78597779b7af05c6da45276d2caa209cef5cbe4df8f9d71e402e26a8`已独审，无native/gen通过声明。唯一新read-only动作`history_projection(id,binding,requestKey,cursor,start,end,scaffold?)`，near ClientRead guard持真实spawnBlocking；同validated ReadBinding选择原Rust parser，单JSON compact后规范化（scaffold≤64KiB），仅普通Note，128frames/实际encoded512KiB，超预算/空parser/未知或坏record保complete=false，Origin/CaptureGap不能借scaffold升级系统事实。Synthetic entry key仅scaffold，不能冒原record定位。

同动作checked Cursor.at_record_range：start/end必须覆盖原fragment且有序/≤原snapshotEOF，sourceID相同，file身份/append/reset校验仍由后续真实record读取承担；返recordRef missing_start/end=true，仅字节选区、不证明JSONL语法。worker只保首cursor+min/max scalar归并，不累计几百fragment游标；完整lexer后才可判断区间完整。新增内部LocalUserEcho复用journal原trusted turn判定，原page仍同自身rawRef，projection可明确跳过可信echo而坏Reference未检索；旧echo原范围/SDKpayload/turn断言保留，仅更新分类variant，不冒旧S38完整同源。

纯projection没有home IOclaim，historySource.project所有结果都会用原cancel owner释放homePending；active从Set变同ownerMap(key,release)，dispose/abort/error/finally共用一次release。最终retired判断放在所有await cleanup之后。保存candidate1旧Source运行新回归真实业务红(object非null)在`projection-release-red-1`，不是编译/timeout；fix后`projection-pure-2`真实Find11/reader22/mobile0、15source前后同（含未测lexer文件不代表它验过），随后仅RustRef保守flags改变。原native5 cases（Node parser/checkedrange/原echo、Core预算2）源码准备，尚未领槽/执行，truegen/types/UI/giant两遍仍待下一片。全文/patch/旧red/pure logs已内盘`h-find-projection-2`。

### 流式lexer与直传giant worker后续（2026-10-30）

lexer `a5c390b3` 两source已独审，实际14项/3真实Worker线程仅证明结构与解码原语；`record-scaffold-1`全文/after-only SHA及镜像保留，types22均旧S/B/H真实gen未出。随后direct-giant-candidate-1十source实际19项/6Worker线程，前后SHA相同，已交独审未fixed：生产worker/controller经原historySource转发单个record/project IO，64KiB块/256KiB合作调度，两遍均计读取预算；仅一块缓存和128个长字符串范围/256B前缀，完整JSON/正文不拼回内存。跨page fragment仅单cursor+range标量；projected template通过同paintHistory/historyText选可见长串，保留静态正文连接符，支持跨块/跨内容块/prettyJSON转义搜索，32hits/64KiB实际响应继续限额。

新增actualWorker正向以两fragment页→同源raw+projection输运→32/32/6精确UTF16位置，hidden JSON键不搜索；另source变更清旧hits，held raw IO卸载原lease cancel且不复活。原始IO只fixture，真实Rust规范化/新gen/GUI入口尚未验。Rust内部BodyProjection改(index,Value)，synthetic key保原record事件索引以便item引用筛选，Core source反例10..139→128th key137仍待native。工具where仅导航元数据不独立呈现，已从search文本清单移除。

直传片尚不能关闭既有Claude/Codex giant工具正文：Codex arguments字符串JSON解析与Claude ToolDone trim+16KiB可见截取必须复用原Rust helper返回明确转换事实再接同lexer，不能以临时coverage=false/原raw导出来冒全H。搜索UI/jump、单message copy/超预算显式导出仍未接；整H不done。首worker写入路径错误已单列record-worker-first-write.json，随后的16项当时仍旧worker，不冒giantworker验证；真正接线后19项才证明上列生产Worker范围。types首出现本片union/implicitany诊断已修，retry无新H诊断、旧22仍保留不称types绿。当前无Native/Cargo/App/browser新批。

独审对直传candidate1补核缓存命中也必须服务worker任务队列：新增同12KiB原chunk被64个模板复扫的actual反例，原源真实false≠true（不是timeout），log direct-giant-cache-red.log。candidate2统一同一256KiB account计真实IO、cache返回slice和static/escaped输出；实际20项/6Worker线程exit0，十source before/after同，manifest592574869b69b470d40f8b84f1a58362cdd8d3726df3a7df0defb674519c7c90。types final actual2仅22旧生成诊断。全文/patch/red/green/after/type logs已双盘h-find-direct-giant-2；该片仍待独审final/fixed，未native/GUI完整声明。Root已授权下一小片仅cagent.rs原result_text trim/MAX/suffix、adaptor/codex.rs原arguments/input选择+JSON解码事实共享，原行为与同常量必须保留。

直传candidate2已独审source通过并fixed `c37dcac42e47281ad6e6ca0c19bc1b685f161ff8`，十git-show SHA=manifest；原20/6线程+source前后映射保留，未native/parser变换/UI声明不变。lexer前置a5c另源已审；原primitive4b3 Rust5与本次内部index改动一并留下一次真正受影响定向批。

### 原有工具正文转换事实候选（source/pure已审，Native待授权）

`tool-transform-candidate-1` 十一source manifest `60d65fe6126527000c32794dcef5c35d811d4a58c28a0144d7e1db3a8569a1d6`，全文/patch/22actual与6Worker线程/source前后同已镜像。仅两typed facts：`HistoryProjection.transforms` 的 `JsonString{pointer,parsed}`（原Codex arguments非null优先/input fallback与serde解码同helper）和 `ToolOutput{item,max_bytes,suffix}`（原cagent ToolDone join/White_Space trim/16KiB UTF8边界/原标记）。普通replay行为保持，scaffold仅推迟会被占位字符串误导的trim/cut；SDK Native来源选择仍在validated binding，不在worker列vendor名单。

worker同一lexer最多解一层被原helper选中的JSON字符串，再交原Rust normalizer确认parsed（包括数值范围规则）；失败仍原字符串fallback。额外原始读、encoded准备、cache复扫与decoded输出均同account/块64KiB/每256KiB合作调度；无完整原record/巨body重组，scaffold64KiB、128ranges、同一外层container范围，输出政策最多保64KiB（实际原常量16KiB）前缀/待定空白。Core facts128/实际serialized16KiB，正文128entries/完整response实际512KiB，原record/rawRef不删。

新增两原规则反例用c37固定findRecord模块做内存override（其余helper/scaffold是当前源，非整旧revision）：实际两个AssertionError分别196038≠66006（未解内JSON位置）和2≠0（未trim NEL），保日志/准确源SHA。新22检查包含hidden encoded字段不被当command正文、超16KiB尾不搜/截取标记可搜、非法inner JSON原样fallback、inner源变化不能被语法catch吞掉、UTF8宽空白截口不跳过后续短空白。它们仍是JS+stub-normalizer证据，真实Rust helper/typed响应及新生成类型尚待；types首实际24条为当前未生成S/B/H类型/词条，不称完整types绿。

既定Native最小11为Node4exact（原projection/checkedrange/echo+新两原helper事实1）、Core3exact（原2边界+新typedfacts1）及真实 `khor-gui-core --test command_tables` 四项。GUI仅普通--lib签名check含RootL02叶7def→作者f908，不跑GUI libtest/App。首次runner把tables错指GUIlibtest，独审在启动前抓到，原35f脚本/纠正记录保留；现957065d22ee08fbeaae7c45f19b4a9f4a5a5c722036693bdd93b5f186d7d6030已准入未启动，所有无谓GUI OUT retention已删除、零cache修改。后真实bindings空staging/同binary managed逐SHA与full noEmit --project；catalog不重gen，Root准按九源与原producer一致复用cc744实际45ac快照（旧author文件可逆保留），Root277assets只作Rust图输入不冒H UI。整批600/步300/双盘5GiB/严格source-env-tools/BUNDLE/owned清理不变。搜索UI/jump/复制与超预算导出仍后续，整H不done。

### 工具事实/原语真实Native检查点已固定

`d418709b99d0cd272314fe9689fd84452f2b63c1` 已独审尾核：十一git-show SHA=candidate60d65f=Native2198输入before/after对应值，完整运行前后/环境相同。`tool-transform-native-1` 同session94683/driver19497真实terminal0，382.771944秒，17phase全0；严格实际四producer（Node/Core libtest、Core integration command_tables、bindings binary），Node4exact各1+Core3exact各1+tables实际4=11全通过；Core/GUI普通签名check0含Root7def→作者f908 L02函数编译，不冒L02App场景。owned remaining[]/TMPabsent/cleanup[]，唯一Native槽已归还Root。

实际bindings 177 managed（含inventory），empty stage与同binary managed出版逐SHA同，新HistoryProjection与HistoryTextTransform真实生成。相较Root cc744实际175 managed，只多两个TS和inventory变化，无其它TS差异。Root实际45ac catalog按九源与原producer一致复用，原author产物保留；full `tsc --noEmit --project` exit0，旧24未生成诊断已被真实产物解决，未手写生成文件/未再跑catalog或旧38/pure22。177产物全文与所有原小件已双盘mirror，fixed-source-map SHA12e45f2c9ec88a221f1e3f132630a0f01cbc26466b97cca06c6d4dfa8dccfbdb，generated manifest fffcc2f1ef63e9f44cca6d076c5b60de65c47a10c31e6d30680cda52e8fd7af4。

恢复索引CONTINUE在commit后由running改terminal/ref；原dual-small-manifest保run时SHA不改，新增post-commit-mirror-manifest d7b1b7628d86767d6b0ce730f324f918d7c28576b1c075ea96f46b9878440d53明确唯一旧条目dc6803→d46dd6及新增元数据，当前双盘小件逐字同，原result/log/proof不回填。未动任何GUI OUT缓存，没有GUI libtest/CLI/App新运行。下一仍H真实搜索入口/结果jump/单消息copy与巨记录明确导出/失败边界/浏览器验收，不把当前原语/工具变换通过冒整个H完成；S startup/L01/L02等原账不销。


### Full-source search UI / copy / original export — source candidate (2026-10-30)

`history/search/ui-copy-export-candidate-1` captures the 22 dirty paths on author d418; this is not a fixed or compiled checkpoint. The actual worker now shares recordText with context/copy, so one body projection covers normal pages and giant references. Copy re-reads one selected part off-main and refuses incomplete or >64KiB output instead of returning a truncated “complete” copy. Native export reuses the existing file_handoff Entry/Job/state/cancel/ready_path; no second registry, parser or public DTO. `history_export_start(transfer,id,binding,cursor,start,end)` validates the same binding and source cursor, then streams 64KiB child Begin/claim/read/cancel operations. At most 32 unfinished history jobs; cancelled workers retain that admission until real completion. The completed app-owned original uses `file_handoff_save`: desktop OS Save panel and sibling-part/rename commit; iOS/Android existing open/share. Browser FSA remains a separate fallback, not the App’s sole outlet. Ordinary files retain the existing handoff behavior.

Search lives in actual ChatView; local Find opens it, normal hits position the reader at the original source fence and pause follow, giant hits inspect a bounded excerpt. Binding is part of the component key and layout cleanup retires the old owner. The existing visibility/zero-height owner synchronously retires search, rather than adding another observer. Copy uses ClipboardItem with promised data submitted in the click handler; graph review found no general native clipboard-write wrapper (existing iOS edit_menu copying is selection-specific). Actual WK clipboard/OS Save/share and browser UI remain to be verified.

Actual JS receipts: 26 find checks / 7 real worker threads, reader 23 (including exact search-fence navigation + explicit latest), mobile-static exit0. These are separate from old native11. Full noEmit is currently exit2 only for ungenerated new catalog fields; the new 19 zh/en labels and deletion of obsolete loaded-only wording require the real catalog producer. Two new Core IO tests and command_tables4 are source-only, not run. No Cargo/App/browser was started in this slice. First test edits with wrong cwd and a types command using nonexistent tsconfig.app.json are retained as setup mistakes, not behavior red or new-test passes; retry logs carry the actual counts.


Native UI/export attempt 1 (`ui-copy-export-native-1`, exec56136/driver94375) reached true terminal false in 26.809088s: metadata exit0, signature check Cargo101/E0624 only (`Node::root` is private). No behavior test ran and catalog was not published. Source/environment stable, observed owners gone, private TMP absent, no forced cleanup; the native slot was returned. Candidate3 is a one-file correction: pass prepare's actual root into chunk/ClientRead::claim instead of opening a private Node method. No Node API was widened, no raw UI leaf was inserted, and no existing assertion/gate changed. Candidate/runner2 are awaiting fresh source admission/slot; attempt1 logs/result remain unchanged.


UI/export retry2 is actual green (`ui-copy-export-native-2`, exec92510/driver1151): terminal0/205.966690s, 10 phases all0, Core IO2 each exact1 + command_tables4 = six real cases; Core/GUI lib signature check0, three selected strict actual producers0, real catalog atomic publication SHA84476d706bc0e8642f91ad9c69993d7352977e118a630ccc2c18a9cc36433d07, full noEmit0. All 2203 input files and environment stayed unchanged, owners empty/TMP removed/cleanup errors empty. No bindings/old11/CLI/App were run. Native slot returned. Fixed commits: backend 44fa2fefb86b4bcd14bd3a3ae07d38431dede347 then UI/catalog 5daaad3d664816d4420613829813af2afb99338f; final22 byte SHA match candidate3 and both actual native input frames (`fixed-source-map.json`). Real catalog is immutable catalog-stage.log and byte-identical published src/gen/catalog.ts; all previous binding bytes unchanged. Run-time dual-small-manifest retained, new post-commit-manifest records only the two added source/validation maps, mirrored byte-identically.

The current fixed UI has source-backed search/context/copy and a native original export for complete search-hit records. Actual browser/reconciliation/visibility/clipboard still pending; actual WK OS Save/share is not covered by the IO tests. Root separately authorized raw HistoryRecords right-click + one shared private export hook, so unparseable/non-searchable records can export without a search hit. Fragment bounds will use existing bounded page flags plus scalar range and checked widening, never call one fragment a complete record. Those frontend leaves are next after this fixed checkpoint; they do not require repeating the native six or unchanged bindings.


Raw UI fixed cb03c4825687b23aa3718993eac64eb0b425d0b9: one ChatView export hook shared by Search/HistoryRecords and the existing visibility owner. Complete raw references do not need JSON parsing; partial references locate adjacent physical boundaries via bounded page metadata/scalars and checked widening. Search preserves false boundary flags only when the source already proved both boundaries and the lexer then succeeds; JSON syntax alone does not upgrade unknown physical boundaries. Pure29 / seven real worker threads and full noEmit0, zero further backend/catalog changes.

Production browser sequence is explicitly not one green batch: Browser1 reached three business groups then failed a nonexistent catalog-key locator (harness), with old plain probe wrapper not claimed as scroll geometry; Browser2 corrected real flex/scroll viewport and reached three groups, then actual UI red at raw-export success (fixture job saved complete expected range, old local copy-limit notice still visible). Minimal two-file product fix425d919f6e5386aed5aeef0b642eb9a361cfeea3 prioritizes current export status and clears only idle export results on new local intents; full noEmit0. Browser3 did not create a page: Chrome.launch180s timeout, zero business groups. All three source/env/tool/owned/TMP checks clean, original logs/reports/screenshots retained. No App/WK/OS clipboard proof is claimed; the browser uses an external clipboard memory sink so tests never overwrite the user's OS clipboard. Next is one same-seven-group/no-longer-timeout continuation with real Chrome launcher/Playwright fingerprints and only proven private-profile process samples, pending Root's slot.


Browser4 retained original false: Chrome launch and exit succeeded and the first six business groups passed (source search/jump, activation-safe clipboard sink, giant limit, full malformed-record job/save, held A→B isolation/notice, 320px/IME). Group7 exposed the fixture retaining completed page/record Request slots; separately the unchanged outer owned guard got EPERM on signal0. Current post-check ESRCH/no members does not rewrite that original result. Product425 is unchanged. Browser5 is prepared, not run: only the transport fixture retires each completed page/record key, while projection remains pending until explicit Cancel. Claimed held IO Cancel marks stopped and retains its slot until response-finally; group7 now requires exact-key once-cancel and zero uncancelled active before release, then final ledger0/no continuation/unmount0 after IO completion. Original seven groups/deadlines/strict helper remain unchanged; preliminary unexecuted a7/88 admission is retained as superseded. Actual App/WK/native Save/share remain separate.

Browser5 exec14557 is retained false: six groups passed again, source/environment/tools/browser-tool fingerprints stable, owners empty/TMP absent and no forced cleanup. Document-hidden stage exact held100 received Cancel and dropped after release; the subsequent zero-height stage left exact held104 claimed but not cancelled while search DOM was gone and newer page polling continued. This is a real failure of the stricter lifecycle oracle, not the old completed-lease fixture error. Root authorized only bounded mode/key/API-sequence and parent/scroller/ResizeObserver content geometry diagnostics before selecting any product change. Browser6 is prepared but not run; no padding/root-cause claim is made from source alone.

Browser6 exec73785 retained false/source-env-tools stable/owned empty/TMP removed. The bounded trace proves the zero-height failure: parent/root rect and clientHeight0, scroller clientHeight24 with 12px+12px padding, original ResizeObserver contentRect.height0; original visibility predicate resumed and kept the exact held105 uncancelled. The actual failure this time is hidden-search-close, not a claim that Browser5 failed at the same event. Minimal fixed8f65423e0f708cc3aa02a0db76fcf3f1f398e613 changes only installHistory's readable content-box fact, seeded from initial dimensions then updated by the existing RO; canFollow/visibility share it. Scroll/anchor clientHeight coordinates remain untouched. Full noEmit0; Browser7 prepared with the unchanged seven-group diagnostic script, pending admission/slot. No native/export/catalog rerun.

Browser7 is actual green: exec37853/driver81919/Node81936 terminal0, seven original groups pass, 2205 outer inputs and 623 UI+gen/script inputs before==after, environment/tools/browser fingerprints stable. Both document and zero-content-height held leases receive exact once-Cancel while claimed slot remains, late response settles then ledger0/no continuation; unmount ledger0, unknownRPC/page errors empty. Owned remaining[], TMP absent/cleanup[], Chrome normal exit. Product8f65423e0f708cc3aa02a0db76fcf3f1f398e613 remains byte-identical; test-only script fixed410c1f1b3dd151a0b272fbbce716d08c66bcb88c. Fixed source map47651442675b9f58fe85cef890482ddbbbb47b945db72662bc89cd4529fe8b6d/resultfe3a6d879a470eb38a7069415440fc9978edf1469d12887b3a8de929a6c656d5, 24 small evidence files mirrored byte-identically in h-raw-export-browser-7. Browser slot returned. Original Browser1–6 failures retained rather than overwritten. This validates actual production UI/Worker/reader/export-hook binding, not the fixture Tauri normalizer, native filesystem, WK clipboard, OS Save/share or touch/OS IME. Root current App verification is next; native6/catalog remain their original separate evidence. Entire H/S/round not marked done.

### Production Mac platform preparation (not executed)

Root chose a default custom-protocol+devtools GUI, full normal App UI and existing CLI import. `khor-ui-assets:/assets/index-*` is the web asset revision, not a URL origin; the current production Mac main document should load embedded tauri://localhost/, while actual DOM origin/isSecureContext/ClipboardItem/userActivation must be recorded rather than assumed. Prior666 --no-default-features/devUrl evidence cannot close production WK Copy. One private HOME/vendor/store contains only a synthetic Claude native source; public CLI workspace/hook/import produce the actual observation identity/locator, no vendor agent/writer/model request. Actual record ranges and SHA derive from generated bytes, not fixture hint IDs.

Frozen preparation plan6b111a7b444673091ae62d43e06f567a50a94a4e5d12ce8a48816309b8c3cf36 in history/search/mac-platform-preparation, eleven small files mirrored in h-mac-platform-preparation. Pure synthetic format self-check only:1201 records~403KB; first two hand-counted candidate-prefix fixture failures retained, no store/CLI was executed. PhaseA future owned App runtime240/200s: real source Find/nativeIO and complete giant original OS Save/byte comparison/normal CUA Quit. Root explicitly rejected JXA/AppKit clipboard helpers: no previous clipboard data/hash read or persisted. PhaseB remains required and separate, user normal Copy/Paste synthetic marker into scratch during fresh-availability Mac IME session. No App/CLI/Cargo/native/clipboard runtime occurred in preparation; final binary/source/gen/admission pins await Root's one current production-variant build. H not marked fully done.

PlatformA is now executable-source preparation (not run): private launch.py/launcher.py/attest.py/contract.template.json and CUA-STEPS reuse the existing strict owned wrapper, source/gen/assets/binary/proof pins, 240outer/200inner/5GiB and normal CUA Quit/10s ACK. Real CLI candidate/import stdout is parsed, actual Observation record must match native exactUUID and returned alias; no hand-written binding, no replaced App index or API. Actual OS Save bytes/SHA are independently checked against the selected original; after App owners exit only its one complete, UID/regular/nlink1/SHA/devino/lsof-verified handoff cache is reversibly retained, unknown or partial TMP files still fail/persist.

Reviewer caught that require_provenance under env-i runtime would incorrectly reject the real build context. Fixed only the private runner: outer temporarily supplies each actual receipt's complete non-credential build environment map, verifies its hashes and unchanged original require_provenance/copy-to-selected SHA, then restores the controller environment. Runtime App/CLI remain clean whitelisted private HOME/store/vendor/env; no helper/receipt is weakened. Final source preparation pins outerf267abd7d4254651b800010363ce5cca61dc7a43b170435a226d5e6c81080977, launcher5ffae61d153ec51d2a9a6122d87530fc5ead1669050e638ce2bc2f04d9c487b1, template6ffaba2c3e2f30dd0e8f56de9b2595b3621e90127e72948c906e7b8d77553d43; four Python AST checks0, no CLI/App seed executed. Source-only review cleared; template ready=false, artifact/proof/environment fields null until Root has one successful current production variant. Root18136 was an actual floor stop and supplies no guessed replacement artifacts. Copy/Paste/IME PhaseB remains separate/fresh user availability.

Replay32MiB boundary audit is read-only and fixed in history/session-recovery/replay-retirement-analysis/RETIREMENT.md + source-inventory.json, mirrored s-replay-retirement-analysis. Actual canonical readonly/open/fresh/lost capture and initial open_one Load/Resume do not instantiate Replay; retained chat_replay HTTP/Tauri/facade still genuinely reaches the32MiB collector, so UI absence alone is insufficient. Root agreed in principle to retiring the unused old operation rather than another spool. No current MainUI/CLI/MCP user caller was found; Handle.replay's sole production consumer is old Replay.start, with only ACP test consumers otherwise. Facts.replays and legitimate explicit initial restore remain. The precise cross-owner list distinguishes obsolete History bracket/API tests from current history integrity, Stop/Close, cancel/reap, Elicit/config, per-view source/fence and lost/reconnect requirements that must stay truly proved. Old GUI smoke/History fixtures are not presented as current canonical green. No Node/ACP/shared source was changed or runtime started; input remains Node startup owner, final cross-layer contract is pending.


### Production Mac A2 partial and existing Inspector route (2026-10-30)

Root-owned actual70439 is terminalfalse, not export acceptance. Actual ws/hook/candidate/import four CLI children all exited0 and persisted the returned observe alias2078916e…49c2d for the synthetic complete nativeUUID. ActualGUI51240 opened; Root CUA reported read-only tail1104..1199, Search and Read original record1. Origin observation was not completed in60.771855s, so Save/Copy/normalQuit were not reached. Outer forced cleanup had remaining[]/TMPabsent. Root's later unintended default-profile reopen and normalQuit are independent post-run facts, not repair of this run. Original89074 import/setupfalse and70439false remain unchanged.

Read-only follow-up `history/search/mac-platform-a-2-readonly-diagnosis` is double-mirrored; report **e456400e6b7972c248ac4cead16133a8e5db96006e7f0770948dab5c789286a2**, facts **e1a0ab8123d39c3dadf0c5119b17913273971067086e9329637fe813f5bd4137**. Existing source keeps the native right-click menu inside real input/textarea fields, so Search-input→system Inspect Element is the next UI route, not background/result-row custom menus. CmdAltI uses Tauri's internal-toggle command and current capability lacks its permission; this is source evidence, not an actual captured ACL refusal. Actual default devtools/Wry enables inspector extras. Whether the real native input menu contains the item still requires actual CUA; document origin/secure/ClipboardItem must not be filled from constants or khor-ui-assets stderr.

A2's .app envelope had only khor-gui. Pairing the already-qualified matching CLI at Contents/MacOS/khor is a reasonable fixture installation correction, not a proved cause of short GUI children: owner records did not include argv, and MCP worker deliberately self-executes the same native binary before window creation. No new App/CLI/process started during this audit; Root owns any next fixture/contract/run. PhaseA still noCopy/noClipboard, PhaseB real CopyPaste/IME remains open.


### Actual Mac A4 partial (not Save acceptance)

Root delegated the single CUA/App run with actualproduction84 GUI533d/CLI5f52 and finalcontract61eb97e5. Five scripts were exact A3 bytes; only the six fresh run/mirror/role/ready fields changed in preparation. Actual outer **42116 terminal1**, driver93831/launcher93987/App94067; four real CLI steps each0. The exact private App was bound only while its original PID was alive. Actual CUA saw main document `tauri://localhost`, synthetic read-only tail1191..1199, Search input native Inspect Element menu, Inspector and Console prompt. The intended read-only JSON expression did not yield captured JSON; AX showed concatenated expression text. No independent SyntaxError/ReferenceError was captured, so literal input-loss versus AX text flattening remains unknown. A subsequent setValue fallback was not executed because the CUA target was inactive at the original deadline. No getApp rebind/relaunch followed.

Inner stopped at origin-and-history **61.21911391697358s**, false; no origin attestation, old-source Find confirmation, Save, Copy or normalQuit. Outer forced cleanup had remaining[]/TMPabsent and all source/environment/gen/assets/contract/scripts stable. Exact driver/launcher/App PIDs were absent after terminal. A separate preparation mistake placed the prep mirror at the final runtime mirror_name early; outer final mirror's assert failed after false owned-result was already written. This error is preserved and acknowledged, not repaired into a green run; future prep mirrors must have a distinct name from the exclusive terminal destination.

Original files under `history/search/mac-platform-a-4` were not rewritten. Additional actual CUA partial **64853a7f3a924bd253ecb069b5986ee18950a6c47e57b8d985f48e682b6e22f1** and29 selected small files/manifest are separately mirrored under recovery/h-mac-platform-a-4-terminal, terminal manifest **15ddddf1f24de07a58458fdaad95ce662d0897585512120dc35ee27bd95b02e5**. No App/CUA/native remains owned by H; slot explicitly returned to Root. No automatic A5, timeout extension, permission change, clipboard use or product modification occurred. PhaseA complete original OS Save and PhaseB real CopyPaste/IME remain open.


### Final automatic platform A5 actual partial

Root granted A5 only with finalcontractbd245a1a, original240/200/origin60/save60/noCopy/noClipboard gates. Samehandle **88908 terminal1**, driver1744/launcher1794/App1861. Four actual CLI phases each0. Actual Search-input system Inspector worked, and documented AXsetValue→Return produced readable Console JSON: **href/origin `tauri://localhost`, isSecureContext/ClipboardItem/clipboardWrite/clipboardWriteText=true, windowLabel main/inApp=true; userActivation isActive=false/hasBeenActive=true**. Original runtime/origin.json was attested live; this proves capability availability, not Copy activation/success. Actual production source search with unique ASCII queryCOPY visibly returned full old marker `H_MAC_COPY_70c1aa1b_中文🙂` while tail1191..1199 remained shown. The first full marker typeText attempt visibly lacked punctuation/Chinese, so query was corrected through ordinary UI; no tool/product unique cause or input fix is claimed.

The original actual-save60 window then expired, inner **119.04041070799576s**, before the raw reference right-click could open its menu. **No export menu/system Save/file output/normalQuit proof, no Copy, no model.** Tool inactive response was immediately followed by originalPID/terminal check; all exact PIDs1744/1794/1861 absent, no rebind/relaunch. Outer forced cleanup remaining[]/TMPabsent and source/env/gen/assets/contract/scripts stable; originalfalse retained. Distinct preparation mirror avoided A4's terminal-mirror collision; no new mirror assertion. This is the last automatic Inspector attempt—no A6. Real Save and human CopyPaste/IME remain open for a fresh human scene.

Original runtime/origin SHA `5ef0661dc57a9eaaac680997c817baaa987d0a359d92774b21ff4da0b4958c76`, innerresult `f0463d756adac96484bcb1b98b78aaee85b0514ed3ad5e15cb6add645ceb7da4`, ownedresult `33c89aa408e3f811f9297c13ce866f5e5ab8e4ddb67ae3f8d26d96464be77d4f` remain unchanged. Added CUApartial `777682022738d618be48d61fbd1fbf4c9d179c5bba6519322c478162eed51ca3` and26 selected small pieces are double-mirrored h-mac-platform-a-5-tail, terminal manifest `a3b5470d17ac3346a608247b1b9647b12efe5a1397a6ee81c16b9cb6e047bd37`; original launcher terminal mirror also exists independently. App/CUA slot explicitly returned to Root; no source/native/gen was run during this platform task.


### Canonical chunk grouping red and design (2026-10-30; pending source authorization)

Root337 real applyAcpUpdate/paintHistory red (chunk-projection-red-337, probe94a30b01/resultd105c33c) exposes same-channel legal chunks being split by the unconditional per-Note boundary. Native complete Utterance boundaries must remain. Current HistoryEntry contains onlykey/frame and Core has erased Native versus SDK/prompt origin, so UI may not infer source from key spelling. Design history/session-recovery/chunk-grouping-design-337/DESIGN.md SHAba5b8e322fa6a76b2495ce6a694971f0b7fb91d345dc23bfbc76b98b964108a3 proposes a single typed starts_message read fact, current optionalmessageId semantics, explicitTurn/Gap/Prompt breaks, current-window-only bounded contribution/display identity, and search/copy/raw/older-anchor verification. Canonical stored bytes/writer/Store/GUIHost unchanged. Design is not product implementation, native/gen/UI green or completion of H. Source facts2d5d2edc0b509b4b846ad02870bffc711fb1510dc589d70ed95f595f55218ac3, mirrored internally.

### Chunk grouping/current consumption checkpoints (2026-10-31)

The preceding design is now implemented in independent source slices, not completed whole-H acceptance. Read metadata8a0fe08e was independently reviewed and integrated; Node three read leaves and four necessary test adapters were formally returned to input. Canonical writer/Store bytes were not changed. Grouping90a2→845c preserves Native/Prompt/Turn/Gap boundaries while allowing SDK chunks and optional unknown messageId fallback. Actual pure red23/25→25/25 and mixed-ID red24/25→25/25 were separately captured; Root independently reproduced25/25. Identitya5b9 retains only bounded-window contribution→displayKey associations, one-to-one keys, reset scope; pure26/26, no global history map.

Core provenance87d6 plus consumer3cd→167b are independently reviewed source checkpoints: record metadata only from validated position/cursor, giant scaffold never invents per-entry provenance; unknown raw references—including overlapping/same-range item slots—break visible grouping conservatively. A mixed complete entry plus SDK tail is not automatically whole-copy complete. These slices were not in Root's current AI runtime cohort and still require current generation/type/native integration. Cross-page query/matching and complete-part reconstruction remain OPEN; declining incomplete copies is not completion.

AI original13652 collapse failure,76618 and73389 next-body timeout, and71253 Edit226 timeout all remain false. The controlled geometry/data-intent split850c was integrated asRootc2;71253 did advance past the previous body timeout and subsequent read/search updates, but failed waiting forEdit diff after one click. Main baseline AgentContent/diff/TraceCollapsible facts do not establish why. Diagnostic-only42e retained60 original assertions;6868 adds bounded main-document navigation, Vite console/stdout, fixed RPC counts and admitted open-lease facts, also without changing assertions/clicks/defaults/queue. No HMR cause is asserted. Current5 contract pinsRoot7e2e, actual179gen/278assets retained lineage, only current devserver source consumption; not new Rust/gen/full noEmit proof. Source admission passed; Root alone grants/runs the actual browser after fresh STORAGE reserve+peak check.

The scroll-compose source fixture5ff→1f29 uses real useChatScroll, AgentReasoning, historySource/Reader with fixed empty-page transport, and rejects unused record reads. It has not run. Anchor9234→2372 is reviewed source only: capture a concrete rendered Text/Range point within retained contributions; short context/suffix validation rejects unsupported changed-render matches, with explicitly limited old top fallback. The same hook owner adopts restoration, and its input-position ticket rejects any old page restoration after newer user input/toBottom/disclosure. Actual same-contribution text-point geometry over several RAF/ResizeObserver settles remains OPEN, not proved by a card-edge measurement or syntax checking.

ACP path-attachment migrationc571 is source-reviewed: original five capability combinations retain actual paste, two completed raw uploads, two placed chips and both returned paths in exactly one Send; images gates inline PNG independently. No synthetic embedded-document count is substituted. The existing absence of a new embedded-text UI entry remains NEXT-ROUND. Registry8d13 migrates to the real launch combobox→manage→attach-agent/catalog controls and reads actual names/details/actions; date, precise registration ID, single ask, four import states, idle/press and geometry semantics remain. Registry source review is pending at this entry; no ACP rerun is claimed. No new product capability/attrs/auth fallback is added.

Current author fix/round2-chunk-grouping source is clean at8d13c85e. Compact source patches/manifests only are mirrored internally under STORAGE; full runtime logs/screenshots remain external canonical evidence. No browser/native/App was started by H during these source slices. Current human Save/Copy/IME, whole-App S recovery and physical iOS acceptance remain separate open requirements.

### 2026-10-31 bounded query / whole-part source checkpoints

- Query fixed `0a25cb09c328917d6162ea23a7c93e3b70638031`: unknown-head non-overlap hits deferred until a real source start; shared append step/forward fold/Worker consumption and bounded Tool/compaction dependency confirmation. Six-source manifest `f58c5c85…136d`, evidence `history/session-recovery/deferred-query-source-1`. Old actual3!=0 phase red, new0 and three finite pure cases passed; reviewer source-approved. Chain1d→f16→8a→0a replaces the former suffix approximation, not approval of its intermediate wrong phase.
- Copy fixed `c0f00e4b5b029c868448d17fcddce120fedf037a`: the same iterator reconstructs one ordinary message through the original captured source fence/real body boundary, only returns complete after end proof, rejects raw tail/source reset/null and output>64KiB. Four-source manifest `0978a9e0…c1d3`, evidence `history/session-recovery/whole-part-copy-source-1`; old action actually rejects incomplete, new exact hello complete=true. Five existing finite cases pass, including actual Worker32+32+6 with copy between pauses/scannedPages2 and the existing Worker context/copy+source-reset case. Source hashes stable; source-only pending nonauthor review/integration.
- Full evidence remains canonical external; compact current source/patch/result snapshots are167KB/127KB internal. No Cargo/gen/browser/App or fullFind30 rerun. Actual held abort on the new dependency/copy path, text Range multi-RAF/RO geometry, final actual gen/noEmit/browser/platform Save/Copy remain separately open. ACP obsolete visual-probe migration remains next test-only responsibility after this fixed closure; no product palette/Trace change authorized.

- Review found c0f's opaque-cursor append ceiling insufficient: same-file append changes cursor versions and could extend whole copy into later same-group chunks. c0f remains an unapproved source checkpoint; its original finite passes are not overwritten. Narrow fixed `5b8c9f3203e3099ad0226ccdf12cb6bc424d5a92` now captures the first real page's last exact contribution/record end/raw-tail status, excludes post-ceiling contributions even from preparation, and refuses missing/unknown ceilings. Actual old c0f three-module probe returns `hello MUST_NOT_COPY` complete=true and fails; new exact `hello` complete=true passes. Four existing finite cases pass with source stable. Manifest `2aa007a9…fd9c`, evidence `history/session-recovery/whole-part-copy-ceiling-source-1`; nonauthor review pending. No new Node/RPC/actual browser/native.

### 2026-10-31 current Range/visual source handoff

- Root reports final query/copy components integrated through04f976ab with actual fullFind32/32(3.318s) independently reviewed; Reader28 remains prior unaffected scope. e9→1bd private history-search DTO/forward-IO fixtures were source-reviewed, Root61625; no original seven-group runtime claim from that preparation.
- `cc85258e0da38199904690117a648174e5709d5a` adds the existing-service retained Range group and explicit --range-cap-only baseline selector, source-reviewed/syntax0. Root55767 actual cap-only business red is independently reviewed: before/after512 markers, tail512/639 evicted, entry200 offset49/sample/key still present, actual top0→10240. No timeout/forced cleanup; strict stable. Canonical range-cap-baseline-29887a47 result SHA376b7236… and log5c1127fa…; no baseline rerun.
- `e122718879428808cd2ae34a897c06d578d1f52e` is the separate four-path source fix: shared projected UTF16 contribution spans≤512, proven sample→entry/offset, retained unchanged entry plus unique current DOM sample before actual Range restore, original source-suffix fallback remains honest for unprovable Markdown. Existing scroll ticket/newer-input rule unchanged. Worker explicitly retires spans with text. Helper/provenance finite probe0 + existing actual Worker phase0/source stable, not DOM green/fulltypes. Source manifest02946d35…dffcb in range-contribution-span-source-1; nonauthor review/actual corresponding Range+original7 still pending.
- `785f06659b820a8fdbd2fcd2e9a4801f6890c7df` ACP current visual carrier migration source-reviewed; only syntax0 so far. Actual TraceTitle/error ink+visible Loader shape/motion and Plan Check/Dot/Circle/.6 opacity replace obsolete Dot/all-pairs lightness assumptions. Nonvisual blocks/fourtheme-motion remain; no product CSS/palette changes. Original18947 suite false preserved; full current ACP actual pending Root.
- Root-authorized pure shared serde DTO `ac953c98cdf8b8d176302e672543b420ae22b6a8` is independently fixed/reviewed, not mixed with Range: SessionId id/previous, DeviceId authoritative home, typed recovery/shell results. DeviceId retains original byte serde; later GUI mapping explicitly hex. No public route, action authority/IO or TSgen delivered by that module. Evidence recovery-shared-dto-source-1.


### S3 app-layer source checkpoint 2026-10-31

Thin transport source b8b6937fbe48df5a83fed777ee47230ad480bc87 +4c604733f2d50ff20b24b578e54a8d0e5c205d96 has nonauthor source approval: three HTTP/async Tauri/TS functions, actual execution home/id, nullable expected_run, stable token and optional cwd forwarded; DirectoryRequired checked against oldSID. Mobile static source check0 only, no compiler. Evidence `history/session-recovery/recovery-api-glue-source-1` and `recovery-shell-directory-glue-source-1` (manifest459431e7…). Shared DirectoryRequired74a6 source already reviewed, Node native methods unchanged, Shell method appends cwd.

New UI source7ed07f94f090a446410b2fafb6fb7488c50b765b is under independent review, not accepted runtime evidence. Seven paths: private IDB caller payload repository/hook/RecoveryActions, DetailPane, existing HeaderSeat export only, two catalogs25 labels. Latest Root decision supersedes global single slot: per(home,id) one unresolved payload, total32,8KiB each, atomic readwrite count/put/conditional delete; no complete history or execution facts, no WebLocks requirement/fallback. Pending/Unknown/transport ambiguity reuse exact payload; user confirmed new cwd only after terminal DirectoryRequired then fresh offer/nonce. History stays independent; UI caller identity/request/lifecycle/mount generation rejects stale delivery. SameSID rebinds outer content and control via view generation or existing terminal Reload (never Stop); only newShell opens freshSID, showing previous. Node alone checks capabilities, native qualification, expected_run and execution state.

Author finite codec checks plus5syntax and TOML25 parse passed; these do not prove real IDB cross-window transactions, React/menu/rebind/picker/Node behavior, actual new gen/catalog/noEmit or WK/OSSave/Copy. Source canonical `history/session-recovery/recovery-ui-source-1`, manifest73cde16620590b3d95e2f4bfae810d8dd87feb709ab08b7162da36a95a9fe1ee, fixedmap0b84e83461bf962f39275d4582f6eda7941e697d4d2f0ded0692490235a13ff0; compact metadata only on inner disk. Old179gen/278assets remain old actual lineage and cannot typecheck/represent this DTO/UI cohort. No native/browser/App launched by H.


S3 app组件最终source独审尾核：author `4baf7f1a83c210f25a03fa76e36d4434a97209e9` clean。独立chain为b8→4c→7ed→3ad→3f→f617→3d→a8→1452→4b；全10枚完整ref和最终15paths SHA在`history/session-recovery/recovery-app-component-source-handoff-1/HANDOFF.json`（a51b1bcbb9f22b348d588da6d323baaf6c7da0573cd8732c0b0cd93657b83522），内盘仅紧凑索引。7ed发现的self-status误退delivery已3ad修；Terminal具体home owner/A→B→A单向retire/旧join与user-send入口/纯presentation等待已后片修，有限actual idle predicate和UTF8 metadata反例红绿只证明各自谓词/codec，不代替React。新UI不依WebLocks：本设备每(home,id)一条未决caller payload、总32、整条8KiB；SID4096按实际UTF8字节，不手截断。新catalog26词条未实际生产。Core3f仅把Chat pending lease与最终Node实际owner准入分开，Terminal旧拒保，未发布observe不会因pending lease获权；必须组合input资格后继，不冒独立恢复可用。后续须actual gen/catalog/noEmit及定向真实IDB/React/Node/App；既有H/Reader/Find/ACP通过按未变来源保留，不重复为本source做无关矩阵。H未启动任何重型进程。


Recovery17 验证source准备固定743a0fa47a815a36a6e7647dc52a65d2001ac5db，3测试路径已独审无已识别阻断，生产15paths逐SHA保持。原98901实际13项都是Capture/读取/迟到重挂，并没有S3按钮或Shell；原13名字与完整分支断言块73acfa8b…2f73d保留，不借原绿冒新能力。另同一既有harness增加4个明确新组：offer-and-same-sid-rebind、idb-retries-and-shared-pages、directory-required-shell、terminal-owner-transfer。真实DetailPane/RecoveryActions/hook、同BrowserContext同origin真实IDB、真实DOM pathdrop→production sender held IO/新lease/Ready与Terminal实例对照；只有外TauriIO受控，不模拟Node资格或执行去重。不新框架/依赖/产品testattrs；三syntax+ProbeTSX parse与source静态核已过，尚无任何新Chrome/Native/Gen/App运行。新脚本明确要求实际新catalog/bindings，不能旧179/shim过；Root当前cc92 gen完成后可按test-only delta复用真实生产types/assets再最终PIN。证据history/session-recovery/recovery17-ui-source-1（manifest b42b285a913b4cf0eccba18f334b50fd8d7ebd30b8453da312b65108a8f94d31），内盘仅compact manifest/fixedmap/semantics。旧H/ACP/Reader/Find绿不重复，本次准备不改旧false。


### 2026-10-31 S恢复证据只读复核（H，供当前进度，不新增运行）

按SESSION-CONTINUITY第2/5/6节，正常GUI退出、异常GUI退出、host死亡、boot变化、远端离线是不同验收边界。此次只读原始收据/合同/日志并与input交叉核对；未启动App/Cargo/模型、未改产品、未把旧false改绿。

| 目标 | 已有实际证据 | 仍缺及不能替代的边界 |
| --- | --- | --- |
| 存活持久host重连 | L05 `validation/app-ca83-l05-term/owned-result.json` exit0/source_stable，`runtime/persistent-reconnected.json` SHA af25c6088b9c53d31f58abcb1df5c75d6cfd64448a9a521231fbac8c5bc577a1：实际正常Quit后同shell/0ff7dddc6a58d94f、host57455/child57457，PTY5认证并读到139B旧marker，不发送Input/Ready。 | 这是正常OS Quit+PTY基础链，不是GUI2会话异常退出/全App再开。`history/session-recovery/full-app-noload-current-preparation/PLAN.md`和REVIEW-BOUNDARIES仍是draft，没有实际result；GUI异常退出→原host继续→同SID重挂→旧历史先显与下一Prompt仅一次，尚无实际App证据。Recovery17受控Chrome和Core view-reopen也不能补该现场。 |
| 无原provider/host后的显式恢复 | 公有Load actual70988：`public-load-runtime-source-6/actual-load-1/load-oracles.json` SHA f71be0342c372529e2bc36c344a537e88955c771c3cc9200ec11028daa6705b9，旧host951/provider952实际结束，新host1013/provider1014，sameSID/native、新run，Load1/New仍1，原binding/raw保，samepayload不再setup。Resume actual57844：`public-resume-unsupported-source-3/actual-restore-1/journal-resume/resume-oracles.json` SHA 58dc39e53b4d4ca004959c2746d81db3e559be964420c090d339308bec4498d4，同SID/newrun、Resume1/Load0、旧490B前缀与早Note保。 | 两场为真实CLI/Core、私有受控ACP，非真人vendor/API记忆或完整GUI cold-start。原result仍false（Load 3d457f86、Resume b62c463d），原业务全断言到达、已独审，why诊断另有冷留存收口，不回填；无需重跑已证的Load/Resume业务。若要求完整App无原进程cold恢复按钮/读史/控制闭环，仍缺实际场。 |
| host死而provider还活 | 当前fb01 actual81606 `host-crash-native-source-2/actual-node-1/result.json` validtrue；真实同boot host85130消失、provider85131同birth/group仍活，offer Unknown/None，显式同epoch Unknown/replay不新增Init，原Meta/Attempt/binding/raw/history保；独审通过。 | 证明不得重复spawn的危险窗口，不等于双方均死后的GUI恢复；不覆盖OS reboot。 |
| 远端resident离线/回来 | actual63373 `public-offline-runtime-source-4/actual-restore-1` 两实际配对home，Offline无localfallback，同home回来/同owner Join；独审通过。 | resident重启仍是同boot，不能代表GUI crash或机器重启。 |
| boot-change身份语义 | `s3-proc-identity-source-1/SCOPE.md`明确私有same-platform不同boot仅授旧foreground PreviousBoot，不查/绑定新boot重用PID；当前proc_identity.rs有纯注入boot a→b/namespace/PID复用case。 | 此索引未找到该pureexact实际运行收据；该源卡明确source-only。更没有真实机器重启/换boot端到端验收。input也确认无已知实际boot-change结果，不能用81606或63373补账。 |

结论：已证并可继承的是正常Quit的PTY重连基础、公有同boot ended Load/Resume、host死/provider活的拒重复执行、两home离线重连。真正未闭的是全App正常Quit/异常退出后GUI2重挂，以及无原进程情况下App恢复入口/历史控制组合；boot-change目前只有源码语义和注入case，实际换boot仍未验。原有NoLoad App草案可作为最小后继准备参考，但此复核不准任何新运行/扩框架或物理重启。G5f4a当前仅source-review通过未Root集成，C009仅准备；均不冒实际绿。


### 2026-10-31 physical mobile最短路线只读复核（当前d4b，不启动）

- Graph先查，相关ship/mobile symbol未覆盖后读既有源与原报告。`mobile/formal-ios-readonly-1/report.json`（31fafab5）与`mobile/native-device-readonly-1/report.json`所列当前Root受查源pin仍全部同。正式路径在`docs/SHIP.md`、release20261028/preflight-ios/COMMANDS.sh.txt；已有Tauri iOS archive/sign/export和team QR8DMW493W。昨日只读报告记录AppleDevelopment证书＋包含该手机的wildcard profile QR8DMW493W.*（2027-10-02到期），这是签名基础，不冒今日签名/安装成功或擅改生产App。
- 现有私有Lab：`evidence/runs/khor-code-quality-evidence/Q010-native-20261025/run-11/derived-data/Build/Products/debug-iphonesimulator/KhorQuality.app`确存在，id io.github.colehank.khor.quality；run11/build.py、xcode.py、ios隔离工程、NativeAcceptance.swift是可复用结构。它是c37151a1/iphonesimulator旧产物，不能直接装物理arm64手机，更不冒当前d4b。旧observer/devURL是127.0.0.1:23840原fixture；16ee editor observer仅MacdevURL23846模板。无已合格d4b physical Lab或observerApp。Root当前gen/apple/Externals与build不存在；actual89221仅ios-sim cargo check0，原strictcleanupfalse另保，未链接/签名/装机。
- 当前61968只在private data1/currentfinal-runtime-env.json是KHOR_WEB_PORT。`khor_web::listen`实际0.0.0.0，原`face::link`产生 http://<Mac可达IP>:61968/?k=<该私有store现key>；不是手机localhost，也不是设备tunnel。Root若已有受控listener真活且HTTP X-Khor-UI与d4b actual85579/dist（bedd563f receipt）一致，手机同LAN可先用Safari此地址做明确web范围的真实touch/IME/viewport观察，无需安装observer。key仅私有store/原分享入口使用，不写入任务卡/日志。此复核未连接端口，不能从env文件宣称当前listener活；原20610内嵌的是fb01前端，不能冒Gfix已在61968。要覆盖d4b必须等本次Gcurrent producer或实际同源webface，而非仅换URL。
- Safari连接61968不等于当前原生App：不能证明iOS phone_page/原生WK导航KVO、native deck/TermKeys/Tauri桥接/原生保存分享等；旧App连接新网页也不能消除旧Rust/ACL/API mismatch。需要原生范围时最短复用：冻结d4b＋实际85579 frontend/188gen/新278assets -> 现有qualitytarget的aarch64-apple-ios `khor-gui --lib` -> 既有Tauri ios build --ci --archive-only --export-method debugging＋独立Lab config/同team -> 验实际native库/link/App UI收据、签名/profile、独立bundle与数据container -> `xcrun devicectl device install app --device <fresh实际ID> <qualified signed Lab.app>`。默认customprotocol嵌入currentassets已足够，不为旧observer另做devURL产物；如以后确需devURL，native与servedJS同source/API/资产pin、真机可达MacIP及既有HTTP安全限制须独立核，旧127.0.0.1模板不能照搬。
- 关键签名边界：project.yml/pbxproj的PRODUCT_BUNDLE_IDENTIFIER目前是正式io.github.colehank.khor；不能只改tauri JSON就假定私有namespace。复用原隔离quality工程并核最终Info.plist/codesign实际独立id，不覆盖已安装用户Khor；此处不生成/复制工程，不执行任何build/install。仅示意命令shape，不是准入：`CARGO_TARGET_DIR=<既有qualitytarget> CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_NET_OFFLINE=true RUSTUP_AUTO_INSTALL=0 APPLE_DEVELOPMENT_TEAM=QR8DMW493W TMPDIR=<owned> npm run tauri -- ios build --ci --archive-only --export-method debugging --config <privateLab/currentassets-config>`；不得沿旧ship脚本main/正式bundle/install/upload，也不得静默下载/新target。
- 真正前置：Root最新只读6264设备仍iPhone15ProMax unavailable（原CoreDeviceID5EB9ACD5-041D-5CAA-980B-713EF9FD0A35），需要可达/解锁/信任/开发通道以及fresh真人可用性；仅61968无法修复CoreDevice tunnel/DDI。物理SDK26.5与iOS27差异不能宣唯一原因。LAN Safari是否可达与USB开发通道是两件事，手机不可操作/无人时都不能把准备销成验收。现不启动/安装/下载/boot/tunnel修复/真机App/用户资料访问，Gstage固定文件未动。


#### physical current d4b source准备与容量（2026-10-31）

准备在 `evidence/round2/mobile/physical-current-d4b-preparation-1/{PLAN.md,preparation.json}`，SHA bd0df74f…4833 /2d4c2a02…a13a，readyfalse。已装aarch64-apple-ios std与iphoneos26.5 SDK；同qualitytarget42.3684GiB，ios-sim6.9853GiB，physical架构子树不存在（冷编译）。复用run11无Rust shellphase/无远端Swiftpackage的quality工程结构，排旧.a/DerivedData；一次current physical Cargo library→一次generic iphoneos Xcode App link，独立id round2physical/teamQR8DMW493W。前端actual85579/188gen/278assets与未来physical native/签名App receipt明确分列，后两仍null；不直接运行旧run11 jobs2/1200/sim脚本。既有4096显式inventory上限、每phase300、ALL/源env/TMP门保持，无下载/新targetroot/签名或device动作。

资源快照外111290503168B/系统14144843776B；拟target增长8GiB＋外artifact2GiB/系统2GiB。外20/内10GiB保留量可过，但42.3684+8=50.3684GiB超50，故**当前不能准入physical cold build**；Root需先在已知owned缓存授权范围回收或证明更小真实峰值，不为追绿改估算/换target。缺连接/真人仍另列，不认为SDK差异是唯一因果。此准备未构建/签名/安装/boot/tunnel/改产品，actual89221原false与冷留存不动。


#### 已知sim缓存归属/最小退役源备（2026-10-31，未执行）

精确子树qualitytarget/aarch64-apple-ios-sim：ino35491551/dev16777244/UID501，40017 regular/9594dirs/8 relative内部symlink，无外部硬链/特殊节点，debug顶层均Cargo生成物；八swift-rs checkout同f64a4514且gitclean，globalSwiftPM同commit实际cat-file0，registry tauri2.11.5/swift-rs1.0.7源仍在。无unique用户会话/源资料；这些Swift源是可重建dependency，但未来offline若缺输入仍必须停。readonly lsof+D1empty、无compileowner；真实用户releaseKhor进程不纳清理对象。

已发现并告Root禁止原无-p `cargo clean --target sim --target-dir qualityroot`：Cargo rust1.95 CLI spec为空直接clean整个target/build，targets仅clean_specs分支生效；权威源链接在maintenance/storage-20261031/sim-cache-retirement-1/PLAN.md（8ad2b022）。Root未执行任何clean。

Root新授权的精确源码准备在同目录 retire.py7cbba00f /contractbc60994a，readyfalse/authnull。鲜exactidentity/set/metadata/所有权/Swift/globalcache/lsof后，single sameFS rename固定fresh quarantine，再核后仅fd-safe rmtree该树（不跟link、不复制/压缩/挪藏7GiB）。保护current56763三canonical+rawnormal/proofs/result、GUI OUT36393165、sim89221原result/source与82node0B冷TMP完整inventory前后；另新receipt只标payloadretired，原false/源码/日志不改。已交非作者源审，无任何mutation。现6.985GiB apparentdu退役后必须鲜du/df，不把APFS共享extent当保证释放量；原physical-flow源可复用，依真正清后50GiB/20+10保留与估峰再准入。


### 2026-10-31 current Mac Copy真红与分层诊断源准备

Root actual38171 normalQuit0/ALL-TMP/source-env稳，但H Copy业务失败：巨正文的真实底部Copy按钮、小14B H_MAC_ROW_1199结果右键Copy均generic `The text could not be copied`，回专用编辑文件真实CmdV/Save仍原38B sentinel（此前独立35B placeholder防no-op），实际UIobs f4147961…e059/原result c74d370e…2019，mac-copy-preparation-2实际pasteoracle独审。不是系统CmdC/CmdV坏；巨记录尚未证明走typedtoo_large。原false保，不加64KiB外Copy能力或注入。

Graph/debug-issue后读现源：HistorySearch.copy唯一出口是浏览器ClipboardItem Promise或writeText；inspect与clipboard同try，clipboard先拒可能遮inspect typedfailure。现无Mac通用nativeclipboardwriteAPI，只有iOS edit_menu原菜单copy_at。Root裁定先只诊断，不添加nativewrite/新API。作者产品仍clean5f4a。

最短源适配 `platform/mac-copy-inspect-preparation-1/probe.mjs`1a84ec80/contract6951354a readyfalse，HANDOFF80ae3b8c。沿原check-history-find的NodeWorker适配和未改production historyFind/historySource/worker，实际Core sessions取freshlocator→session_content取freshbinding→两query freshhit→inspect(copy)。外部仅真实history_begin/page/record/projection/cancel；小正文可回已知synthetic14B+SHA，巨正文只typederror/size/hash，不读/写系统clipboard、不注入UI、不启provider。Node --check0而非执行，尚未esbuild/Worker/HTTP。Root复用其actual56763 normalbridge/data1 privateKHOR/新loopbackport和原ownedrole guard启动/清理两进程，诊断60s within原300。该结果将只定位inspectvswrite，不冒真实UIcopy通过；若缺wrapper由Root指定既有骨架，不自造新平台。


## 2026-11-01 发布决定与历史记录边界

用户最新明确授权“先直接commit到main并ship”。本卡以上实施、失败及阶段权限记录原样保留；本次授权及发布状态以[本轮发布记录](../ROUND-2-RELEASE.md)为准。原 Mac 真人中文 IME 与真实 iPhone 操作仍未验，按用户决定移到发布后，不登记为通过；本卡记录提交不代表已合并 main、完成任一 SHIP 目标或已发布。
