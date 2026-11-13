# 桌面拖放与固定标题材质（2026-11-13）

用户在 MacBook Air 的 RC.16 中报告三项问题：导航和标签栏整条变实色；本地文件拖入 MacLab 终端完全无反应；毛玻璃窗口滚动时固定列表标题过度增强背景颜色。Air 已确认深色、毛玻璃、不透明度20%，系统降低透明度关闭。

## 已修复

原生拖放监听只订阅了 Webview 目标。Tauri 2.11.5 在主窗口创建浏览器子视图后，把 WindowContent 拖放改投 Window/WebviewWindow 目标，旧监听收不到事件。837a10b0 改用当前界面同标签的 WebviewWindow 目标，覆盖两条自身通道，并继续排除网站子视图。文件上传、取消、落地后粘贴和坐标换算未改。

新增协议回归固定上游 manager/window.rs、manager/webview.rs 的原始源码与摘要，使用真实 SDK、回调注册和清理，只控制原生 IPC 投递。旧源码89f4f5eb的43355387136四项新检查全部先红：打开子视图后收到1次而应为2次；修复837a10b0的43355962194通过48项事件检查、共享unit/gates/mobile static、16项导航及热更新。此验证不等于 Air 的实际 Finder 拖放验收。

固定标题的 `band-blur` 原来叠加了 `saturate(1.6)`。20%透明度下，六组纯色对照实测多出15.58–18.58级RGB偏移。75678e3e移除该饱和度增强，保留20px模糊、渐变、正文遮罩与滚动逻辑；token、设计系统JSON、比较器及行为文档同次更新。Chromium/WebKit、浅深主题、三种背景的12组对照全部通过，最大偏移0.42级。

## 最终验证及现场

最新可执行验收源码dfa81ecf只在75678e3e上补了Lab就绪检查：浏览器开跑前确认Python预览已监听，且tree/snapshot commit与本次构建一致。此前43357712345的颜色12项通过，但首项导航在服务监听前连接失败；这次失败保留，未当成产品回归通过。

43358502953@dfa81ecf通过颜色12项、导航16项、共享unit/gates/mobile static和热更新。43357726932@75678e3e完整Lab88项及热更新通过；两份正式UI源码逐字一致。实际查看了最终Khor手机导航、完整App桌面列表及颜色前后截图。缩短视口、协议及CSS合成测试不替代原生窗口和物理设备验收。

现场：http://127.0.0.1:23961/app/?scene=khor&machines=3 ，列表可改为 `scene=list`；手机通过MacLab借网。静态源dfa81ecf/tree e4fe9adf62eb63bd8f1423ad75e123f6eec251b6，Python PID23296 / exec54630。17个保留版本、51个JS的HTTP与字节摘要已核对。只替换已核验归属的旧Lab进程，没有重启用户App或宿主。

证据在 `/Volumes/disk0/Khor/evidence/desktop-glass-drop-20261113/`：drop-red、drop-green、material-red、final-material-dfa8、final-lab-dfa8、final-full-7567均有 `verification.json`。最终颜色产物14006908912 SHA256 `c219f9d5fc3f32cee54e91753f3b5b01c8e85e826985d58d42fcdcbcbc3e2b07`；专项Lab14007218063 SHA256 `c91aaf4e9a23fe84c990004cceaa2c6ef4ee174863bfaa009da895c91f3ba790`；完整Lab14007675925 SHA256 `78a1d40e90738e38d280595002d993bd6869cee13ee470231cc04f0fe9ace976`。均已下载并核验ZIP CRC及对应来源；原生协议绿色产物14006486004也核验了源码指纹稳定性。

## 尚待定位

Air整条深色导航/标签栏实色的问题尚未复现。已排除用户指出的材质、20%不透明度和降低透明度配置；用户也明确不是浅色选中项。已请求Air当前完整窗口截图，待收到后继续。没有依据相似WebKit问题猜改原生绘制逻辑，亦未宣称此项已解决。

分支 `fix/desktop-glass-drop-20261113` 包含前序旧底栏清理。尚未合入main或发布新包，RC.16保持不变；两项修复仍需后续安装包的实际设备复测。临时验证workflow的desktop_probe只用于隔离回归，不合入产品。所有构建和浏览器测试在远端进行，本机仅源码、Git、Python静态服务和证据校验。
