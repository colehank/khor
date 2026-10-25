# 当前状态

更新：2026-10-25 · 用户重新激活完整治理目标，继续执行。

## 目标与边界

完整范围仍为 [PLAN.md](PLAN.md) P1–P5：全仓审查、分批功能修复与结构治理、工程规范和整体验收。不能以首批或单项通过代替全面完成。[AUDIT.md](AUDIT.md) 保存发现与后续优先级。main/发布没有本轮授权；独立分支和隔离验收继续。

## 当前路径与版本

- 主仓：`/Users/zgh/Desktop/working/khor`，main `8704a503a27b36ad58f922690de50e766f8bd485`，tracked clean。
- 权威协调：本文件所在目录，plan/code-quality 分支。其他 worktree 的同名文档是快照。
- 集成实现：`/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，integrate/code-quality-wave1，当前 HEAD `3d10bbaff062b0ccb32b3b9d36d5432275812b20`（后端Q033最终SendCommitted已集成；GUI待审）。
- 内盘恢复：`/Users/zgh/Developer/Khor/worktrees/recovery/`，4 个恢复副本，均保留真实差异；禁止重建旧 Desktop/working 路径或按摘要覆盖。
- 外盘资料：`/Volumes/disk0/Khor`。存储与迁移索引见 [STORAGE.md](STORAGE.md)；内盘备份 `/Users/zgh/khor-backups/{disk0-20261025,working-20261025}`。
- 上次 ship 的 Mac 二进制仍在主仓 target/release/khor-gui，SHA-256 与发布收据一致；没有把它当成本轮修复版原生验收。

## 已集成与确认的证据

| 任务 | 当前结论 |
| --- | --- |
| Q001A/B/C | 首轮结构盘点与重点抽样完成；GUI 525 文件清单/42 重点文件，后端 165 文件清单/36 关键路径；尚非全仓深入审查 |
| Q010/Q011/Q013 | 首批滚动、提交跳转、Lab 已提交并集成。固定版本浏览器 Chrome/WebKit 26/26，通过对应版本 gen/types/unit/build；Q010b已在真正Tauri iOS模拟器取得键盘红绿与真实touch-hide绿、commit真实触摸通过；物理iPhone仍待验 |
| Q020 | 自动保存摘要回环、取消与编辑线隔离已集成，24 回归与独立编辑器检查通过；显式“用我的覆盖”另列 Q020b |
| Q021 | 已知锁状态刷新失败保护已集成 882e3c50；首次未知失败恢复在 Q021b |
| Q022 | 身份原子首建 e89da2e6 已集成；真实 Cargo 9 通过+1 专用 worker、64 子进程；断电目录项持久性与 Windows 实跑不作保证 |
| Q023 | dry-run、purge 范围和现代指纹 hostname fallback 拒绝已集成至 9b175b42；安装 binary 的真实 owner 证明仍待 Q023b |
| Q024 | netlab 失败/空跑退出码、mobile 静态/编译分层、质量 CI 与平台 Tab 修正已提交；GitHub CI 实跑、最终完整交互和 Q025 native/mobile 静态收尾仍待完成 |

## 本轮实际分工

| 任务 | 所有者 / 工作副本 | 现场与下一步 |
| --- | --- | --- |
| Q021b | root；内盘 recovery/khor-quality-recovery | 作者提交 4c616670、集成 c3fe828b；20 hook、types、unit和独立审查通过，Chrome/WebKit 4个App场景通过。确切集成版Chrome/WebKit全交互30/30通过，源码指纹稳定；RECOVERY.md 文档另留 |
| Q023b | store_owner；内盘 recovery/khor-quality-store-owner-recovery | owner/Node/CLI/onset 严格回执已落盘；上次 80 场景红绿，rustfmt 后待复跑；真实CLI精确测试1/1通过，已释放槽；轻量最终80/80通过，作者提交fb0c6700；root独立源审/6规则+80场景复验通过，已集成7251f2db，精确集成回归待收齐 |
| Q030 | root 复审集成；外盘 current/khor-quality-agent-protocol | 作者提交 9d09899c，仅4个输出文件；真实整组8通过+1专用worker。root 非作者审查记录 Q-030-review.md；已集成 de27d3f3，Q031差异保留，集成版对应测试待跑 |
| Q030b | agent_redaction；同上独立后续提交 | 作者提交63f31737；真实Cargo17通过、2专用子worker由父测试执行；live receipt确认中途无凭据前缀，结束完整redacted。root独立源审通过，已集成3f73f8ec，等待精确集成回归 |
| Q010-native | native_acceptance；wave1 仅hook/对应测试可写 | 上次用户暂停时 Cargo SIGTERM，未安装/启动隔离 App，不计通过。真正App已跑原生键盘开合与第6/8 commit稳定定位；又取得ACP长稿关闭gap164真红，允许仅hook/对应测试做JS候选红绿。旧c3基线证据保留，不把整套报绿；通知初始化拒绝Q015另列 |
| Q025 | root接手；外盘 current/khor-quality-native-io | queue/connection/chat/term/native dispatcher 作者2ab71a6c→集成918127ea；最终core8/8、std7+4、native参数/ACL2/2均通过且source稳定；独立审查完成。扩跑全core123过2失败1基准忽略，另列Q036，不报全绿 |
| Q020b/c | root审完已集成；内盘overwrite-recovery现复用Q036 | 作者cab465c5/93438e43→集成919e6194/355b7e3a；模块30、作者组件26通过；已全局登记，集成types/unit通过，精确集成overwrite Chrome/WebKit26/26通过 |
| Q031 | agent_redaction；外盘agent-protocol | 作者d464f6be→集成908d60e4；真实Cargo4通过、1专用子worker由父执行，JSONRPC/REST跨进程恢复有证据；root独立源审通过，精确集成回归待跑 |
| Q032 | store_owner；内盘web-input-recovery | 作者5ce2721f/c4ec0fa1→集成d82d4a47/0e667094；真实Web26/26与vendor5/5过，root独立审查/指纹复核过。固定8KiB清理内存，不声称解决慢连接deadline |

## 正在推进的下一项与集成记录

- 集成HEAD为3d10bbaf；main仍原8704a503。Q035 CI空channel兼容cc3a3295已提交并以已有内盘Chromium4/4验证；外盘cache缺headless shell那次为环境失败，未下载。
- Q034/Q035/Q036已集成，详见任务卡；真实App资源句柄和Q015通知cap仍待精确集成App重建验证。旧浏览器/旧App结果不移作最新HEAD证据。
- Q033 backend：21994830→dd15b9d1，SendCommitted后续3d0dc41f→3d10bbaf；独立复审通过。真实core15、gui_host gated链4、Node3、terminal handshake2、native2均通过；HTTP null仅admission，GUI各transport均等commit帧清pending。旧host只能退役未commit上传attachment再显式重连；不谎称已commit取消。
- Q033 GUI：store_owner作者3f2b8b18+8da0d063+7746378f，内盘store-owner-recovery干净冻结。root审出丢弃/加入/切tab/关后重开四种旧retry可再发，作者取得4真红再修；新增选择性notice保护，作者52/52后独立审查又复现旧cancel撤掉新notice，作者774补对象身份保护与正例，最终Chrome28+WebKit28=56/56及types通过。agent_redaction正在非作者复审最后补丁；未集成，门登记由root负责。root App retain与5条catalog、registry基座已集成。
- Q038：root作者29fcad93→集成c5ab273d；固定预算stdio叶7/7、独立11/11、真实khor_agent Cargo25过+2专用worker由父执行，8MiB真实sidecar过；独立复审完成。不代替取消期限或总RSS治理。
- Q037：root复用外盘native-io，fix/quality-agent-cancel-deadline基于3d10bbaf。真实Prompt/Stop fixture确收abort仍运行15s，已取得目标红；首个token非法导致的setup失败不算业务红。common supervisor/Unix WNOWAIT anchor/可取消nonblocking pipes/独立serialized input已真实编译；首轮已可Stop后回收，但下一正常快速done因Darwin zombie-only killpg EPERM失败，原日志保留。正补仅唯一未reap leader的保守macOS分支及更多真实管道测试。native_acceptance Windows Job/overlapped pipe作者b51a7d9c→实现分支55d72f50，root补Drop等待与公共supervisor，窄Windows交叉check通过；Windows实跑未有。store_owner只读审第一候选；MCP同步tool与delegation远端取消阻塞仍待处理，不能仅侧进程绿即关Q037。
- Q039：agent_redaction研究提交fcd0ad29，外盘agent-protocol的research/quality-sync-catchup。真实合法25.4MB差集超过4MiB并重启不收敛，反向小更新与epoch正常；实验测试为明确执行的研究红，不登记为已过门。生产协议未改。分片完整blob候选及输入/缓存/RSS边界写卡；固定chunk不代表端到端内存有界，需先审兼容与预算政策。
- ea0e2c57的精确集成证据：types、unit（含save-road30）、lock-state20、mobile静态、std11、overwrite26与scroll34均过，source稳定；Python14中1项仅因平台跳过，store-owner Rust/SSH确实执行。后续ChatView/retention修改后按影响范围再验，不能直接沿用为最新HEAD全通过。
- 集成仅遇50-gates.md尾部追加冲突，已保留锁检查与native I/O两条；无业务实现二选一。WebKit缺默认cache那次是环境失败，使用已存在外盘cache后26全过。Q035新增门原先硬编码Chrome，root正在补空channel兼容；首次CI Chromium启动又因外盘cache无headless shell失败，改用已有内盘cache验证，不下载。

## 验证与资源

- 证据根 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`，各任务卡记录确切 source/lock hashes 与平台边界。
- 同时只有一个Cargo/native构建槽；当前槽由root持有供Q037真实Node回归，Windows窄check已结束，禁止另起完整编译；native阶段仅Swift driver/UI诊断，下一次Rust重编需重新申请。jobs=2，沿用 quality-target；主发布缓存不写、不清。
- 外置盒风扇故障、USB SMART 温度不可读；用户明确允许继续。已按manifest逐项清理quality-target旧single-link first-party rcgu对象约5.25GiB，保留16个现存rlib/测试二进制并复核元数据；最近外盘约9GiB、内盘约13GiB可用（启动下一构建前复核）。各构建仅监测/停止自己的进程组；掉盘、连续 I/O 错误或低于5GiB时保存日志并停止该构建。
- 不按旧 PID/句柄判断进程。已在隔离iOS模拟器实际运行本轮c3 App，commit第6/8项真实触摸通过；ACP长草稿收键盘gap164→0已红绿，真实touch hide/reader drag也通过，候选093d4115；精确当前集成App仍待重新构建验证。不得把未完成候选或旧Lab当完整原生验收。
- Q021b 的 face gate 上次被用户暂停中断，不计通过；dialogs、words 与 types 已过。新的局部与集成结果须按当前版本记录。

## 尚未完成的完整范围

首批原生/物理设备/辅助技术验收；新发现Q015通知插件初始化ACL拒绝、Q033上传停止仍可能启动turn；Q020b/c/Q023b/Q025/Q030b/Q031/Q032；native page迟到归属、chat轮询结束/退避、sidecar取消期限、4MiB同步缺口恢复、once锁回收；工具链与产物可复现、installer checksum和mirror一致性；模块职责重构、格式/命名/错误/注释规则、剩余模块深入审查，以及最终确切集成版本的完整用户流程验收。完整清单不因本轮优先项缩减。

## 历史与恢复

本文件只保留当前事实。暂停/拔盘/两次目录整理前的总账快照见 [history/STATE-before-resume-20261025.md](history/STATE-before-resume-20261025.md)，详细过程读单任务卡。恢复时先读当前用户指令、本文件、PLAN和对应任务卡，再核对真实 worktree/HEAD/diff/进程；历史摘要、通过日志和图谱风险分数均不能替代当前证据。
