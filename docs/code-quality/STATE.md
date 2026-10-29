# 本轮代码治理：交付状态

更新：2026-10-29。**上轮已发布；用户已授权启动下一轮，当前E/H/C基础已集成，A/G继续实施与验收。** 原失败测试及下一轮问题保留，不声称全仓逐行重构或全测试/平台全绿。交付见[DELIVERY](DELIVERY.md)，范围和用户决定见[PLAN](PLAN.md)。此前详细检查点保留在[历史](history/STATE-before-os-business-closeout-20261028.md)，不能按其中“当前运行/缺权限/0拖放”重新启动。

## 最新执行点：第二轮active，输入/历史/内容链路已集成，尚未整轮完成

授权仍为用户“开始吧”及目标“完成第二轮”；28反馈＋5遗留共33项不变。main/ship未授权，main仍e7c335c8干净。当前集成integrate/round2@250d64ed；两作者当前轮换为A统一观看（round2_history）与G共用材质（round2_input），独立reviewer按固定源码审查。勿按小提交数计算条目完成率。新增G共用band62eb3c98→14e0cc6d，A1持久内容来源631c0d0a→05c499a5；两者固定版独审无阻断。G局部sticky/真实CommitPage消费者660f7e2f已独审并集成250d64ed。

已集成：E刷新/merge迟到接纳、选区退出归属/搜索清理、文件草稿owner计数；H有界窗口、尾页/older/newer、记录分块、取消lease、实际ChatView有界消费；C正文/终端链接按普通点击与Cmd分流、App/ChatView接线、当前工作区返回入口。最近H GUI18项、C3导航6项及集成TypeScript通过，非作者审查无阻断。H旧全量RPC清理a3e9b6ad已集成b35756d1，159项真实producer产物和6项源码指纹验证后同步，集成tsc/mobile静态通过；Lab499dbe4f独立保留，不当生产修改。

浏览器验证现已可用：已读Playwright skill，以隔离可见Chrome操作真实组件，不再以缺CUA作整个浏览器阻塞。23834真实路径/网址框1000/390宽、双方向拖选共8项通过，选区保留且后续输入替换正确；外部首动作/Escape/字段ShiftLeft亦有浏览器证据。初次harness把Mac Home/End当其他平台、错误使用textbox角色导致假失败，修正后记录明确保留。该结果不等同Mac WK中文IME、用户回复框ShiftLeft或真实Quit验收。

终端Lab第一轮超时为harness观察错DOM，第二轮目录命中相同前缀文件为harness行匹配过宽；两次证据保留。修正后19项真实鼠标验证绿：宽窄宽软折行各段、Cmd OSC8四地址、拖选与程序mouse tracking守卫。生产无需为这两次假失败改动。随后追加中文宽字符行尾case确实复现产品故障：xterm末列code0占位被拼进路径。作者40a828a3→集成8f9506d3，仅跳过明确的宽字符软折行占位，保typed space与hardline；真实producer旧2红→新4绿，独审/集成4项绿，原40cols浏览器case红→绿且一次打开精确绝对中文文件。H实际ChatView Lab23836/PID88855的13项浏览器验证通过：20万逻辑记录惰性夹具首屏有界、empty poll DOM身份、5次older锚点精确保留/缓存淘汰上限、500条四批追赶无缺口、raw单段64KiB/UTF8、source reset清raw、隐藏/卸载取消后lease0且停止poll、legacy0/unknown空。夹具不证明真实raw旧来源拒收/取消，按真实后端证据分开；首harness中文按钮标签写错超时原样保留。C3实际SessionContentPane Lab已就绪，23835当前PID92646，/lab/workspace-return.html；桌面/窄屏四流程均已验：返回最新cwd、browser返回保address，原dirty文字与同CodeMirror DOM保留。最后未知trash_here断言失败保留；确认该API纯const能力查询后Lab e58f21a1显式false并单独HTTP验绿，不回填旧整脚本成功。23834/PID3175及已验参考23832保留；不重启用户App或session。

