# 开发存储与产物保留规则

更新2026-10-31。用户指出两盘长期被压到5GB、目录混乱。此规则覆盖旧“每轮保存完整二进制/gzip并双盘镜像”的执行惯例；不修改原始失败、来源验证与证据内容，不把已删除的历史payload冒作仍可执行。

## 已确认的原因与清理

外盘Khor原约223GiB Rust缓存、58.5GiB旧实验；历史功能/实验各自保留target，构建依赖重复。系统盘本轮recovery原约8.3GiB，含多轮私有程序、静态库及压缩备份。机器16GiB RAM，2026-10-31测得swap约3.8GiB，系统余量会随内存压力变化；不手工删除swap或用户数据。

旧缓存24项du量109.6048GiB，经真实清理外盘可用增加104.7152GiB（两数不混同）。原清单/身份/无占用/执行结果在 `/Volumes/disk0/Khor/maintenance/storage-20261030`，源码、日志、会话、worktree、发布产物及三个保护target保留。系统清理结果以同目录internal-binary-cache-result及historical-mirror-dedup-result为准，不能用文件逻辑大小冒实际释放量。

## 执行前与执行后

- 新Cargo/native、打包、浏览器或App批次前，显式估算两盘该批峰值新增空间，包括临时文件、私有程序、导出及内存压力可能带来的swap；实际可用必须覆盖 **外盘20GiB／系统盘10GiB保留量＋该盘估算峰值**。用 `/Volumes/disk0/Khor/maintenance/storage-preflight.py --external-growth-gib <估算> --system-growth-gib <估算>`，结果随现有准入合同保存。不得省略估算或为追绿随意降低。实际原helper的5GiB中止门保留，只作最后停止线。
- 重型任务继续唯一槽、jobs1、incremental0。生成器与子Cargo显式用同一quality-target；不在每个worktree或实验目录再开target。TMP/profile/DerivedData先约定外盘专用位置，不默认溢入系统盘。
- 本轮只复用 `caches/rust/khor-quality-target` 和小型catalog目标；用户既有 `khor-target` 发布缓存/正式二进制另保。当前专用quality缓存以50GiB为复核线、内盘recovery以5GiB为复核线；达到后先列实际增长并淘汰已完成的旧产物再启动下一批，不清源码/会话，不改阈值掩盖增长。必须临时超线的单批写明实际理由及结束后回收项。
- 每批结束核owned进程清理，移除专用TMP、Chrome profile、确定已不用的私有程序与可重建fixture。需要下一紧邻批复用的程序只留一份，记录owner与到期检查点；不得每轮另压缩一份永久保留。

## 证据与恢复分工

- 详细原始日志、截图和完整测试证据只有外盘 `Khor/evidence` 一处权威原件。保留原失败和真实来源SHA；完成批次的可重建程序/静态库可删除，记录payload已退役，不以未来仍能运行为完成依据。
- 内盘recovery优先保当前STATE/PLAN/refs/runtime、当前未合并源码bundle或真实diff、新文件、紧凑结果/来源索引。禁止把每次测试的完整二进制、二进制gzip/zip及全量截图再复制一份。
- 旧轮次内盘证据若与外盘权威原件逐文件SHA一致，可将重复文件替换为指向原件的归档引用；当前轮未提交源码/独有日志不能只按摘要删除。真实源码/关键恢复资料保实际内盘字节，避免外盘断开时只有空引用。
- 既有历史mirror manifest保持其当时事实；另附清理/引用索引解释哪些缓存已退役、哪些历史字节由唯一原件承载，不能回填旧manifest声称仍有完整双副本。

## 目录

新资料统一在 `/Volumes/disk0/Khor/{worktrees,caches,evidence,archives,releases,maintenance}`；系统只用已登记的recovery入口，不在桌面/外盘根继续新建khor*目录。旧根目录khor*多为零体积兼容链接，不能当成多份数据相加。每个新大目录先明确owner、用途与回收点。

当前已审只读preflight SHA256 `e70d4b2f4d574c518e6a0065b10cd6c6ecb7c45981d8d81d5c8b32e1820384c5`；内盘只保该小脚本恢复副本，不复制缓存本体。
