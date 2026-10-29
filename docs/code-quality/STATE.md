# 第二轮：当前执行状态

更新：2026-10-29。目标 **完成第二轮** active；范围 **29条反馈＋5遗留，共34项**。用户已授权实施，**本轮 main 合并/ship 未授权**。main `/Users/zgh/Desktop/working/khor` 保持 `e7c335c8`；集成 `/Volumes/disk0/Khor/worktrees/current/khor-round2`，`integrate/round2`，**ce69e0bd**（本次核验 tracked clean）。旧阶段详[归档](history/STATE-before-round2-ce69-sync-20261029.md)，其中运行与待答语句均是历史。

## 恢复顺序与现场

1. 最新用户消息 → 本文 → [执行计划](ROUND-2-EXECUTION.md) → 当前任务卡/专项 → 核实际 refs、diff、agent 与进程。恢复镜像在 `/Users/zgh/Library/Caches/khor-code-quality/recovery`，核最新 receipt，不用旧摘要覆盖外盘未提交源。
2. 用户已回答 R2-028：**“codex输入区，是因为根本没有组合键的配置机制”**。唯一实施归属 **M/R2-026 自定义终端键行**，以配置/保存/发送 Shift＋Left 验收；不是待诊断的 Mac 物理快捷键拦截。原入口问题已结束，不再重复问。E 仍负责真实 Mac 文件中文 IME、草稿完整性和退出保护。34 项不增不减。
3. 两实现作者：`round2_input` 做 W watch；`round2_history` 已完成 S 首个 attachment lease slice，现获授 M 自定义键行，完成后回 S 剩余。`round2_reviewer` 对固定版本独审。root 负责集成、恢复与小型 E 接线，不开第三条大实现线。
4. **native/Cargo 槽空闲**：Root native-editor-ce69e0bd exec11064已终态0/254.270s，11实际阶段全0，2167源/env稳定、provenance完整、remaining[]/cleanup无错/TMP不存在；只build，没有App。Root native0478、S green2、W leaf15也均终态。新候选先登记精确源/命令/预算再授槽。target `/Volumes/disk0/Khor/caches/rust/khor-quality-target`，jobs1/incremental0、专用外盘 TMP、双盘每阶段 5GiB floor。用户 App、会话及共享缓存不动。
5. 本任务没有 App 打开。focused E脚本独审＋actual私有CLI ws add/ls预检通过；已异步询问当前约3分钟真人IME可用性，**须收到现在方便才启动**，此问不是R2-028入口澄清，不重复旧拖放。旧 W Vite23837（原PID98386）保留；新浏览器与23838/23839、S23827/28/29均已由 owner 清理。恢复必须再核实际身份，不按旧 PID 直接结束进程。
6. shared Core API/lib/Tauri/native_io/gen/catalog 仍由 `round2_history` 单一 owner 串行接 W DTO。Root 当前 bindings 是 S green2 实际生成 **165 managed files（164TS＋inventory）**，逐 SHA 与文件集核准；catalog 沿用真实产物。Root types0。生成根还含既有 lock/catalog，不能把整个目录数量当 managed binding 数。W leaf 不改变接口。

## 当前包、证据与缺口

| 包 | 已实现与证据 | 剩余工作 |
| --- | --- | --- |
| E 输入/草稿 | 单 accepted buffer owner、同步草稿登记、迟到读取/merge/mine拒收、standalone discard；最后 E refresh `aba5ac53/64a70347/c4243305` → Root `dbba8321/6bd7023b/4e63a4b4`：仅保 fresh-read intent，composition end 读新版本，单物理 stat/text 请求并合并后续 intent。原红保留；最终 refresh18、accepted20、sync9、merge9、types0，独审通过 | 真实 Mac 文件中文 IME、最后一键→正常 OS Quit；不得拿合成事件冒物理证据 |
| H 长历史 | 有界 IO/页/记录/缓存/取消，实际 ChatView 大历史定向证据 | source 全文搜索/复制、最终原生组合；loaded DOM 不代表完整源 |
| C 内容打开/导航 | 普通内部/Cmd系统直接分流；终端软折行/CJK路径；返回最新cwd保草稿 | 原生组合验收 |
| G 材质 | 共用正文渐隐、窗口底材质归属和菜单厚底，消费者已封口 | 三种窗口材质、深浅色、原生/触屏组合 |
| W 工作区 | 正式布局/独立commit/shared preview/pin/dirty Undo；完整分支 all-parent 固定HEAD分页后端及 UI 已集成。UI `31d1336d→0478df63`，pure9/tabs6/memory5/types0，宽8/窄前5＋窄修后1＋余2分阶段真实证据。最新 watch leaf `d979ffaa→ce69e0bd`，Git真实控制路径、私有有界owner规则、过滤Access；实际15例绿，独审通过 | watch runtime/DTO/前端订阅与真实kernel证据、Face关联及原生组合；first leaf不是实时更新已完成 |
| A 外部观看 | A1/A2能力身份、A3 journal capture、A4真实双Node含取消/断线/响应预算及 Core/GUI实际生成；DetailPane→reader浏览器5组绿 | 全能力矩阵及最终组合；live GUI hydration 尚不由 A3 证明 |
| S 恢复/生命周期 | 首 slice `94131dc8→9112f1b7` 精确 opaque view lease贯穿begin/open/read/write/control/leave，旧A不可重查id操作新B，双视图非破坏性读，pending有界/取消owner明确；实际Registry9＋后批43共52行为，三lib check0，真实gen。test-only `7246b665→4b1c64bc` 真实ChatView lateACK/retry/cancel/replay5绿 | liveGUI NoLoad/journal hydration、startup forwarding字节有界、完整GUI/terminal宿主丢失/重启恢复；L01原040 deadline、L02原page_steps竞态仍独立未解；M后继续，不丢范围 |
| B 浏览器 | 只读调查 | 五项正式功能与真实WK/mobile验收均未完；CLI200不能证明WK拒绝已修 |
| M 移动输入/切换 | 连续跟手参考已验；键行原方案 TERM-KEY-ROW.md 已锁定。history作者获授 model/codec/storage→UI 两checkpoint | 正式跟手、自定义组合键/序列、设备保存和触屏/真实Codex验收；新M产品尚未交付 |
| D/N | 旧卡死未知；Busy多入口已定位 | 限定调查与并发夹具、明确结果；H绿不关闭D |
| Q/R2-029 | 共享括号公式转换，保护code/escape；MD门/9文件/7合同/types绿、独审 | 原生文件渲染组合 |
| V | **L04/L05 done**：真实来源/严格产物绿，focused L05正常CmdQ与宿主认证replay、私有清理独审绿 | 不替代E/S全轮验收；原270s失败保留 |

