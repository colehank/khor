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
