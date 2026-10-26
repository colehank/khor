# 当前治理状态

更新：2026-10-26。完整治理活跃，尚未完成。先读本文件与 [PLAN.md](PLAN.md)，再核现场 HEAD、diff、agents、PID 和证据指纹。详细历史见 [前一检查点](history/STATE-before-mcp-worker-runtime-20261026.md)；任务事实以当前卡及真实日志为准。

## 用户决定与边界

- 用户要求完成全仓功能与代码治理，授权多智能体协作、分批重构及验证，完成目标前持续工作。
- 最新决定：“不必考虑旧端兼容，我们还没有上线，按着纯净的开发就行”。统一当前 Khor 协议，删除旧 Khor fallback 与迁移矩阵；平台适配、外部 ACP 能力协商、认证、epoch 隔离和数据完整性持续保留。
- 本轮没有 main 合并或 ship 授权。main `/Users/zgh/Desktop/working/khor` = `8704a503a27b36ad58f922690de50e766f8bd485`，本次现场 tracked clean。发布 GUI SHA `9c3e078f355aa65e3f9d0137ae38548a1f2d75d21bf261528335e031ee6dbc01` 只证明既有发布版。
- 本目录、分支 `plan/code-quality` 是权威协调入口。实现树同名文档只是快照。复用现有 worktree，资料放 `/Volumes/disk0/Khor` 对应分类。

## 集成事实

集成树 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，分支 `integrate/code-quality-wave1`，HEAD **927422502316576b75405a7b65b140b4dc535fd9**，本次 tracked clean。

本次新增集成：当前 inventory `1d2640d2`、P4 test-only hop helper `5fe7869a`、严格 Doc apply `6e82847f → 92742250`。此前 GUI replay/gate、Q045 SSH 生命周期、blob core、Q041c、pending 完整性等保持。作者分支绿仅适用于其精确输入；9274 全组合尚未重新验证。

| 范围 | 已确认事实与边界 |
| --- | --- |
| Q010/11/15 GUI | 真 Tauri iOS run11验证键盘/历史锚点/commit第6、8跳转/native page。后续 Safari 长稿164px真红已修，候选 JS 在真实 MobileSafari/UIKit 26指标通过，ACP/Khor各6次 settled gap0；组件36/scroll34/types绿。物理 iPhone/IME和最终全源 App仍待。 |
| Q020–Q040既有修复 | 自动保存/冲突/锁刷新、身份首建/purge、FIFO/连接owner、凭据裁剪/A2A/输入预算、上传receipt/poll等分批集成。各任务卡保留覆盖；Q033真正GUI→Rust上传取消、Q035 borrow lease未关闭。 |
| Q043 native helpers | 双引擎22+38、原c371 Rust质量App配候选JS的7listen/7unlisten与child WK title通过。OS drag/windowclose/fullscreen和最终全源App未知。 |
| Q037阶段2/replay | Darwin unreaped退出窗口、ACP事件fence、host取消、GUI writer及完整历史replay已集成；真drip取消约2.015s并同provider下一轮，Closed/Ready先红后绿。root双引擎replay26/26，七源稳定，无console/page/network errors；首轮favicon404失败保留。内部SDK总RSS/host listener不宣称全部有界或join。Windows harness有Mac10/10，Windows runtime未跑。 |
| Q039 pending | pending-green-01：6+147+12相关Rust通过；147包含pending unit，不重复计。真实absorb→compact遗失pending block已修，未把pending当已应用。 |
| Q039 current inventory | 删除Versions→legacy全量fallback/双路径，仅Versions admission失败可typed InventoryNotAdmitted触发一次签名Vouch后重试。双方逐doc捕获load失败，继续其它doc并保留首错；epoch明确部分同步。原两红分别local报错但Seen0/Pinfalse，remote fallback吞错成Ok；现均具体Err且Seen123/Pintrue。current-inventory-green-01 normalcheck+166不同Rust及完整scripts绿（nested31；outer51pass+1skip），1060输入稳定。最终两注释调整后normalcheck+6current+20proto+GUItypes通过，1063输入含真实catalog生成稳定。大差集25MB本体仍cap拒绝，尚未收敛。 |
| Q039 strict Doc apply | apply_checked 校验epoch、before及on_disk covers base、候选无新pending且covers before∨base∨target；保本地并发/旧pending，不替换live Doc。真实有界fence、checked pen退休、merge+flush返回私有FlushedImport，明确不是网络Applied/power-loss receipt。doc-apply-red-01真实pending compact红；green-01 22Doc+6pending+147lib+12transfer =187不同普通Rust通过，9源冻结/44输入稳定，子worker不重复计。非作者源和日志审无阻断。Windows身份比较仅静态核API，runtime未知。 |
| Q041 install/build | installer31/mirror13/site7真实scratch及独立审；未远程部署。Q041c预检17、真实Darwin Bun1.3.14 artifact 76,474,082B SHAaf955540…三smoke通过，26agent源稳定。完整fresh-source onset/Linux/musl/Rust嵌入链仍待。本次MCP首次CLI build因无GUI dist失败，正常同源frontend producer已成功，登记Q041d后续改构建入口，不算业务红。 |
| Q046 scope | hop::middles仅6个测试调用，projection移cfg(test)，生产routing算法未改。helper-scope-green-01 normalNodecheck+既有hop9/9，源稳定，独立审无阻断；5fe7869a已集成。 |

## 当前所有者与下一动作

