# 第二轮当前状态

更新2026-10-30，目标 **完成第二轮 active：29反馈＋5遗留＝34项**。用户已授权实施；本轮没有main合并/ship授权。main仍 `e7c335c8dd81de091bd12b92cd704a63f25d03f3`。本页为最新协调入口，覆盖旧排队、旧运行及“只剩验收”的说法。

逐项标准见 [34项验收总账](ROUND-2-ACCEPTANCE.md)，执行规则见 PLAN、ROUND-2-EXECUTION、ENGINEERING。完整前态与证据细节保在 [337检查点前态](history/STATE-before-337-checkpoint-20261030.md)，沿其链接可追溯旧失败，不能回填原false。

## 最新磁盘决定（2026-10-31）

用户指出两盘总被压到5GB且目录乱。已暂停所有新增重型任务并完成定向清理，具体规则优先读 [STORAGE](STORAGE.md)，旧“完整二进制/gzip双盘永久保留”惯例已被替代。

- 外盘原Rust缓存约223GiB/旧实验58.5GiB；24旧cache实际释放104.7152GiB（du109.6048GiB另列），当前可用约112GiB。main/quality/catalog保护target、源码、worktree、会话和正式产物未动。
- 系统盘248个inactive私有程序/静态库及仅binary压缩包实际释放4.0037GiB；旧轮3195个重复metadata/map/log文件逐SHA一致后改指向外盘唯一原件，实际增加1.7753GiB。原日志未丢，当前轮source/docs/独有证据未动。recovery约8.3→2.2GiB，系统现可用约13.5GiB；16GiB RAM下swap当前约3.8GiB，余量会随内存压力浮动，未手动删swap。
- 清理事实/完整名单在 `/Volumes/disk0/Khor/maintenance/storage-20261030`。三maintenance任务36625/91738/53452均已实际complete，不恢复poll。reviewer已尾核范围与实际df算式，历史mirror只说明当时事实，不冒当前仍双份。
- 后续运行先外20GiB/内10GiB保留量＋显式峰值预估；原5GiB仍仅紧急中止。只读preflight脚本已审，6/2估算实际pass只是门验证，不是任何任务准入。原heavy暂停可在各自新合同和存储准入完成后逐批解除；不直接执行旧合同。

## 实际进度与剩余关键工作

Root `integrate/round2@c4aa4a3f6675b8a201c0479f54359998cd85f535` clean。337之后已独审整合H首片8a0fe08e：Node读取来源事实starts_message贯通Core普通与giant投影，9路径含4必要test模式适配，writer字节不变。只源码/parse检查，不冒编译、生成或分组行为已通过。增量bundle fccf6288、receipt3ee8a94e，verify0并双盘。当前179生成文件、278 dist资产仍是先前255→16ee lineage，因新增HistoryEntry字段已需最终真生成，不冒当前产物。

**尚在实现阶段，不是仅剩验收。** S3审计确认既定目标仍缺：跨次运行代次拒写、启动provider前唯一恢复reservation、显式能力驱动恢复入口、普通Shell新运行与前任关系。另需保留Load/Resume实际收到的新历史：Load完整新display base保旧代，Resume追加实际新Note，不再跳过启动记录。设计v2已独审；尚未实现，不称可用。

H读取首片及TS分组纯规则已整合（90a2→3d45f8ba、845c→0066e68d），阅读消费仍待。H原实际生产函数红例确认：两个合法同channel hel/lo分片在共享ACP投影合为hello，但canonical paint错误拆两条。需由读侧真实来源区别完整Native/Prompt与ACP chunk，保Turn/Gap、可选messageId、稳定折叠状态、分页阅读点与原文/copy预算。设计已独审，分组规则已修，Root实际25/25且独审；a5b9→300de46f已接有界1:1 displayKey、非首贡献导航及合法required DTO；olderanchor/find-copy消费尚未接完，AI原断言保持且未重验。该红例不能冒AI折叠失败唯一原因。

用户最新询问进度，已修正剩余估计为 **3–5个工作日**（包含桌面/手机实际验收，配合验收等待另计）。此前1–2天偏乐观，遗漏上述实现缺口。范围保持34项，不追加独立问题。

## 现在的并行源码写权

