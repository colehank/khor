# 本轮收口与候选发布（2026-11-10）

用户最新明确授权：`做完这一轮，直到ship完毕`。解除先前main/ship HOLD。根智能体串行；不新派智能体；本机继续source-only，所有编译／执行验证走GitHub托管CI。既有用户App/会话优先保留，原生验收先用隔离环境，不随意重启真实宿主。

## 目标和顺序

1. 集成main的App图标12d89226，保留本轮源码与规范；最新主机文案一起纳入。已合并为7ca9bd5f，仅两处文档冲突，双方新增内容均保留。
2. 整轮Quality(all/all)跑出明确结果，处理旧WebKit滚动／窗口缩放红项；检查跨平台文件重命名／移动、更新修复和原生UI承载边界。
3. 补齐原生通知覆盖浏览器/PDF的链路；利用可信CI/隔离原生探针验证实际可用路径。物理iPhone/不可访问硬件不伪称已测，有必要的硬件限制明确保留；不把浏览器模拟当真机。
4. 更新最终真实App Lab，包含新App图标资源与Khor主机/SSH主机文案；定向与整轮检查通过后合并main。
5. 沿用现有v0.2.0候选线，准备下一个不可变tag v0.2.0-rc.12（已核对rc.1–11存在、12未占用）。按docs/RELEASE.md由GitHub执行全Quality、四CLI/两MacApp/iOS归档、签名公证、TestFlight处理、更新清单签名和GitHub预发布；直到分发完成。当前没有创建tag或发布。
6. 稳定tag会额外提交App Store正式审核；本次已向用户明确沿用RC线，目标GitHub安装包/更新清单/TestFlight。成功RC不等同Apple外部Beta审核或实际安装。

## 已知完整验证缺口

- scoped60App+hot通过，不代表fullQuality；旧scroll-lifecycle WebKit reader-resize需复验。
- 原生通知在browser/PDF siblingview之上的承载未闭环，需要先查清现有menu_overlay/native-page-yield再决定最小修复。
- 文件操作已有3平台隔离Rust+SSH脚本证据与实际FileTree浏览器拖动；仍需原生集成边界验证。Air更新已有Linux/macOS/Windows及真实WebKit自动化证据，未在Air真实安装；本轮发布后可检查更新可见性，不虚构安装。
- 任意用户历史消息编辑+重新生成，继续按用户“不行就算了”明确延期，不阻塞本次发布。

## 当前状态

分支integrate/settings-gui-20261109（原HEAD5eb971f5，已合并main为7ca9bd5f）。源码main的未跟踪lab-preview保留。旧Lab23950仍dc946c83/PID6019/exec99444，后续替换仅核对过归属的静态服务并保留旧版本资产。

后续精确SHA/run/tag/receipt持续写入本文件及外部evidence状态，compact从本节和实际git状态继续，不重走旧HOLD。


## 首次全量回归

main图标同步后effe2793/Quality42779150845前端在catalog生成时红：最新SSH主机文案和已有gui.add-ssh-host同文，目录禁止两个key同文本。已改为复用原有add-khor-host/add-ssh-host，删除新增choice别名，App菜单/弹窗标题共用同一词，Khor统一大小写。没有绕过去重门。原生通知确认BrowserPane子WebView不能由DOM盖住，正在复用既有menu_overlay可信浮层通道，实际通知状态/React事件仍由主界面拥有，不造第二套业务或外部系统通知。

## 原生通知补齐（进行中）

45c5a375为先红探针，独立verify/native-notification-red-20261110运行42783272495；74791dd9全量Quality42781818432仍在跑。原生通知复用menu_overlay的可信lease、resolved DOM快照以及同一份自包含输入桥（桌面脚本禁用iframe与iOS私有WKUserScript），状态、计时、滑动阈值、点击均回送原始React组件。native menu_input只接受当前lease现存候选的有限原始输入，远端page不能调用。系统文档预览通过promise租约暂停通知计时，返回继续；不会宣称DOM盖住QuickLook。refusal才短暂yield retained webpage，正常不隐藏浏览器。

全量74791dd9的GUI完成：WebKit滚动/resize本次已过，updates-ui仍找旧dialog-footer关闭钮，已改新TaskActions leading槽；recovery的directory-required-shell报告facts=null/errors=[Browser page crashed]，是原生WebKit测试进程崩溃，尚不当作业务结论，最终源码会复跑。45c5先红有探针未显式初始化transport的问题，已用独立baseline分支5c8ef87e/42785087686修正再跑，不能把前一次ReferenceError说成预期红证据。新增共享DOM native notice桥已在8dec7839/e0bdb71f，完善iOS生产adapter隔离simulator探针与CI receipt，非物理触摸。

## 17:00收口进展

