# Windows 迁移 W0 审批包

## 1. 目的

本审批包把 W0 已完成的事实、建议范围、关键技术决策和 W1 前置条件集中提交一次审批。审批通过后才进入 W1 真实构建；未批准前不进入产品功能实现。

## 2. 已完成事实

- 独立 worktree：`F:\#Gong\#AI4S\khor\worktrees\s2-windows`
- 分支：`steward/s2-windows-20261114`
- 基线：`d3781adce2e38bf966acd684233069e21c20a741`，随后 W0 文档提交
- s2 分支 clean；Khor main 未改动
- 真实源码入口已读：Cargo workspace、CLI、GUI/Tauri、Agent、Web build、quality/frontend/release workflow
- 未修改产品源码、manifest、锁文件或 CI
- 未合并 main、未 push、未签名、未发布

## 3. 建议批准范围

### 3.1 平台
- Windows 11 x64：主开发与验收平台
- Windows 10 x64：承诺范围，W7 保留兼容验证
- Windows ARM64 与 Windows Server：不自动纳入，需用户显式追加
- Linux/macOS/iOS：保持现有兼容性

### 3.2 功能
按 [FEATURE-MATRIX](FEATURE-MATRIX.md) 中 REQUIRED 清单执行：

1. CLI / Web 基础命令；
2. GUI 桌面窗口与本地/远程节点连接；
3. 设备配对、连接、重连；
4. Session 创建/恢复/关闭与跨设备同步；
5. 终端与 ConPTY；
6. 文件读写/复制/打开/删除；
7. 进程、Job Object、取消与清理；
8. Agent 进程与 IPC；
9. 安装/更新/回滚/凭据；
10. Windows 构建与 CI 证据。

不默认包含：签名、商店分发、自动发布、Windows ARM64/Server。

## 4. 技术路线

### 4.1 同库渐进适配
- 不另建 Windows 专用产品分支或复制产品代码；
- 在现有 Cargo workspace 和 Tauri 应用内按 `cfg(windows)`、target-specific dependencies 和平台适配层实现；
- 所有平台共享同一套业务协议、Session 语义和 CLI/GUI 行为契约；
- 平台差异收敛在显式适配层，禁止在业务层散落 `cfg(windows)`。

### 4.2 显式适配层规则
- 路径、进程、文件、凭据、终端等平台能力有单一接口；
- Windows 实现和 Unix 实现共享同一契约测试；
- 失败的平台能力必须返回结构化错误，不得静默降级；
- 禁止用未文档化的第三方兼容层绕过系统边界。

### 4.3 证据规则
- 源码阅读只能证明入口存在；
- 构建/测试必须记录 cwd、HEAD、工具版本、退出码、日志和 clean 状态；
- NOT_RUN/BLOCKED_ENV 必须显式列出；
- 不允许把 CI 配置声明当作本机执行结果。

## 5. 基准工作负载与性能比较

### 5.1 必测负载
- CLI 冷启动与 `--version`；
- GUI 冷启动到可交互；
- 本地 Session 创建/恢复；
- 设备配对与断线重连；
- 终端输出吞吐与取消；
- 文件扫描/复制/同步一组固定文件；
- 更新检查与回滚。

### 5.2 比较方法
- Linux/macOS 作为现有基线，Windows 作为被测平台；
- 同 HEAD、同依赖锁定、同工作负载；
- 记录中位数、P95、资源峰值和失败率；
- 预热期：前 2 次运行不计入稳定值；之后至少 5 次有效运行；
- 资源增长判定：相对现有平台超过 30% 视为需解释/优化项，超过 100% 视为阻断项，除非有明确平台原因并经用户批准。

### 5.3 早期短时验证标准
- W1：构建成功、CLI `--version`、GUI 启动、诊断入口运行；
- W3：进程/终端/文件 smoke；
- W4：设备配对与 Session 端到端；
- W7：性能与稳定性矩阵。

## 6. S1.5-P0 保护机制

- 不自动合并 main；
- 不 push、不签名、不发布；
- 所有产品改动限定 s2 worktree/branch；
- 高风险决策（架构、依赖、平台边界、签名）必须人工闸门；
- 证据不可用时标 BLOCKED_ENV，不以模型自述替代；
- 合并与发布由用户最终决定。

## 7. W1 前置条件

1. 用户批准本审批包；
2. 确认 Windows 10/11 范围与不包含项；
3. 确认同库渐进适配与适配层规则；
4. 确认基准负载与资源阈值；
5. 确认 W1 只做构建/诊断/CI 基线，不做功能扩展。

## 8. 待用户决策

- **D1**：是否批准上述 REQUIRED 范围？
- **D2**：Windows 10 是否需要在 W7 做真实兼容验证（建议是）？
- **D3**：Windows ARM64/Server 是否本轮追加（建议否）？
- **D4**：资源增长阈值是否接受 30% 解释、100% 阻断？
- **D5**：W1 是否允许安装必要依赖以完成真实构建（建议允许，仅限隔离 worktree）？

## 9. 结论

W0 的基线、功能矩阵和证据边界已齐备。建议批准进入 W1：真实构建前端、最小 Rust CLI、诊断入口和 Windows CI 基线；仍不合并 main、不 push、不签名、不发布。
