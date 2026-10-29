# 第二轮：当前执行状态

更新：2026-10-29。目标 **完成第二轮** active，范围 **29条反馈＋5遗留，共34项**。用户已授权实施；**本轮 main 合并/ship 未授权**。main `/Users/zgh/Desktop/working/khor` 保持 e7c335c8。最新集成 `/Volumes/disk0/Khor/worktrees/current/khor-round2`，`integrate/round2`，**34f84dcb**，tracked clean。具体旧批次/原失败见[归档](history/STATE-before-round2-4cf-sync-20261029.md)及任务卡；旧“当前运行”句不再代表现场。

## 恢复入口与调度

1. 读本文、[执行计划](ROUND-2-EXECUTION.md)、当前任务卡，再核实际 refs/diff/agent/进程；不按摘要覆盖外盘未提交源。`.codegraph` 无索引时跳过；code-review-graph 可能落后当前 branch，零覆盖后才定点读源。
2. 两实现线：`round2_history` 在 history 树做 A4 Core/GUI 接线；`round2_input` 在新 git 树做 W 全分支历史分页。`round2_reviewer` 固定版本只读审查。root 负责集成/恢复，并做小 E 同步计数补丁，不另开大型实现线。
3. **唯一 native/Cargo 槽当前已释放**；W Git red/green均终态，A4 shared-1 严格失败终态101/96.76s；Core+Tauri lib check0，command_tables1/2（旧finder漏跨行or-pattern），尚未gen。A4桥bridge-1 已终态0/83.10s，无本任务 App 实例/待用户操作。用户现有 release/khor-gui 进程不能误分类成私有验收实例，不触碰。复用槽先核实际终态；预检非0必须停止后续启动，不以“未碰用户进程”掩盖启动编排失败。
4. native 共用 target `/Volumes/disk0/Khor/caches/rust/khor-quality-target`，jobs1/incremental0、外盘专用 TMP、每阶段5GiB floor。不得结束用户 App、真实会话，或清共享缓存。
5. shared `core/api.rs`/`core/lib.rs`/Tauri/生成导出由 A4 唯一 owner 接 W DTO，接口稳定后一次真实 gen。现 Root gen 为 ca83 真产物；不能手写新绑定或把过渡 types 通过冒新 shape 已生成。

## 已实现、证据与未完成

| 包 | 当前证据 | 剩余工作 |
| --- | --- | --- |
| E 输入/草稿 | 迟到刷新/merge拒收、字段选择和搜索退出定向绿；W 编辑接纳与同步计数已验。root a0c34d13→2dc9d635 三集合owner同步登记：旧源3个首编辑前commit漏报，修后6/6、旧count9/9、types0，独审核准已集成 | standalone首键计数72591cae→d4dcc1a5已独审，8同步/9计数/8刷新/9merge/types绿。accepted base/flush三实际回调反例已红，待修；真实Mac IME、ShiftLeft、最后按键→正常Quit组合 |
| H 长历史 | 有界IO/页/记录/缓存/取消；实际ChatView 20万逻辑记录等定向证据 | source全文搜索/复制缺口、最终原生组合；loaded DOM不冒完整源搜索 |
| C 内容打开/导航 | 点击内部/Cmd系统直接分流；软折行/CJK路径；返回最新cwd保草稿；定向真实浏览器及终端绿 | 原生组合验收 |
| G 材质 | 既定DOM消费者封口，消费者/排除详glass/CONSUMERS.md；菜单/表格/CodeDiff/SessionSide证据 | 三种窗口材质、深浅色和原生/触屏组合 |
| W 工作区 | 0f5a75af→3f16f146，正式布局/独立commit/fullhash身份/shared preview/pin/dirty Undo/单菜单；最后真实FullScreen窄屏6组绿。集成tabs6/memory5/types0 | Git分页c6b0c766→7bfc5164已独审集成，旧merge4/6及260→200真实红，新Node9绿；Core mapper check0。UI分页消费正在实现，真实watch/相关旧Face与原生组合待做 |
| A 外部观看 | A1/A2身份能力、A3 journal capture已集成；A4服务ced98d3b→4cf7cb91，生产check+12绿，真实双Node含取消/断线/版本/响应上限 | Core/GUI binding41ed240b→26dda9c2已独审集成，parent-key cde0bf01→34f84dcb已修owner-only更新重挂；shared实际Core/Tauri check0，真实gen尚未运行；全能力边界与组合待完 |
| S 恢复/生命周期 | 基础链及持久宿主认证重连证据 | 完整GUI/terminal恢复、取消deadline、close/reopen竞态、startup forwarding有界性 |
| B 浏览器 | 已只读调查；真实WK拒绝根因仍未知 | 五项正式实现与原生/mobile验收；CLI200不冒WK修复 |
| M 移动 | 已验连续跟手/键行参考已保存 | 正式跟手接入、自定义键行及触屏验收 |
| D/N | 旧卡死未知；Busy已定位多个入口但未复现归因 | 限定调查/并发夹具、明确结果，不能因H绿关闭D |
| V 工具 | **L04/L05 done**：ca83 GUI/CLI/core actual+strict绿；focused L05真实正常退出、认证replay、私有清理绿，独审核准 | 不替代E/S或全轮验收，旧270s失败保留 |
| R2-029 | ecf36408→b338ced2；共享括号公式转换，保护code/escape，MD gate+9文件+7合同/types0，独审 | 原生文件渲染组合 |

