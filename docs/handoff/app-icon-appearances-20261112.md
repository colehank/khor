# 安装图标的浅深外观 — 2026-11-12

> 2026-11-13：用户取消本浅色方案，安装图标统一恢复原 07 深色版。以下是历史实现和验证记录，不再代表当前外观策略。当前规则以 `apps/gui/src-tauri/icons/src/README.md` 为准。

实现位于独立 worktree `/Volumes/disk0/Khor/worktrees/current/khor-app-icon-appearances-20261112`，
branch `feat/app-icon-appearances-20261112`，main 基线 `97bbf2a4`。
初次交付时尚未合并 main；用户随后要求推 main，并明确选择等待、重试全量 Quality。
图标实现已以 `7f60f9b00ac7297f4ae65dd3e7b7be1cff4e6328` 快进合入并推送 main。
本次未构建发行包、发布或安装 App，未更改其他运行中的 Lab 或用户的 App。

## 唯一维护入口

- [App 图标规范](../../apps/gui/src-tauri/icons/src/README.md)
- 同目录 `app-icon-appearances.json` 为生成器读取的选择映射；默认与兼容降级均为浅色。
- 浅色 = 用户封存的 v14 balanced（green response 0.70），原文件指纹
  `119590e8557255105b8b83ea2fa747497b10b0a46b9779232a11a21bf8fde210`。
  自包含母稿已进入仓库，不依赖 `/Volumes/disk0/Khor/experiments/`。
- 深色 = 原 07，参数、TS 绘制程序与 SVG 未改。新 `app-icon-dark.png` 与原母图逐字节相同；
  浅色母图和封存 `artwork.png` 的 RGB 像素一致。
- macOS 26+ Icon Composer Default / Dark 与 iOS 18+ Any / Dark 使用对应的同一母图；
  两种外观的 iOS/macOS 1024 PNG 分别字节相同。iOS 使用 Single Size，保留 iOS 15 部署目标。
- 系统选择外观，khor 窗口主题不强制切换安装图标；Windows / Linux 暂为浅色静态兼容图。
- 会话、其他 session 和托盘字形不变；没有重新引入 Blender。

## 验证

产品实现 `a555b70f52ccfa3f19108066a98e4f2d7de24bdb`：
[App icon 43056687761](https://github.com/colehank/khor/actions/runs/43056687761) 两项 job SUCCESS。
strict TS、4 个 Node 测试、actionlint、完整 Python 229 tests（1 个既有平台 skip）通过；
完整 `app_icon.py check` 验证 14 个生成资源、定稿指纹、深色参数/SVG 和两端两种外观母图一致。
回归夹具移除深色映射、篡改 iOS 深色图及浅色稿，即使重算清单也会失败。
macOS 与 iPhone/iPad catalog 编译通过；原生 Default / Dark 导出的中心区域均值分别约 233.90 / 67.87。

原生 artifact `13871842003`（7 天）ZIP SHA-256
`bb3e76cdf46236dda20ea264fb61de060416c98f17a2b97d30d47f3aec38b455` 与 GitHub 声明一致，CRC 无错误。
本地只做源码静态检查、既有母稿封装/系统图片导出及图像核对；Node/TS/Python 测试、actool 编译在 GitHub 运行。
缓存 `/Volumes/disk0/Khor/caches/app-icon-appearances-20261112/native-catalog/` 保留实际 catalog 与清单。
清单确认 macOS `IconImageStack` 包含 Aqua / DarkAqua，iPhone 与 iPad 各有 Any / Dark 且不同内容摘要。
本地与 CI 系统导出存在最多少数通道值的量化差异，母图一致性不要求系统合成后的逐像素一致。

最终增强检查 `dfc1cdff3dde9ff4d56a53d341c9c5ce5116a486`：
[App icon 43057299526](https://github.com/colehank/khor/actions/runs/43057299526) 两项 job SUCCESS。
新增的实际编译 catalog 中 macOS / iPhone / iPad 两种外观持续断言通过；同一轮完整脚本、TS、actionlint 与资源检查均通过。
产品图稿与生成器未再改。

## 交付边界

实际安装后的 Finder、Dock、启动器、iPhone/iPad 主屏幕，以及自动/固定明暗、着色、透明和旧系统降级，
仍需在后续安装构建中验收。原生 catalog 与预览通过不代表安装验收通过。
本次已完成 main 合入；没有 tag、商店提交、发布或替换用户 App。

## main 合入验证

[完整 Quality 43058179751](https://github.com/colehank/khor/actions/runs/43058179751)
在精确候选 `7f60f9b00ac7297f4ae65dd3e7b7be1cff4e6328` 上，最终 13 项 job 全部 SUCCESS。
第一轮 Lab 的中文字体下载遇到 Ubuntu DNS 失败；GUI 的浏览器依赖安装遇到 DNS 失败和旧软件包 404，
两者均未开始浏览器断言。保留首次失败记录，仅重试这两项；第二轮两项均通过，其他 11 项通过结果保留。
没有改代码、断言、门或依赖获取逻辑来规避失败。

源 main 从 `97bbf2a4` 快进至已通过完整检查的 `7f60f9b0`，没有引入其他 worktree 的改动。
推送从干净的图标 worktree 执行；本地 main 原有修改和未跟踪文件在合入前后做 SHA-256 核对。
本提交只补充合入记录，产品源码、图稿和资源与上述绿色候选完全一致；main 的常规自动 CI 仍会正常触发。
