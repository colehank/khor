# 本轮代码治理：当前执行状态

更新：2026-10-29。用户已授权“开始吧”，目标 **完成第二轮** active；29条反馈＋5遗留共34项。**本轮main合并/ship未授权**。main仍e7c335c8干净；当前集成树 `/Volumes/disk0/Khor/worktrees/current/khor-round2`，分支 integrate/round2，HEAD **d4232d11**。旧状态已存[历史](history/STATE-before-round2-867c1446-20261029.md)，其中旧PID/权限/待实施语句不代表当前现场。

## 现场覆盖更新（2026-10-29，本次状态核对）

本节覆盖下方尚未刷新段落中的旧HEAD、作者任务和native槽。用户追加Markdown括号公式支持，范围为**34项（29条反馈＋5遗留）**。实际集成HEAD **0a885e25**，工作树干净；main仍e7c335c8、无本轮合并或ship。

- W首片段3a9ae447已集成：跨工作区最后编辑、草稿计数和源码/预览阅读位置保留。完整布局、Git分页和实时更新未完成。
- A3两片段d4232d11、540ea302已集成：统一ACP持久捕获、分页读取与覆盖边界。A4实际内容home/远程只读路由正在实现，首slice候选2为31项定向测试计划，尚未运行Cargo；等待独审复核。完整恢复S尚未完成。
- V f964cf9d、0a885e25已集成。第三批实际catalog/bindings生成、types/Vite、CLI和指定原生测试编译均过；GUI编译成功但严格来源验证因capabilities目录未纳入输入而停止。未运行新App或真实退出验收。作者已交还唯一native槽，仅做两文件目录输入窄修：28项Python测试绿、待独审固定。三次原失败均保留，不能称GUI已验。
- 新增R2-029由root负责：已定位现有括号公式转换仅用于会话、文件入口绕过且不加载纯括号公式数学模块，尚未修改代码。
- 调度为两作者（A4；V窄修后回W）＋root协调/小公式补丁＋只读独审。native目前空闲，下一批须重新明确授槽并核实际进程；不得依据下方旧running/PID重启。

剩余仍包括W后续、A4/S、B、M、H源搜索/复制、E真实Mac验收与其他入口退出计数、D/N调查及最终组合验收。最近对用户估计为**约10–16小时有效工程时间，另加真实设备验收**，不是完成承诺。内盘5.1GiB/外盘17GiB；native保5GiB floor，不删用户或共享缓存。

## 恢复后先做

1. 读 [ROUND-2-EXECUTION](ROUND-2-EXECUTION.md) 与当前 [A](tasks/R2-A.md)、[G](tasks/R2-G.md)、[REVIEW](tasks/R2-REVIEW.md) 卡；核实际refs/diff/agents/进程，不按摘要覆盖外盘内容。
2. 当前两个作者：round2_history在khor-round2-history/fix/round2-history做A3a统一journal；round2_input已转khor-round2-workspace/fix/round2-workspace做W首片段，原C树/G Labs冻结保留。root集成/总账，round2_reviewer只读独审；无第三实现线。
3. **native槽由A3 vertical批占用**：首writer8a7ae1aa→d4232d11的6IO已过且独审。第二片段host/read/coverage十五路径已接但未提交；vertical-1两lib检查101/14.265s/源不变，Native/Journal身份方法迁移和未知opening match两编译错误已窄修，原日志保留、未到测试/gen。vertical-2沿原31测试+真实gen串行批执行，当前阶段/PID以history/observation-a3-check/vertical-2/running.json和真实ps为准，root不重启/并发编译。作者exec句柄不同命名空间不要接管；终态看result+实际进程。旧running.json/PID不是仍运行的证明；新批先核现场再授槽。保持共享target、jobs1/incremental0、外盘TMPDIR；不重启用户App或真实session。
4. 下一动作：A2已固定3ff8013c并集成6f103d9b；G最后SessionSide/CodeDiff eaaf7c39→b84187b7已独审集成，作者正式转W，准备树已ff同HEAD。A3a已明确派发最小writer/事务及读链，A4排队；不把A2当全部ACP观看完成。

## 最新进展

**A2：固定提交3ff8013c，独审后集成6f103d9b，切片验证已过。** 39路径实际git-show与固定manifest匹配，最新源码独审已解除“自有host授权借给foreign view”的阻断：agent TUI仅可连经证明自有PTY，GUI走ACP；普通明确shell/tmux资源保既有交互。观察alias独立observe身份，状态真None、删除只删本地记录、控制入口拒alias；vendor opaque nativeID不按字符串前缀误禁，显式接管仍独立动作。

