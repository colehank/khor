# 开发资料存储与目录恢复

本次目录整理时治理暂停；当前执行状态以 STATE 最新记录为准。主仓仍为 `/Users/zgh/Desktop/working/khor`，main 未修改、未发布。

外盘唯一资料根目录：`/Volumes/disk0/Khor`。

| 目录 | 用途 |
| --- | --- |
| worktrees/current | 本轮治理的 9 个外盘工作副本 |
| worktrees/history/merged | HEAD 已在 main 且无 Git 可见改动的 26 个历史工作副本；保留其忽略文件 |
| worktrees/history/preserve | 其余 13 个历史工作副本，含未提交 Lab 与未确认合并的分支 |
| caches | Rust 构建缓存和已安装开发依赖 |
| evidence | 运行记录、截图与测试证据 |
| releases | 历次发布收据与相关资料 |
| build-support | iOS 构建支持、设备符号与构建临时目录 |
| experiments / references / archives | 实验、参考材料与既有归档；内容未删除 |

内盘恢复分支没有移动。迁移前备份、完整 old→new 映射、校验结果和缓存清理记录在 `/Users/zgh/khor-backups/disk0-20261025`，外盘入口 README.md 提供链接。

所有外盘 Git 工作副本通过 git worktree move 迁移，Git 登记同步更新。主仓 target、node_modules 和 Xcode Externals/build 等链接已修正。根目录保留少量兼容软链接以支持当前代码/发布文档的旧路径；未来路径配置统一后再移除，不能按重复目录删除目标。

新任务必须在对应分类中创建工作副本、缓存和证据，记录所有者与用途。任务交付后核对分支、未提交及未跟踪/忽略内容，再归档。编译缓存可独立清理，但不能依据 target 之类的目录名判断内容可删。历史记录里的旧绝对路径通过迁移清单定位，不批量改写历史证据。工作副本图谱缓存可能包含旧绝对路径，下次使用前核对并按需刷新。

## 内盘工作副本归位（2026-10-25）

`/Users/zgh/Desktop/working` 下只保留主仓 khor。内盘需要保留的独立工作副本统一在 `/Users/zgh/Developer/Khor/worktrees`，其中 recovery/ 保留外盘中断期间的 4 个恢复分支，history/ 保留 3 个旧功能分支。迁移未删除工作副本或源码；恢复分支待实际整合后再撤销。备份与映射在 `/Users/zgh/khor-backups/working-20261025`。
