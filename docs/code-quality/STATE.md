# 第二轮当前状态

更新：2026-10-30。目标 **完成第二轮 active**，范围 **29条反馈＋5遗留＝34项**。用户已授权实施；本轮 **main合并/ship未授权**。main `/Users/zgh/Desktop/working/khor` 保持 `e7c335c8`；集成 `/Volumes/disk0/Khor/worktrees/current/khor-round2` / `integrate/round2` / **5c726b0a79e9a1454dc3bbf5435a3cfa7ee19173**，tracked clean。

## 恢复与授权

先读最新用户消息、本文、[执行计划](ROUND-2-EXECUTION.md)、当前任务卡，再核refs/diff/agent/实际进程。旧阶段完整记录在[本次归档](history/STATE-through-watch-m025-20261030.md)，其中运行语句均为历史，不能直接复用PID或重开producer。

内盘恢复入口 `/Users/zgh/Library/Caches/khor-code-quality/recovery/RESUME.md`；当前运行表 **current-runtime.json**、refs表 **current-worktree-refs.json**、逐SHA文档镜像 **current-code-quality-docs-manifest.json**。实际handle/result优先于旧摘要；不得覆盖外盘更新的dirty源。

- 用户已澄清R2-028：Codex输入区缺少组合键配置，归M026自定义键行；不是待诊断的Mac物理拦截，不重复问入口。E仍负责真实Mac文件中文IME和最后一键→OS Quit。
- 用户明确未上线、不考虑旧端兼容；不为旧host新增v2/fallback/矩阵。当前协议须一致、错误显式、不重发；已有用户数据不破坏。
- 新独立问题只登记下一轮；现34项继续做完。Clean Code按领域命名、单状态/资源owner、纯规则与效果边界、清晰失败/取消及删除旧路审查；不是机械删掉平台/协议必要注释。

## 运行与写权

**Cargo/native与browser均FREE，无App/Codex。** S green3实际exec78796已terminal(false)/137.563s：真实GUI Cargo0，但严格来源拒绝fresh khor_acp无匹配producer；剩6/gen仍未执行，未发布，source/env/owned/TMP净。前38＋inner1已实际绿且独立列出。停止逐包补来源的循环；H先完整归档、只读定位全量不匹配链，一次给出完整方案，不自动重开Cargo。B shared可在此已终态后进入source接线窗口，由H唯一写公共源。新producer先固定来源/独审/授唯一槽/登记真实handle；jobs1、incremental0、offline、双盘≥5GiB，target `/Volumes/disk0/Khor/caches/rust/khor-quality-target`。旧23837 Lab保留，处理前核实际身份，不按旧PID结束。

| owner | 工作副本与当前边界 |
| --- | --- |
| Root | 集成5c726b0a；W watch/publicAPI/consumer收口与验证已完成到分层checkpoint。原watch ref `fix/round2-watch-ui@26fdebe7`保留；同worktree已复用为Root固定验证 `validate/round2-session-browser@cc744307`，从Root5c组装S Capture/WIP＋B已审source，tracked clean。B4 DOM/native deck坐标窄delta已组装并独审解除阻断，未Cargo/gen；不与作者并写，不新增目录。Root继续集成/实际验收/恢复。 |
| round2_history | 同 `khor-round2-history`，S20已固定WIP `d83d9b73`，旧38＋inner1实际绿/剩6与gen未过。当前同worktree `fix/round2-browser-shared`，B共享基础e5fb与final29ff均source-only已审；H正常record后台worker4源固定531a/8项Node与workerthreads实际通过、UI/giant未完成；现窄抽tool输入正文单helper，下一巨record投影准备最小DTO。B差异独立提交，不把S未验部分冒完成。shared Core API/lib/gen、src/api/catalog、native_io/Tauri胶水由其单owner，B不得并写。 |
| round2_input | B private最终df92/前端27220/异步错误d5与shared29ff/04c已fixed独审，40纯门/完整22源与3.35MB内盘镜像保留，Rust12/平台仍未验。现同 `khor-round2-input` / `fix/round2-state-concurrency@5c726b0a` 做N只读查询＋实际同步RootGate跨进程固定夹具准备；仅cfgtest checkpoint授权，未native或生产修改。 |
| round2_reviewer | 固定源/收据只读独审，不启动native/browser。S candidate3待尾核；B后片按既定边界审，不扩新专项。 |

