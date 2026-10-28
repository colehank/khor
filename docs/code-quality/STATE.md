# 本轮代码治理：交付状态

更新：2026-10-28。**本轮实现与既定业务验收已完成，已合并main并完成Mac/iOS/集群发布；当前进入实际使用反馈，未开始下一轮实现。** 原失败测试及下一轮问题保留，不声称全仓逐行重构或全测试/平台全绿。交付见[DELIVERY](DELIVERY.md)，范围和用户决定见[PLAN](PLAN.md)。此前详细检查点保留在[历史](history/STATE-before-os-business-closeout-20261028.md)，不能按其中“当前运行/缺权限/0拖放”重新启动。

## 最新进展：工作区 Lab 已指定为下一轮参考

本轮最新用户决定已落实：文件/提交共享单击预览、双击固定，各commit以hash独立保留；提交正文显示标题和正文，标签用用户历史SVG；文件默认展开（覆盖较早默认收起），手动收起按commit保存。左侧仅日期链条借鉴GitHub，右侧保持Khor平面列表；最新反馈要求其只是更高的文件行，故左右文字/状态列、右边距和选中背景均按文件树对齐，禁止卡片化和额外右侧留白。返回文件在原摘要行，与文件名同起点。工作区picker恢复正式头像/名称+branch/Git计数/摘要+时间。右上最新改为两块玻璃 `[会话 VS Code] [更多]`，更多在最右（用户最后调换），覆盖旧三图标同胶囊；标签栏与文件标题行之间不画分割线/下沿阴影。

最新左侧头部复用正式 PaneBar：返回、工作区名、新建、搜索；搜索展开替换名字但保留返回。下行按用户最终修正为 **机器头像 : 完整路径**，branch 与文件树顶部最近提交合为一行（分支/摘要/时间），取消独立分支行。头像提示机器名，头像宽高1em与路径字号一致（桌面实测12/12px）；路径可复制和折行。文件树/提交列表共用此头部。连续新建暴露菜单关闭抢输入焦点，PaneAction 增加可选 afterClose，仅 Lab 启用；先释放菜单焦点再打开命名框。TypeScript及5组布局/搜索/新建/窄屏检查通过，证据v5/machine-path-check，原三次失败均保留；未重跑旧31项矩阵。

最新分支/提交合行已通过TypeScript及5组布局检查，证据v6/ui-check；整行点击仍进入日志，返回行保留分支和数量。

文件树和目录列表取消悬浮下载，下载统一进右键菜单；状态字母/目录点同固定末尾槽，反馈不挤动标记。所有下载在Lab仅走隔离mock，返回“Lab 模拟结果（未写磁盘）”。正式App/main未改动、重启、合并或ship；S/C/B专项和工作区正式产品接入仍未开工。

Lab工作副本 `/Volumes/disk0/Khor/worktrees/current/khor-workspace-layout-lab`，分支 `lab/workspace-layout-20261028`，源码提交 `c501029490bf5997a271e252941582911d421a8f`，基线main `e7c335c8`；入口 <http://localhost:23832/lab/workspace-layout.html>。说明在Lab `apps/gui/lab/workspace-layout.md`，最新决定与历史在 [WORKSPACE-LAYOUT](WORKSPACE-LAYOUT.md)。服务继续供验收。

TypeScript通过；布局5组、预览/固定3组、阅读位置3组、commit呈现/折叠/完整文件/跳转14项、下载菜单/状态列/返回文字对齐6项通过，无浏览器运行错误。纯重命名完整文本用例先红后绿；位置脚本原hover后立即wheel的失败留存，稳定布局后完整位移验证通过，没有为其修改产品。最后间距样式反馈追加定向几何与截图；随后仅顶部材质/按钮分组微调，TypeScript与1360/390视口截图几何检查通过（零边线、零阴影、相接52px、无横向溢出），证据另存v4。详见 `/Volumes/disk0/Khor/evidence/labs/workspace-layout-20261028/v3`。旧check-face强制hover下载按钮的门与新用户决定冲突，正式接入时须更新，不声称旧全仓门通过。

用户已指定本版Lab作为下一轮修改参考。最后标题→路径、路径→提交摘要文字间距已统一，当前桌面实测均12px；此次CSS定向几何检查，不重跑功能矩阵。正式后端完整历史分页、文件/Git自动更新、原生/iPhone验收仍属下一轮，不用模拟通过代替。旧运行版本核验见专项，是当时证据，不代表当前进程仍存活。

## 浏览器五项问题已登记，仍未开始下一轮

