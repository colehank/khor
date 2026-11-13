# 原 07 深色安装图标恢复 — 2026-11-13

用户取消米白浅色方案，要求恢复深色。实现以远端 main `c3b098bd` 为基线，
worktree `/Volumes/disk0/Khor/worktrees/current/khor-restore-dark-app-icon-20261113`，
branch `fix/restore-dark-app-icon-20261113`。本次仅处理图标资源、生成流程、相关检查与文档。

生产图稿、生成程序、清单与 Apple / PNG / ICNS / ICO 资源恢复为浅色方案之前 `97bbf2a4` 的原 07；
所有原始生成输入、输出 SHA-256 已核对一致，iOS / macOS 的 1024 母图逐字节相同。
删除生产浅色稿、浅色确认记录及明暗映射，不再提供米白变体；历史草稿仍可从 Git 历史或实验目录恢复。
统一规范入口仍是 `apps/gui/src-tauri/icons/src/README.md`，生产参数回到同目录 `app-icon.json`。
会话类型图标、托盘、App 内主题与后续 RC16 功能保持原样。

恢复原有资源检查与旧系统各尺寸 iOS 目录；保留 macOS / iOS 实际 catalog 编译检查，
并核对 Icon Composer 的 Default / Dark 两种系统导出均为原深色图稿。
本地仅静态核对和源码操作；测试、Node/TS、原生 catalog 编译和全量 Quality 均交由可信 GitHub。

恢复提交 `7a32ea78c4ef9dad262a1a185bff0de8ff70e372` 已快进合入并推送 main。
用户明确“只合并到main, 不ship”；没有 tag、发版、商店提交或替换已安装 App。

## 验证与合入

- [App icon 43352928004](https://github.com/colehank/khor/actions/runs/43352928004)：两项 job SUCCESS。strict TS、4 个 Node 测试、工作流检查、231 个 Python 测试（1 个既有平台 skip）、完整 `app_icon.py check` 全部通过；检查 25 个生成资源、参数/SVG 和两端母图字节一致。
- macOS / iOS 实际 catalog 编译通过。Icon Composer 两端的 Default / Dark 导出中心区域均值均为 67.8711，确认均选中原深色图稿。
- [完整 Quality 43352960198](https://github.com/colehank/khor/actions/runs/43352960198)：精确候选 `7a32ea78` 上 13 项 job 首轮全部 SUCCESS，含实际 GUI、Lab、iOS、Mac 与 Node/SSH 回归。没有跳过或减少检查。
- main 从 `c3b098bd` 快进到候选 `7a32ea78`，保留 RC16 后续功能，不引入其他 worktree 的未合入修改。
- 本地 main 的旧交接文档内容与目标 main 完全相同；备份并核对后，将这份已发布内容同步到索引再快进，未改写文件内容。合入前后 60 个既有用户文件 SHA-256 均保持一致；`lab-preview/`、`output/` 保留。
- 本次补充仅为交付记录；图稿、资源和产品代码与绿色候选一致。已安装 App 不因源码合入改变，留待后续用户授权的构建与发布。
