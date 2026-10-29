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