| 所有者 | worktree / branch / 基线 | 当前任务 |
| --- | --- | --- |
| root | wave1@92742250；plan总账 | 公共接口、独立审、集成与恢复。源码集成后按影响范围验证，主发布版保持。维护Q041d fresh build缺失GUI prerequisite。 |
| mcp_worker_owner | `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`，fix/quality-mcp-cancel@7550e4f0 +16源dirty | Q037 owned MCP worker/Running Control/hidden CLI+GUI hook、remote sweep、Windows typed durable handoff。当前唯一Cargo/gen槽。冻结145036源，2047输入含同源frontend/dist。CLI/stub/normalcheck/test compile通过，原真实IPC取消回归1父通过；worker/generation/approval/no-id/default进程检查继续。卡Q-037-mcp-worker.md。 |
| doc_apply_owner | `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery`，fix/quality-transfer-initialization@92742250 +transfer.rs test-only | strict apply已交付。现Q039初始化crash恢复设计+首红fixture477b88e6，生产prefix未改。single exact .initializing、全gate、白名单控制、atomic no-replace方向认可，平台primitive待核。无Cargo槽，未授生产。卡Q-039-transfer-init.md。 |
| doc_apply_review | 只读同上固定源；独占review卡 | MCP145036源审无阻断，原queued approval/cancel次序阻断已修。transfer init首红fixture/设计独审无阻断，写Q-039-transfer-init-review.md；不编译。MCP真实结果最终复核仍需派发。 |
| 旧fanout_owner/store_owner/native_helpers | 已交回源/卡/PID，无活动编译 | 大上下文capacity错误后停止，不自动重试。已有原始证据/refs保留。新派发使用最小上下文。 |

### Q037 当前精确证据

- 原first-red-01：真实McpSession→真实IPC Handoff/Get扣reply，Stop后Pi24.579ms ESRCH，但13.001s Saved仍running/map=true/claim_busy=true。失败前释放owned socket/join并确认生产claim/map收口，未留300s等待。
- 新candidate-20261026T145036Z：16源码冻结，khor_agent 88036f8e…；after-event只poll_worker，完整approval/heartbeat仍idle，防排队cancel前批准写入。新queued approval测试覆盖此契约；缺RPC id原有guard未丢，root错误静态推断已撤回，no-id新增项仅行为覆盖。
- build-01因本树缺apps/gui/dist/index.html而setup失败，旧CLI未用于测试。随后正常npm offline prebuild/gen_ts/gen_bindings/tsc/Vite exit0，1615输入stable；277 dist逐文件SHA与producer记录。无旧dist借用或npm下载。
- build-02真实CLI SHA98b86989…/257,287,672B，stub22d438cb…/15,578,504B；normalcheck及test-compile-01通过，Node test binary5edbb33c…/229,015,368B。2047输入稳定，jobs1/offline/incremental0。
- ipc-cancel-green-01 exit0/2.690s：1普通父+实际专用子（不重复计），真实IPC仍扣reply时Pi35.786ms ESRCH并下一Prompt done/第二Pi ESRCH，最后才释放IPC/join/claim-map drain。16Stop仅一个sweep owner，late释放不改新receipt。其余检查未完成，不标全部Q037完成。
- 本地worker可回收不等于远端取消确认：remote sweep每run单独owner、最多4handles/5s，pending/confirmed=false/scheduled或not_admitted原因保持。公平轮转/可查询远端receipt仍开放；部分后台handles在下次注册/进程退出join，不能声称即时全部join。Windows runtime未知。

## 仍开放的完整目标

1. Q037完成真实worker/批准取消/默认ACP-Pi-host/进程检查及独立证据审，再commit、集成及精确组合验证；远端取消receipt/fairness继续。
2. Q039真实初始化中断红→恢复实现；当前DocTransfer descriptor/canonical VV/outbound spool/Node chunk、公平轮转、25MB真实收敛及Pair bootstrap。Stored和FlushedImport不能冒充网络Applied；不抬帧上限绕过。
3. Q041完整fresh-source构建与制品来源；Q033 native上传取消、Q035 borrow lease；P1其余深审、P4工程约定与历史代码分类、P5最终同源App/用户流程。
4. 每个任务关闭须满足PLAN规定的实现、适用检查、独立审、集成验证和必要App验收。中间基座不替代全仓完成；物理iPhone/IME/Windows/远程部署未知项明确保留。

## 资源与恢复

- 唯一Cargo/native/gen槽属于mcp_worker_owner，其余不编译。统一quality-target，CARGO_INCREMENTAL=0；当前jobs1降低峰值，平时上限2。切树更新第一方mtime并核SHA不变/日志真实重编，防共享target陈旧artifact。
- 内外盘可用每次须≥5GiB（5,368,709,120B），掉盘/连续3I/O错误/容量越线只停自有PGID。外盘风扇坏/SMART不可读，用户允许继续；不动主App/服务/release/system swap。
- 新storage-audit-20261026-9精确清理治理internal incremental：6805单链接+2518全部link均在同一managed cache的hardlink组，共11841paths/4,283,499,880 unique bytes；逐stat/link/无compiler与open handles核对。内盘3,915,485,184→8,224,882,688B。源码、证据、archives/binaries、用户服务未动。旧7/8回执仍保留；清理须新manifest，不复用旧授权路径。
- 本次构建测得最低内约7.12GB/外约6.99GB；可用会变化，下次重新量。vm.swapusage曾used10.37GB只是观测，不当成所有容量变化原因。
- 全证据 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`，活动任务另有内盘源快照。恢复入口 `/Users/zgh/Library/Caches/khor-code-quality/recovery/RESUME.md` 与current-code-quality-docs只做应急副本；外盘dirty source不得按摘要覆盖。
