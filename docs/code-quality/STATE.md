# 当前治理状态

更新：2026-10-26。完整治理仍在执行，不能以当前批次代替全仓完成。范围、协作与压缩恢复按 [PLAN.md](PLAN.md)。旧检查点已移到 [历史记录](history/STATE-before-current-protocol-checkpoint-20261026.md)，下列现场事实优先。

## 用户决定与工作边界

- 最新原话：“不必考虑旧端兼容，我们还没有上线，按着纯净的开发就行”。统一当前Khor接口/协议，停止旧Khor fallback、迁移分支和双版本矩阵；已有legacy实验仅保留历史证据。平台适配、外部ACP能力协商、认证和数据完整性持续保留。
- 用户授权完整全仓治理、多智能体协作、分批实现验证，并要求目标完成前持续工作。**本轮没有main合并或发布授权。**
- main `/Users/zgh/Desktop/working/khor` = `8704a503a27b36ad58f922690de50e766f8bd485`，现场tracked clean。已发布 `target/release/khor-gui` 内嵌前端，可直接启动；SHA `9c3e078f355aa65e3f9d0137ae38548a1f2d75d21bf261528335e031ee6dbc01`，不能证明本轮治理版本。
- 权威文档仅本目录，分支 `plan/code-quality`；实现树同名文档只是快照。资料统一 `/Volumes/disk0/Khor`，复用现有工作树，不新造散落khor目录。

## 集成与已验证事实

集成树 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，分支 `integrate/code-quality-wave1`，当前HEAD **9543b233**，另有root当前inventory候选4文件dirty（link/proto/catalog en+zh），尚未运行，不计绿。GUI replay正式gate已独立审查并提交ee51fd54；Q045 8a22→31b1991f、a162→53ade1c2；blob core2211→fdeda78b；Q041c f7c12ae3→76db834b已进入集成。Q041c新scripts已在精确76db root17/17/source稳定；最终Rust组合仍待精确运行。ed460 pending修→9543b233也已集成。

| 范围 | 当前结论与证据边界 |
| --- | --- |
| Q010/11/15 | 首批键盘/commit跳转已集成。c371同源真正Tauri iOS run11验证键盘、历史锚点、commit第6/8跳转、native page句柄；物理iPhone/IME未验。后续Safari长稿164px真红已修，实际候选JS的MobileSafari/UIKit26指标通过，ACP/Khor各6次settled gap0、reader锚点保持。组件36/scroll34/types通过；最终整合Rust App尚未重建。 |
| Q020–Q040既有批次 | 自动保存/冲突/锁刷新、身份首建/purge、FIFO/连接owner、凭据裁剪/A2A/输入预算、上传取消receipt/poll/native page等逐批集成。各卡记录真实覆盖，见AUDIT。Q033真正GUI→Rust上传取消、Q035 borrow lease仍待，不用mock冒充。 |
| Q043 | hear/helpers已集成；非作者双引擎22+38通过；真实原c371 Rust质量App+候选JS确认7listen/7unlisten和child WK title等资源链。OS drag/windowclose/fullscreen值及最终全源App仍未验。 |
| Q039隔离 | 真实25,448,146B devices差集阻断同轮Seen/Pin真红，生产8d87→8f55002b隔离各doc，保留首mandatory错误；同源Node2父/实际worker2通过。大差集自身尚不能收敛，未以隔离修复关闭Q039。 |
| Q041a1/b | 严格单资产SHA/固定release选择、canonical shell单源及逐字嵌入检查、独占join stage/receipt；不可变镜像代+原子VERSION+site保留历史。59da→e5f6、a723→de408已集成。31 installer、13 mirror、7 site有真实scratch脚本证据与独立审查；真实SSH/镜像部署未跑。 |
| Q037阶段2 | 8d916→07ba5f04、aa601→4da1804c、24b6478→c719741c已集成：Darwin自有unreaped退出窗口、有限ACP事件fence、可取消host、有界GUI writer。真实超慢drip取消约2.015s并同provider下一轮，Close问题独立复现后修，Closed/Ready尾部回归先红后绿。stage2 delivery保留9源SHA。SDK内部总队列RSS与专属host进程listener线程不冒充全部有界/全部join。 |
| Windows fanout | root dependency-free std harness/CI 58450bbc已集成，Mac10/10与YAML检查通过，**Windows runtime未跑**。 |
| 精确58450bbc集成 | integrated-5845-final：正常Node libcheck、host25/shared12/NodeACP12/Pi29/join8/modern sync2/ACPlib13/conversation19 =120条不同Rust通过；完整scripts unittest通过。source_stable=true，325.659s，源mtime刷新并实际重编，日志/完整输入指纹/最小容量已存。2可选real agent不计。此证据早于fe265 GUI和未集成Q045/core。 |
| Q037 GUI replay | 1c76→fe265已集成：failed/cancelled replay保留完整旧历史，成功空历史可清空，gone/reconnect不提交半批。作者真红4/4、26/26及旧poll22/types；root最终双引擎26/26通过，report七源before/after相同，console/page/network errors为空。首次root25/26只因fixture favicon404，inline favicon修后单项+完整26绿，保留原失败且未过滤console。正式gate四文件非作者审无阻断，ee51fd54已提交。 |