用户在下一轮收集阶段补充浏览器加载进度循环、默认白底、Mac 页面连接失败、手机甲板挡网站底栏、列表左滑缺删除，已保存 [BROWSER-EXPERIENCE](BROWSER-EXPERIENCE.md)，登记 R2-014–018。已确认循环动画为主动实现、网页子视图未同步默认主题背景、左滑筛选遗漏关闭项；手机布局存在 frame 覆盖与 scroll inset 不能保证网页固定栏避让的结构风险，尚未同页实机复现。

Mac 对截图 URL 当前直连及显式系统代理 HEAD 均 200；标签 device 为本机，按代码应直接打开。UI 将无法连接/连接丢失合并为“连接被拒绝”，缺原始 WebView 错误，不能归因网站拒绝、远端网络或宣称已修。详见专项调查事实；一次误向 IPC 端口发起的 CONNECT 超时已明确排除为网络证据。此次只有只读核查与文档，无产品修改、Lab、构建、重启或发布。恢复时读三个专项及 ROUND-2，不自动开工。

## 会话内容交互纳入下一轮，尚未开始实现

2026-10-28 用户确认六项交互任务“这些都放入下一轮”，最终决定见 [SESSION-CONTENT](SESSION-CONTENT.md)，登记 R2-008–013：普通点击内部 / Cmd 点击系统打开，不做网络可达性判断；按“任意路径换行即失效、网址似乎可用”的反馈实机查差异；左上角切换按钮回到 session 工作区，**推荐中不加工作区项**；共用拖选/外点退出、搜索退出和工作区推荐菜单可读性。此前网络路由扩展和置顶推荐提议不采用。

本次只更新规划；与下方会话连续性专项均为 proposed，用户后续允许才开始修改。两条只读审查及 root 核查已完成，现有 links/field-interaction 检查通过，但不代表用户折行、桌面拖选与菜单材质故障已实机复现或修复。main 产品代码未改，未启动实现、Lab 或构建，未操作运行中会话。下一轮范围与授权入口为 [ROUND-2](ROUND-2.md)；恢复时同时读两个专项，不遗漏本节用户修正。

## 会话连续性方案已存盘，等待授权

2026-10-28 用户明确：“这个任务可以存盘, 和之前的一样, 在我允许后, 进入下一轮修改”。下一轮候选范围见 [SESSION-CONTINUITY](SESSION-CONTINUITY.md)：S1 长历史有界加载 → S2 所有已接入 ACP 的能力驱动观看 → S3 持久身份与重启恢复。状态为 **proposed，尚未授权实现**；只保存规划，不启动实现、构建、Lab、接管或发布。旧 PLAN 的开工记录和上轮发布授权不能越过本决定。

已只读确认 YufeiEnd 由 ChatGPT.app 内 Codex 后台持有，原生历史存在，前端仅 Claude 有外部只读入口；后端已有 Codex 历史/增量读取。用户另报长历史 Claude 导入后卡死，代码确认首读全量、整批返回和全量渲染；缺具体会话与卡死时采样，不冒唯一根因。细节、源码入口、验收、多智能体和压缩恢复规则均在专项文档。main 保留 e7c335c8；中央规划分支新增文档，实际提交与内盘镜像见 current-worktree-refs.json。下节发布数字和 PID 为当时收据，不作为现在的会话数量/存活证明。

## 已发布记录：用户测试与反馈阶段

用户明确授权“先合并到main, 然后ship, 我们来测试一下总结一轮问题再到下一轮”，已执行。产品ecab575e与中央最终文档plan6f3c07d5已合并；构建提交**320a48d95d18222aaa4216702408caa9eaffca51**，后续main提交仅发布记录/反馈文档。最新实际refs以恢复集current-worktree-refs.json为准。

- Mac CLI/GUI正式release构建、完整UI和Pi字节嵌入、签名及独审通过。CLI已原子安装，GUI主PID79598真实监听5467，UI收据`khor-ui-assets:/assets/index-BjJjcA2m.js`；默认命令解析到~/.cargo/bin/khor。
- iOS新静态库/归档App/签名/独审通过，0.1.14（60）已Upload succeeded/EXPORT SUCCEEDED，Apple开始处理；不冒真机已安装。临时两签名文件已逐字恢复。
- hinton/turing/lecun一次onset安装及keeper自然换新，真实5467监听exeSHA均e32f2a5f41d76509472980c817954a0dd312eefcbfbb27661fa371686b5c8ca7，三机UI收据一致；Mac CLI确认三台直连。每机postinstall成功--all观察均保留原10IDs。两次只读查询报document transfer state busy，原失败保留，未反复追绿/重装，登记下一轮。

完整发布收据在`/Volumes/disk0/Khor/evidence/releases/20261028-code-quality`，发布记录见[SHIP](../SHIP.md)，反馈见[ROUND-2](ROUND-2.md)。目前无编译/上传/本轮installer在运行；正式GUI、用户终端、Codex后台与三机resident是用户运行服务，不清理。