## 当前工作副本与下一检查点

- **Root**：`khor-round2` / `integrate/round2` / `ce69e0bd`。E三片、S两片、W leaf顺序集成，Root受影响JS/types绿。Lab测试文件不自动进入产品。
- **M / S / shared owner**：`khor-round2-history` / `fix/round2-history` / 起点 `7246b665`。M授权路径：新增term-key-model/codec、term-keys/kitty、TermKeys、KeyRowEntryEditor、AppearanceRows、TerminalPane窄origin guard、zh/en catalog、TERMINAL及定向脚本。Ctrl/Alt/Shift；custom不继承锁、不repeat；sequence先全部校验编码再一次输入；per-Terminal编码origin须try/finally；隔离试按不持有terminal输入能力；旧预置顺序/隐藏迁移、坏配置未重置不覆盖；Mac Settings入口可见。固定首片后交独审。W公共DTO由此owner接线，M完回S剩余。
- **W watch**：`khor-round2-watch` / `fix/round2-workspace-watch` / 首片 `d979ffaa`，runtime dirty/new `watch/native.rs` 是作者当前未提交源，不覆盖。复用同Watches、无新服务；机器稳定ID+lease、active TTL、预算/fence、HTTP服务端有效wait0/native20s；实际gix gitdir/common/ref；本地kernel完整覆盖与partial诚实fallback；前端单inflight。先固定runtime/38 far-stream资源回归计划再申请native。
- **旧 W pager**：`khor-round2-git` / `fix/round2-workspace-git`，UI `31d1336d`、Lab `74028700`；已完成源码/浏览器交接，不复跑无变更绿批。浏览器宽/窄各阶段的SHA范围保留，不冒同一最终SHA整批通过。
- **root E tree**：`khor-round2-math` / `fix/round2-draft-sync` / `c4243305`，已全部集成且clean。Root三次collection、standalone、accepted/刷新反例详 input/draft-sync，不再扩工具专项。

## 实际原生与来源边界

`/Volumes/disk0/Khor/evidence/round2/validation/native-editor-0478df63/result.json`：exec97920终态 **0 / 253.027s**，实际metadata/catalog/bindings/types/Vite/CLI/corefixture/GUI完成；source/env稳定、provenance完整、remaining[]、cleanup无错/TMP不存在。最低双盘约7.65GB/11.85GB。**仅build，没有App/IME/Quit证据。** 该产物早于E新刷新和S lease API，不能用其旧二进制运行当前Root前端并声称当前完整证明。新E真人实例须先准备同源产物、私有工作区/直接编辑入口，再询问用户可用性，不能让270s时限消耗在准备上。

S证据：`evidence/round2/history/session-recovery/green-2`，exec95761终态0/246.613s；1479源前后同，43行为绿，加green1 Registry9。ignored app worker明确未跑。原green1两个HTTP夹具缺lease失败保留，修的是3个fixture JSON，不冒生产源变化。2个stage自有小TMP后续按身份清理，原记录未改。S浏览器5/9.537s，193源/gen前后同、ownChrome/端口/profile已清。Root首次脚本路径猜错与source-scope字典比较错误是记录/命令setup，正确入口补验绿，不冒产品红。

Clean Code按每片段的领域命名、状态/资源单owner、纯规则与效果边界、明确取消失败和移除替代旧路审查；平台/协议/并发必要注释保留。**新独立问题只登记下一轮；既有34缺口继续。** 先前剩余8–12小时只是当时估计，当前尚非收尾，真实设备时间另计，不按提交数报完成比例。

本次实际当前源产物：`validation/native-editor-ce69e0bd`（Root ce69），GUI SHA `06e3809ca745ecacb8e3e6d947ea70cda83feb30a63d54bd9320ba1f7c79e9d7`，CLI SHA `531221ce20337204d2607324f9c27b0efe9332a0064433ba21e02ba10f571039`。已逐复制二进制SHA再核，仅build，尚未启动file-only App。历史0478来源边界仍保留。

M首pure `9f034973` 固定，new11/旧keys33×9/types绿，尚未集成。独审发现preencoded focus-report过滤及坏配置可被toggle/move覆raw两处阻断，作者按原spec收口，原失败保留；UI仍待后片，不能以pure绿标M完成。
