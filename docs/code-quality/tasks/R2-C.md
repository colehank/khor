# C 内容打开/导航

2026-10-29状态working。用户目标完成第二轮active。owner round2_input轮换至C作者，E已提交/待真实验收（不是done），旧E树和23834Lab保留。
作者树/Volumes/disk0/Khor/worktrees/current/khor-round2-content，fix/round2-content，基线c95d318aa601fad5c64f906607c64e231b5065a5。源规则SESSION-CONTENT与ROUND-2-EXECUTION，条目R2-008/009/010。
唯一写权PathLinks.tsx、Markdown.tsx、linkHome.tsx、term-links.ts、新content-link-intent模块/测试；TerminalPane必要最小接线也由C唯一owner。App.tsx由root小接线，ChatView.tsx为H唯一owner，以契约/补丁交H。
先确认当前客户端系统打开与provider机器launch语义，不能偷换为remote宿主启动。普通内部/Cmd客户端系统仅依意图，不作网络可达性预检、不改写目标；路径折行先真实复现；左上按钮回会话工作区，不在推荐置顶。
API/原生bridge若需新增先报共享路径，生成类型只走producer。证据evidence/round2/content，非作者reviewer独审；native槽root协调。只可写本中央卡，root统一总账。

## C1 prose/系统交接首checkpoint

提交 `bb21b8d3d9a07e4f912dac79ae57cfdbe69ed404`，review-ready。共享模型content-link-intent只按meta/ctrl意图区分；原生URL openLink、路径既有本机launchOn空machine；Web URL window.open，路径只交客户端file URI，绝不调用Web服务端launch。代码已核api.call在Web走同源服务、Node.resolve_or_here空machine为服务器自身，所以不能用于Web客户端系统打开。未加网络可达性判断。

PathLink普通home.open，系统helper；Markdown普通home.openPage（新可选LinkHome接口）/系统helper，文内anchor保持跳转。未提供session URL内部provider的普通文档上下文沿用系统打开。失败PathLink按目标隔离显示dotted/title且可重试，不永久禁掉重用目标。系统file URI编码保留空格/%/#/?及POSIX反斜杠，仅明确Windowsdrive/UNC归一分隔。

13项实际组件回调/URI/Web动作检查旧红新绿、完整types与diff绿。独审先发现POSIX反斜杠目标被改写、拒绝态永久失活，已修并补用例；async provider返回也await捕获。证据prose-intent-receipt.json及red/green/types日志；未实测OS opener或Web file交接。window.open noopener返回null有歧义，不能宣称成功或判为确定拦截。

供应端协调：App由root接askOpenPath/askOpenUrl；ChatView由H接普通path/URL，不再preview。首checkpoint仍带原有unused launch字段以保独立types，不新增deprecated兼容层；消费者改好后须在type与App/WorkspacePane/ChatView三个provider一起移除。root已批准C仅改WorkspacePane provider的launch清理，不动W布局。TerminalPane C1接线、C2实际折行、C3回当前工作区尚未做，C包不标done。

## C1 供应清理与终端切片（2026-10-29）

`f6d9819dec80ef5cf0a4bc1d75207b761ab2e284`移除LinkHome.launch及WorkspacePane最小provider/import；H的`efeaa12b`胶水已在C树cherry-pick为`7778c0bd`，App归root统一集成。没有新兼容接口。

终端切片checkpoint `ec9cf06c328c622dcd430fad572556f9af25575f`，独审最终无阻断：真实TerminalPane的URL addon、OSC8、path provider及coarse tap统一意图；plain内部，Cmd/Ctrl交当前客户端。xterm 6.0的Linkifier mouseup会激活当前link，不能假定它已保护选区，因此共享guard保护已有选区/非主按钮/application mouse tracking，显式系统modifier仍不误开选区。coarse tap和异步path回包同样检查。URL保留既有http(s)协议约束，不检查网络可达性。

