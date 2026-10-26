# Q053 格式规范基线与独立迁移批次

状态：baseline recorded，未实施格式迁移；root，2026-10-27。用户原目标P4包含统一可重复的代码风格，格式迁移与功能变化分批。

在治理集成670347d7 clean上实际执行 `cargo fmt --all -- --check`，只读，8.075s，exit1：205个第一方Rust文件有差异（crates175、apps30；无vendor），完整原日志1,935,425B。1923 tracked输入前后SHA一致，worktree仍clean；未格式化、未编译、未改任何源。实际工具 `rustfmt 1.9.0-stable (59807616e1 2026-04-14)`。双盘证据 `P4/format-baseline-670347d7` 含raw/result/source。这是原有风格差异，不当功能回归或已通过CI。

查已读Cargo.toml、GUI package.json及quality workflow未找到统一fmt/lint入口；全仓非vendor/非node_modules专用rustfmt/prettier/biome/eslint配置检索无命中。这个检索范围不意味着设计token/行为门不存在；后续不得以普通formatter替代它们。

后续在N1/R1/Q052功能改动收口集成后刷新清单，以Rust官方fmt/default edition workspace为统一机械入口；纯格式单独commit，不与逻辑修复混批，不动vendor/生成器输出归属。格式检查固定入口/实际工具版本进入工程指南和适用CI；不能直接开门让现205个既有差异阻塞无关任务，也不能用忽略全部源解决。前端格式按既有编码风格核清实际工具后定界，新增工具/锁变更另记来源，不凭关键词直接塞依赖。

完成标准：当前完整第一方格式检查通过；门与文档命令一致；生成代码从producer格式策略解决；适用类型/构建/既有行为门在最终整合版本通过。函数长度、注释数量不是成绩；保必要平台/协议/并发约束，失效历史说明迁决策记录。
