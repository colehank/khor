# 当前状态

更新：2026-10-25 · 用户重新激活完整治理目标，继续执行。

## 目标与边界

完整范围仍为 [PLAN.md](PLAN.md) P1–P5：全仓审查、分批功能修复与结构治理、工程规范和整体验收。不能以首批或单项通过代替全面完成。[AUDIT.md](AUDIT.md) 保存发现与后续优先级。main/发布没有本轮授权；独立分支和隔离验收继续。

## 当前路径与版本

- 主仓：`/Users/zgh/Desktop/working/khor`，main `8704a503a27b36ad58f922690de50e766f8bd485`，tracked clean。
- 权威协调：本文件所在目录，plan/code-quality 分支。其他 worktree 的同名文档是快照。
- 集成实现：`/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，integrate/code-quality-wave1，当前 HEAD `c3fe828b`（Q030/Q021b 已集成；仅放开 Q010b hook/对应回归，其他集成等待原生候选完成）。
- 内盘恢复：`/Users/zgh/Developer/Khor/worktrees/recovery/`，4 个恢复副本，均保留真实差异；禁止重建旧 Desktop/working 路径或按摘要覆盖。
- 外盘资料：`/Volumes/disk0/Khor`。存储与迁移索引见 [STORAGE.md](STORAGE.md)；内盘备份 `/Users/zgh/khor-backups/{disk0-20261025,working-20261025}`。
- 上次 ship 的 Mac 二进制仍在主仓 target/release/khor-gui，SHA-256 与发布收据一致；没有把它当成本轮修复版原生验收。

## 已集成与确认的证据

| 任务 | 当前结论 |
| --- | --- |
| Q001A/B/C | 首轮结构盘点与重点抽样完成；GUI 525 文件清单/42 重点文件，后端 165 文件清单/36 关键路径；尚非全仓深入审查 |
| Q010/Q011/Q013 | 首批滚动、提交跳转、Lab 已提交并集成。固定版本浏览器 Chrome/WebKit 26/26，通过对应版本 gen/types/unit/build；真正原生 App/物理 iPhone 待验 |
| Q020 | 自动保存摘要回环、取消与编辑线隔离已集成，24 回归与独立编辑器检查通过；显式“用我的覆盖”另列 Q020b |
| Q021 | 已知锁状态刷新失败保护已集成 882e3c50；首次未知失败恢复在 Q021b |
| Q022 | 身份原子首建 e89da2e6 已集成；真实 Cargo 9 通过+1 专用 worker、64 子进程；断电目录项持久性与 Windows 实跑不作保证 |
| Q023 | dry-run、purge 范围和现代指纹 hostname fallback 拒绝已集成至 9b175b42；安装 binary 的真实 owner 证明仍待 Q023b |
| Q024 | netlab 失败/空跑退出码、mobile 静态/编译分层、质量 CI 与平台 Tab 修正已提交；GitHub CI 实跑、最终完整交互和 Q025 native/mobile 静态收尾仍待完成 |

## 本轮实际分工

| 任务 | 所有者 / 工作副本 | 现场与下一步 |
| --- | --- | --- |
| Q021b | root；内盘 recovery/khor-quality-recovery | 作者提交 4c616670、集成 c3fe828b；20 hook、types、unit和独立审查通过，Chrome/WebKit 4个App场景通过。确切集成版Chrome/WebKit全交互30/30通过，源码指纹稳定；RECOVERY.md 文档另留 |
| Q023b | store_owner；内盘 recovery/khor-quality-store-owner-recovery | owner/Node/CLI/onset 严格回执已落盘；上次 80 场景红绿，rustfmt 后待复跑；真实CLI精确测试1/1通过，已释放槽；轻量最终80/80通过，作者提交fb0c6700；root独立源审/6规则+80场景复验通过，冻结窗口结束后集成 |
| Q030 | root 复审集成；外盘 current/khor-quality-agent-protocol | 作者提交 9d09899c，仅4个输出文件；真实整组8通过+1专用worker。root 非作者审查记录 Q-030-review.md；已集成 de27d3f3，Q031差异保留，集成版对应测试待跑 |
| Q030b | agent_redaction；同上独立后续提交 | 作者提交63f31737；真实Cargo17通过、2专用子worker由父测试执行；live receipt确认中途无凭据前缀，结束完整redacted。root独立源审通过，等待集成窗口 |
| Q010-native | native_acceptance；wave1 仅hook/对应测试可写 | 上次用户暂停时 Cargo SIGTERM，未安装/启动隔离 App，不计通过。真正App已跑原生键盘开合与第6/8 commit稳定定位；又取得ACP长稿关闭gap164真红，允许仅hook/对应测试做JS候选红绿。旧c3基线证据保留，不把整套报绿；通知初始化拒绝Q015另列 |
| Q025 | root接手；外盘 current/khor-quality-native-io | queue/connection/chat/term/native dispatcher 未提交，std-only7+4通过；配置超车修复后core8/8通过，paste锁等待取消新增修复后尚待最终core重跑；命令普查2/2、mobile静态、真实native参数/ACL2/2通过且source stable；第三方已审通过，待root复跑/提交 |
| Q020b/c | root待审；内盘overwrite-recovery | 作者提交cab465c5/93438e43，最终模块30、组件Chrome/WebKit26通过；新入口待全局登记，冻结结束后集成 |
| Q031 | agent_redaction；外盘agent-protocol | 作者提交d464f6be；真实Cargo4通过、1专用子worker由父执行，JSONRPC/REST跨进程恢复有证据；root独立源审通过，等待集成 |
| Q032 | store_owner；内盘web-input-recovery | 第一阶段提交5ce2721f，流式预算候选18/18轻量绿；发现tiny_http析构按Content-Length分配的真红，已授权最小vendor补丁与锁依赖边。唯一Cargo槽已交其做真实Web/HTTP验收 |

## 验证与资源

- 证据根 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`，各任务卡记录确切 source/lock hashes 与平台边界。
- 同时只有一个完整 Cargo/native 构建槽，当前分给 store_owner（Q032 Web/HTTP；Q025 native已退出0）；native阶段仅Swift driver/UI诊断，下一次Rust重编需重新申请。jobs=2，沿用 quality-target；主发布缓存不写、不清。
- 外置盒风扇故障、USB SMART 温度不可读；用户明确允许继续。最近外盘约11GiB、内盘约13GiB可用（启动下一构建前复核）。各构建仅监测/停止自己的进程组；掉盘、连续 I/O 错误或低于5GiB时保存日志并停止该构建。
- 不按旧 PID/句柄判断进程。已在隔离iOS模拟器实际运行本轮c3 App，commit第6/8项真实触摸通过；ACP长草稿收键盘gap164真实红仍在修复。不得把未完成候选或旧Lab当完整原生验收。
- Q021b 的 face gate 上次被用户暂停中断，不计通过；dialogs、words 与 types 已过。新的局部与集成结果须按当前版本记录。

## 尚未完成的完整范围

首批原生/物理设备/辅助技术验收；新发现Q015通知插件初始化ACL拒绝、Q033上传停止仍可能启动turn；Q020b/c/Q023b/Q025/Q030b/Q031/Q032；native page迟到归属、chat轮询结束/退避、sidecar取消期限、4MiB同步缺口恢复、once锁回收；工具链与产物可复现、installer checksum和mirror一致性；模块职责重构、格式/命名/错误/注释规则、剩余模块深入审查，以及最终确切集成版本的完整用户流程验收。完整清单不因本轮优先项缩减。

## 历史与恢复

本文件只保留当前事实。暂停/拔盘/两次目录整理前的总账快照见 [history/STATE-before-resume-20261025.md](history/STATE-before-resume-20261025.md)，详细过程读单任务卡。恢复时先读当前用户指令、本文件、PLAN和对应任务卡，再核对真实 worktree/HEAD/diff/进程；历史摘要、通过日志和图谱风险分数均不能替代当前证据。
