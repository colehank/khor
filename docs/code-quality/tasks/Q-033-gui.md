# Q033 GUI：停止上传后的草稿、附件与重新连接

状态：implemented，等待root非作者复核/串行集成；所有者 store_owner；后端/token/API由agent_redaction独占，App接线/catalog由root独占。
工作副本：`/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-store-owner-recovery`，分支 `fix/quality-upload-cancel-gui`。从精确集成 `d5ff206ca9009c04d4246f3ccccd107a2ec74cd2` 起步；此前从b732创建的同名新branch没有实现内容，核对clean后按root指示重新指向该基线。root文案127e2798已cherry为f8c8a034（仅两TOML各4词），使用既有catalog build-script同源生成ignored TS，无Cargo。

## 授权与设计

允许ChatView.tsx、chat-files.tsx恢复方法、新轻量lib/chat-drafts.ts及专用真实组件/registry测试与本卡。不改api.ts/后端/全局package/CI/现有hook。无Cargo槽。root接App retain入口和catalog。

已读 khor-design SKILL、设计README、20-states、Button说明及30-behavior相关输入/窄屏验收约束。取消稿沿现有输入区用文字、Button和spacing token；不加玻璃/弹窗。窄屏截图检查输入/最新消息可见，布局由人眼截图验收，不把临时坐标写成永久门。

稳定拟契约：chatSay尾参token?:string，非空<=128安全ASCII，浏览器crypto.randomUUID。ChatFrame send_cancelled {token:string|null,reconnect:boolean}，token只在本机关联收据不进host wire；null不恢复猜测草稿。frame只一次，legacy旧attachment退役前进入frames，不再重复Unsent；已commit的turn仍走原Turn(Cancelled)。等待agent_redaction固定提交后cherry，不复制漂移API或手造types。

空composer自动恢复原始draft/docs/附件（snapshot浅拷贝元数据、base64字符串共享，thumb=null，不复用clear已revoke URL）。有新输入时保留独立取消稿条目，显式“加入输入框”追加正文、合并附件/docs，不替换当前输入；“丢弃草稿”显式移除。多个取消按session/token独立，不指向可被新send覆盖的lastSay。optimistic user行需携本机token，移除精确原行，避免同文重发删错。Stop真实失败复用controlError，不能吞；legacy reconnect=true走明确openWhy文案+现有retry/setAttempt。

4词条由root统一提供：chat-upload-stopped-reconnect / chat-cancelled-draft / chat-add-draft / chat-discard-draft；中文分别为“上传已停止，重新连接后可继续”“已停止上传的草稿”“加入输入框”“丢弃草稿”。

## 生命周期归属

DetailPane实际key=row.id，切session会卸载ChatView，因此组件本地state不能独自持有取消稿。root批准小registry与App清理：`retainCancelledChatDrafts(ids: readonly string[]): void`（lib/chat-drafts.ts），在App现retainSessionDrafts(openTabs)旁调用。retain保存最新允许id集合，不只清当下Map；明确关tab后拒收迟到snapshot/receipt，普通切换保留。订阅返回稳定数组供useSyncExternalStore。pending snapshot留到完整提交或稳定取消/Unsent收据，避免新send覆盖lastSay使旧取消丢稿。若旧view迟到收到cancel frame，仅可归档仍open的原session，不paint/调用旧owner API；closed集合拒收。模块store不序列化大base64，不deep clone字符串。

## 当前现场与下一步

起步时GUI生产未改，先建立实际ChatView/React+production api.ts/Tauri seam fixture：真实send/stop/图片选择/docs handOff；已有Stop错误被catch忽略可独立真红。类型契约尚未到，不把fixture当真实后端已可用。随后补唯一token、空稿恢复、原始空白、图片/docs/path、thumb回收、保留新稿+加入/丢弃、多token同文、keyed切session/明确close、旧owner/nulltoken、legacy重连与Stop失败。两引擎、source固定、types、窄屏截图；纯registry实测retain拒绝迟到/允许重开/多个id分离。


## 实现检查点（2026-10-25）

依赖已固定：后端21994830→本树373f3467；本机SendCommitted followup3d0dc41f→0b0d2e3f。确切生成ChatFrame.ts从后端owner的固定产物取得，SHA9e81590d847ae30f95fc45776d0141b78dc02044c6a078b7055f3e2ddc38c46a；没有手造类型/复制API函数，也没有Cargo。只有{kind:send_committed,token}确认写线完成；HTTP chatSay null只是admission，所有transport均不按Promise resolve清pending。SendCancelled和Unsent均为nullable token，old/null只兼容说明、不强配新稿。

初始registry独立提交d4c91e9c已由root审查/集成97e1a200。root App retain与unknown词条从7994ab26仅取App/TOML三文件patch为533e6571（没有package/docs）。root后来只需cherry本任务自己的GUI/registry后续提交，不搬这几个已集成依赖。