仅两作者＋非作者审查，Root协调集成。无Cargo/native/生成/browser/App运行准入；运行槽FREE。不得因准备合同过期反复构建。

| owner | 实际分支及本次边界 |
| --- | --- |
| Root | `khor-round2` / `integrate/round2@c4aa4a3f`；协调、有限集成、总账。 |
| H | 同原worktree，保旧6eda分支，新 `fix/round2-chunk-grouping@33738990`。首读取片8a0fe08e固定、非作者通过并Root整合ae9030a2后，已正式交还三Node叶与4test；继续已设计TS投影/身份/锚点/find消费端。禁止writer/GUIHost/ACP改动；设计系统已读。 |
| input | `fix/round2-recovery-contract@33738990`；首片仅 `crates/node/src/live.rs`、`gui_host.rs`、`lib.rs`：RunKey、短row OS gate、旧report/exit同临界区拒写、必要cfgtests源码。首片3efd已独审精确整合Rootc4aa：GUI state内嵌GuiRun单Meta原子快照，GUI读无锁不filter_map缺行，短gate只RMW写且旧代拒；set_pid不退GUI代，显式takeover等host结束→reseat→PTY。5反例只有源码/parse检查，未编未跑；begin_run仍在Journal前，后片事务顺序必须修，不能冒S3完成。旧state数据保留，无新cache/service或全Shell重构。H三读取叶/4test已交还，可顺序接入8a0fe08e；首RunKey片固定交审后，获授下一有界reservation/Load事务slice（含必要ACP capability guard源码），不得混成大提交；Shell/public routing尚未授。 |
| reviewer | 非作者只读审小片、来源/预算/并发和具体Clean Code边界；不写产品/不自动重跑。idle用followup_task唤醒。 |

恢复Core/API/UI阶段尚未授写。先稳定Node合同，再由唯一owner接线、一次真生成。不能两作者同时写共享叶，不能靠大冲突收尾。新独立问题仅NEXT-ROUND。

## 最新真实验证

| 验证 | 实际结果与边界 |
| --- | --- |
| Store | 95235 terminal0，16父＋2辅助。真实1025×64KiB=67,174,400B payload完整写入canonical、有界读取/Ready有限前缀/关闭释放；已独审双盘，不代表完整App/S。 |
| 前端 | 11451 terminal0/15.224s，完整noEmit0/Vite0；frontend receipt087f4263。gen255→frontend16ee来源准确，后续test-only差明确记录。 |
| ACP定向 | 63174 terminal0，3exact各1pass；原wrapper类型错失败保。 |
| Node定向 | 60432 terminal0/175.606s，13父＋7辅助；actual stub＋3Node selected均真实no-prior验证，来源/清理闭合。result45e9a355。 |
| 恢复界面 | 98901 terminal0，13项真实ChatView/Capture/H reader/paint over controlled transport；result23b3e579，不冒WK/App/系统Save/全S。 |
| 集成身份 | Root300实际26/26 exit0/0.228s，9源前后同；resultfec4d49d/log2552a53e双盘独审。a5b9源片独审整合；增量bundle d956099a7dd2d69d726be5e3e2295476e3f2fc14a75149a17922ba33e84879a9/receipt 2d4ebdd0cfcb0a9e5d37377aaff3ed58ec4f5e0f31ab4985f46bef6397893799，verify0双盘。不冒真实锚点或fullAI。 |
| 集成纯投影 | Root0066实际node check-history-reader exit0/0.213s、25/25、7源前后同；result ac550fe3/log7323bde6双盘独审。原23与新增边界/mixed-ID断言保留；不冒fullAI/ChatView身份/anchor/find-copy/gen/types/native。增量bundle 8e209561c61d20a8c1be68723f74c4f653bdab81cf54553efb64f8fb0e445f52；receipt bbf45bdf1731971ef0be324945848499197755aeaf94bf7212b48832dda5a9d1。 |
| 完整Read | 62914 terminal0/17.6122s，完整原脚本，正文主题/窄屏、tool/blockquote与machine实际探针通过；result4ceefb05。 |
| 完整ACP消费者 | 74275 terminal1/118.113s。配置/权限/NoLoad/login等已有实际通过，附件旧chip探针0/0及旧wizard入口失败，未全绿。main与337上传实际源码相同：通用附件发送存储路径，inline image受images控制。H仅按实际upload完成、真实chips、发送路径/次数和现有registry控件迁移test；不能伪造embed覆盖。 |
| AI4 | 旧13652首组manual-collapse失败保。当前300实际76618 terminal1/36.775s，首组越过manual-collapse与keyboard-open后，在line167 exact下一正文可见30s超时；未四组完成。result524627e1，source/env/tools/gen稳、owned空/TMPabsent，原失败全保。failureJSON只有focus/tools，尚无body/network，原因未知；H获授有界失败诊断-only片后再定必要实际，原assert/timeout/queue/4contexts不变，不加anchor新case混测。 |