## 当前所有者与唯一写权

| 所有者 | 路径 / 分支 / 固定源 | 当前任务与下一步 |
| --- | --- | --- |
| root | wave1集成、plan总账；内盘 `.../recovery/khor-quality-store-owner-recovery`，fix/quality-install-integrity HEADa723bbba | Q041c root四源已提交clean：onset7611d688、build56d83b00、onset test07c9dbd6、bundle test654aa4d6。先准备目标Pi再Cargo、private输出/target目录/dry-run本地完整预检；非作者发现serve-up验证过晚真红，已前移并固定副本，最终独立17/17/source稳定。Q041c f7c→76db已集成，root精确17/17；真实Bun Darwin arm64编译通过，3/3无凭据独立制品smoke，source/providers稳定；Rust嵌入/Linux/musl/onset全链仍待。 |
| fanout_owner | `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`，fix/quality-ssh-lifecycle HEADa1625c7f clean冻结 | Q045 8a22cfdb+a1625c7f交付待root集成。单次reaped回执、并行有界I/O、统一deadline含stdin、finite SSH不建持久master；阻塞200ms旧3.036s真红→约226.6ms，双pipe/取消/超限/尾部/独立sentinel等135相关测试通过，36fixture ESRCH。Windows/真实SSH master/远端lease仍未验。MCP审计已交付；当前fix/quality-mcp-cancel@a162仅3文件测试fixture/cfg接线，未运行，等非作者审与槽。同步MCP可能阻断execute/claim是静态风险，尚未取红。 |
| native_helpers | `/Volumes/disk0/Khor/worktrees/current/khor-quality-native-io`，fix/quality-sync-chunked HEAD2211daa9；唯一untracked pending_cache.rs 首次SHA336ee2f3 | 当前协议blob core5文件已交付未集成：固定manifest/摘要、256KiB chunk、持久offset、进程owner锁与cancel配额释放；12 integration+146lib通过，源码稳定。Stored/VerifiedComplete不冒充Applied，无Node/Pair/Doc接线，初始化中断可留InvalidStage须处理。Doc merge_report草图已写；真实pending-cache2例已实际1绿1红：cache hit后creation到达，create=true/edit=false/target=false。compact另有真实红（snapshot后删唯一edit block）；最小修复ed460→9543已集成，真实report13Doc、cache安全判据、compact guard、2live登记、每writer maxend归并。6业务+147lib+12transfer通过/source稳定。当前只准备真实Node inventory加载失败2回归，生产归root，等待授新槽。 |
| store_owner | 只读wave1 root四文件 | Q041c审查已完成并交还源码权；GUI gate及真实Pi编译已交付。当前inventory首轮审交回：typedgate收窄、文档清理、local/remote inventory load提前终止待真实Node红；现只读审MCP首红fixture，不Cargo/改源。 |