## 当前固定版本与下一动作

- Root集成 34f84dcb；W正式源0f5a75af、Lab2d85a50d独立，Lab不进生产。A4固定Node服务ced98d3b，真实417输入前后同，transport-4 90.773s全绿；原编译/夹具失败不回填。
- A4 history树固定41ed桥已集成26dda9c2；metadata owner更新不重挂cde父key已集成34f84dcb。新增locator/binding按真实content home固定。shared4注册＋W mapper生产Core/Tauri check已真0；command_tables旧逐行finder漏7个既有跨行or-pattern命令，1/2失败停、未gen，源码前后892输入同。已批准仅测试finder修正+反例，不能加ONE_SIDED；固定独审后定向tables→真实bindings/catalog，不重无变化lib check。GUI Lab四路径另授，待gen源stable后准备新端口，23837已有W旧Lab。
- W Git作者树固定c6b0c766，Node red2（非编译失败）/green9实际0、源前后同，ownedTMP/子进程清理，独审核准已集成7bfc5164。新RepositoryLog独立旧path log，完整all-parent snapshot、40/100页、JSON≤256KiB、shallow明确不完整；Core mapper后续shared check已0。下一UI消费获准：正式平面log行/日期链＋末尾较新/更早轻导航，固定page0＋7页LRU≤2MiB payload，不保无界cursor链；淘汰回访从可用前页/首page.next重走，披露深页成本。显式刷新才重建snapshot、失败保旧页可retry，返回保页/scroll；不冒全branch搜索。watch后片未做。
- root E复用math树，fix/round2-draft-sync。a0三集合→2dc独审/集成；72591standalone→d4独审/集成。真实回调defer commit tests8/8、原count9/9、刷新8/merge9/types0；初始夹具setup失败均保留。继续FileView accepted-base/flush：`input/draft-sync/file-acceptance-probe.mjs`实际AST取onLanded/onText/setEdit/publish/flush，当前3/3业务红：首键后flush未写、保存B后同task回A错误clear、继续C仍was=A不是B。未改这片产品，非作者正核accepted状态与merge/clash permission的最小设计；这是既定L03/E缺口，不是新增专项。
- 之后按既定队列推进W watch、A→S、B/M及H/E既定缺口；新独立问题仅登记下一轮，不静默缩减原34项。

## 恢复、资源与验收边界

内盘恢复根 `/Users/zgh/Library/Caches/khor-code-quality/recovery`。旧完整b338 bundle已verify；copytime/library增量及A4/W固定包另存。root正在更新34f完整checkpoint（含当前两作者真实dirty/新增源及E固定提交），以实际新receipt为准。旧包/原失败保留，不能覆盖更晚外盘源。历史3份编译ZIP已双SHA迁external archives，内部兼容链接；源码恢复包仍内盘。

V最终native-gui-ca83fe58 exit0/254.051s，GUI SHA8939bab13bfd14f7ef8ee2d7c8eec4b1cfeba3ae10c88b614b72fa3f6c687619。是隔离devUrl/devtools变体，非ship或当前4cf完整产物。先前人工复合OS批270s超时/cleanup权限失败保原结果；私有剩余已按身份清理。focused L05 outer80421 exit0：term_open→ready→drop各1、chat调用0；真实CmdQ正常exit0、host认证replay139B、remaining[]。仅证明该单项，不是新GUI完整重启恢复/IME验收。**用户目前无需操作，也不再重复已验拖放。**

Clean Code逐片段落实命名、状态/资源唯一owner、纯规则与效果分离、失败/取消明确、移除替代旧路；必要平台/并发注释保留，不以格式量或零注释算完成。最近告用户剩余约 **8–12小时有效开发验证，人工/真机另列**，尚未收尾；不按提交数算完成比例。
