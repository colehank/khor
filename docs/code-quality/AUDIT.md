# 代码治理审计与批次

基线：main `8704a503a27b36ad58f922690de50e766f8bd485`，2026-10-25。
此表是首轮结构巡检和重点流程审查的结论，不是全仓逐行深审或所有功能已经通过。

## 当前处置索引（2026-10-28）

本节只记录已开始范围的当前处置；确切源码、证据与剩余验收读[STATE](STATE.md)。首轮巡检表及下面按时间记录保留历史，不代表当前尚未实施。

| 问题/范围 | 当前事实 | 边界与剩余 |
| --- | --- | --- |
| Q010/11/15 | 两项用户故障已修复集成；run11真实Tauri iOS键盘/历史锚点/连续commit跳转、Safari长稿占位与RO调度均有真红绿；后续按源码影响映射复用 | 不新增物理iPhone/IME矩阵，不把模拟器称真机 |
| Q020/020b/020c、Q021/021b | 自动保存、显式覆盖digest假冲突、锁刷新均已集成并有独立回归/源审 | 原失败证据不回填；并发失败分层记录 |
| Q022/023/023b | 身份原子首建、store/purge目标范围与真实owner已集成 | 不宣称断电目录项/Windows runtime验证 |
| Q025/036/032/040 | FIFO写完成/连接owner、输入预算、canonical cache/once弱锁回收均已集成；既定组合已执行 | 确切旧版本/当前版本覆盖分别见STATE |
| Q030/030b/031/038、Q037 | 脱敏、A2A caller隔离续聊、输入/事件预算、stderr drain、Pi/ACP/MCP/远端取消与退出已按已启动任务集成并完成既定组合 | 原040 deadline与062 Node Busy真实失败归下一轮；Windows runtime/真实SSH等不新增本轮矩阵 |
| Q033/034 | 当前协议、取消收据/GUI恢复、poll退避/gone均已集成；最终f763 GUI实际四图A取消→B重发Allow/EndTurn→C取消、mode/option和正常Quit已独立补验 | 原run05因观察服务143被一次性harness漏认而nongreen，42项独审补验另列，不伪造旧green；Q055另保缺口 |
| Q035 | native page owner/LRU已集成，模块及Chrome/WebKit行为已验 | 原四native已执行2pass/2fail（关闭查询channel closed候选下一轮），生产UI双owner仍待；首次Pair FinishPending保失败、一次现有Resume接线待执行 |
| Q039 | 统一当前Node/Doc/Pair分块、pending cache/compact数据保护与恢复已集成；真实大数据及bootstrap守卫证据与版本映射齐 | Node打开Busy按下文准入分类更正，收尾现有N1已改边界；fixture独立晚写等登记下一轮，不扩矩阵 |
| Q041 | 真实Darwin Pi bundle已编译/嵌入当前GUI，安装/镜像/来源入口已集成；normal GUI/core产物已完成 | Q055自动环境输入覆盖保持非绿；Pi账户页catalog不启动sidecar，真实账户管理被Credential home目录Busy阻塞，未有manage/runtime，提取/启动未验证；不做provider登录 |
| Q043 | native adapter/helpers及shared consumer异常隔离已集成；38项回归和真实七注册/撤销/child WK title证据齐 | 剩OS拖放、全屏值、window-close与最终App，不能用旧native Rust或browser fixture代替 |
| Q045 | 本地SSH deadline/I/O/reaped回执已修，源审/定向及既定组合按确切版本完成 | 真实SSH/master及remote lease不新增本轮前置 |
| P4/P5 | Clean Code已改边界结构收口完成并独审，机械格式216 Rust/共同入口单独提交；原115父全部执行（113过、2失败下一轮） | 正在完成上述既定真实App剩余流程，最后对齐任务卡和交付；main/ship未授权 |

没有把首轮文件清单当逐行深审，没有把局部回归或113通过称全仓全绿。

## 覆盖与证据

