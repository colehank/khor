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

## 2026-10-27：历史编译副本可逆归档

内盘可用5.597GB、距既定5GiB门仅约228MB，不启动新编译。确认本任务recovery/runtime-artifacts下23个文件均owned regular files、lsof无持有后，归档到 `/Volumes/disk0/Khor/evidence/archives/runtime-artifacts-20261027/historical-binaries.zip`，SHA256 `94ce0218b7ea5e6dc42e33f535e40aaf41f18097c4129852c32f40434b25a8f3`。逐成员解压流SHA与原字节核齐，二次核原inode/mtime/size/SHA后，只移除14个>50MB的内盘历史编译副本，释放3,076,318,184B；9个小metadata原件保留，所有源码/日志/WIP不动。实测内盘可用8,672,993,280B。

完整23成员SHA、原mode/mtime与位置在同目录manifest.json；内盘原runtime-artifacts/ARCHIVE.json指向该档案。需要旧原始可执行文件时按manifest恢复指定成员及mode/mtime；归档不冒新的编译来源证明。现役shared quality-target、用户App/release未动，本次没有创建新工作副本或放宽资源门。

## 2026-10-29：第二轮旧编译归档迁外盘

A4本轮定向验证在reference5通过后实测内盘低于5GiB，按门停止未启动的26项，未并发续编译。仅将本任务recovery下cache-offload-20261028-01.zip、cache-offload-20261028-02.zip、node-old-large-variants-before-669fe9f8.zip三份历史编译归档迁至 `/Volumes/disk0/Khor/archives/recovery-compiled-20261029`。逐份核uid/regular/inode/size/mtime、lsof无持有及迁后全文件SHA，原路径原子替换为兼容软链接；共2,983,487,798字节，源码恢复包、当前构建缓存、用户App/数据均未移动。双盘manifest记录每文件SHA和原身份，旧档案内容保留。迁后内盘6,110,167,040B，外盘15,145,791,488B；不能把此时可用值当后续恒定值。

外盘收据 `archives/recovery-compiled-20261029/manifest.json`，内盘 `recovery/compiled-archives-offload-20261029.json`。本次迁移不冒新编译来源或历史任务重新验收。
