# 内容面板修正 · 2026-10-17

## 本轮反馈

内容滑动没有带动面板高度；网页药丸应与文件一样，上行 session 名、下行内容名；来源按钮在药丸内过重。

## 原因与修改

- `useContentSheet` 原先在 touchstart 发现任一祖先 scrollTop > 0 就放弃整次手势。现在保留监听：半高上推优先展开；全高下拉先滚内容，到顶部在同次手势中交给面板；增长至全高的剩余位移继续滚动。输入、选字、横向操作仍保护。
- 原生网页是独立 WKWebView，DOM 面板收不到它的触摸。新增 `page_sheet` 几何配置和 `page:sheet` start／move／end／cancel 事件，使用 UIScrollView 现有 panGestureRecognizer 的额外 target，不替换 WebKit delegate。原生分配高度和内容位移，主界面显示同一进度；取消、失焦、隐藏和退出释放控制。这个适配仍需真机检验识别器时序与惯性。
- 网页与文件共用两行身份样式，网页优先显示真实页面标题，没有标题时才使用地址；地址编辑继续独占药丸。
- 手机来源入口为无玻璃底的轻量图标，保留触摸尺寸、键盘焦点与菜单图标；桌面独立工具栏继续标准玻璃按钮。

## 验证

- 新增 content-pan：旧 main 在“已有目录滚动位置时上推”断言复现失败；修复后的 Chrome／WebKit DOM 事件回归通过，包含同手势顶部交接、取消、输入保护、原生进度契约、迟到结束与失焦释放。
- content-ui 完整手机交互流程通过，网页两行、来源图标、菜单、编辑、宽度、标签切换与导航恢复均有截图。
- Rust 原生位移分配 3 项测试、前端构建／TypeScript、图标／像素／入口门通过；iOS release 目标 check 通过。
- npm run mobile 完整 iOS 编译与命令声明检查通过；补齐此前漏登记的 console_catalog、seen_snapshot、set_window_appearance、window_appearance_support，逐项确认是静态目录、本地文档或本机配置操作。
- 证据：/Users/zgh/khor-drop/sheet-interaction；原生 iOS 页面滚动、选字与惯性仍需安装后真机确认。浏览器消息契约不等同于原生触摸验收。

## 交付

独立 worktree：/Volumes/disk0/khor-sheet-interaction，分支 mobile-sheet-interaction。本轮未替换已安装版本。