G首段21项实际浏览器检查已完成（宽窄/明暗/solid-clear-frost12、主题anchor4、真实标题增高2、二级标题2、reduce1）；7项受控owner与集成7项、px/icons/type/tsc通过。实际检查分批源与两次Lab问题见glass/browser/verification-scope.json，不称全21在最终SHA重跑；Lab15196297与生产提交分开。G消费者继续：同注册体watchBand/band/stickyGroups，保唯一port owner和既有滚动布局。真实CommitPage宽窄宽jump对照暴露desktop首52px遮挡（新band跳转没扣可见header），narrow正常；现按实际scrollPaddingTop修复，flush正文mask/标题blur尾部同收于标题下沿、普通正文尾部不变。真实宽窄宽三次jump及静止/小滚4状态均过，目标bodyTop=bar.bottom且不加空白、port唯一owner、标题满值/正文滚入才fade；原jump-inspect红与root误要求必须有mask的harness失败保留（mask:none正确满值）。11 owner/4 jump与集成types通过。机器表原无独立scroller，不为材质新增内滚动；图表/metadata兄弟fade也不能漏。

A1现按打开解析并登记最小持久ObservationBinding（仅Khor自己元数据），固定home/id/provider/完整nativeID，未知/歧义拒绝；page/raw复用，source换代page reset/raw旧引用拒，UI结果按id+home归属。已删除拟临时FACTS缓存及其列表副作用。原6native、真实gen/catalog、19历史+7观察回调/tsc/mobile均过；raw修证只cfg(test)，有效cursor先成功再换源后精确拒，一条真实重跑过，生产prefix SHA相同。162产物含catalog逐SHA核后同步到集成，集成19/7/7/tsc通过。root首次映射错catalog所在路径，验证在修改前停，过早tsc缺新类型失败保留；正确同步后仅types重跑绿，不计产品失败。A2身份/执行轴已定并进入实施：observe/JSON tuple hash独立alias，core Session.state真正Option，普通Some/观察None，SessionRow.word nullable；kind OBSERVE先走观察，不用thread_unbound/Idle伪装、不加第7运行State。内容home由binding后端判断，alias删除本地记录、不kill writer；CLI/MCP/GUI控制同禁。必要Rust与UI闭包记R2-A，G所持文件不并发。A3/4仍未开始。

未完成：E真实Mac IME/回复ShiftLeft/最后按键与Quit；H最终原生同源组合验收；C后续原生组合验收；A全部ACP观看、S持久化恢复、G材质、W正式工作区布局/历史、B浏览器、M移动手势/键行、D/N调查及V遗留等后续包。不得把历史有界读取当旧卡死根因已查明。最新诊断仅发现DiagnosticReports无khor命名报告，R1仍未知。

证据位于/Volumes/disk0/Khor/evidence/round2：input/browser-fields含8项拖选与退出报告；content/browser保存终端首次失败；history各阶段含预算、独审与真实生成收据。内盘已保存8f9506d3的bundle与201项文档镜像，新250d64ed检查点继续同步。A1 native6/gen/catalog和raw修证单条都已终态，槽当前空闲；观察曾错误替换旧check-session-content脚本已恢复原字节、新测试独立命名，独审核无旧覆盖丢失。保持root＋两作者＋独审，不新增第三实现线。

compact先核本节、ROUND-2-EXECUTION与R2-E/H/C/A/G/V/REVIEW任务卡、实际refs/diff/agents及唯一native槽。下一动作：G机器表+frontier完整普通兄弟fade、菜单、设计文档/旧表头rule断言最小适配；A2只读导入/观察行Option执行轴实施，A3全ACP双向journal/replay事务和A4 home读取排队。只读footer未证明“原终端正在运行”的文案已列A2中性化。V既定L04/L05最小实施准备见R2-V，未加第三实施线；首整合native App前插入现作者槽。B/R016仍未知旧WK根因，CLI双HTTPS200不冒WK绿。目标保持active。

## 历史与恢复

此前检查点移至 [STATE历史](history/STATE-before-round2-b35756d1-20261029.md)，其中旧权限、进程与“尚未实现”均不是当前事实。上轮交付见 DELIVERY.md；当前范围见 ROUND-2-EXECUTION.md；最新实现细节见 tasks/R2-E.md、R2-H.md、R2-C.md、R2-REVIEW.md。