A2 `history/observation-a2-check/final-1/result.json`：三lib check、最新9项原生测试、空staging真实bindings/catalog全部exit0，163.322s，作者Rust/catalog/Cargo输入前后不变。三处nullable胶水后15JS/types/mobile-static/words绿、源码不变；None不计busy/blocked，也不触发busy已结束的刷新。补编译closure-2四命令CLI+web、Node三个已改integration tests、gui seen_snapshot、sync/commands tests全exit0/28.539s；首批缺dist setup失败保留，随后真实Vite建277文件/15,154,979字节再检查，不伪造资产。Tauri仍待V/最终App准备。

集成真实产物161 bindings＋catalog逐SHA核复制，输入核对为**327相同＋1已审差异**：web.rs仅既有C1 system_target/open_link函数体/测试/注释，WebBorrow与gen根/api签名不变；保两SHA和完整patch，不谎称328全同。初次严格核验在写入前停止，补独审确认后才同步。集成15观察JS/types/words过；生成收据integration/generated-6f103d9b.json。SessionSide现已明确交给G，A3a现已派发，A4内容home只读网络路由未开始。A3a唯一journal writer位于gui_host单loop，首次bootstrap pending+消费fence后原子base；已有canonical时replay只运行时回放，不覆盖/重复append/版本堆积。保accepted文字/图片/文件与turn边界；实际New/Load/Resume开源事实由acp Handle叶字段提供，Ready或forwarder fence均不冒完整历史，宿主须消费完有限Notes。覆盖范围如实，离线外部段不文本hash伪融合。具体paths/两小提交与预算见R2-A；A3a当前writer首片段6IO已授；原错误加入pending总32MiB cap在运行前已挡并撤销，只保64KiB缓冲/16KiB manifest与真实IO失败，不收窄总历史。commit_base任一IO失败必须poison writer；Prompt到manifest间crash读侧须按缺completion保unknown，不能靠manifest推完整。

**G：主材质、CommitPage、Machine、R013菜单及工具条/TermPeek已集成，全部既定DOM消费者实现已封口；原生组合验收未完成。** 主band14e0cc6d、局部/Commit250d64ed、Machine/frontier535c3fb5、路径菜单f6755103、规则/既有门867c1446、工具条/TermPeek46f03ca5。固定源码均独审无阻断，集成owner13/jump4/字段13/types绿。Machine实际宽窄/列轨/物理sticky边界/排序/普通兄弟fade5项绿，未新建内滚动。R013实际12项绿，修的是工具条菜单错误内联/被正文挡点击：显式overlay与stack；原表单adaptive和网页禁推荐保留。

G既有scroll-surfaces门36项真实全绿。Face精确sticky段原2rAF采样opacity0.815295失败保留；真实同held 0/100/240ms为0.815758/0.999314/精确1，几何不变，180ms产品动效未改。仅门改有界等待终态，exact1/flush/排序等保障保留，精确段绿，**不称完整Face套件绿**。7路径规则/门指纹与收据一致，集成17定向/语法/types通过。

新消费者547a9496→46f03ca5：ContentToolbar对DOM末位/CSS order的landing用显式locateUnder，默认邻接不变、同owner/observer与仅自身under marker清理；TermPeek真实tbody/表头消费共用材质。14受控checks、实际extras12与原SessionContentPane菜单/脏CodeMirror同DOM宽窄2项绿；tbody mask的实际截图像素差证明有paint，非只computed attrs。初Lab缺CSS和CM fill假设两harness失败保留，Lab/生产分提交，独审无阻断；集成owner14/字段13/types均终态绿（integration/checks-46f03ca5.json）。

G最后eaaf7c39→b84187b7仅CodeDiff/SessionSide两小hunk，独审固定指纹/真实6+inner-owner1无阻断。CodeDiff实际单cm-mergeView有效口、内部两口无独立range；去绘制border后用既有hair padding保原头高23px/range1519/两列track，窄屏无新头、回宽唯一owner。SessionSide保A2 nullable，集成按A2源加精确import/header hunk核，不复制旧整文件。集成types已终态绿，收据integration/checks-b84187b7.json。

完整消费者适用/排除见glass/CONSUMERS.md；G不再扩普通标题。SeatHead流内头、底部输入/任务操作、横轴sticky与原生网页正文均排除有据。23835/PID92646为旧owner的冻结隔离Lab，按真实进程核，不重启。W首片段已正式派发：App工作区选择/显示与WorkspacePane retained UI/必要新workspace模块、真实四态验证；Git backend/生成器尚未授，与A3共享路径进入时再协调。W卡由作者维护，已验参考c5010294不整支Lab复制。

