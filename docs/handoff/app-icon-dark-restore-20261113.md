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

实现提交后运行 App icon 与全量 Quality，通过后再合入 main；不发布或替换已安装 App。