消费者三结果汇总 `history/session-recovery/consumer-three-current-337-results/results.json` SHA b6aee6ee；Read4cee、ACPa0b1405e、AI8ee047dc，均已独审镜像。严格源/env/gen稳定、owned空/TMPabsent。旧41905/48444/7229/96347失败保留，不冒绿。

## 原生与实际平台仍待完成

没有合格的当前337整包GUI/Lab。旧84生产、16ee成功CLI、16ee GUI实际Cargo0但来源门失败，各自准确保留，不互相冒用。

- 61197 terminal1：CLI实际0＋原V/copy；GUI外盘原5GiB门中止，非产品断言。48383 GUI-only terminal1/103.539s：Cargo0但原V拒旧生成缓存。只读聚合已确认唯一GUI精确OUT的hash资产复用；最终源稳定后，仅该OUT完整SHA/身份/lsof核并可逆retain，再真实cold构建。未retain，旧16ee准备已过期，不能直接启动。
- 最终顺序：H/S源码与独审集成→真实gen/catalog一次→noEmit/Vite与受影响consumer→C/Core定向原生→最后当前CLI/GUI生产构建。C会改shared target CLI，所以在最终生产前。未受影响Store64MiB/A/B/旧H不重跑；H读取变化只跑实际影响的原文/预算/投影回归。
- 完整S仍需同host正常Quit重挂、host退出/重启/断线、显式恢复/新Shell；当前NoLoad准备只拟验证其中一小场景。私有home/peer，不操作用户真实会话，不用真实模型/API。
- E真实Mac IME/路径选区/草稿所有入口与保存退出仍待；passive observer需要匹配当前devUrl GUI变体，默认GUI不能冒接线。系统Save/Copy仍待，A5只有production origin＋Find partial，原Save超时无Copy/正常Quit。不开自动A6。
- iOS正式archive/signing流程已有有效基础，旧包仅模拟器。本轮physical私有Lab与reachable设备尚缺，tunnel/DDI unavailable不推唯一SDK原因。需真机连续手势/键行/viewport/fixed底栏/滑删验收。人验前重新询问可用性，旧回答不复用。

## 资源与压缩恢复

共享专用quality-target、jobs1/incremental0/offline；新STORAGE保留量＋峰值预准入优先，原5GiB紧急门不降低。已完成的精确缓存可恢复归档见前态，停止扩大空间治理；用户release/源/会话数据不清理。旧16ee cold合同、C/Core/E/S草案均不自动成为当前运行准入。

内盘 `/Users/zgh/Library/Caches/khor-code-quality/recovery` 同步STATE/实际refs/runtime及逐文件SHA镜像；dirty任务卡按文件快照，不声称全局原子。读STATE/PLAN后核实际branch/diff/句柄，不用旧runtime排队文字恢复启动。34项逐项对账＋非作者Clean Code审＋当前集成版本及必要真实平台证据齐备才完成。

源码交接补记：H42e诊断单叶、87d6可信record元数据已源码独审可整合，尚未Rootcherry。H3cd消费片有完整record混后续未知SDK尾时copy-complete误判候选，reviewer已要求窄修；跨page query/whole-group与真实文字点anchor仍OPEN，不以拒绝copy替代原目标。input第二reservation/Load已授source-only，可写remote两真实binding调用与acp_process真实PID/出生事实传出、opt-in stub方法名日志/原unsupported case；无新runtime，公开Core恢复API/Shell路线仍待后续交接。