| 范围 | 已完成 | 实现深度及余项 |
|---|---|---|
| GUI / core / native | [525 文件清单](tasks/Q-001A-inventory.md)、[审计报告](tasks/Q-001A-report.md) | 42 文件深读/抽样，3 个模块运行探针；其余登记职责与后续范围；全部原生分支、终端和图表算法仍需按流程审查 |
| Rust / Pi / 协议 | [165 源文件清单](tasks/Q-001B-modules.md)、[审计报告](tasks/Q-001B-report.md) | 36 个关键路径文件、119 个结构巡检、9 个测试/fixture、1 个生成文件；150 个外置测试路径登记；发现为静态控制流证据，未冒充运行复现 |
| 验证 / 构建发布 | [审计与测试边界](tasks/Q-001C-audit.md) | 133 个 GUI script、131 个 Lab、7 shell、2 workflow 登记；115 MJS/7 shell语法、gates登记检查运行；mobile静态10失败、后续编译中断 |

图谱报告 HEAD 匹配，但节点缺失及源码陈旧已由三路审计交叉确认。图谱用于导航，当前源文件与运行证据决定结论；tests_for=0 不等于没有测试。

## 首批交付范围

| ID | 用户行为或风险 | 状态与证据 |
|---|---|---|
| Q-010 | 键盘弹起后最新消息不见；输入框增高引起尾部偏移 | 原始复现确认；共享滚动 owner 修复，ACP/Khor 分别验尾部和历史；[任务卡](tasks/Q-010.md) |
| Q-011 | commit 跳转短暂成功后回到顶部 | 原始复现确认：Radix 关闭时恢复顶部按钮焦点；改为选择后交给目标文件，取消维持原行为；[任务卡](tasks/Q-011.md) |
| Q-012 | 已有测试未覆盖变化过程与真实摘要契约 | [漏检分析](tasks/Q-001C-audit.md)，新场景独立运行、记录动画后稳定位置及源码指纹；旧 AI Elements 的返回按钮选择器已漂移 |
| Q-020 | 自动保存内容回环产生假冲突；丢弃后失败请求重写旧文本 | 原始模块探针及正式回归先红，独立修复；同时隔离不同编辑线队列；[任务卡](tasks/Q-020.md) |
| Q-013 | 可操作的本轮验收环境 | 生产 App/CommitPage 的隔离 fixture，窄屏可滚动，真实键盘全屏入口与原位模拟分别标记 |

这批没有改变模型协议、权限产品语义或已确认的会话导航设计。main/发布状态由 STATE 单独记录。

## 下一批建议顺序

以下均未在首批修复，不应因审计记录而标为完成。

| 顺序 | 对应发现 | 动作及完成标准 |
|---|---|---|
| 1 · 数据与身份保护 | GUI-A03；B-01；V-01/V-02 | 已锁定状态刷新失败保持锁定；identity 首建原子发布；卸载 dry-run 与共享 HOME purge 只操作目标 store。分别隔离回归、独立提交。锁首次未知的降级策略另列产品决定。 |
| 2 · 验证结果可信 | V-03/V-04/V-06 | netlab 传播失败退出码；定义PR最小交互检查；mobile静态普查与编译拆分，修正10项漂移并核对实际等待语义。不得用删除失败断言制造全绿。 |
| 3 · Agent 互通与可取消性 | B-02/B-03/B-05；GUI-A05 | A2A Message context 可续聊且隔离caller；空key安全脱敏；长轮询结束停止、失败退避；取消请求与执行结束有明确期限及状态。 |
| 4 · 生命周期所有权 | GUI-A04/R01；B-07 | 迟到的网页取得按当前owner释放；资源计数和旧owner竞态回归；once锁有边界地回收。再按owner提取 attachment/native-page/navigation模块。 |
| 5 · 协议与输入边界 | B-04/B-06 | 大于4 MiB同步缺口的统一当前协议恢复；认证JSON请求的大小、读取失败和非法JSON处理。按最新用户决定移除旧Khor兼容与双路径，先固定当前契约再实现。 |
| 6 · 可复现构建与分批结构整理 | V-05/V-07/V-08/V-09；GUI-R02；B-R8/R9 | 按功能拆大型测试；锁定/记录工具链与产物来源；镜像同release原子切换和checksum；Lab维护分类；随后按职责整理大入口和失效历史注释。 |