用户确认旧GUI卡死/无法识别session后强制kill，9个旧会话失败来自宿主结束；原10IDs仍可在Mac --all中找到（当前11条），没有按默认年龄折叠误判数据丢失。普通import只登记观察状态，没有接管外部iTerm PTY；此路径与旧8704一致，但缺实际rowflags，不冒排除所有终端流回归。当前Codex0.165的工具进程祖先是独立app-server89847→daemon1548，不隶属Khor终端宿主；后者关闭与后台agent结束应分开表达。旧GUI卡死唯一根因未定位，无冻结瞬间主线程栈，不把资源压力当已证原因。

先让用户测试、汇总并确认下一轮范围；不要自动实施导入/接管、重启同一Codex会话或重跑已绿治理矩阵。下方版本与未授权语句为治理收口时历史，被本节最新决定替代。

## 治理收口时的版本与运行状态（历史）

- 主仓库：`/Users/zgh/Desktop/working/khor`，`main@8704a503a27b36ad58f922690de50e766f8bd485`，tracked clean，既有发布服务不动。本轮无main合并或ship授权。
- 集成：`/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，`integrate/code-quality-wave1@ecab575e3f978f6570bff2c05a9542be93fd7342`，tracked clean。最后产品改动仅main webview特定fullscreen查询grant，用户已明确“纳入”。作者树overwrite-recovery同提交，原N1分支保留e110。
- 协调：本目录，`plan/code-quality`；最终文档提交及全部实际refs以恢复集`current-worktree-refs.json`为准。
- root唯一native槽已释放。所有本轮exec均terminal；没有受控App/Cargo/helper/observer/TMP残留。不再读取任何已退出CUA App对象，以免重唤。专用Finder窗口已关，用户原窗口/服务保留。

## 交付与验证

| 范围 | 已验证事实 | 限制 |
| --- | --- | --- |
| 首两项故障 | 键盘/输入框增高时最新消息跟随、读历史锚点；commit连续跳转及焦点交接。真实红→浏览器/真实Tauri iOS模拟器/Safari既定验收，相关源与最终提交映射已核 | 不冒物理iPhone/IME |
| 已启动模块治理 | 编辑原子发布、保存代次/digest/owner；身份和存储范围；网络/Node/Pair当前协议恢复；Pi/ACP/MCP输入取消、资源/退出归属；错误语义与共享纯规则已集成 | 模块范围与证据在AUDIT/tasks，不冒所有文件逐行重构 |
| Clean Code | 命名与角色、职责分离、单registry/constructor/RAII、共享规则、明确副作用/失败和资源owner；机械格式单独成批，保留必要协议/并发解释 | 不用注释/行数减少替代质量判断，ENGINEERING含来源与具体约定 |
| 组合与N1 | 原115父113pass/2fail保；原062 Busy/LocalSet反例当前修复后14父/22summary/19phase绿并独审 | 不回填旧115；原040下一轮 |
| Q033/Pi | 真实四图取消恢复→同稿重发Allow/EndTurn→小图取消、mode/option；实际嵌入Bun启动/空secret Cancel/原5s退出门/正常Quit/清理均通过独审 | 无provider登录/API key输入；旧harness失败保 |
| 生产借网 | 双私有peer实际原五阶段、native与新TCP、重开释放、正常Quit与清理通过独审 | 同机fixture仅既有KHOR_NAME初始区分；不冒真实SSH/master/remotelease |
| 最终OS业务 | 用户两次真实Finder拖放，chat_put_paths与term_drop各唯一成功/顺序与落地内容正确；fullscreen成功query false→true→false；全局WorkspacePane拒写→关闭阻止→Don't quit→fixture恢复权限→真实成功保存精确20B；实际CmdQ/GUI exit0 | 原whole脚本因持久session契约不匹配仍false，详下，不补写原green |

最终交付对照：`E/P5/final-delivery-current-audit02`。E=`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`，I=`/Users/zgh/Library/Caches/khor-code-quality/recovery`；同名证据双盘。

## 最新真实OS验收及原失败分类

`P5/fullscreen-query-os-owned-run03`，exec98281 terminal exit1，215.248890583s，result **537bdd936401ba2a998abe05ae90575a631ccddf89cfe22a8566ffb771e5c5c0**，whole/manual false保持。原run/launcher/observer字节不变，plan03仅5个命名空间数据键变化；270/285/300720/双盘5GiB/512/来源/cleanup门都保留。

原`business-oracles.json`完整成立：chat_put_paths seq76/term_drop seq95各唯一success，不发送消息/终端不回车；native drop3观察是两个真实拖动的广播记录。两处landed SHA **b32de19c93bcdd6330e2f945f35224d941975f235a080f98fa0d2fff51deeccd**；full压缩[false,true,false]；write1–6拒绝/write7成功；Not saved/Don't quit真实操作及20B保存一致。App47945实际CUA CmdQ正常exit0，launcher_sent_app_signal:false；accepted在app-exited后6.142836875s写下，早于launcher错误。

原失败是launcher在读取accepted前等所有观测App后代消失。该Shell的host.json/meta.json把50774/50775明确映射为本次创建的持久host/child；`docs/SESSION.md`109寿命、host.rs1–5/628、Node::open_persistent明确允许发起窗口离开后host继续。原门强于产品持久契约；不能称产品退出泄漏，也不声称原日志记录了10s末完整remaining名单。按用户“新独立问题下一轮”，登记harness契约纠正，不改原结果/断言、不要求用户重复拖放。非作者独审`P5/fullscreen-query-os-owned-run03-review01/report.json` SHA **97a42fdc03ba9a0f9c651e05f9513844c484f30c22c93f7f452d94ee50a413f4**；原902entry两盘逐SHA零缺件/差异/extras。最终cleanup无errors/remaining/footprint，TMP不存在；source/helpers/dist/bundle稳定、资源门无失败。

run01人工ChatView成功与fullscreen已证，原270超时整组false，独审a8dc670a…aaa；run02ready前资源门停止，result27afcfc4…040b、独审bf10bd3c…6167，无业务操作。全部历史失败保。

## 当前构建来源

- full2389 source frame **32101973afa2bdb0d2969154a0b4819a0cc275c7d04f34e5a06a8832e1277438**。
- normal GUI `.../caches/rust/khor-quality-target/debug/khor-gui` SHA **39b300263fe72763a437eedea7d14017674744ac79a7f74c7ec0498f995458e0**/374444376B。GUI producer40922 resultd159564b…522f原exit1仅Q055环境来源覆盖缺口；实际normal build/artifact/inputs/cleanup成功及生成ACL非作者独审ea482c96…0b63完成。没有伪造完整GUI accepted provenance。
- Core `debug/deps/chat_upload_cancel-a98ef2c294b70f0c` SHAd84f1167…ba1bbc8/75861904B；新source producer13238 exit0/result5a7fbeef…315，完整receipt **6644ec805d3c75a24bf87ecc25994f7f819d7a3b7ab1f09b1319bd86c76e6d50**，root实际full require和非作者独审9d8fdabc…b559通过。相同bytes来自实际新producer，不冒旧source收据。
- 五实际来源`P5/fullscreen-query-real-app-actual-producers01.json`；runtime重新校验。CLI e0a4d3e6…f69b06未改；旧收据仅历史。

## 明确留下一轮

1. 原040取消deadline；native四case中的late-close/close-reopen两channel closed，唯一race未证明。
2. Q055 GUI自动环境输入覆盖TAURI_CONFIG/REMOVE_UNUSED_COMMANDS；不冒完整proof。
3. 会话内/机器文件入口未向全局退出保护报告草稿；本轮已验全局WorkspacePane，不外推全部入口。
4. OS一次性harness把刻意持久terminal后代当全部必须随GUI退出，需按产品生命周期分别验证与fixture收尾。原结果不回填。

这些独立项按用户最新收口规则登记，不追加本轮实现或重跑已绿矩阵；没有因它们宣称全仓无缺陷。后续是否合并/发布或启动下一轮，由用户决定。

## 存储、恢复与继续指南

旧归档索引在历史及各独立archive index；原字节/模式/mtime恢复前旧fullmanifest只代表归档前事实，inode历史。当前normalGUI/Core/CLI/source不退役，没有global clean。run01/run02两份375MB stage逐SHA/lsof/独审后复用`fullscreen-current-gui-stage-archive01/current-gui-39b.zip`(SHA103530b1…214f)可逆退役，index01/02记录。run03当前完整双盘尚保留。

旧task-owned内盘incremental（无当前Core收据引用、current incremental0）11229files/4100120197B逐entry校验后压缩留`/Volumes/disk0/Khor/caches/archives/incremental-retired-20261028.zip`（正式发布期间逐SHA迁移，旧I路径为历史），SHAd15a2d88ee7eab5ac6f0aa60647fab25aca6335785feea70bb48965af63071d0/1710737465B；原目录退役，inventory/stat/report双盘`P5/fullscreen-old-incremental-retirement01`。实际释放约2.4GB，未动用户缓存或外盘currenttarget。

上下文恢复先读本页/PLAN、当前用户决定及实际refs/diff；外盘缺失读I/RESUME.md与current-code-quality-docs-manifest.json核内盘快照，禁止按摘要覆盖外盘未提交内容。此轮已收口，勿按历史live句柄重新启动App/Cargo/重跑矩阵；需后续任务时按新授权登记范围、所有者和证据。
