# 初始实施历史（非当前状态）

以下内容保留原文，包含当时在途状态；当前事实只读 STATE 与 RUNS。

# feat/net 当前状态与恢复入口

更新时间：2026-11-09

## 当前状态

**实施已启动，当前 N0。** 用户设置活动目标“完成计划”，授权按本计划实施和验证；此前仅规划状态已被本次指令替代。已取得单元红/绿与真实网络基线；当前修正对照准备并运行同条件候选。

| 项目 | 值 |
| --- | --- |
| worktree | `/Volumes/disk0/Khor/worktrees/current/khor-net` |
| branch | `feat/net` |
| 创建基线 | `6485cb83d3b1f0bb93d4633825dc956623d78af9`（当前 main / rc.8 基线） |
| 权威计划 | [PLAN.md](PLAN.md)；[验收协议](ACCEPTANCE.md)；[设计与职责](DESIGN.md) |
| 当前所有者/方式 | root，串行；无本任务子智能体 |
| 活动远端任务 | corrected baseline smoke `42506330051`；candidate smoke `42506345592`（恢复时先查询实时状态） |
| 证据目录 | `/Volumes/disk0/Khor/evidence/net/`（预留，暂无测试产物） |

规划提交的 SHA 以实际 `git log` 为准，避免把自引用 SHA 写进本文件。主仓既有 `lab-preview/` 及其他任务 worktree 不属于本任务。

## 必须保留的用户决定

- **速度/体验优先，不能直连硬优先。** 这已是产品方向，不需要恢复后再询问是否坚持直连优先。
- 目标是极致个人设备网体验；netlab 用于复现、比较和回归。
- 遵守 macserver + 私库 `colehank/khor-build` 的远端开发规范，本机 source-only。
- 最新用户活动目标“完成计划”已授权实施；不再受旧的仅规划状态限制。合并、发布和中继购买/部署仍按用户后续指令。
- 中继数量/位置、双家、切换阈值、流量隔离方式和 Linux runner 仍是待测提议。

## 阶段进度

| 工作包 | 状态 | 证据/下一步 |
| --- | --- | --- |
| P0 计划与恢复入口 | 已完成文档准备 | PLAN、STATE、NET 索引及本地 AGENTS 入口；仅文档检查 |
| N0 基线与实验入口 | 进行中 | Linux namespace/CLI 已跑通，单元产品红和 p6r 产品红已取证；修正 p6d 双向连接准备后再对照 |
| N1 速度优先选路 | 单元通过，真实网络待验证 | 全承载 RTT 排序、统一滞回、确认新鲜测量、所有 bind 接线；候选 smoke 在途 |
| N2 连接与恢复 | 未开始 | 候选竞速、旧地址、请求总预算、恢复事件与安全重试 |
| N3 流量调度与后台 | 未开始 | 混合负载、背压、公平性、耗电与恢复并发 |
| N4 中继与寻址 | 未开始 | 端到端测量、故障域、双家实验与部署设计 |
| N5 整体验收 | 未开始 | 补丁覆盖、持续运行、真机与最终源码验证 |

## 已有事实与缺口

- 静态审阅基于上述 baseline：`crates/net/src/carry.rs::choose_settling` 的直连硬优先已确认，原测试包含 90 ms 直连优先于 5 ms 中继。
- 连接复用、退避、SSH/设备转发、journal、netlab 和 vendor 补丁都已有基础，不从零重造。
- 历史文档中的实验数字不是本轮测试结果。当前线上可用性、性能基线和所有验收门槛尚未验证/冻结。
- 新增 `net-unit/netlab-smoke/netlab` 与 `--builder-ref`；控制仓库 `feat/net` 分支已提供独立 Linux job，prepare + Linux 单元编译已实际成功。完整 namespace/netlab 仍待在途任务结果，不宣告完成。`lab` 仍指 Mac 原生 Lab。
- 中央其他 GUI 工作仍独立进行。本分支不自动合入 GUI 集成树，也不继承其他任务的实现/发布授权。

