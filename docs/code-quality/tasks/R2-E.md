# E 输入与草稿完整性

状态 working；2026-10-29用户“开始吧”授权下一轮实现。main/ship未授权。
owner: round2_input；工作树 /Volumes/disk0/Khor/worktrees/current/khor-round2-input，fix/round2-input，基线e7c335c8。
条目 R2-011/012/027/028/L03；先完成R2-027迟到读取/组词最小复现与保护，不声称整包完成。
允许写：CodeEditor/FileView及其纯模型/定向测试；其他输入hook/路径/草稿入口需先向root报告新路径所有权。禁止改历史链、全局App/TerminalPane/样式/生成协议。
规则见EDITOR-INPUT、ROUND-2-EXECUTION、ENGINEERING。先图谱、失败证据、最小修复、职责整理；不要用定时延迟掩盖竞态。记录真实IME尚未验证。
证据 /Volumes/disk0/Khor/evidence/round2/input；更新本卡记录实际文件、接口、验证、下一步（仅本卡可以由owner写中央文档）。native/Cargo槽须root协调。保持用户App/会话。

## 首片段：后台文件读取拒收旧回包

作者树提交 `447419cf81a811a9bc7da446acb96b7eacdffd56`；状态review-ready，整包E未完成。

确认旧again闭包可在读取中发生输入、撤销或组词后覆盖新输入；旧源同8例1过7失败，新源8/8通过。检查执行真实again/onText/setEdit回调，输运回包顺序受控，不挂载React或启动浏览器。生命周期失效模拟、composition DOM事件与真实IME不冒通过。修后tsc、diff检查通过，证据refresh-receipt.json/refresh-red.log/refresh-green.log/types.log位于既定input目录。

实现：file-refresh纯协调器持有编辑代次、请求序列与composition活动状态；所有setEdit在调度React状态前同步失效旧读，machine/path layout清理失效旧请求；后台回包只有无中间编辑/组词且仍最新才能同时更新merge基线、缓冲与缓存。CodeEditor只增加组词状态通知，不更改任意外部text替换策略。

Clean Code：删除已不完整的长refresh历史说明，保留merge基线不变量；读取接纳规则由纯协调器唯一持有，组件保留输运/呈现副作用。

独审round2_reviewer未发现首片段阻断；按意见明确真实挂载/组词结束最终事务时序尚待验证。接下来R2-011/012路径/搜索退出先报告新增写路径并独立复现；R2-028、通用外部text组词策略和L03退出草稿未完成。无native/Cargo/App/真实用户文件操作。

## 第二片段：拖选归属与搜索退出

用户授权范围内，经root批准新增use-field-dismiss.ts、MobileSessionContent.tsx、check-field-dismiss.mjs唯一写权；提交 `577f888d80d0a5813f9d9417faefb9f740120150`，review-ready。原有PageField blur链和真实拖选验收仍未完成。

hook跟踪主pointer起点区域，内起拖外放产生的click不退出；下一pointerdown、pointercancel与键盘click不会复用旧归属；右键不强行关闭。外部首次点击执行原动作后才退出。ContentPicker搜索增加明确onDismiss，空白/Escape退出并清query，clear保留编辑；open/directory/source/close动作先清搜索，防组件卸载取消延后dismiss留下隐藏过滤。

实际hook和ContentSearch、picker JSX/stopSearch/open回调级13例绿，旧源码同测红；field-interaction、刷新8例、tsc与diff通过。非作者审查无阻断。测试不挂载React、不操作真实浏览器；保留初版harness的Node capture移除与TypeScript转译错误日志，区分其与业务red。证据field-dismiss-receipt.json及对应日志。

下一步：独立核PageField onBlur及ContentPathField/PathField的选择焦点链，先报告新路径；root集成后按受影响范围验证。整包E未完成。

## R11真实组件选择Lab，待用户手工反馈

root批准PageField、必要时PathField和隔离Lab写权。当前只新增4个field-selection Lab文件，未改这两个生产组件；PageField立即blur/rest路径与PathField自身延后blur机制需实测，不能因源码风险直接扩大修改。

