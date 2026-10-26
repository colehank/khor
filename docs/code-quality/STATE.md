# 当前治理状态

更新：2026-10-26。完整治理仍在执行，不能以当前批次代替全仓完成。范围、协作与压缩恢复按 [PLAN.md](PLAN.md)。旧检查点已移到 [历史记录](history/STATE-before-current-protocol-checkpoint-20261026.md)，下列现场事实优先。

## 用户决定与工作边界

- 最新原话：“不必考虑旧端兼容，我们还没有上线，按着纯净的开发就行”。统一当前Khor接口/协议，停止旧Khor fallback、迁移分支和双版本矩阵；已有legacy实验仅保留历史证据。平台适配、外部ACP能力协商、认证和数据完整性持续保留。
- 用户授权完整全仓治理、多智能体协作、分批实现验证，并要求目标完成前持续工作。**本轮没有main合并或发布授权。**
- main `/Users/zgh/Desktop/working/khor` = `8704a503a27b36ad58f922690de50e766f8bd485`，现场tracked clean。已发布 `target/release/khor-gui` 内嵌前端，可直接启动；SHA `9c3e078f355aa65e3f9d0137ae38548a1f2d75d21bf261528335e031ee6dbc01`，不能证明本轮治理版本。
- 权威文档仅本目录，分支 `plan/code-quality`；实现树同名文档只是快照。资料统一 `/Volumes/disk0/Khor`，复用现有工作树，不新造散落khor目录。

## 集成与已验证事实

集成树 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，分支 `integrate/code-quality-wave1`，当前HEAD **1d2640d2**，tracked clean。已集成GUI replay/gate ee51、Q045 31b/53ad、blob core fdeda、Q041c 76db、pending完整性9543、inventory测试34ef和当前inventory修复1d2640d2。当前未接Node完整chunk/Pair，不把inventory收口等于大差集已收敛。

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
| root | wave1 integrate/code-quality-wave1 **1d2640d2 clean**；plan总账 | 当前inventory/166相关Rust/最终26+types/producer均收口。主分支8704clean不动；维护公共接口、独立审、集成、恢复。唯一Cargo槽当前空闲，所有新执行须排槽。 |
| mcp_worker_owner（新干净上下文） | `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`，fix/quality-mcp-cancel **7550e4f0 clean** | 旧作者首红测试25478已保留audit/mcp-cancel-red-20261026，root将该测试重放在1d2上。现只提精确worker/Running Control/hidden hook/Windows typed durable handoff方案与allowlist；未授生产或Cargo。新卡Q037-mcp-worker由其独占。 |
| doc_apply_owner（新干净上下文） | `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery`，fix/quality-sync-apply **1d2640d2** | 旧store_owner确认无半成品，root快进后移交。获store.rs最小module/helper、新store_apply.rs、新tests/doc_apply.rs写权，准备真实candidate/fence/checked pen/flush API与测试；不Node/proto/锁/依赖，不Cargo，先冻结/排槽。新Q039-doc-apply-owner卡独占。 |
| fanout_owner / store_owner / native_helpers（旧长上下文） | 所有旧源码/卡已交回、无自有PID/构建 | capacity错误后停止，不自动重试长任务；保留原始证据/refs。native_helpers已完成最终inventory限定证据审，无新增阻断；新工作只向新owners派发。 |

## 下一批依赖与仍开放事项

1. root收口GUI gate独立审、Q045/core源审与集成、Q041c源码/真实制品验证，按最终影响范围串行验证；作者绿不直接作为新组合绿。
2. Q039统一当前传输：Doc真实success/pending回报与严格apply/flush边界、pending-cache实测、Node当前chunk协议、公平轮转、大差集25MB真实收敛及Pair/table_b64 bootstrap。移除旧Versions→legacy fallback，认证/epoch保护保留。core只是基座，不关闭任务。
3. Q037同步MCP、远端取消/丢receipt/cleanup lease；Q041构建工具/制品来源及安装完整链；Q033 native取消和Q035 borrow lease。
4. P1其余模块深入审查、P4职责/格式/命名/错误约定与历史代码分类、P5最终同源App及用户全流程。物理iPhone/IME/Windows等未知项如实保留，不能静态审查冒充实跑。

## 资源、证据与恢复