## 已实现与剩余

| 包 | 已有证据 | 尚缺 |
| --- | --- | --- |
| E 输入/草稿 | accepted buffer单owner、同步草稿登记、迟到读取/merge/mine拒收、composition fresh intent、单physical stat/text及1followup；refresh18/accepted20/sync9/merge9等已有源映射，WDOM又证dirty保留、Undo回clean后读最新disk | 真实Mac中文IME、最后一键→正常OS Quit。focused E实际ce69产物和私有file-only脚本已准备，当前3分钟可用性问题未答，未开App。 |
| W 工作区 | 布局/独立commit/preview-pin/dirty Undo、固定HEAD完整分支分页；watch runtime/API/五DTO/前端单owner已Root5ddc集成。实际Mac kernel分层、HTTP0/Core/tables7、CLI exact1含38far sources/control、组件DOM6组4＋2，见下方 | 真App/手机最终组合（含机器头像/branch头部）。通用MachineFiles/FilesPane非选中工作区浏览链为独立后续，不自动扩本轮全局retrofit。 |
| M | M026键行pure9f+fb4/UI2423→Roote409，实际catalog/模型12/旧keys33×9/Chrome9/type绿；M025 `072ba190→aaf91`正式公共跟手engine，16源同author，Root engine18/types0 | M025真实手机触屏、M026系统软键盘；真实Codex两路径原raw含keyup一致及当前独立draft已实际取得/离线1独审核，原suitefalse保留，Codex未选择；不以Chrome冒真机。 |
| A 外部观看 | 身份能力/来源＋A3有界capture；A4双真实Node和Core/GUI gen、DetailPane reader5实际DOM | 全能力矩阵、最终原生组合；不等于S liveGUI恢复已完成。 |
| H 长历史 | 有界IO/页/记录/缓存/取消和大历史定向证据 | 全source搜索/复制、最终原生组合；loaded DOM不能冒完整源。 |
| S 连续性 | view lease `94131dc8→9112f1b7`、Registry9＋43、三lib检查/实际gen、真实ChatView5；Node Capture ac535+b14分源5实际绿，无ACP load/resume，原半关闭cleanup失败保留 | Core/GUI canonical hydration实际闭包、startup字节/队列有界、完整宿主丢失/重启；L01原040 retry deadline与L02原page_steps竞态独立未解。 |
| B 浏览器 | B5已Root5c集成：实际Node3、回调7及types过；B1–4私有导航owner/前端32纯规则绿，H单writer接四DTO/两label/native glue | 真实typedgen/native/WK进度、主题初始底色、错误实证、手机几何/真实swipe。非WK Wry只知终结，新增Settled清pending而不伪造成功100；CLI200不是WK根因。 |
| C/G/Q | 内容点击/Cmd分流、软折行路径/返回cwd；共用渐隐材质；括号公式转换均source/定向门过 | 原生/深浅材质/触屏/文件渲染组合。 |
| D/N | D当前材料限定调查/最小实际卡死采集步骤已落R2-D，原根因仍unknown；Busy多入口已定位 | D真实现场仍缺，H绿不能关闭；N一固定私有并发夹具待input B固定后接。 |
| V | **L04/L05 done**（真实来源/严格产物/正常CmdQ及宿主认证replay范围） | 不代替E/S/整轮验收。 |

## 最新固定证据

