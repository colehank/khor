# 第二轮当前状态

更新：2026-10-30。目标 **完成第二轮 active**；范围 **29条反馈＋5项遗留＝34项**。当前用户已授权实施，没有授权本轮 main 合并或 ship。main 仍是 `/Users/zgh/Desktop/working/khor@e7c335c8dd81de091bd12b92cd704a63f25d03f3`。

## 权威入口与恢复

本文只记当前状态。此前阶段（包括失败的构建/脚本及旧运行语句）保存在 [最新历史](history/STATE-before-current-cc744-20261030T022419Z.md) 和 [早期归档](history/STATE-through-watch-m025-20261030.md)，旧 PID 不可复用。执行规则见 [ROUND-2-EXECUTION](ROUND-2-EXECUTION.md)、[PLAN](PLAN.md)、[ENGINEERING](ENGINEERING.md)。具体要求与失败证据在各包任务卡。

内盘恢复入口 `/Users/zgh/Library/Caches/khor-code-quality/recovery/RESUME.md`；同目录 current-runtime.json、current-worktree-refs.json 和 current-code-quality-docs-manifest.json。先核现场再恢复，不按摘要覆盖作者 dirty 源。

用户决定：不做旧端兼容；保护已有用户数据。R2-028 是 Codex 输入区缺组合键配置，归 M026，不重复询问入口。新增独立问题只登记下一轮，原34项继续完成。Clean Code 按命名、规则一处、状态/资源 owner、效果边界和明确失败审查，不以零注释或格式化量作为成果。

## 现场与责任

当前 native/Cargo、browser 槽均空闲，无本轮测试 App/Codex。Root exec15482 已终态通过；N exec24467 已终态失败并清理，不能按旧 RUNNING 重启。后续实际调用必须按单槽授予，jobs1/incremental0/offline、两盘可用≥5GiB，target 继续是 `/Volumes/disk0/Khor/caches/rust/khor-quality-target`。不得按 basename 结束用户 khor-gui：持久宿主可能同名。

| owner | 当前职责与工作副本 |
| --- | --- |
| Root | 协调、集成、真实验收、中央状态。`khor-round2` / integrate/round2 **cc7443070c7cc5a6cc3b76541ffded5763798d8a** tracked clean。固定验证复用 `khor-round2-watch` / validate/round2-session-browser，同 HEAD/源；旧 watch ref26fdebe7保留。不成为第三条大实现线。 |
| round2_history | `khor-round2-history` / fix/round2-browser-shared，最新 H 固定 **c37dcac42e47281ad6e6ca0c19bc1b685f161ff8**；其后的 native 工具转换 helper/facts 正在写，先核 diff。负责 H、后续 S；共享 Core API/lib/gen、src/api/catalog、native glue 单 owner。新 H 未混入 cc744。 |
| round2_input | `khor-round2-input` / fix/round2-state-concurrency **150715f207dec7dcfd759feb2edad4dca2f31842**。N 根锁1秒共同截止修复的 IO 单测绿，但真实 CLI 并发仍 Busy；当前仅准备同 fixture 的第二阶段 cfgtest 定位。N 实际通过后接 S startup 字节/队列，尚未实施后者。 |
| round2_reviewer | 非作者只读审查源码与原证据，不自行启动 native/browser。H direct giant candidate2 已审；待 N 固定阶段夹具，并核 Root 整合尾。 |

## 当前集成与证据

**cc744 已实际整合 S Capture/canonical hydration 与 B 导航/主题/几何源码。** B22逐字等作者df92、共享8等29ff；S20中的18源同d83，仅两public API文件因B新增不同，不能称全S frame相同。

证据根：`/Volumes/disk0/Khor/evidence/round2/integration/session-browser-cc744`。

- Core 导航合同2：native-1 实际通过。
- command tables4：native-2 实际通过；同真实 Cargo 产生 bindings generator。
- 真 bindings/catalog：native-3 实际生成发布；175 bindings管理项＋catalog＋lock＝177文件。原 tsc -b 写出自身缓存导致 source guard false，原结果保留；该文件经身份/SHA核对后可逆保留，原源码已精确恢复。
- TypeScript **noEmit**、Vite：native-4 实际通过，产出277文件/15,208,767B；GUI libtest Cargo0。GUI选择器 kind 原误填lib，按实际metadata只修 request.kind，同原完整Cargo日志通过未改strict verifier；原false及修正均保留。
- GUI 导航12＋native IO2：gui-tests-final exec15482 实际14通过，源/gen/assets/env稳定，无超时、强清或残留。**不是一次20项完整suite，不代表 App/WK/手机已验。**
- combined-receipt.json SHA **293ad498a4ab54a701b4dfcc58d4c4e24ff1c6eb19107e95d15b59384aa63d3f**；内盘 mirror `round2/session-browser-cc744-actual` 共215件/6,363,307B，manifest SHA **517f9d403472fc2cee83f5d439a96ad166f124031a8da2d17c9fb679e5ba5e42**。五次原false保留，不重跑已绿构建。
- Root 与固定验证 tracked 源逐SHA相同，gen177/assets277同真实producer。Root dist从只读snapshot复制的权限已改回目录0755/文件0644，279节点仅权限变、内容不变，readonly snapshot未动。