- 唯一Cargo/native槽当前 **空闲**；root最终26/normalcheck/GUItypes已结束，下一批MCP worker和strict Doc apply先准备/源审，未授编译。任何新编译须root明确派发；其余不编译。所有治理Cargo/gen显式 `CARGO_TARGET_DIR=/Volumes/disk0/Khor/caches/rust/khor-quality-target`，jobs2，主发布target不动。跨worktree曾实际复用陈旧第一方artifact，故切树必须更新选定第一方mtime、确认SHA不变并核实际重编。
- 外盘风扇坏/SMART不可读，用户允许继续。内外盘可用须>=5GiB；掉盘/连续3I/O错误只停自己的PGID。最近core验证最小外6,805,020,672B、内11,059,986,432B，下次构建重新量测。清理必须新manifest并确认归属；源码/日志/App/archives/主release不清。
- 证据 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`。已结束自有质量App/23840/23841不得误作活动；用户主App/服务不动。iOS质量模拟器FE04AC7C-B375-4F2E-AE7E-B6F8F5608947可保留booted。
- 内盘恢复入口 `/Users/zgh/Library/Caches/khor-code-quality/recovery/RESUME.md` 与 `current-code-quality-docs`。恢复先读用户指令→本STATE/PLAN→实际HEAD/diff/agents/PIDs→源SHA/日志。未提交源以现场为准，不按摘要覆盖。
- 本次校核各树：main8704clean；wave1 9543四root dirty；SSH/MCP a162新增fixture测试准备；native-io ed460 clean后准备inventory回归；root Q041c f7c12ae3clean。中央任务卡若未提交亦是当前工作，汇总前尊重所有者。


## 本次新增真实证据

- Q039 `pending-green-01`：6/147/12，51.42s/source46稳定；147含新增1unit，不重复计。最低外6,257,414,144B/内10,952,970,240B。三own PGID已结束，scratch空。root独立审见Q-039-pending-root-review。
- Q041 `real-bundle-darwin-02`：私有锁SHA cache npm ci成功，Bun1.3.14真实3084modules编译；MachO ARM64 76,474,082B SHAaf9555409de4af93cf166aeedc4b88f19196b5848dc89dd496ea0c650b79d646，三binary smoke全过。26agent tracked源/providers稳定，ignored依赖link恢复，19 observed PID全结束。首次npm config双/dev/null配置故障保留为setup失败。Rust嵌入/Linux未验，不外推。
- 当前inventory4源尚未类型/运行：删legacy fallback、typed InventoryNotAdmitted仅Versions触发Vouch，selected doc交换失败均报告；但opening/answer_versions加载提前失败仍待新增真实回归与修复。宏?控制流已纠正：远端已知doc load会整体Err，而非elsecontinue静默；unknown doc名称才有静默skip。不能把静态猜测当运行红。


## 最新收口：当前inventory已经集成（优先于上方历史角色描述）

- 用户最新“继续”，完整治理活跃；本轮仍没有main合并/ship授权。main基线8704保持，最终现场再核。
- native_helpers已交真实inventory测试78e978a8→34ef67c3。原effbb fixture两红：local accounts文件冲突报具体Err却Seen0/Pinfalse；remote整体inventory失败后client旧fallback吞optional错误最终Ok2160/Seen123/Pintrue。不能仅用macro?静态分析猜最终outcome。
- root修复1d2640d2：当前Versions单路径，移除sync_everything/legacy全量回退；只有Versions真实admit失败InventoryNotAdmitted允许一次签名Vouch+重试同inventory。普通API/Watch拒绝语义保留。双方inventory闭包捕获每doc load的?，typed DocumentInventory Version/Failed明确返回；client按原names顺序继续+首错，epoch隔离明确部分同步。新两类why走本地化，unknown OS诊断保留。
- `Q039/current-inventory-green-01`正常Node libcheck+166不同Rust通过（135Q045相关+6current+20proto+4realNode+1table_lost，子worker不重复），完整scripts入口nested31 OK、outer52=51pass+1skip。342.78s/source1060稳定；最低外8,357,208,064B/内5,834,117,120B。4 realNode包含新两红同fixture转绿、原25MB cap隔离及epoch隔离；table_lost签名自愈过。大差集本体仍cap拒绝。
- 非作者source审后仅修两proto注释，真实catalog gen_ts成功（355959B SHAa91e9c41…）并与ignored GUI生成文件逐字一致。`current-inventory-final-01`当前source重验normalcheck+6current+20proto+GUItypes全部exit0，1063源稳定含gen。重复26不另外计。当前proto e071，早期f67仅两注释差异有独立逆向还原精确SHA证明，明确不是当时快照。完整源审store_owner/限定证据审native_helpers均无新增阻断，卡已交回。
- MCP首红真实成立：`Q037/mcp-cancel/first-red-01`旧Rust同步MCP→真实IPC扣reply，Stop后Pi24.579ms ESRCH，但13.001s Saved仍running/map=true/claim_busy=true。失败前已显式释放socket+join fixture+确认production claim/map收口，未留300s等待。源码21份稳定，exit101/目标红，非setup/guard。当前3源fixture/cfg接线未生产修，Windows job继承对durable host的风险已核，不能简单把全部MCP套默认tree而杀掉成功创建的host。
- 原长上下文agent出现服务capacity错误，均停止代码/编译并交接：store_owner已确认新fix/quality-sync-apply@34ef树clean、无源码半成品；fanout_owner只保留MCP红fixture3源/设计部分待短交接；native_helpers已交完全部inventory证据/卡，无PID。新执行者须用本STATE/PLAN+具体卡/实际diff继续，不重造实验或凭摘要覆盖源。
- strict Doc apply仍仅方案，未生产实现。已确认before和on_disk covers base；candidate no-pending+before∨base∨target，保local并发/live旧pending，不替换Doc；FlushedImport不是Applied。shared+stamp Missing fail-closed，None须真实有界fence；pen退休失败需checked并在live导入前处理可预测的own-writer推进。Windows可用窄cfg FFI FileIdInfo完整identity比较，root已查官方来源，不能用mtime假身份。实施前仍核精确类型/失败阶段。

## 本轮资源整理回执

- storage-audit-20261026-7：新manifest确认无Rust compiler/open handles，删除13,343个quality-target闭合单链接第一方rcgu对象5,121,151,640B；630 archives/bin元数据保持，主release/source/App/logs不动。
- 外盘恢复约10.7GB后内盘后台可用降至4.72GB触守卫，所有新编译停止；vm.swapusage观测used10.37GB但不声称它是全部原因。
- storage-audit-20261026-8：仅自身internal incremental旧6个Node根内闭合单链接1724文件4,898,055,041B；保留最近2 Node根/884多链接/其它crate缓存与所有archives/bin。回执内盘4.72→9.62GB。后续统一CARGO_INCREMENTAL=0减少本治理新增内盘incremental，仍共用quality-target/jobs2；可用持续变化，每次重测，低于5GiB只停ownPGID。禁止动用户主App/服务/系统swap。