入口http://127.0.0.1:23834/lab/field-selection.html，作者树运行Vite，exec26585，独立缓存round2-field-selection；服务归本任务保留供root CUA。真实PageField/ContentPathField＋固定API数据，页面记录选区/焦点/离开次数，打开只显示本地文本。Lab TypeScript通过，源码与server收据field-selection-server.json；真实拖选/Tab/候选/外点尚待用户操作，未冒通过。root当前浏览器控制工具不可用，已请求用户两框拖选反馈。

当前Lab checkpoint为 `e294e53529fb43ab03c8a711028e083a89f0ee08`，4个Lab文件已提交；没有PageField/PathField产品补丁。compact后应先确认root已做的UI操作与服务再继续，不启动重复服务或外部Playwright。

## L03只读入口盘点（待新写权）

FilePage的5个生产调用方：WorkspacePane已有reportDrafts；SessionContent/ MachineFiles/ SideWorkspace有既有draft集合但未报告；StorePage无onDraft，由FileBody.own持有。不能只补会话入口便宣称全部覆盖。

建议共享useDraftCount仅派生现有owner的计数、layout更新/清理、实例唯一reporter；FileBody仅无onDraft上游时报告自身dirty，避免同一缓冲双计。此方案不新增草稿状态、不改App全局退出判断。已向root提出MachineFiles/SideWorkspace/new hook写权请求；尚未写实现或测试。

## L03草稿退出发现能力接线checkpoint

root已批准SessionContent/MachineFiles/SideWorkspace、lib/drafts.ts、use-draft-count.ts及FileBody无上游owner分支。提交 `5c75f50dfc2fe8046c663b320c811a361eaa3f70`，独审无阻断、review-ready，L03不标完整done。

所有生产FilePage调用方已盘点：WorkspacePane原有report不动；SessionContent/MachineFiles/SideWorkspace分别报告现有草稿集合；StorePage无上游onDraft，由FileBody自身dirty报告。共享hook稳定Symbol ref避免跨React root/字符串reporter碰撞，layout更新count无中间0，真正owner卸载才清自己的报告。隐藏但保留集合owner仍报告，不复制正文或另存草稿。未改App全局Quit规则。

实际hook与生产owner计数表达式9项旧红新绿，包含saved/discard、隐藏保留、多实例、既有Workspace隔离、无双计、无中间0与StrictMode replay；最终tsc/diff通过。证据draft-count-receipt.json与red/green/types日志。测试不挂载React、不冒原生退出；最后一键立即Quit、上游onDraft被动回报间隙和拒写/退出流程仍待真实App验收。

当前作者树全部源码已提交，Lab23834唯一实例保持待用户手工拖选反馈。下一步root集成本片段后验组合；作者等待真实R11反馈或再约定R27外部text/组词结束接纳策略。R28尚未确切入口复现。

## 外部text接纳：merge owner迟到覆盖checkpoint

提交 `cb67db78c2fc400bea83725b91b9448d152c0aca`，review-ready，非作者审查无阻断。真实merge闭包在磁盘读取/合并计算期间输入、输入后撤销、组词起止无docChanged、身份失效或后续冲突action后仍会覆盖新text/merge base。新版本复用fileRefresh的编辑revision checkpoint及已有overwriteEpoch action票据；两个异步边界核对，file/edit/clashes整体接纳，旧错误不替换当前反馈，旧finally不清新action忙状态。不重定义Draft.by，不重跑已失效合并。

9项实际merge/onText/setEdit回调回归旧红新绿；刷新8项、草稿计数9项和最终tsc/diff通过。证据file-merge-receipt.json与red/green/types/组合日志，纯控制回包未运行浏览器或真实IME。

外部text来源契约梳理：初载/theirs是显式加载并卸载旧editor；again按编辑revision接纳；onLanded保留当前text只更新base/digest；mine读取leaving最新text并受action/保存血统保护；merge现已按编辑revision/action接纳。CodeEditor仅最后out字符串判断echo仍是API边界；人工n→ni→host n只能证明任意旧props危险，没有真实React旧echo乱序证据，未加textRevision/onText元数据。

未完：Mac真实IME、R11真实两框拖选、R28确切宿主、上游onDraft被动回报间隙/真实退出，以及组词期间自动保存策略与最终候选事务交接。父任务允许先承接独立C包调查，E未被标done；Lab23834/PID3175保持待用户反馈。