- W Root5ddc：`integration/watch-source-1.json`及`watch-generated-5ddc/receipt.json`；36源等author26fd，885 Rust/Toml/lock等actual shared producer，bindings169＋catalog4b4b，不手写gen；旧DirWatch已从managed集移除并可逆保留旧产物。
- `workspace/watch/shared-1/result.json`：exec67436=0/154.478s、13phase0、DTO1/HTTP2/tables4共7actual、2174 source/env稳定/provenance/cleanup；CLI/Vite/DOM属于后续，不混数。
- `workspace/watch/cli-current-5ddc-1/result.json`：exec84078=0/182.225s、6phase0、actualCLI/test来源；exact1/51.26s含38far watch sources、held wait下begin/leave/ordinary ls。200ms受控RTT下新增文件276.160ms，回树状态0B/286.75µs；不是任意网络SLA。
- `workspace/watch/dom-final-5ddc/receipt.json` SHA **2c7bdc03e49809833949f5fd53c83950463a07a54d38834084c4a98df10a1d0c**：生产组件Chrome DOM6，分v4四组＋v5两组。2182 source/171gen两批逐dict同；v1/2 setup、v3 dirty期待不符既定defer、v4 Store无本机fixture原失败均保留，不回填一次final6。独审/94件双盘镜像通过；不是WK/App/IME。
- `integration/m025-aaf9-receipt.json`：`072ba190→aaf91`，16源逐字等，Root engine18/types0；author另有原Lab971对应/导航迁移纯门逐源证据，不把迁移断言冒未变旧断言。
- focused E固定 `khor-round2-validation@ce69`，2167源与actual native-editor-ce69相同；GUI SHA06e3809c…9d7、CLI531221ce…039。contract1591744a、launch b5385bfb、launcher d4edc12a、observer d50ddf13详原归档/E证据。其167旧gen不可用当前W171替换；只准备，没有IME/Quit通过。

## 下一步

1. H完成B共享源与input固定叶整合；S20 d83独立source checkpoint，保存38＋inner1既有来源。严格producer全量差异交reviewer一次查清，停止逐包试错；S剩6＋B必要新测试/gen合一批同源验证。
2. Root已补M026正式TerminalPane→实际Khor PTY→独立Codex：原raw含keyup等价/独立draft实证，原oracle失败不回填；离线1和独审尾核通过。系统软键盘/真机仍待验。M025 Chrome5通过不替真手机。
3. H全文搜索/复制、S startup/L01/L02、D/N逐原卡完成；共享源稳定后一次当前版原生组合，E真人验收按当前可用性启动。不要把构建/测试工具又扩成新专项。
4. 阶段完成即source/证据/恢复落盘；本轮34项保持，main/ship待用户最终指令。

B5 Root整合5c726b0a：三源逐字等2ba5，Roottypes0；actualNode3沿author源/guard-Cargo-derived producer及83.581s原收据，不声称5c整树新native。S echo已按A3当前闭包授最小可信localturn投影关联，原bootstrap/外部/同文不同turn保留，不删除原record；先actual old5/target3红1，再candidate4正向/其余43闭包。

M025正式DOM实际证据：mobile/session-follow-dom-5c-1/receipt.json，5组同Root5c/同fixture一次全过、27小件含截图已内盘逐SHA镜像；Root原source/engine18/type0与本DOM不互替。

M026私有clone准备已用于实际受验链；最终限定结果见R2-M及下条derived证据。旧准备阶段不再代表未运行。

M026最终限定证据：mobile/codex-input-real-pty-4/derived-analysis.json与analyze.py，实际exec71277原false/4.859s保留；标准/键帽完整press+keyup相同、current abcdeX/pqrstY、focus已过、无消息提交/模型请求、strictclean。offline1＋独审通过；37小件镜像m026-codex-pty-4。不是完整green suite/选区支持/真机。

固定验证checkpoint：Root复用旧watch目录，从5c干净cherry十个独立源提交至cc744307，含S ac535/b14/d83与B0b/09/e5/4eb/d5/272/29ff，全无冲突；只是source组装，未Cargo/gen/type。原Root5c/main e7c均clean。B4坐标df92已接为cc744；B22源全等author df92、共享8源全等29ff，S20仅两public API文件因B新增不同（不得冒全frame相同）。H新worker在作者树继续，不与此验证树并写。

B最终source checkpoint：作者df92收据e32cd021…fe8320/22源、projection11+hook10+failure10+viewport9=40纯；真实Pages Rust12/Core2/typedgen/WK仍待。Root固定cc744/clean源收据在integration/session-browser-cc744/source-receipt.json。恢复bundle round2-cc744-source.bundle已verify，SHA见同名json，包含cc744/df92/04c三个实际refs，H4源虽在author ref中但不在Root固定native cohort。