《Clean Code》的项目落点是意图、状态所有者、副作用和失败语义。此次两个滚动故障来自布局/用户意图及焦点所有权混淆，自动保存来自摘要/代次和取消所有权混淆。先把这些规则及回归建立起来，再按独立改变理由拆模块。函数行数与注释数量不作为成绩；必要平台、协议和并发约束保留，失效的实现历史迁入决策记录。

## 审查追加项

Q020非作者审查在父/子提交都复现“用我的覆盖”刚读到历史digest却被推进到旧head的假冲突（P2）。这是首轮提交时的既有显式覆盖入口问题，后续已在Q020c按FileView.mine与新pin来源契约修复并集成；原审查记录仍保留，不能让所有asked保存绕过compare-and-write。证据见 [Q020审查](tasks/Q-020-review.md)。

## 持续执行与上下文恢复

执行规则以 [PLAN](PLAN.md) 为准，活动事实以 [STATE](STATE.md) 为准；任务卡保存已确认事实、未证实假设、确切版本、验证、遗留风险和接手下一步。三个审计者轮换到实现与非作者审查，公共文件只有root写。集成后重新验证；浏览器、WebKit、模拟器系统键盘与真实设备 App 分开记录。


## 2026-10-27收口后仅记录下一轮

遵用户“开始了的做完，后面只记录不新增”，本节不派本轮新实现或测试专项。

| 条目 | 当前证据与边界 | 下一轮 |
| --- | --- | --- |
| Q055 GUI producer环境覆盖 | shutdown run02功能37与CLI/Node来源全过，normalTauri check0；自动verify拒绝未记录TAURI_CONFIG，实际build-script还声明REMOVE_UNUSED_COMMANDS，整run原非绿保留。不是产品运行红，也不是完整GUI来源证明。 | 下一轮统一显式环境输入覆盖，不本轮改helper或重跑已过37。 |
| N2既有normal devices计划协调候选 | candidate16 README已记无关normal plan与Pair grant可能需协调，尚未作为独立运行缺陷复现；当前保记录/fail closed，不安全自动删plan。当前Pair04采样为重复body解析，有自身既定最小修，不能据此宣称计划冲突。 | 下一轮按证据核并存/恢复语义，不扩当前Pair矩阵。 |

| source24原case11 setup Node重打开Busy候选 | dev diag17在pairing.rs492尚未Pair就Busy；recover与bootstrap gate都可返回该文案，未捕获唯一分支。runtime19不改父/代码，仅一次同产物重放完整通过。 | 下一轮核公共Node打开的并发准入契约；本轮不新建锁/owner治理或改原父重试。 |
| 原配对fixture setup失败后的晚写残留 | diag17原cleanup门非绿，2个私有usage文件在Scratch Drop后出现；进程全退，root另作dual归档/postcleanup，原非绿不回填。 | 下一轮整理fixture serve任务先停/join再删Root，保原业务断言。 |
| Q033原Fixture构造前setup owner gap | 与HEAD比对，原先已spawn host再等待ready，Self构造前panic缺本层owner guard；驻留新增部分不扩大该路径。 | 下一轮治理既有setup路径；当前最终App helper必须现owned-entry外层300s/PGID回收，setup失败不计绿。 |
| Node26前端工具TMP cache分类 | 原npm build0/99.915s、源码/锁稳，但受控TMP留250编译缓存文件，原frontend run保cleanup非绿；root按原stat/hash双盘ZIP归档、独立postcleanup后移除，原结果未改。 | 下一轮若统一工具清理策略再处理；本轮不改Q055/f7或重复构建。 |
| 取消状态即时查询偶发Unavailable | 21b1原case030在Retry已DurablyQueued后Query返回Unavailable；raw缺code。仅增强原断言diagnostic后同生产源码一次通过，未定位/未改产品，不据此回填原红。 | 下一轮若再现从新snapshot.code定位；本轮不反复重跑、猜测回滚或另开状态服务专项。 |
| 原取消siblings夹具将Node打开Busy提升为父失败 | run04原case038 peerErr=document transfer state busy，respond:334 Node::open?在answer→Refused前返回；生产IPC路径会封Refused，不退出resident。raw无具体gate，后续siblings强断言尚未执行。 | Node准入治理留下一轮。随后run05原039同一夹具提前失败；按PLAN既定验收必要接线，仅fixture将精确Busy封Refused，immutable home保全部身份/操作校验，其他错误硬失败、全部原测试体/强断言/时限保持。草案root与非作者独审通过，待apply/原038与039定向；两个旧run保失败不计pass，不加sleep/retry。 |

