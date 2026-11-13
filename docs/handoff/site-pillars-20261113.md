# 官网亮点重做：P2P、Khor Agent、Terminal

用户否定 `cd12bc2a` 的机器／Sessions／工作区三段，要求回到产品亮点讨论，并接受了「连接设备 → 调动智能体 → 接手执行」的方向。最新「可以的」授权本地实施。本轮替换原三段，保留首屏、原始 SVG／Meet Khor 故事、导航、下载与静态手机策略；没有发布或合并 main。

## 当前展示

- **P2P — Your computer. From your phone.** 同一 Atlas Web 本地网页在 MacBook Pro 与手机界面中打开，地址为 localhost:5173，订单 A1048 与日期／CSV 内容一致。桌面采用来源与接收端组合，手机聚焦真实窄屏网页视图。用具体资源访问解释 P2P，移除机器性能图和三头像关系图。
- **Khor Agent — Put your agents to work.** 一次请求交给网络中的 Khor Agent，发现设备与 agents，委派 build-01 的 Codex 检查支付回调、Mac mini 的 Claude 整理待办，再汇集两个结果及两项待确认事项。使用真实 MachineChat／KhorAgentResult，配简短结果摘要；不再用一次直接编辑代码来代表网络调度。
- **Terminal — Same terminal. Any device.** 真实 xterm 中先显示 8 项导出测试通过，再显示 atlas-web 开发服务在 MacBook Pro 的 localhost:5173 运行。Desktop／Phone 按钮切换同一份输出的静态视图；支持键盘和触屏，手机默认 Phone。切换不会重载 App、创建进程或改变会话。

三段均自然滚动。媒体进入视口后只做一次短显现，文字从一开始可读；减少动态效果下完整静态展示，Terminal 选择仍可操作。没有增加首页 iframe、WebGL 或背景流。

## 真实界面与虚拟数据的边界

`apps/gui/dev/site/tour-scenario.ts` 保存有限虚拟任务与唯一的终端输出；`tour=p2p|agent|terminal` 是网站截图场景，不连接真实设备、模型、agent 或 PTY。默认 hero 场景保持原任务与固定下载回复。只有 Terminal 截图场景开放其选中会话，正常演示的第一项限制继续保留。

App 原生网页区域通常由单独 webview 呈现。网站专用 Vite alias `browser-pane.tsx` 在原来的页面位置提供一个有限的同源 iframe，展示 `/lab/atlas/` 静态网页；地址栏、标签、甲板及其余外壳来自生产 App。没有声称完成真实 P2P 传输测试，也没有修改生产 App 的 BrowserPane。浏览器 fixture 的页面标题已经固定，adapter 不发送动态标题回写。

媒体来自独立 Chrome 截图：P2P／Agent 为 2×，Terminal 为 1×。Terminal 的 2×／透明截图曾漏掉 WebGL 字形，尽管真实 buffer 已含输出；最终在前台刷新原 xterm 后，以 1×、非透明截图捕获，人工确认文字，并保存 buffer 收据。没有用 HTML 或图片编辑伪造终端文字。P2P 手机用实际抓手展开现有全屏内容层后截图。

12 张 WebP 共约 1.44 MB；picture 按主题／设备选取，媒体懒加载。已删除被替代的 19 张旧介绍资产。首页仍纯静态，原交互 GUI 仅在桌面首屏加载，手机保持首图策略。

## 验证与继续入口

证据 `/Volumes/disk0/Khor/evidence/site-story-20261113/pillars`：原始截图、captures.json、look-*.png、verification.json、构建日志与可复用脚本。

- 演示 TypeScript（包括新 browser adapter）、网站脚本 TypeScript、演示打包与 Astro 静态构建通过。
- 39 项集中 Chrome 检查通过，无页面错误：三项顺序、主题／图片、Terminal 双向切换、键盘、切换高度稳定、四档布局、减少动态效果、手机无 App 请求／iframe、Chrome 捏合、原 hero 消息／下载回复回归。
- 有界源码检测结果 `[]`。未运行 Rust、原生或 WebKit 矩阵；上轮 Safari 用户验收继续作为手机静态策略的依据，不冒称本轮做了新 Safari 真机测试。

预览 <http://127.0.0.1:23820/#connect>；Agent 为 `#khor-agent`，Terminal 为 `#terminal`。当前仍仅本地。
