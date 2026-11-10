# 共享 App 图标与可调工具（2026-11-10）

用户确认以 07 对齐的程序化图稿继续，要求将图标和生成工具纳入 main。

生产入口是 `apps/gui/src-tauri/icons/src/app-icon.json`；纯 SVG 渲染器与浏览器工具在
`apps/gui/dev/icon-lab/`。`scripts/app_icon.py` 提供 serve / generate / check，完整用法见
[图标源码说明](../../apps/gui/src-tauri/icons/src/README.md)。工具不进入产品构建或路由。

macOS Icon.icon 与 iOS marketing 使用字节相同的 RGB 1024 母图，全部 iOS 尺寸及
旧系统 PNG/ICNS/ICO 统一派生。旧的独立 native SVG 已移除。生成清单记录输入及输出哈希，
可发现某个平台资源未同步的问题。07 原图作为工具中的视觉参照，不参与生产渲染。

本地复核：Node 光源/材质/参数兼容测试、Python 输入与 HTTP 边界测试、生成资源一致性、
workflow actionlint；Chrome 桌面及 390 × 720 手机视口的方向参数、撤销、分类和导出入口。
JSON 与 PNG 导出均到达系统保存面板；未覆盖实验目录中的已有文件。
Icon Composer 已导出本版 macOS/iOS Default 预览；实际 catalog 编译交给 `App icon` CI。

本轮只集成源码与图标资源。安装后的 Finder/Dock/iOS 主屏幕验收须使用下一份原生构建，
不能把网页的圆角或 Icon Composer 导出当作设备安装验收。此前实验目录和未提交的
`lab-preview/` 保持原状；不清理其他开发任务的工作副本。
