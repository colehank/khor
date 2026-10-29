# 第二轮当前状态

更新：2026-10-30。目标 **完成第二轮 active**，范围 **29条反馈＋5遗留＝34项**。用户已授权实施；本轮 **main合并/ship未授权**。main `/Users/zgh/Desktop/working/khor` 保持 `e7c335c8`；集成 `/Volumes/disk0/Khor/worktrees/current/khor-round2` / `integrate/round2` / **aaf91b82d9b8a1e9b7c829579646b97a66deeb78**，tracked clean。

## 恢复与授权

先读最新用户消息、本文、[执行计划](ROUND-2-EXECUTION.md)、当前任务卡，再核refs/diff/agent/实际进程。旧阶段完整记录在[本次归档](history/STATE-through-watch-m025-20261030.md)，其中运行语句均为历史，不能直接复用PID或重开producer。

内盘恢复入口 `/Users/zgh/Library/Caches/khor-code-quality/recovery/RESUME.md`；当前运行表 **current-runtime.json**、refs表 **current-worktree-refs.json**、逐SHA文档镜像 **current-code-quality-docs-manifest.json**。实际handle/result优先于旧摘要；不得覆盖外盘更新的dirty源。

- 用户已澄清R2-028：Codex输入区缺少组合键配置，归M026自定义键行；不是待诊断的Mac物理拦截，不重复问入口。E仍负责真实Mac文件中文IME和最后一键→OS Quit。
- 用户明确未上线、不考虑旧端兼容；不为旧host新增v2/fallback/矩阵。当前协议须一致、错误显式、不重发；已有用户数据不破坏。
- 新独立问题只登记下一轮；现34项继续做完。Clean Code按领域命名、单状态/资源owner、纯规则与效果边界、清晰失败/取消及删除旧路审查；不是机械删掉平台/协议必要注释。

## 运行与写权

**Cargo/native与browser均空闲，无测试App/Codex。** 最新W CLI exec84078真实0/182.225s；WDOM续批exec85775真实0/7.258s，均owned空/端口空/TMP不存在。旧23837 Lab保留，处理前重新核身份，不按旧PID结束。新producer先固定来源/独审/授唯一槽/登记真实handle；jobs1、incremental0、offline、双盘≥5GiB，target `/Volumes/disk0/Khor/caches/rust/khor-quality-target`。

| owner | 工作副本与当前边界 |
| --- | --- |
| Root | 集成aaf91；W watch/publicAPI/consumer收口与验证已完成到分层checkpoint。watch副本 `khor-round2-watch` / `fix/round2-watch-ui@26fdebe7` clean保留，不与作者并写。Root继续集成/实际验收/恢复。 |
| round2_history | 同 `khor-round2-history` / `fix/round2-journal-hydration@919dbeef`＋S dirty候选；原 `fix/round2-history@b14`保留。S Capture→Core/GUI canonical journal hydration、发送UUID关联、bounded reader；candidate3正在修明确语义/失败点，尚未native/gen。shared Core API/lib/gen、src/api/catalog、native_io/Tauri胶水由其单owner，B不得并写。 |
| round2_input | 复用 `khor-round2-input` / `fix/round2-browser` / 基线aaf91，做B5 tabs.rs＋BrowseLanding和定向检查；B1–4只读合同/路径。M025固定 `072ba190`已交接/Root集成，不继续写W。新基线gen应actual W169＋catalog4b4b，不能沿用M旧165。 |
| round2_reviewer | 固定源/收据只读独审，不启动native/browser。S candidate3待尾核；B后片按既定边界审，不扩新专项。 |

## 已实现与剩余