当前GUI候选：每次send/重试唯一token；浅snapshot保留原始空白、inline字符串/文件路径/docs，恢复attachment重新分key且thumb=null；按token移除原optimistic行，旧同文receipt不清新turn。原稿空框自动恢复，有新稿则Saved条目“加入输入框”显式追加/“丢弃草稿”；条目在消息scroll区域，不增长浮动composer。Stop真实错误显示controlError；legacy明确openWhy+原retry入口。setAttempt/切session不复制错误草稿。当前代码尚未最终提交。

有效bug红：Stop抛synthetic Stop denied时8s无alert，red-stop-valid pageerror0；typed API已到但旧GUI不传token的两feature红在red-contract。另开发中发现并实际红：reject先到→新同文token已发送→旧cancel不应把原稿自动填入新turn输入（red-old-receipt）；旧卸载view的gone不应把同id新reader正在发的token标unknown（red-old-owner-gone）；晚到取消从前view归档后，当前空same-session composer需自动恢复（red-late-switch）。对应修复后单独场景已绿/正在收齐。

registry未知态：RPC reject保留pending，1s无稳定收据时标unconfirmed，仅可丢弃，不自动恢复/重发；commit/cancel/Unsent/close清timer。明确lost/gone/fresh restart对缺收据数据标unknown，不猜取消。新增内部reader scope（不进wire）：旧stopped collector仅处理自己reader，当前collector可消费合法重放，fresh仅标更早reader；同id新attempt的RPC/Stop catch也校验reader。open-tab owner cap仍负责close/reopen拒收。模块12/12实际绿，registry-final-reader.log。

浏览器首轮两引擎16×2功能通过，不把2份布局记录额外算测试；类型检查识别失败标题是模板函数，已修实际调用。初次完整App窄屏切版时旧DOM脱离，fixture已按生产cursor保留重放并等新keyed view稳定，非生产修复；narrow-fixed真实4/4且已查看WebKit实际App390×844截图：最新消息、取消稿条目与底部输入均可见，无横向溢出/遮罩。最终会记录两引擎固定source的完整矩阵、types与截图。


## 固定交付（2026-10-25）

自己的最终实现提交：`3f2b8b1858db8c2f11df2c53b1d1810d796d4dcc`（7文件；当前worktree clean，冻结供非作者复核）。root只需cherry此提交：初版registry d4c91e9c已集成97e1a200，App/catalog patch、两份backend依赖由root已有/自行集成，不再重复搬。

除前述真红，还补到两个实际边界：legacy reconnect收据先于batch.gone时旧界面没有重连入口（red-retirement-first），现依据明确reconnect收据结束poll；fresh restart虽将旧send标unknown，却遗留旧inTurn挡新发送（red-restart-unknown），现仅清更早reader的busy/progress，当前reader若已开始新send不清。原稿保持结果未知、禁止自动恢复/重发。所有新增行为都有真实组件动作，未只按源码断言。

固定源码结果：
- `CHAT_CANCEL_WEBKIT=1 PLAYWRIGHT_BROWSERS_PATH=/Volumes/disk0/Khor/caches/dependencies/khor-playwright CHAT_CANCEL_CHECK_DIR=<evidence>/final-fixed node scripts/check-chat-cancel.mjs`：Chrome21/21、WebKit21/21，共42/42；真实ChatView/production api.ts与完整App切换/关闭，synthetic IPC/HTTP seam。report.finished=true，前后source SHA一致，所有pageerror为空。覆盖Stop真实错误、空稿原始空白及图片/docs/路径恢复、保留新稿+显式加入/丢弃、多取消、同文不同token、retry新token、HTTP admission先成功后取消、无收据1s未知、未知后稳定收据、lost commit、fresh restart、旧reader gone与晚到取消、完整App关tab/reopen拒收、窄屏。
- `node scripts/check-chat-drafts.mjs`：12/12，实际registry owner/reader/timer边界；base64字符串共享，thumb不复用已revoke URL。
- `CHAT_POLL_WEBKIT=1 PLAYWRIGHT_BROWSERS_PATH=... CHAT_POLL_CHECK_DIR=<evidence>/poll-regression node scripts/check-chat-poll.mjs`：原Q034 Chrome11/11、WebKit11/11，共22/22。
- `node_modules/.bin/tsc --noEmit`：exit0；`git diff --check`与staged check：exit0。
- selector负面3/3：空、重复、未知场景均在启动server/browser之前退出非0，避免0测试绿。

实际source SHA（完整依赖/脚本清单在final-fixed/report.json）：ChatView `4ed35058cbe6f89e75ce715c5745854a3335a3609589c672631ca1744a538665`；chat-drafts `6d9167492a1eaa5acadc862978600856bfdb5c1d2c1a6f4a41f7551aabbadc2b`；chat-files `e18d0ca83e5e83909d53f0586bb9f04cc40bd1274658a7c159190a56f0472380`；生成ChatFrame `9e81590d847ae30f95fc45776d0141b78dc02044c6a078b7055f3e2ddc38c46a`。