W首片段已进入真实验证（未固定/未整包完成）：23837/PID76388/exec46226隔离Lab，真实Pane/CM/model，API未知400/写409，不代表App整壳已验。最后编辑+同批切workspace原丢17字已红绿修复，真实逐键输入恢复且无React/CM循环；FileView现onDraft同步交owner，共用dirty判定，不改revision/composition。model1而Quit registry0也实际红，改model生命周期同步订阅后probe1/1，不冒真实OS Quit；E其它三入口仍layout count风险待实测，已记R2-E，不打断本片段也不静默移到下一轮。CM原700→0阅读red经现slot scrollSnapshot/scrollTo修到700，保doc/selection；preview与树仍在验证，空reading Map已移除不冒完成。树placeKey device/path隔离，Markdown preview需真实red/既有有界恢复核心。当前剩Undo/selection/preview及集成App路由等证据按R2-W。

调度：W当前首片段固定后，Git大接线前短插既定V L04/L05准备，便于尽早真实Mac E验收；不新增第三实现线。用户问剩余与时间，已按现范围估计还需8–12小时实现/独审/自动验证，设备验收另列，未知D/B拒绝/N根因不假承诺复现。此为工程估计，不是缩范围/新增预算或完成声明。


## 全轮完成边界

| 包 | 当前状态 |
| --- | --- |
| E 输入/草稿 | 刷新迟到、merge接纳、选区退出/搜索清理、草稿owner计数已集成；真实浏览器8路径/网址拖选及字段ShiftLeft过。Mac真实IME、用户回复框ShiftLeft、最后按键→Quit未验。 |
| H 长历史 | 有界页/记录/缓存与取消lease、真实ChatView消费及旧全量RPC清理已集成。20万逻辑记录Lab13项绿；真实后端source/raw另有定向证据，最终原生组合未验。 |
| C 打开/导航 | 普通内部/Cmd系统、软折行及CJK末列占位修复、返回最新cwd且保脏编辑器已集成。终端19＋CJK4、当前工作区实际4流程绿；原生组合未验。 |
| A/G | A继续writer/read链；G实现封口，原生/组合证据仍未齐，不能整包标done。 |
| W 工作区 | [R2-W](tasks/R2-W.md) working首片段；最终参考c5010294，作者已接手正式选择/保留状态与布局；完整Git分页/实时更新尚未接入。 |
| S 恢复/生命周期 | 尚未实施本轮完整恢复、取消deadline/native close遗留。 |
| B 浏览器 | 尚未实施；R016旧WK拒绝根因未知，当前CLI双HTTPS200不证明WK已修，不加猜测ATS/权限例外。 |
| M 移动 | 跟手参考97101142、自定义键行待正式接入/真机验收。 |
| D/N | 旧卡死和transfer busy未解决；H绿不等于旧卡死根因已证明。[R2-N](tasks/R2-N.md)已完成一次源定位、列多个Busy入口但未证原现场，待私有并发夹具，不加第三实现线。 |
| V 工具遗留 | [R2-V](tasks/R2-V.md) queued；TAURI_CONFIG/REMOVE_UNUSED_COMMANDS来源输入与持久宿主退出harness最小修正须在首次相关native App验收前插现作者槽。 |

逐项范围/完成条件以ROUND-2-EXECUTION与各卡为准，不按提交数计算比例。新增独立问题记后续轮次，既定33项缺口继续完成。Clean Code审查逐片段核状态/资源owner、命名、错误/取消和旧实现移除；格式量或零注释不是成绩。

## 证据与恢复集

证据根 `/Volumes/disk0/Khor/evidence/round2`：input/browser-fields、content/browser、history分阶段、glass/browser及integration各固定SHA收据。原失败/harness错误均保留，不回填旧报告。G完整范围见glass/gates-receipt.json与browser/verification-scope.json。

内盘恢复 `/Users/zgh/Library/Caches/khor-code-quality/recovery`，最新已核包 round2-d4232d11.bundle，SHA4d2fabf88efb62631a4fc6584743288d51fc3648837fa40cd75f58357faa9b93，快照round2-source-checkpoint-d4232d11含A/W/G当时实际diff/新增文件/真实生成类型与205文档逐SHA镜像；后续checkpoint继续更新，不按旧包覆盖现场。外盘不在先读内盘RESUME及manifest。

隔离Labs沿原owner保留：23832参考、23834字段、23835内容/材质、23836历史，PID以实际现场为准。API受控，不操作真实文件/发送消息；浏览器证据不冒WK原生材质/真机触屏。当前没有新全App构建或用户App重启。上轮交付见[DELIVERY](DELIVERY.md)。