## 下一次接续

1. 读取 STATE、PLAN 与最新用户指令；核对 `git status --short`、分支和 HEAD。
2. 已获准实施，从当前 N0 继续；例行实现与验证自主完成。
3. 先查询 corrected baseline 42506330051 与 candidate 42506345592；下载核对 source.json 后比较 p6 与 p2/p3。最终产品代码的单元已在 42506109052 通过，新增的测试准备修正由 smoke 验证。
4. 用户新决定、实际代码变更、运行 ID、验证版本和未完成项及时更新本文件。不要积累互相矛盾的“当前状态”；历史移到简短记录中。

外盘缺失时，在 `/Users/zgh/Desktop/working/khor` 用 `git show feat/net:docs/net-optimization/STATE.md` 和 `git show feat/net:docs/net-optimization/PLAN.md` 只读恢复已提交计划；不覆盖外盘原未提交内容。

## 本轮记录

- 2026-11-09：用户明确创建 `feat/net` 并保存计划。以 main `6485cb83` 创建独立 worktree，保存六阶段计划、体验指标、15 类场景、远端验证边界和恢复规则。
- 本轮仅验证文档差异、链接/路径与 Git 状态；未启动网络实现、构建、测试、购买、部署或发布。

- 2026-11-09：用户设活动目标“完成计划”。实施启动；控制仓库独立树 `/Volumes/disk0/Khor/worktrees/current/khor-build-net`，分支 `feat/net`（从远端 main 克隆），准备网络专用 Linux 调度。已核对原始图谱对应 baseline；发现下层 `confirm_move` 还存在转向直连无条件放行，需要 N1 一起修正。

## 远端证据与当前差异（2026-11-09 实施首轮）

- 控制仓库 `/Volumes/disk0/Khor/worktrees/current/khor-build-net`，branch `feat/net`；提交 `9701c4e` 新增 Linux network，`dc17b38` 失败仍保存编译缓存，`f01d070` 提供格式失败修复 patch。均仅推控制 feature branch，未改其 main。
- baseline unit **42505117705 FAILURE（预期产品红）**，tree `30a55de95e0e12a1be829fab6c74d9ec33727ebe`，snapshot `660617a6d01b694524101ca872111ff1ae48e997`，control `9701c4e`。Rust 格式通过；60 passed / 1 failed / 1 ignored。唯一失败是新增速度优先回归：90ms Direct 压过 5ms Carried。原始 source/environment/format/net-unit 已下载到 `/Volumes/disk0/Khor/evidence/net/baseline-unit-42505117705`，不是编译失败。
- baseline smoke **42505323213**，tree `b0c5a92c6a561e17c7bd7c2f8560e7cf77de4e5f`，snapshot `27f3176471bf413e8955ea27e66e6476cc4f4cdb`，control `dc17b38`；包含新的 p6 真实网络及原 p2/p3，保持旧策略。最后观察 network job in_progress，需重新查询。
- candidate unit **42505672250 SUCCESS**，tree `b66d034354461653a524a1f6b34a8a91da8dd81d`，snapshot `2fb97bed509f1eecbdc8aa5ae5653d7c4d5ed19f`；初版统一 RTT 比较、5ms/25% 滞回（近期切换绝对底线10ms，比例仍25%）、所有承载都经过 fresh 确认，短停顿保护扩展所有承载。最后观察待结果。
- 验收目标已写 [ACCEPTANCE.md](ACCEPTANCE.md)，策略修改前保存。N0 整体仍未完成：需真实拓扑基线和分阶段指标补全；N1 也未完成，需候选实测与防抖/转发回归。
- 本地轻量 Python：remote-development 9 tests passed；控制仓库 materialization 1 test passed；bash 语法和 git diff --check 通过。没有本机 Cargo/Node/native 编译。
- 下一步：先取得两个在途任务的结果和准确 source/tree；若 smoke 编译/执行器有问题，修执行器再使用同样旧策略重跑基线；取得产品红后提交候选 smoke，并重复场景。N2-N5 保持完整待办。