## R11浏览器真实DOM验收进展（root执行）

root告知Playwright工具fallback已完成Chromium DOM 1000/390×URL/path×左右外放8场景全绿，选区/焦点保留、键入替换精确。证据input/browser-fields/drag-corrected-report.json；原Home未清Mac全选触发native drag-existing-selection的假失败及textbox/combobox locator错误保留在single-trace/label-controls，修正测试前提后通过。

因此不凭立即onBlur候选新增PageField/PathField补丁。R11已得浏览器实际拖选覆盖，原生WK及用户实际场景仍待；原待用户手工反馈为此前工具不可用时状态，当前由此检查点更新。Lab服务继续由root协调保留/结束。


## W交叉验证对L03的新增实际证据（2026-10-29，root协调记录）

W真实CM交易后同批切workspace已复现末17字丢，FileView同步onDraft红绿后恢复；同dispatch后model.draftCount=1而Quit所读draftCount()=0也已实际复现，W新增WorkspaceDraftOwner改model生命周期同步订阅后probe1/1。两者是既定E/L03交叉风险，不是整E或OS退出完成证明，原red/green见workspace证据目录。

root定点查当前其它入口：SessionContent、MachineFiles、SideWorkspace仍是React state草稿集合＋useDraftCount(count) layout发布，FileBody无上游入口也沿原布局报告。W本片段只修新WorkspaceMemory owner；不能把W 1/1当所有FilePage入口已同步。尚未用这些真实组件重跑同批probe，状态为待证风险。后续L03收口需对应实际入口检查最后已接受edit→Quit读数/移除保存→0/隐藏保留/卸载只清自身及真实Mac立即Quit；若确认同一间隙，按现单一owner窄修，不增加全局正文store/重复计数。不打断当前W阅读位置或A3 native批，不偷偷移到下一轮。


## L03同步集合owner片段（2026-10-29，root）

root复用干净math树创建fix/round2-draft-sync基于4cf7cb91，固定a0c34d13（6源），待非作者独审。useDraftState持唯一accepted集合ref和React投影；pure updater在回调里计算，accepted→同步report→project，不在React updater中发布副作用。private registration由layout激活/卸载清自身，StrictMode replay恢复保留count；卸载后的旧setter拒收。SessionContent/MachineFiles/SideWorkspace全部setDrafts路径统一用该setter，包括移动/删除。无新global正文store，原useDraftCount保留FileView独立模式。

实际三个生产state声明/onDraft由TS AST抽取、真实hook编译，夹具延后React commit：旧prod3/6（全部first edit期待1实0），新6/6；同task两文件、重复draft、保存清零、隐藏、多owner、StrictMode/latecallback涵盖。原count9/9及tsc0。证据input/draft-sync/candidate.json逐SHA；red-correct-lifecycle.log为真实业务red。首red.log的replay夹具次序错误、count.log的fixture缺两个unused React export均保留并明确；不回填旧结果。

本片不覆盖FileView standalone accepted disk base/flush时序，更不冒真实Mac输入/OS Quit。root下一片先独立复现这些路径，不扩global退出规则。


## L03 standalone同步首键片段与接纳基线复现

72591cae→d4dcc1a5已独审并集成。useDraftCount返回稳定report，FileView实际publishDraft无上游时同步0/1、有上游只委托不双计。真实onText/publish/计数defer commit旧7/8（standalone首键0期1）→新8/8；原count9、刷新8、merge9、types0。source指纹在standalone-candidate.json。standalone-merge.log只测试callback依赖提取器旧匹配失败，修提取及注入report dependency后9绿，原失败保留。

接下来真实旧回调3例已复现，文件input/draft-sync/file-acceptance-probe.mjs及file-acceptance-red.log：①cleanA→typeB→flush为0写，期1；②onLanded B后同task typeA误回报null，应脏A；③onLanded B后typeC投影最终Draft.was仍A，期B。测试实际抽生产onLanded/onText/setEdit/publish/flush、React setter延队列，未改产品、非真实OS证明。Root下一片需要唯一accepted file/edit状态贯穿基线/草稿/flush，并保merge/clash拒写；独审先核最小设计，不加全局正文store、不拿两片计数绿关闭E。