| 包 | 已有证据 | 尚缺 |
| --- | --- | --- |
| E 输入/草稿 | accepted buffer单owner、同步草稿登记、迟到读取/merge/mine拒收、composition fresh intent、单physical stat/text及1followup；refresh18/accepted20/sync9/merge9等已有源映射，WDOM又证dirty保留、Undo回clean后读最新disk | 真实Mac中文IME、最后一键→正常OS Quit。focused E实际ce69产物和私有file-only脚本已准备，当前3分钟可用性问题未答，未开App。 |
| W 工作区 | 布局/独立commit/preview-pin/dirty Undo、固定HEAD完整分支分页；watch runtime/API/五DTO/前端单owner已Root5ddc集成。实际Mac kernel分层、HTTP0/Core/tables7、CLI exact1含38far sources/control、组件DOM6组4＋2，见下方 | 真App/手机最终组合（含机器头像/branch头部）。通用MachineFiles/FilesPane非选中工作区浏览链为独立后续，不自动扩本轮全局retrofit。 |
| M | M026键行pure9f+fb4/UI2423→Roote409，实际catalog/模型12/旧keys33×9/Chrome9/type绿；M025 `072ba190→aaf91`正式公共跟手engine，16源同author，Root engine18/types0 | M025正式DOM/真实手机触屏、M026真实Codex输入与系统软键盘；不以Lab/Chrome冒真机。 |
| A 外部观看 | 身份能力/来源＋A3有界capture；A4双真实Node和Core/GUI gen、DetailPane reader5实际DOM | 全能力矩阵、最终原生组合；不等于S liveGUI恢复已完成。 |
| H 长历史 | 有界IO/页/记录/缓存/取消和大历史定向证据 | 全source搜索/复制、最终原生组合；loaded DOM不能冒完整源。 |
| S 连续性 | view lease `94131dc8→9112f1b7`、Registry9＋43、三lib检查/实际gen、真实ChatView5；Node Capture ac535+b14分源5实际绿，无ACP load/resume，原半关闭cleanup失败保留 | Core/GUI canonical hydration实际闭包、startup字节/队列有界、完整宿主丢失/重启；L01原040 retry deadline与L02原page_steps竞态独立未解。 |
| B 浏览器 | 只读交接，B5正式source开始 | 真实WK进度、主题初始底色、拒绝原因实证、mobile几何和list删除；CLI200不是WK根因。 |
| C/G/Q | 内容点击/Cmd分流、软折行路径/返回cwd；共用渐隐材质；括号公式转换均source/定向门过 | 原生/深浅材质/触屏/文件渲染组合。 |
| D/N | 旧卡死未知、Busy多入口已定位 | 限定诊断与并发夹具；H绿不能关闭D。 |
| V | **L04/L05 done**（真实来源/严格产物/正常CmdQ及宿主认证replay范围） | 不代替E/S/整轮验收。 |

## 最新固定证据

- W Root5ddc：`integration/watch-source-1.json`及`watch-generated-5ddc/receipt.json`；36源等author26fd，885 Rust/Toml/lock等actual shared producer，bindings169＋catalog4b4b，不手写gen；旧DirWatch已从managed集移除并可逆保留旧产物。
- `workspace/watch/shared-1/result.json`：exec67436=0/154.478s、13phase0、DTO1/HTTP2/tables4共7actual、2174 source/env稳定/provenance/cleanup；CLI/Vite/DOM属于后续，不混数。
- `workspace/watch/cli-current-5ddc-1/result.json`：exec84078=0/182.225s、6phase0、actualCLI/test来源；exact1/51.26s含38far watch sources、held wait下begin/leave/ordinary ls。200ms受控RTT下新增文件276.160ms，回树状态0B/286.75µs；不是任意网络SLA。
- `workspace/watch/dom-final-5ddc/receipt.json` SHA **2c7bdc03e49809833949f5fd53c83950463a07a54d38834084c4a98df10a1d0c**：生产组件Chrome DOM6，分v4四组＋v5两组。2182 source/171gen两批逐dict同；v1/2 setup、v3 dirty期待不符既定defer、v4 Store无本机fixture原失败均保留，不回填一次final6。独审/94件双盘镜像通过；不是WK/App/IME。
- `integration/m025-aaf9-receipt.json`：`072ba190→aaf91`，16源逐字等，Root engine18/types0；author另有原Lab971对应/导航迁移纯门逐源证据，不把迁移断言冒未变旧断言。
- focused E固定 `khor-round2-validation@ce69`，2167源与actual native-editor-ce69相同；GUI SHA06e3809c…9d7、CLI531221ce…039。contract1591744a、launch b5385bfb、launcher d4edc12a、observer d50ddf13详原归档/E证据。其167旧gen不可用当前W171替换；只准备，没有IME/Quit通过。

## 下一步

1. S candidate3固定 typed PromptRecorded／capture失败重试／origin错误可见和具体persist失败语义，独审后运行最小真实CoreNoLoad/NodeOrigin/桥/gen批；不提前宣称恢复完成。
2. B5固定和独审；B1–4定native view/nav/progress/error/frame合同，再由shared唯一owner一次接胶水，避免字符串假进度/重复布局偏移。M025真实组件/手机验收仍在队列。
3. H全文搜索/复制、S startup/L01/L02、D/N逐原卡完成；共享源稳定后一次当前版原生组合，E真人验收按当前可用性启动。不要把构建/测试工具又扩成新专项。
4. 阶段完成即source/证据/恢复落盘；本轮34项保持，main/ship待用户最终指令。