系统opener已从纯意图模块拆出client-opener。路径Web只客户端file URI，native原生本机launcher；OSC8系统file URI原样交Web window.open或native openLink，不再pathname丢authority/drive。root批准新增写权core/src/web.rs仅system_target验证用于open_link，page_only网页规则不变；H独占iOS open_link单行改为同验证。没有改remote launch/新协议/生成类型。失败仅TerminalPane局部可关闭alert，不占用connection why或持久notice，晚到失败由action身份隔离。

21项实际terminal回调/guard与16项prose/client-opener检查、既有真实xterm Node链接检测、types/diff绿；旧实现16项只有部分通过且exit1。收据terminal-intent-receipt.json。Rust validator两项只断言不spawn，root授权唯一Cargo槽实际2pass/142filtered、exit0，叶文件rustfmt绿；以上不是实际DOM点击或OS opener成功证明。

root已批准23835真实TerminalPane独立Lab，cacheDir在Khor/caches/vite/round2-content-links，唯一PID82610/exec64218已启动。须复用真实provider/实际addon，本地fixed目录数据/opener观察日志，无用户session/真实外发；root用Playwright实测plain/system、拖选/右键/TUI及完整ASCII路径/目录与URL软折行宽→窄→恢复各行命中。E 23834原服务保持。C2/C3与真实验收尚未done。

## C3 返回当前会话工作区checkpoint

提交`a965c16fd4581f4a7136d3fc46a76376b4817da7`，独审源码无阻断。ContentPicker新增明确动作，desktop复用ContentToolbar onBack，mobile正常流顶部GlassCircle（不绝对覆盖树）；在另一目录/机器或browser picker时可一键返回。点击结束search/edit后navigation.update(scope:null/source:workspace/query:'')；实际navigation hook证明null清scope，后续root取最新row/home cwd，不捕获旧cwd。不清address/exit/document tabs、草稿或阅读位置，不新增推荐。browser page history仍独立。

6项实际hook/update/条件/callback检查旧缺动作exit1→新6pass、原field13回归/types/diff绿，收据workspace-return-receipt.json。静态owner核：此picker SideWorkspace始终onOpenContent委托且无start/spot，文件进入SessionContent保留文档与上层draft集合，不在换root的picker中挂载FilePage。实际DOM未保存文档保留与按钮布局仍待Lab，不能用独立未引用draft fixture冒通过。


### 当前Lab检查点

终端真实DOM首脚本在wide FILE hover等待超时，尚无action，原browser/link-failure.json与terminal-failure.png保留，不记路径成功/失败。定点读xterm6源码：CoreBrowserTerminal491将screenElement传Linkifier，Linkifier332把pointer class加在.xterm-screen；脚本却等.xterm根上同class，属于明确harness定位错位。已告root校正观察节点后独立复测，不改业务断言或产品实现。DIR既有target.tidy会去末尾slash，应核同目录及dir:true而非把斜线文本差异当错目标；其它目标继续精确核。23835冻结保持待root本批结束。

C3完整SessionContentPane Lab入口与types已备（workspace-return.html），尚未扩本地API fixture，因此未可用/未DOM验；root当前终端批次结束后再一次更新配置，避免HMR影响。Lab source均当前C作者树未提交、对应server.json已写source SHA；不是生产checkpoint的一部分。


root修正实际observer节点与按前缀识别逻辑行后，终端真实DOM19项通过：宽→窄→宽各软折行片段、OSC8+Meta四种、拖选不误开、app mouse。browser/link-actions.json/terminal-verified.png及耐久terminal-link-check.js保留；前两harness失败独存，不改生产追绿。

root允许本批结束后扩fixture，23835已单例重启为PID92646/exec53679（旧82610按精确PID正常TERM退出143）。workspace-return.html现已开放：实际SessionContentPane/useSessionContent/useContentNavigation/FilePage，内存tabs/fixed read_text、write_text明确本地409拒写，所有/api由本地fixture答复。HTML/tabs_list/types绿，DOM待root。server02.log与content-links-server.json记录新PID/源SHA；未启动用户App/session或真实外部opener。