## 下一批依赖与仍开放事项

1. root收口GUI gate独立审、Q045/core源审与集成、Q041c源码/真实制品验证，按最终影响范围串行验证；作者绿不直接作为新组合绿。
2. Q039统一当前传输：Doc真实success/pending回报与严格apply/flush边界、pending-cache实测、Node当前chunk协议、公平轮转、大差集25MB真实收敛及Pair/table_b64 bootstrap。移除旧Versions→legacy fallback，认证/epoch保护保留。core只是基座，不关闭任务。
3. Q037同步MCP、远端取消/丢receipt/cleanup lease；Q041构建工具/制品来源及安装完整链；Q033 native取消和Q035 borrow lease。
4. P1其余模块深入审查、P4职责/格式/命名/错误约定与历史代码分类、P5最终同源App及用户全流程。物理iPhone/IME/Windows等未知项如实保留，不能静态审查冒充实跑。

## 资源、证据与恢复

- 唯一Cargo/native槽当前 **空闲**；native_helpers准备inventory两红，fanout_owner准备MCP红（等非作者审）。任何新编译须root明确派发；其余不编译。所有治理Cargo/gen显式 `CARGO_TARGET_DIR=/Volumes/disk0/Khor/caches/rust/khor-quality-target`，jobs2，主发布target不动。跨worktree曾实际复用陈旧第一方artifact，故切树必须更新选定第一方mtime、确认SHA不变并核实际重编。
- 外盘风扇坏/SMART不可读，用户允许继续。内外盘可用须>=5GiB；掉盘/连续3I/O错误只停自己的PGID。最近core验证最小外6,805,020,672B、内11,059,986,432B，下次构建重新量测。清理必须新manifest并确认归属；源码/日志/App/archives/主release不清。
- 证据 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`。已结束自有质量App/23840/23841不得误作活动；用户主App/服务不动。iOS质量模拟器FE04AC7C-B375-4F2E-AE7E-B6F8F5608947可保留booted。
- 内盘恢复入口 `/Users/zgh/Library/Caches/khor-code-quality/recovery/RESUME.md` 与 `current-code-quality-docs`。恢复先读用户指令→本STATE/PLAN→实际HEAD/diff/agents/PIDs→源SHA/日志。未提交源以现场为准，不按摘要覆盖。
- 本次校核各树：main8704clean；wave1 9543四root dirty；SSH/MCP a162新增fixture测试准备；native-io ed460 clean后准备inventory回归；root Q041c f7c12ae3clean。中央任务卡若未提交亦是当前工作，汇总前尊重所有者。


## 本次新增真实证据

- Q039 `pending-green-01`：6/147/12，51.42s/source46稳定；147含新增1unit，不重复计。最低外6,257,414,144B/内10,952,970,240B。三own PGID已结束，scratch空。root独立审见Q-039-pending-root-review。
- Q041 `real-bundle-darwin-02`：私有锁SHA cache npm ci成功，Bun1.3.14真实3084modules编译；MachO ARM64 76,474,082B SHAaf9555409de4af93cf166aeedc4b88f19196b5848dc89dd496ea0c650b79d646，三binary smoke全过。26agent tracked源/providers稳定，ignored依赖link恢复，19 observed PID全结束。首次npm config双/dev/null配置故障保留为setup失败。Rust嵌入/Linux未验，不外推。
- 当前inventory4源尚未类型/运行：删legacy fallback、typed InventoryNotAdmitted仅Versions触发Vouch，selected doc交换失败均报告；但opening/answer_versions加载提前失败仍待新增真实回归与修复。宏?控制流已纠正：远端已知doc load会整体Err，而非elsecontinue静默；unknown doc名称才有静默skip。不能把静态猜测当运行红。
