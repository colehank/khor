# 本轮代码治理：当前执行状态

更新：2026-10-29。用户已授权“开始吧”，目标 **完成第二轮** active；28条反馈＋5遗留共33项。**本轮main合并/ship未授权**。main仍e7c335c8干净；当前集成树 `/Volumes/disk0/Khor/worktrees/current/khor-round2`，分支 integrate/round2，HEAD **46f03ca5**。旧状态已存[历史](history/STATE-before-round2-867c1446-20261029.md)，其中旧PID/权限/待实施语句不代表当前现场。

## 恢复后先做

1. 读 [ROUND-2-EXECUTION](ROUND-2-EXECUTION.md) 与当前 [A](tasks/R2-A.md)、[G](tasks/R2-G.md)、[REVIEW](tasks/R2-REVIEW.md) 卡；核实际refs/diff/agents/进程，不按摘要覆盖外盘内容。
2. 当前两个作者：round2_history在khor-round2-history/fix/round2-history做A2收口；round2_input在khor-round2-content/fix/round2-content做G剩余固定标题。root集成/总账，round2_reviewer只读独审；无第三实现线。
3. **native槽当前已释放**：A2 final-1和补编译closure-2全部真实进程终态，由作者明确释放。旧running.json/PID不是仍运行的证明；新批先核现场再授槽。保持共享target、jobs1/incremental0、外盘TMPDIR；不重启用户App或真实session。
4. 下一动作：A2已固定3ff8013c并集成6f103d9b；G ContentToolbar/TermPeek已集成46f03ca5，补齐SessionSide与只读CodeDiff既有固定标签后交W。A3a已明确派发最小writer/事务及读链，A4排队；不把A2当全部ACP观看完成。

## 最新进展

**A2：固定提交3ff8013c，独审后集成6f103d9b，切片验证已过。** 39路径实际git-show与固定manifest匹配，最新源码独审已解除“自有host授权借给foreign view”的阻断：agent TUI仅可连经证明自有PTY，GUI走ACP；普通明确shell/tmux资源保既有交互。观察alias独立observe身份，状态真None、删除只删本地记录、控制入口拒alias；vendor opaque nativeID不按字符串前缀误禁，显式接管仍独立动作。

A2 `history/observation-a2-check/final-1/result.json`：三lib check、最新9项原生测试、空staging真实bindings/catalog全部exit0，163.322s，作者Rust/catalog/Cargo输入前后不变。三处nullable胶水后15JS/types/mobile-static/words绿、源码不变；None不计busy/blocked，也不触发busy已结束的刷新。补编译closure-2四命令CLI+web、Node三个已改integration tests、gui seen_snapshot、sync/commands tests全exit0/28.539s；首批缺dist setup失败保留，随后真实Vite建277文件/15,154,979字节再检查，不伪造资产。Tauri仍待V/最终App准备。

集成真实产物161 bindings＋catalog逐SHA核复制，输入核对为**327相同＋1已审差异**：web.rs仅既有C1 system_target/open_link函数体/测试/注释，WebBorrow与gen根/api签名不变；保两SHA和完整patch，不谎称328全同。初次严格核验在写入前停止，补独审确认后才同步。集成15观察JS/types/words过；生成收据integration/generated-6f103d9b.json。SessionSide现已明确交给G，A3a现已派发，A4内容home只读网络路由未开始。A3a唯一journal writer位于gui_host单loop，首次bootstrap pending+消费fence后原子base；已有canonical时replay只运行时回放，不覆盖/重复append/版本堆积。保accepted文字/图片/文件与turn边界；实际New/Load/Resume开源事实由acp Handle叶字段提供，Ready或forwarder fence均不冒完整历史，宿主须消费完有限Notes。覆盖范围如实，离线外部段不文本hash伪融合。具体paths/两小提交与预算见R2-A；新增native批尚未授权。

**G：主材质、CommitPage、Machine、R013菜单及工具条/TermPeek已集成，剩SessionSide及CodeDiff固定标签。** 主band14e0cc6d、局部/Commit250d64ed、Machine/frontier535c3fb5、路径菜单f6755103、规则/既有门867c1446、工具条/TermPeek46f03ca5。固定源码均独审无阻断，集成owner13/jump4/字段13/types绿。Machine实际宽窄/列轨/物理sticky边界/排序/普通兄弟fade5项绿，未新建内滚动。R013实际12项绿，修的是工具条菜单错误内联/被正文挡点击：显式overlay与stack；原表单adaptive和网页禁推荐保留。

