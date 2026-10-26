# Q037 R1 root 二轮源码审与执行授权

2026-10-27，root非实现作者。01独审Q-037-remote-r1-review五项已由作者02处理；root完整读01核心队列/绑定diff后，逐读02关键delta、新issues模块/五新增case、修改原helper，再逐核03仅三源delta。图先minimal，当前图f721旧，不把图0flows当正确性证明。

02已normal Node --lib check通过15.101s/2350输入稳/Node current fresh=false/独占PGID退出；未编tests，所以root发现原helper scratch重命名漏一处_scratch（明确cfgtest编译阻断）。另issues::child可解析但version/intent/child不匹配旧issue会被覆盖resolved，须保原且未知计数。03仅mcp_worker_tests变量、issues先校身份保护、新真实Retry/三合法Task RPC与原件保存父，共三源；现场全部19源逐SHA=03冻结，公共link/proto/lib与01原patch一致。五项主要逻辑加这两处delta当前源码审无额外阻断，允许focused执行，不声称已green。

已核正向：known delegate在beforeSend持task id，真正HTTP续输入先取得Working答复再hold原reply，测试事件在真实副作用之后写；child坏源/receipt单列issue并推进cursor/合法兄弟，scan/issue保持unresolved；pending只当前有界marker页按64hex直定位同hash<=4KiB intent、保坏原件且大marker不搬不覆；step返回Typed ReadyAt，父scheduler不把发现重复的not-before提前，实际PID启动退避测试；坏metadata reply按Persist Unknown/Status Unavailable且核requested run。原R0同真实node Store的6 Prompt/Stop/4实际held Cancel/5,6实际到达oracle保留，非实际远端ACP全终态证明。

当前源03 external Q037/remote-r1/candidate03-20261026T193642Z 与internal同名，19源1,474,758B，2350 fullinputs，18 exact父清单。root已授唯一槽：实际CLI build→Node libtest no-run→原R0先exact，绿后原authorization/parent helper+16新父（18不同父）；single exact+include-ignored可统一选择但summary必须恰1，真正ignored子worker不另计。STEP300/TOTAL720/MIN5368709120、jobs1/locked/offline/shortTMP/sharedTarget/source-binary-current依赖实际编译/cleanup/双盘，不执行npm/gen、不改失败源自动重跑。失败立即停保raw，编译/setup/资源与业务红分别记。

Scope仍是R1已知child可靠取消/投递与状态；未知首次Send的task发现/late-send tombstone为R2继续open，不把18若green当整个Q037或全goal完成。集成须先原R0 seed f010，再R1固定生产修复（避免缺fixture冲突）；与Node后续link/proto/lib在主协调下合并。main/ship无授权，当前还不能提交集成R1。
