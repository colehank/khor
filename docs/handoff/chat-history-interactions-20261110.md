# 已交付 Lab · 长历史与消息交互 · 2026-11-10

本任务已完成并更新同一Lab；没有合并main或发布，发布继续HOLD。根智能体串行，本机source-only，无本地Node/Rust构建测试、无用户App／宿主重启。

- LIVE来源 `dc946c8366460cd803453ecfcacdde0c2f07c2a6`，tree `e9174e0e0650cbd881bdd536cfd6d5d6a9356e82`；[完整长历史Lab](http://127.0.0.1:23950/app/?scene=gui&machines=3&history=long&review=history-v1)，240轮，普通页128entry；追加`&short-pages=1`可看4entry压力分页。
- [Quality42772593033](https://github.com/colehank/khor/actions/runs/42772593033) 全部所选作业绿：frontend；GUI单位／类型／静态及notification-ui/key-row-ui/appearance/dialogs；60项真实App Lab（包含桌面／手机×普通／4entry长历史）与热更新通过。
- 修复真实underfill：预读去重误用boolean，第一页后停止；按实际opaque cursor区分。相邻小页按已完成分页请求确认连接并合并，保留8逻辑页／512entries／2MiB边界。每个wirepage的恢复游标保留，避免切换标签丢阅读位置。400entry／100wire恢复边界模型与既有严格3px锚点回归通过。
- GUI/Khor/终端共用右下角下箭头GlassCircle；消息不再常驻整条复制和带回输入。桌面hover/focus复制图标；手机长按复制／选择文本；复制拒绝可重试。选择页复用TaskOverlay，手机截图已看，系统原生选择手柄仍待真机。
- 用户明确回答`选择文本`和`修改用户消息并重新生成`。任意历史用户turn编辑＋重新生成暂不提供：当前宿主未暴露可靠anchoredrewind／native messageID映射；依据用户原话“不行就算了”省略，已明说；没有伪装追加prompt为编辑，也没有失效修改入口。
- 图片沿用正式AgentResource，长历史含思考／工具／表格／代码／链接／图片。代码块独立复制保留，不是整条消息常驻动作。
- 官方artifact13742594387，提取`chat-history-dc9-review`；完整App285文件＋全部JSON＋本任务截图共311项逐项官方ZIP CRC/size校验。其他264截图保留在官方artifact，不冒称已下载。范围helper`fetch_chat_review.py`已对4d完整包做285文件集合/SHA256自检，当前只需1.67MB网络传输。证据`chat-history-dc9-verification.json`和`chat-history-live-verification.json`。
- 仅停止已核对argv／listener的旧LabPID33675，当前ownedPID6019／exec99444；21项HTTP校验通过（新页／模块／runtime.tree，以及13bd、0286、98a、294、3a、394、ce0、269、pinned旧快照与JS）。没有强制刷新用户标签。

原生iPhone键盘／惯性／系统选择手柄、原生浏览器/PDF上的通知层仍未验证；全仓／全平台发行检查未在本次重新跑，不声称可立即ship。用户后续验收或允许发布时，再按整轮剩余项与发行检查推进。

以下保留实施过程与失败证据，当前状态以本节为准。

---

# 长历史与消息交互（2026-11-10）

用户在通知精简处理中追加，原通知改动保留；当前串行进行，不派智能体，不发布。本机source-only；构建／执行仍走公开托管CI。

## 本次要求

1. Lab加入很长的结构化对话历史，验证此前GUI会话初始加载过短、自动向上补页与阅读锚点问题。
2. 回到底部统一成右下角仅下箭头的玻璃按钮。
3. 移除消息旁常驻“复制／带回输入”。手机长按菜单；桌面悬停出现复制与修改图标。
4. 检查任意历史turn编辑能力，不支持时可以不实现，但不能把追加新消息冒充编辑。
5. 正在等异步澄清：“选择文件”是否是选择文本；编辑是否只指用户消息并从那里重新生成。

## 计划

先定位共享历史渲染／滚动及消息动作，复用正式组件制作长历史数据；检查对应agent/ACP能力；完成前端明确项，覆盖真实App Lab宽窄滚动／操作回归。保留通知新改动，最后更新同一Lab供用户验收。必要能力缺口如实列出。

## 通知并行状态（同一根线程串行，不是另一智能体）

当前HEAD13bd01ecf65d02b1deced0ea6370280ffa32a8ac，tree d89c8ad77a762af378011096b7c1d583f2826bef，分支integrate/settings-gui-20261109。Quality42760880544前端/GUI已绿，完整App仍在跑。通知已去掉状态词，桌面hover/focus才显X，手机无X；这轮已有代码、检查，保留。

LIVE23950仍是0286，ownedPID57328/exec92502。新GUI证据notification-clean-13b-gui已下载，6组results待核对。高效可信下载helper /Volumes/disk0/Khor/evidence/more-network-gui-20261109/fetch_review_cached.py --run RUN [--name ARTIFACT] --previous PREVIOUS_ROOT --destination NEW_ROOT；从官方ZIP目录校验每项CRC/size，复用已验证文件，只下载变化项，再核对manifest/source/tree。旧包基线notification-demo-028-transfer/extracted-fast。发布HOLD，实际iPhone/原生浏览器叠层等验证仍未完成。

## 已写入（尚未验证）

- 新components/ScrollToBottom.tsx：GlassCircle+已有IconArrow down，右下角，保留既有data选择器／fresh辅助语义；ChatView、MachineChat、TerminalPane三处复用。Terminal的fold-lift CSS未改，保留8px原间距。
- dev/lab/long-history.ts：240轮，每轮20流式片段；每12轮包含思考/工具/表格/代码/链接，每40轮图片。fixture options.longHistory在第0GUI会话使用整个长回放；bootstrap query history=long并进入identity。
- 即将补实际App long-history/short-pages检查。现有useHistoryPrefetch visit最多8wirepages；HistoryWindow也最多8wirepages/512entries/2MiB。在4-entry小页+流式碎片下，很可能只有1轮可见就触顶，需要先测红，不能靠抬高整体内存限制解决。可能方案：合并相邻可连接小wire页到128entry/512KiB逻辑页，再保留8逻辑页与512entry总界限；需先证据再实现。
- 待做消息操作组件。AgentContent内部、ChatView用户泡泡和MachineChat普通消息需要共用；旧CopyText在AgentContent和ChatView，BringBackToInput在ChatView1767；Markdown代码块自身CopyText是独立代码复制，用户说不要常驻复制应只针对整条消息，保留代码块语义除非后续指示。

## 通知交付更新

13bd所有验证已绿且LIVE23950，serverPID33675/exec66857；通知无状态词、桌面hover/focus X、手机无X截图已看。来源notification-clean-13b-review/developer-lab，tree d89c8ad77a762af378011096b7c1d583f2826bef。旧0286与98a等快照保留。无需重新为13bd跑检查；后续新chat代码再跑。

## 新状态

- dae42cfe已push，Quality42764941090是长历史／箭头基线，等待实际红例；未发布或合并。
- 消息actions开始写：正式共享MessageActions使用已有RowMenu长按／右键，桌面hover/focus显复制icon，手机无常驻按钮；复制失败保留原位置重试。选择文本使用共享TaskOverlay＋原生可选文本。`选择文件`暂按常见的选择文本处理，已向用户说明，仍可纠正。
- 任意turn修改暂不能可靠实现：当前crates/acp control仅标准会话生命周期；apps/agent ACP没有锚点回退／truncate，GUI的HistoryEntry也没有vendorUUID映射。官方ACP schema没有anchored rewind；不显示虚假修改入口，不把追加新prompt冒充编辑。用户允许“不行就算了”。
- 原CopyText仍留Markdown代码块，不是整条消息常驻动作；ChatView用户/AgentContent/网络普通消息复用新动作。

## 实际验收清单

- `scene=gui&machines=3&history=long`：240轮结构化历史，停在最新，首屏自动补足；向上自动补页时不跳回最新；回到底部只见右下角下箭头玻璃，按下回到第240轮，抵达后消失。
- 追加 `short-pages=1`：4-entry服务分页，仍能自动填充，不靠点“加载更早内容”；后台缓存保持512entries／2MiB／8逻辑页，不因页变小而丢掉最新尾部。
- 桌面消息：闲置不显示常驻复制；hover和键盘focus显示icon；拒绝clipboard后原位可重试，成功才报已复制。
- 手机消息：正常拖动滚动不弹菜单，长按复制／选择文本；选择页沿用TaskOverlay，内容可由系统选择。浏览器触摸模拟不等于物理iPhone系统选择手柄验证。
- 代码块自身复制保持原有行为；图片仍由正式AgentResource显示，绝不把base64当正文。
- 普通Khor网络消息同样使用共享操作，终端／Khor／GUI的回到底部复用同一GlassCircle组件。
- 任意turn修改不做前端伪造；当前缺可靠host能力，需后续按agent的真实会话分支／回退接口接入再开放。

## 已看到红例

Quality42764941090（dae42cfe）：前端构建和GUI绿，完整App56既有案例绿。长历史两种小页场景都Timeout15000ms，尚不能在DOM重建第240轮；普通页两例已通过fill/read步骤，到玻璃data属性断言`true!==''`才失败（检查已修为hasAttribute）。红例原始日志chat-history-red-dae-app.log已保存，官方artifact正在校验下载。

修复：receiveHistoryPage只合并边界游标严格相等、各自有bodyentries的相邻页，合并结果仍<=128entries/256refs/512KiB；reference-only原替换行为保留。总8逻辑页/512entries/2MiB不变，容量满仍按原方向逐页淘汰并保留reload游标。预读每visit最多512wire读（与最低1-entry页填满现有entry预算一致），重复游标／失败仍停止，实际向上输入重置visit。

## 官方红例进一步定位（不能仅归因8页）

Artifact已校验564项CRC/size。小页diagnosis只有2次backward（tail+1次older），724/792px视口仅1个残缺agent条目，之后22次无新增的newer轮询。直接原因是useHistoryPrefetch中`const older=...&&canPrefetchOlder()`最后是boolean，seen键一直`older:true`，第一次补页后误当重复游标停止。新增historyPrefetchTarget保留真实opaque cursor，hook用direction+cursor去重；新增模型反例。前一轮pack和IO预算仍解决后续小页容量问题，不能说红例已走到8页。正常页红例读取锚点/自动补页已经工作；玻璃data属性测试错误已修。

## 原生游标边界核查

进一步核对apps/gui/core/src/chat.rs和crates/node/src/adaptor/transcript_page.rs：原生before/after游标可用不同record position表示相邻边界，不能只靠字符串相等合并。receiveHistoryPage新增可选requestedCursor，由单pending的historyReader传入当次真实游标；只在匹配当前window边界时允许新入页和既有边页合并，旧页间仍需要一致边界。新增不同opaque编码与stale请求模型反例。不解码前端游标。该补充当前dirty，等5daCI返回一并处理实际结果后再提交最终版。

## 已复现阅读位置回归并修复

5da/Quality42766916003：frontend、GUI units/types/static/4交互组绿；App在既有checkGuiExperience的切换标签后恢复锚点严格<3px检查失败，尚未运行新长历史案例。日志chat-history-5da-app.log，官方artifact transfer42853进行中。原因：原rememberReading保存匹配页的newer；合并页后这个newer可能距离当前条目很多个wirepages，小页恢复读回最后4entries，缺少原条目。缓存合并现在保留每个原始wirepage的checkpoint（仅游标与已有entrykey引用，数量<=512entry上限）；historyResumeCursor恢复精确原wirepage，ChatView复用。模型验证400entries全部100个原wire恢复边界及原生不同opaque编码。既有严格3px检查保留；harness失败会捕获诊断并继续其余App案例，不把单一失败掩盖其余结果。e9d4/Quality42767913845只含原生边界适配和本地化，目前运行；新checkpoint修复尚dirty。


远端e9d4/Quality42767913845前端类型检查报TS2367：tail分支已返回后，冗余`direction!=='tail'`比较被TS判为不可能。已删除多余比较；前端不是通过。e130/42768731685同源也会受影响，需取消旧run，重新验证本修复。不得以Python语法解析冒充TS类型检查。

## 用户已明确澄清

用户异步回答原话：`选择文本`；`修改用户消息并重新生成`。不再是待确认／推测。选择文本实现符合回答。编辑需求确认为任意历史用户消息编辑并从该处重新生成；当前宿主缺可靠anchoredrewind，沿用用户原话“不行就算了”本轮暂不开放，已明确告知用户。不能把appendprompt称为编辑。

## a24验证结果

Quality42768918548：frontend+GUI全绿；原有56App案例全绿（严格<3px会话切换锚点已恢复）；桌面long-history普通页和4entry小页均绿，包含填充/自动older/回到底部/复制失败重试/选择文本。两项手机longhistory在进入会话前等待scroller失败：fixture会话标题改为长历史，而bootstrap仍按旧标题找要点击的行。改为真实会话ID与行data-row匹配，继续走正式row.click，避免任何标题改名造成Lab无法进入；不改生产组件。a24官方artifact transfer5067进行中，到chat-history-a24-review。新手机菜单尚未验证，不能声称全部完成。下一run只含Lab入口修复。

## 4d3手机检查结果

Quality42770674720：frontend/GUI/既有56App/桌面2longhistory全绿；手机两例已通过fill/older/returnglass/copy失败重试，失败在第二次长按后点选择文本。日志明确先解析到select-text项，再等待稳定时退场脱离DOM。测试openMenu第二次立刻wait copy，匹配上一菜单退出动画仍保留的旧DOM，提前结束长按，导致它退出后没有新菜单。检查改为开始每次长按前等待旧菜单detached，等待open-state新菜单；仍使用CDP真实touchStart/hold/touchEnd，不强制点击。选择截图禁用有限动画以看清最终布局。生产消息组件未变。官方artifact transfer99157进行中 chat-history-4d3-review。新测试修复需完整App回归通过后才能更新Lab。

## 可信产物拉取提速（本机仅Python HTTP／校验，无编译或App执行）

新external helper `/Volumes/disk0/Khor/evidence/more-network-gui-20261109/fetch_chat_review.py` 复用已验证Range/ZIP/CRC逻辑，只选择完整developer-lab（所有文件）+全部lab-report JSON+long-history/gui-experience PNG。其他264截图保留在官方immutable artifact，不下载。输出明确`selected_entry_crc_size_verified=true`、`all_entry_crc_size_verified=false`（若未全取），不会冒称全归档已验。对完整4d3做自检，app全部文件集合和SHA256逐项完全相等；选中308项只用138228网络bytes，probe删除保留报告。未来最终handover用此helper即可；完整artifact仍可用旧fetch_review_cached.py，4d3完整572项CRC/size已校验。