| 原取消重试deadline时间断言失败 | run06原040在remote_cancel_tests.rs1311 `run_starts.iter().skip(1).all(|(_,_,at)| *at >= due)`红，0新过/余72未跑。raw无at/due值，未定位产品/夹具/时钟原因；source/helpers稳/51dual/50组101owner全退/TMP清，原失败保。 | 遵用户边界下一轮定位，不加诊断/重试/改源码或时限；本轮继续其余既定72，不将该父计pass。 |

| 原normal serve重启夹具Node打开Busy | run07原062 helper在runtime_close_tests1051 Node::open unwrap Busy，父789 restart失败；raw无具体gate。63累计过/本父失败/余50未执行；57dual/source稳/99组402owner全退/TMP清。 | 归已有Node打开并发准入候选下一轮，不新增源码/诊断/重试，不将helper另计父或充pass。 |

| 真实GUI首次打开/多图上传遇Busy | Q033实App run03/run04首次open均显示document transfer state busy，UI Retry后成功；run03并发4图put有2 rejected，UI补齐后4图确实进入23MB Say。run04逐图put全部成功。 | 归已登记Node打开准入下一轮；不以手动补齐或单图路径声称并发多图已修，不扩本轮产品实现。 |


## 2026-10-28 已改边界的Clean Code收口

此节是原任务的代码质量准则，不是新增功能专项。以下均为内部整理并已独审；既定115父现已全部执行，113过、2个新失败记录下一轮，确切版本映射见STATE。Q033当前协议真实App及正常Quit已完成并独审；Q035/Q043仍待，不把fixture结构整理称整个产品架构重构。

| 范围 | 具体理解/维护成本 | 收口与保留约束 |
| --- | --- | --- |
| Pair settlement | 平铺参数与dialed布尔让调用点看不出邀请双方角色 | runtime请求结构与Issuer/Joiner；不改serialized dialed、journal布局或原settlement副作用顺序 |
| Pair/normal边界 | 废弃devices_only分支仍暗示错误Pair scope；固定base规则两处重复 | 删无调用支、共享纯谓词；保两处enforcement/各错误上下文和exact operation helpers |
| R2 input取消 | discover/collect重复input模式规则，sticky merge的业务意图隐藏 | 命名纯getter/merge；publish/锁/re-read与严格拒绝分支保 |
| Pi/ACP setup | 两Job重复解释相同JSON终态 | 私有只读终态规则；wire字符串、取消专用判断、native join/资源owner分离保 |
| Q033 fixture | session初始化同时承担relay线程/socket/gate细节 | UploadRelay集中网络资源归属与有限join、for_app构造；业务断言/时限/网络块保；Drop对owned poisoned Vec可清理，worker/join错误不转绿 |
| P4检查入口 | import阶段解析CLI并可执行工具/写源，隐式root全局 | main入口、显式root；带--write受限import探针无副作用，既有执行序列可逆恢复 |

不以文件数、行数、注释数量作为质量分；必要平台/协议/并发约束保留，独立新缺陷仍归上一节下一轮清单。

### 下一轮记录：Vite受控TERM退出码

