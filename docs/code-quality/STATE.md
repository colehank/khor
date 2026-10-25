# 当前状态

更新：2026-10-25 · 用户重新激活完整治理目标，继续执行。

## 目标与边界

完整范围仍为 [PLAN.md](PLAN.md) P1–P5：全仓审查、分批功能修复与结构治理、工程规范和整体验收。不能以首批或单项通过代替全面完成。[AUDIT.md](AUDIT.md) 保存发现与后续优先级。main/发布没有本轮授权；独立分支和隔离验收继续。

## 当前路径与版本

- 主仓：`/Users/zgh/Desktop/working/khor`，main `8704a503a27b36ad58f922690de50e766f8bd485`，tracked clean。
- 权威协调：本文件所在目录，plan/code-quality 分支。其他 worktree 的同名文档是快照。
- 集成实现：`/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，integrate/code-quality-wave1，当前 HEAD `c3fe828b`（Q030/Q021b 已集成，原生验收期间冻结）。
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
| Q030b | agent_redaction；同上独立后续提交 | 跨chunk真红后已落候选；12项轻量流/Recorder测试通过，真实fake-sidecar回归已写但尚未Cargo类型/运行验证；完整槽排native之后 |
| Q010-native | native_acceptance；wave1 只读 | 上次用户暂停时 Cargo SIGTERM，未安装/启动隔离 App，不计通过。现场旧进程已确认终止；现获唯一槽，按冻结HEAD c3fe828b续建真正Tauri iOS App，独立bundle/fixture，不改用户App |
| Q025 | root接手；外盘 current/khor-quality-native-io | queue/connection/chat/term/native dispatcher 未提交，std-only7/7；root补5个真实core协议用例和核对Tauri解码oracle，待串行Cargo，不计编译通过 |
| Q020b / Q031 / Q032 | 保留，未关闭 | 覆盖入口仅新增回归未完成；A2A Message context 只有红测试；HTTP JSON 为审计准备，尚未实现 |

## 验证与资源

- 证据根 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`，各任务卡记录确切 source/lock hashes 与平台边界。
- 同时只有一个完整 Cargo/native 构建槽，当前分给 native_acceptance；store_owner 已完成并释放。jobs=2，沿用 quality-target；主发布缓存不写、不清。
- 外置盒风扇故障、USB SMART 温度不可读；用户明确允许继续。外盘约21GiB、内盘约24GiB可用。各构建仅监测/停止自己的进程组；掉盘、连续 I/O 错误或低于5GiB时保存日志并停止该构建。
- 不按旧 PID/句柄判断进程。上次 native 构建和 fixture 已按暂停要求终止，所有模拟器尚未用于本轮完整 App 验收；旧 Lab 地址未经当前核验不宣称在线。
- Q021b 的 face gate 上次被用户暂停中断，不计通过；dialogs、words 与 types 已过。新的局部与集成结果须按当前版本记录。

## 尚未完成的完整范围

首批原生/物理设备/辅助技术验收；Q020b/Q023b/Q025/Q030b/Q031/Q032；native page迟到归属、chat轮询结束/退避、sidecar取消期限、4MiB同步缺口恢复、once锁回收；工具链与产物可复现、installer checksum和mirror一致性；模块职责重构、格式/命名/错误/注释规则、剩余模块深入审查，以及最终确切集成版本的完整用户流程验收。完整清单不因本轮优先项缩减。

## 历史与恢复

本文件只保留当前事实。暂停/拔盘/两次目录整理前的总账快照见 [history/STATE-before-resume-20261025.md](history/STATE-before-resume-20261025.md)，详细过程读单任务卡。恢复时先读当前用户指令、本文件、PLAN和对应任务卡，再核对真实 worktree/HEAD/diff/进程；历史摘要、通过日志和图谱风险分数均不能替代当前证据。