- 原生通知源码已到236cef38（远端integration同），新增iOS实际adapter隔离simulator探针，源码bridge+WebView层级/响应器/原网页保留都会验；物理UITouch仍不宣称覆盖。上游MenuFace旧fixture对新menuInput改namespace导入保持未调用路径的兼容。
- baseline5c8ef87e/42785087686 notification-ui按预期showMenu5000ms超时，明确无ReferenceError；外部native-notification-red-receipt.json保留。前一45c5探针初始化错误不能算红证据。
- ed99da53全量42786231805正在运行；scripts只报3Rust格式文件（lib.rs/menu_overlay.rs/ios.rs），其余动作lint过。用临时baseline验证分支覆盖quality.yml仅作remote rustfmt写出source-only patch，run42786844142固定checkout236cef38。补丁待取回并核对SHA/gitapply，临时workflow不合并进main。
- 74791dd9全量42781818432曾有GUI两红（旧footer selector已改，WebKit页面崩溃待最终复跑）。为让新源码验证开始，旧运行取消于node-native最后的signedSSH验收，不把cancelled称绿；新全量会重跑。
- 42784959928/42785435107为相同integration ref排队的过期运行，GitHub pending replacement可自动cancel；不拿这些作发布证据。
- 下一步：格式补丁、新native UI/原生compile探针错误修复，最终全Quality；Lab handover；合main/tagRC12/全发行分发直到TF+updater+GH完成。尚无新tag/release，用户已解除HOLD。

## Native notice active debugging / do not publish yet

HEAD468c3e8b integration pushed. dc731902 applied official rustfmt patch from hosted42786844142 (source236 exactSHA). 971416c0 fixed StrictMode early-srcDoc reattach; 468c adds hover failure geometry/animation/calls JSON and screenshot. Remote fast verification uses temporary baseline worktree quality.yml only (DO NOT merge that temporary workflow), reads generated types from official ed99 frontend artifact42786231805 with explicit originalSHA/run verification. Generator/API crates unchanged (git diff verified). Fast source971/run42787832001: types passed, native presenter acquired/sized and frozen banner exists, but banner.hover never stable; potential renderer feedback/measurement loop. 468 diagnostic run42788979384 currentlycancel requested because hung past4mins (likely page.evaluate or browser.close after hover timeout), always artifactupload maystillretainnotice-fast files. Fetch those beforeguessing. Temporaryverificationworkflow now45baf955 adds job5min and timeout90s fornewruns; notpushedyet. Need restartafterevidence or instrumentfurther.

ed99full42786231805 cancelrequested also: scriptsonlyRustformat(redfixed), gui/nativeMacstillrunning; macstepcompilation passed then sharednoticeUI/simulator stepstarted09:03:49UTC hung (>10min), likelyUIexecutable nohardtimeout. Need boundUI subprocess inproductionMacCI (Python processgroup wrapper or dedicated fixturedeadline) afterdiagnosis. Do notcall cancellationgreen. Baseline5c8 UIred proofvalid. Mac nativeoverlaylease unitcompilepassed(beforenewformatter only), iOSrealprobe notyetproved. userstillwantscomplete roundthroughship, no main/tagcreated.

Native trace738/run42790489690：source与DOM所有记录均update:start/end完成，多次frame→snapshot→mirror；无hover几何JSON（页面evaluate无回应），超时90s正确结束。trace已在evidence/round-ship-20261110/notice-trace-738/native-notification-trace.txt。改23d1ff53：特殊通知snapshot收集源CSSKeyframesRule notification-arrive/depart与源已解析animation/transition，native复用源CSS动画而非逐帧serialized HTML；源owner不再每RAF克隆DOM，姿态只随真实state/gesture改变而同步。验证run42791368364固定23d1ff53，正在跑。后续加Mac CI UI子进程组120s硬超时，避免故障browser.close卡住整个60minnativejob（未pushed最新这commit）。上一ed99mac证据确认app update15、menu_overlay4、companions2 passed，iOSsim未跑到；all run已取消而非green。

## Native desktop simplification

Desktop render now952c988e/da29415f uses a trusted sanitized ShadowRoot in the existing native menu window, no iframe/scripting-disabled child document. Same notificationDOM factory receives a structural document adapter (real Document on iOS, shadow tree on desktop), same source React controller/gesture events. Source CSS keyframes are reused; owner marks native/inert once instead of repeatedly causing observer mutations. Fast42794835131 fixedsource da29415f is pending. New stale-key nodes are replaced rather than reused as a different notice; pointer capture starts on the actual button at down. All test waits have defaults; parent viewport coordinates drive raw mouse input; completion query reads owner document only (Playwright shadow-piercing must not count retired mirror DOM). Tests of the iframe were not proof of a product feedback loop; disabled frame helpers/context and unbounded waits made diagnostics ambiguous. Old reports preserved, no native pass asserted yet.

Native UI GREEN385aaaa2 / quick42795790704 after ShadowRoot adapter listener type fix. Types + same owner renderedcontent + hover forwarding + physical pointer swipe + no-open-after-swipe passed. Download and retain receipt. Current extra test will also seed next actual controller notification and verify physical click opens owner, assert no script/resource markup. Then fullQuality(all/all) incl mac/iOSprobe. No tag until finalgreen.
