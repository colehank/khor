# Apple 原版网络图标对照（2026-11-16）

用户要求 Wi-Fi、有线、蜂窝三枚都换成 Apple 的先看看。已完成独立的本地对照页，未部署。

入口：<http://127.0.0.1:23820/preview/apple-network/#connect>。

- LAN：SF Symbols `cable.connector`
- Wi-Fi：SF Symbols `wifi`
- Cellular：SF Symbols `cellularbars`，原生四格

生成命令 `node apps/site/scripts/preview-sf-symbols.mjs` 使用本机 AppKit / SF Symbols 离屏渲染 96px 透明原稿，无原生项目构建或桌面窗口。SVG alpha mask 保留原稿形状并继承图中颜色，每个图实例使用独立 ID。设备节点、网络成员、超边标签与动画保持。

图稿仅缓存于被忽略的 node_modules/.cache，不提交 Apple 图稿。独立路由在开发模式提供，正式构建的 getStaticPaths 返回空列表；普通首页继续使用现有 Khor 图标。这是 Apple 原版外观的本地比较，不是公开网站资产替换。

Headless Chrome 1440 / 390px 检查三枚图标、组网动画、无重复 ID／溢出／页面错误。正式静态构建通过，并核对没有对照 HTML 或任何三枚 Apple 图稿数据出现在 dist。脚本语法、detector 与 diff 检查通过。

证据：`/Volumes/disk0/Khor/evidence/site-sf-preview-20261116/`，包含截图、检查结果与 production-isolation.json。


## 已确认正式采用

用户要求「以此为准，commit 并发布部署」。正式首页现直接导入 src/assets/network 中三枚已确认的 SVG 图稿，不再依赖开发模式、查询参数或 node_modules 缓存。与获批对照的三份图稿逐字匹配。导出脚本改为 render-network-symbols.mjs，仅在 macOS 手动重新生成时使用；本地旧对照路由保留作兼容入口，正式构建不输出该别名。来源及版权说明与资产共同保存。发布状态以 Pages 收据为准。