G既有scroll-surfaces门36项真实全绿。Face精确sticky段原2rAF采样opacity0.815295失败保留；真实同held 0/100/240ms为0.815758/0.999314/精确1，几何不变，180ms产品动效未改。仅门改有界等待终态，exact1/flush/排序等保障保留，精确段绿，**不称完整Face套件绿**。7路径规则/门指纹与收据一致，集成17定向/语法/types通过。

新消费者547a9496→46f03ca5：ContentToolbar对DOM末位/CSS order的landing用显式locateUnder，默认邻接不变、同owner/observer与仅自身under marker清理；TermPeek真实tbody/表头消费共用材质。14受控checks、实际extras12与原SessionContentPane菜单/脏CodeMirror同DOM宽窄2项绿；tbody mask的实际截图像素差证明有paint，非只computed attrs。初Lab缺CSS和CM fill假设两harness失败保留，Lab/生产分提交，独审无阻断；集成owner14/字段13/types均终态绿（integration/checks-46f03ca5.json）。

G当前剩SessionSide与CodeDiff：固定A2单文件基线后只做SideBody固定头小接线，避免回退nullable。SeatHead在SideWorkspace普通滚动流内不是固定组块，排除不改；底部输入/任务操作、横轴sticky及原生网页正文亦不冒作DOM固定标题处理。原生WebView的empty端仅DOM占位尺寸验证，未冒真实WK材质。CodeDiff盘点漏项已查依赖：wide实际单cm-mergeView口、两个cm-scroller为visible，不造双滚动框架；root仅授固定label行band及真实宽窄/位置基线验收，不改editor状态，narrow不新增标题。完成这两项封G后轮换W，已验参考与正式缺口在R2-W。

## 全轮完成边界

| 包 | 当前状态 |
| --- | --- |
| E 输入/草稿 | 刷新迟到、merge接纳、选区退出/搜索清理、草稿owner计数已集成；真实浏览器8路径/网址拖选及字段ShiftLeft过。Mac真实IME、用户回复框ShiftLeft、最后按键→Quit未验。 |
| H 长历史 | 有界页/记录/缓存与取消lease、真实ChatView消费及旧全量RPC清理已集成。20万逻辑记录Lab13项绿；真实后端source/raw另有定向证据，最终原生组合未验。 |
| C 打开/导航 | 普通内部/Cmd系统、软折行及CJK末列占位修复、返回最新cwd且保脏编辑器已集成。终端19＋CJK4、当前工作区实际4流程绿；原生组合未验。 |
| A/G | 如上，仍working，不能整包标done。 |
| W 工作区 | [R2-W](tasks/R2-W.md) queued；最终已验参考c5010294，正式布局/真实完整Git分页/实时更新未接入。 |
| S 恢复/生命周期 | 尚未实施本轮完整恢复、取消deadline/native close遗留。 |
| B 浏览器 | 尚未实施；R016旧WK拒绝根因未知，当前CLI双HTTPS200不证明WK已修，不加猜测ATS/权限例外。 |
| M 移动 | 跟手参考97101142、自定义键行待正式接入/真机验收。 |
| D/N | 旧卡死和transfer busy未解决；H绿不等于旧卡死根因已证明，按既定有界调查处理。 |
| V 工具遗留 | [R2-V](tasks/R2-V.md) queued；TAURI_CONFIG/REMOVE_UNUSED_COMMANDS来源输入与持久宿主退出harness最小修正须在首次相关native App验收前插现作者槽。 |

逐项范围/完成条件以ROUND-2-EXECUTION与各卡为准，不按提交数计算比例。新增独立问题记后续轮次，既定33项缺口继续完成。Clean Code审查逐片段核状态/资源owner、命名、错误/取消和旧实现移除；格式量或零注释不是成绩。

## 证据与恢复集

证据根 `/Volumes/disk0/Khor/evidence/round2`：input/browser-fields、content/browser、history分阶段、glass/browser及integration各固定SHA收据。原失败/harness错误均保留，不回填旧报告。G完整范围见glass/gates-receipt.json与browser/verification-scope.json。

内盘恢复 `/Users/zgh/Library/Caches/khor-code-quality/recovery`，最新已核包 round2-f6755103.bundle，SHA33c7264fff68b9d30b23f6917ce789660481798dee3d196f71177a0a5dc77a9e，快照round2-source-checkpoint-f6755103含A2/G当时实际diff/新增文件/真实生成类型与202文档逐SHA镜像；后续867/A2新checkpoint继续更新，不按旧包覆盖现场。外盘不在先读内盘RESUME及manifest。

隔离Labs沿原owner保留：23832参考、23834字段、23835内容/材质、23836历史，PID以实际现场为准。API受控，不操作真实文件/发送消息；浏览器证据不冒WK原生材质/真机触屏。当前没有新全App构建或用户App重启。上轮交付见[DELIVERY](DELIVERY.md)。