证据根目录：`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q033-gui`。最终报告final-fixed/report.json、final-registry.log、final-types.log、poll-regression/report.json、final-negative-selectors.log。固定源码截图final-fixed/{chrome,webkit}-app-narrow-cancelled-draft.png及bare窄屏图；已目检WebKit实际App390×844图：最新消息、行内取消稿条目、底部输入均可见，无横向溢出/额外遮罩。布局不固化临时坐标永久门。

限制：测试后端为隔离transport seam，不代表真实设备/物理旧host现场Stop验收；backend真实core/native及生成测试由其owner另交付。本任务无Cargo、无真实用户send/Stop/文件操作、无main合并/ship。registry仅内存，关闭tab明确释放，大base64不序列化；结果未知只允许丢弃，避免不确定发送后的重复投递。


## 非作者阻断收口：失效重试动作（2026-10-25）

root复审3f2b8b18发现：body丢弃只清registry，失败PillNotice仍持snapshot；retry动作未检查原owner/存活snapshot/mounted。保留3f原提交，独立后续提交 `8da0d063`（ChatView与真实fixture两文件），等待root复审后按3f→8da顺序集成。

四条真实组件红在`Q033-gui/red-notice-retry/report.json`：Unsent(token)后body丢弃、加入输入、keyed切走、实际App关闭/重开同id，再调用从实际noticesOf捕获的原retry.run，均令chatSay数量1→2；所有pageerror为空，非fixture语法错误。修复把body丢弃/加入统一为选择性撤回同token notice；retry执行时校验mounted/current session、原owner身份与saved内仍为failed的同一snapshot，不能用新owner重新授权旧数据；加入输入入口也校验mounted及仍可恢复状态。不以隐藏按钮代替回调权限检查。

最终固定源码完整检查 `notice-final-fixed/report.json`：Chrome26/26、WebKit26/26，合计52/52，source前后固定，全部pageerror为空。额外第五用例证明丢弃旧token不清新token的notice，后者仍能正常重试。原retry-token正常新token、附件与docs保留用例继续通过。`notice-final-types.log` tsc --noEmit exit0，git diff --check exit0。poll调度/registry未改，不将旧22/12宣称本次重跑；本次全矩阵覆盖旧21场景与新增5场景。

最终SHA：ChatView `f9742cf69795050217a460096b990f3c24546f796aa9146d0b9b0948bbc8274c`；fixture cases `cdfa83cb6073def3fe11061fb46a4272cb71b530ae416421875be3dec0e4df0e`。此前42/42对应3f版本，当前52/52对应8da候选；证据其余边界/限制不变。工作树再次冻结，未改App/catalog/api/全局门，未跑Cargo或真实服务。


## 第二轮非作者阻断：旧cancel误撤新notice（2026-10-25）

agent_redaction独立审查在固定8da候选取得Chrome/WebKit各1真红，证据`Q033-gui/review-stale-cancel`：A失败捕获实际notice.cancel→B失败→body丢弃A（B notice保留）→调用旧A cancel，无条件setUnsent(null)撤掉B，但B saved仍在。作者未覆盖既有提交，先把相同真实动作加入正式fixture，`red-stale-cancel-author`在未修生产上独立红，pageerror为空。

新增后续提交 `7746378f`（只ChatView与cases两文件）。cancel回调按原PillNotice对象身份选择性撤回，且检查当前mounted/session；有效原稿恢复仍经过已有snapshot/owner保护。legacy/null notice同样使用实际发布的同一对象，仍仅允许dismiss、不猜草稿归属。正式fixture新增stale-cancel-notice证明旧A不会撤B且B实际retry仍可用；cancel-notice证明当前有效cancel正常恢复精确原始空白、两个文件和docs且不重发。

最终固定源码报告`stale-cancel-final-fixed/report.json`：Chrome28/28、WebKit28/28，共56/56，finished=true，source前后固定、pageerror全空。`stale-cancel-final-types.log` tsc --noEmit exit0；git diff --check exit0。ChatView SHA `a9e621364270310fc38c115be45df873b839ff15804a782061e373c7419b5395`；cases SHA `c66665b548a002200f8ebdf268615aa8bc0c9012ead6ad410fc5f7273b6e26cb`。当前实际交付顺序3f2b8b18→8da0d063→7746378f，各提交保持独立；树clean冻结待非作者复核，未改全局/后端，无Cargo或真实服务操作。


## root最终集成检查点

作者3f2b8b18/8da0d063/7746378f按序集成e56eef64/87674beb/bd58566a；root门85a78fe9。精确该运行源码gen成功、完整双引擎56/56、types/unit/gates101脚本94条通过，旧poll双引擎22/22也过；report14项指纹逐项与集成源一致，见Q033/integrated-gui/root-checks.json、report.json和integrated-poll。独立审查捕捉4类过期retry及旧cancel撤新notice的真红→绿均保留，不只认作者全矩阵。真实native GUI/Rust取消全链仍待，不能把组件HTTP/IPC mock说成App验收。后续Node/CI集成未更改这些GUI源，当前HEAD c37151a1。