- 静态调用链复核发现 `bind_pinned` 仅在 carry 启用时注册 `RoadOrder`。已移至统一 builder，覆盖一次性客户端。该新增差异不在 candidate unit 42505672250 的 tree 内，后续须验证最终源码。当前 worktree 的 code-review-graph 是空图（0节点且基线旧）；detect_changes 的低风险输出没有证明力，已用 baseline 主图确认调用关系并人工核对实际 diff。

- 候选 42505672250 已实际 SUCCESS：61 passed / 0 failed / 1 ignored，格式通过。收据和原始日志已保存 `/Volumes/disk0/Khor/evidence/net/candidate-unit-42505672250`。仅证明 tree b66d0343 的首版策略，不覆盖后续 endpoint 注册和新 settling 测试。
- 最终源码补充验证 **42506109052**：base `2fee06a6`，tree `d0fdb004c164b19b3efa9de4eb789c6897e8306c`，snapshot `0d7bfad4f8cc072445c0679dcfb3595eb40b74d9`，control branch feat/net；待结果。新 [DESIGN.md](DESIGN.md) 记录状态所有者、首版参数和仍需完成的动态采样/备用/调度/寻址设计点。
- 首批实现已提交 `2fee06a6`。当前新增 DESIGN 与状态记录会另作检查点；尚未推产品 feat/net，也未合 main/发布。

- final-source unit **42506109052 SUCCESS**（产品代码含所有 bind 注册与 settling 回归）；原始收据/日志目录 `/Volumes/disk0/Khor/evidence/net/final-unit-42506109052`，62 passed / 0 failed / 1 ignored，格式通过。
- baseline smoke **42505323213 FAILURE** 已下载 `/Volumes/disk0/Khor/evidence/net/baseline-smoke-42505323213`：namespace/netem/nft 与 CLI 构建成功；p2 green（3.8s，retired 0）、p3 green（worst 944.2ms，0 failed/retired）。p6r 为产品红：两端约302/303ms直连，p50 326.98ms、p95 351.32ms，未选40ms中继。p6d 是测试准备红：beta 尚未建立 outgoing wire，18.6s 时仍 Idle，不能算快直连产品失败。
- 已补 beta→alpha 显式读请求建立反向 outgoing wire，再开始10秒预热，未修改任何验收阈值。为保证同条件对照，新建 detached baseline worktree `/Volumes/disk0/Khor/worktrees/current/khor-net-baseline` @6485cb83；仅复制修正后的 `crates/cli/tests/netlab/paths.rs` 与提交入口 `scripts/dev.py`，生产源码全部原样。不要在该 baseline 树修产品代码。
- corrected baseline smoke **42506330051**：base6485cb83，tree `e73903a3cfab4455baba0a35664c2c148c61fb99`，snapshot `5a8bcad37c04740c732719f6e9e77f4ec136e2c0`，control f01d070。candidate smoke **42506345592**：base2fee06a6，tree `ee158fdb53a8f5cda8020218772a53cfed89f4b4`，snapshot `3c92b03d8e53468010da6dc1f71a6834950049ef`，control f01d070。两者均用相同新版 p6 fixture，待实际结果。

- 检查点 `4fe743b1` 已保存设计与 p6 fixture 修正。之后仅补充本条状态。final-unit 42506109052 为62 passed / 0 failed / 1 ignored；当前 smoke 的产品代码与该单元源码一致，harness 新增了反向连接准备。
- 恢复后必须先查 42506330051 / 42506345592 的实时状态并下载结果。原 baseline p6r 红仍是有效产品证据，p6d 准备红已修；不能将两者合并称为两个产品故障。
- N1 余项包括：动态变速/恢复、关闭候选后的重新发现、样本老化/跨连接测量、全承载防抖与转发回归、hop 看守使用固定10ms而选路使用统一相对门槛的一致性。首批 RTT 排序与单元通过不代表这些已完成。N2-N5全部保留。