Q033 run05全部实际UI与业务及正常App/helper退出已完成，但现有一次性launcher只认observer0/-15，当前Vite SIGTERM handler server.close后以128+15=143退出，原运行保nongreen。来源与原未到达末尾42项独立补验见双盘P5/Q033-real-app-run05-postreview01，原result/launcher未改，当前产品无新增实现。未来维护该运行入口时统一受控TERM语义；本轮不重跑已完成UI或扩建工具治理。

### 2026-10-28 真实P5新失败记录（不新增本轮产品实现）

| 观察 | 当前证据 | 处置与边界 |
| --- | --- | --- |
| native page_steps关闭交接返回channel closed | Q035 native run02 late-close、run03 close-reopen两个原父失败；资源最终null，后者新owner中间Some。源码支持get_webview与排队WithWebview之间Close丢回调/oneshot候选，但raw无唯一失败invoke/调度序列。 | 下一轮核native查询/关闭语义，不改断言/重跑失败父。Strict与cap两原父通过，原四2pass2fail0unrun，不能称全绿。独审46be305a…808a。 |
| Pair FinishPending阻断真实双peer setup | Q035 borrow run01真实Pair返回精确pending，原d1d338…abf9保失败，未UI。不是Node首次bootstrap未完成；无现有外部Finish-ready状态。 | 仅已存在的同ticket/savedintent Resume可作真实恢复流程，plan02最多一次且共享70s，其他错误不纳入，不改产品或新grant。 |
| 模型账户目录Busy阻断embedded Pi UI | Q043 Pi run01三次打开账户管理均Credential home无候选、Connect disabled；实际只有read/catalog，零manage/secret/runtime。原ad3f9…5417保非绿；App真实CmdQ return0，observer失败路径回收。 | 按下文准入分类更正纳入现有N1改动边界的窄审；提取/启动unverified，不从redacted reject推断具体原因。独审33项证据检查不是业务测试数，2bb4e6…190d。 |

### 2026-10-28 准入分类更正

源码确认main8704已有GUI每API Node::open；本轮14931新增RootGate/try-lock、21b1延长构造fence，故不能把当前GUI Busy一概记为main既有独立缺陷。归现有N1已改边界的回归/契约缺口候选，先窄设计/原target红证据，再必要纠正；唯一Busy gate/owner仍未知，不能猜测21b1单独责任。此不撤销“新独立问题下一轮”，也不重启全仓专项。新卡Q-039-node-open-admission归现有N1集成收尾。OS default capability与main逐字同，fullscreen缺口则保持既有候选下一轮。

OS run01最终b4a3…f831非绿/helper原270s期限；0 drag/fullscreen回调/guard write、无正常Quit。外盘DS_Store/UI描述晚写分别保全，独审45dfed…e048不冒当前双盘全树绿。borrow run02一次既有Resume成功后在脚本按id rename处失败；plan03仅修准备参数顺序，未执行。


2026-10-28后续可信红：test-only a663b374在原libtest单父实际持same-root控制lease，另一线程正常Node打开立即返回Busy并触发业务断言（非setup/编译红）；原result8df725e9…2bac保nongreen、113.900s/来源清理与20dual过。这证明已改准入的普通打开冲突契约缺口，不定位Pi/OS唯一gate。按既有N1授权窄实现，候选待交付；具体设计与非作者审查见STATE/Q039卡，不把静态设计或红算修复完成。


2026-10-28 OS分类补足：不能仅因ACL/conf与main相同就把未观察到fullscreen回调整体排出本轮。非作者核实main已有同挂载/resize时机win.isFullscreen与同SDK权限需求；本轮listenNativeEvent生命周期确有修改，仍须原真实App验收。默认权限继承已实际展开：core默认集合包含查询，但项目capability均未引用其default或具体grant，所以静态缺授权候选成立；未捕实际SDK拒绝原因，不能归唯一原因。此为已开始验收边界的只读分类纠正，未加权限/运行/探针；独立旧授权问题继续按用户边界记录下一轮。