## 各包真实缺口

| 包 | 已有实现/证据 | 尚未完成 |
| --- | --- | --- |
| E 输入/草稿 | accepted buffer单owner、composition/迟到读取/退出草稿保护及定向门；focused ce69真实GUI/CLI产物已准备 | 真实 Mac 中文 IME、最后一键→正常 OS Quit。最新3分钟可用性问题未答，不能自行启动限时实例。`khor-round2-validation@ce69` 的167 gen保持原样。 |
| W 工作区 | 已验布局/独立commit preview-pin/完整当前分支分页；Root5ddc实际watch Core7、CLI exact1含38far source/control、DOM6分4＋2批 | 最终 App/手机组合。通用非选中MachineFiles/FilesPane实时浏览属后续独立项，不扩本轮。 |
| M 手势/键行 | 公共跟手引擎＋生产DOM5；可配置键行模型/目录/UI；真实 TerminalPane→Khor PTY→独立Codex两路径完整press+keyup一致、独立draft证据和offline1独审核 | 真实手机触屏/软键盘。原Codex suite因oracle失败保留；已证相同编码，不声称该Codex支持ShiftLeft选区，未发送消息/模型请求。 |
| A 全ACP观看 | 身份/能力政策、A3有界capture，A4真实Node/Core/DetailPane DOM；源码能力矩阵40项（22自动/18手动）已落盘 | 最终原生组合。40项是源码矩阵，不是安装并实际跑过40个agent；只读观察不夺控制权。 |
| H 长历史 | 有界页/传输/渲染；后台Worker全源搜索，c37 direct giant两遍range候选20项/6真实Worker、缓存复扫取消红→绿已审 | 原生 parser/normalizer 和 Claude/Codex既有工具正文转换共享helper/facts、actualgen、搜索/复制UI及原生组合。不能用loaded DOM或coveragefalse替代既有支持完成。 |
| S 连续性 | view lease、无load重挂canonical journal；旧 hydration-green-1 **38顶层＋inner1**实际绿。上述cc744补齐剩余typed/native6，旧38按原源映射保留 | startup 32768条仅count而无byte界/两段unbounded队列；完整host死亡/重启、GUI/Shell身份与控制恢复；L01原retry deadline、L02原page_steps关闭/重开竞态各自未销账。 |
| B 浏览器 | source df92/shared29ff；40纯规则，cc744实际Core2/GUI导航12/nativeIO2、真typedgen | 真WK进度/主题初始底色/访问错误实证、手机原生甲板几何与真实swipe删除。CLI200不证明WK原因；非WK Settled不伪造成功100/导航ID。 |
| C/G/Q | 点击内部/Cmd系统、折行路径与返回cwd；统一渐隐材质；括号公式沿原数学链，均实现/定向验证 | 原生内容打开、三种外观/深浅/滚动、触屏/文件渲染组合。 |
| D 卡死 | 已限定调查当前材料，R2-D有最小实际卡死采集步骤 | 原冻结无stack/IPC现场，唯一根因unknown，仍未解决；不能用H性能改善关闭。 |
| N 并发 | 真实旧RootGate诊断43bf已复现；当前1507真实CLI/Node编译0、5项IO单测过 | exec24467真实并发query仍Busy。第二源路径：root释放后，Journal.next未rename触发needs_recovery→State.open→metadata Busy；仍需实际阶段夹具，不能仅凭mtime宣布根因。 |
| V 工具遗留 | L04/L05已有真实来源与正常退出/持久host契约证据，done | 不替代E/S/整轮验收；工具修正停止扩新专项。 |

## 下一动作与边界

1. N只在同一次真实Journal.save已flush未rename处加cfgtest阶段证明：root可进入、metadata仍held、next有效，复用实际1507 CLI，不重编CLI或重跑已绿5IO。固定source/合同→独审→Root授一次Node单case槽；先证失败阶段，再最小产品修复，不能盲目加重试/旁路。
2. H继续原cagent/codex最小转换helper/facts，复用唯一现有normalizer/paint；稳定后一次完整必要Node/Core/gen闭包，不逐包反复试错。H新DTO不可手写假gen。
3. N真绿后作者槽承接S启动字节/队列，已做readonly备查在`history/startup-readonly-cc744`。bounded第二队列不能在consumer返回前await预装而死锁；不得静默丢长历史或伪Ready。
4. 源稳定后构建当前私有App，做既定交叉组合/平台验收及L01/L02；E按新鲜可用性回复启动。新增独立问题只写NEXT-ROUND。
5. 阶段检查点同步refs、源bundle/真实diff、证据与内盘SHA镜像；不覆盖他人任务卡、不stage all。完整34项逐条证明后才可完成目标；main/ship仍待用户指令。
