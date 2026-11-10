# 通知交互与调度（2026-11-10）
## 当前用户调整（进行中）

去掉通知状态词（包括无内容时的状态兜底），保留会话标题和最近内容。桌面关闭键仅鼠标悬停或键盘焦点进入时显示；手机不渲染关闭键，保留上滑关闭。桌面隐藏时保留位置，避免悬停导致文字跳动；粗指针平板也不显示关闭键。

扩展现有通知检查：默认隐藏／hover显示／focus显示／离开隐藏；手机无关闭节点及触摸上滑；无摘要不回退状态词；实际App重复触发场景同步走手势。发布保持暂停，旧0286 Lab暂保留，待新可信产物替换。

## 用户反馈后的演示修复（已完成）

用户反馈普通通知没有反应。发现旧Lab数据在bootstrap创建时按绝对时间给固定Plan会话发事件：慢导入会把事件当首屏历史消费，已打开Plan则正常被onScreen抑制。此前CI只在新状态下进入，未覆盖同identity重开。

改为首个会话快照后开始计时，避开当前选中会话并尊重静音。通知演示页添加标明Lab的“再次触发／汇总”按钮，经数据fixture生成新事件，仍由正式App收集和绘制；不放宽生产抑制规则。增加恢复已选Plan后reload、手动再次触发、慢启动基线与实际生产汇总检查。已切换到新可信0286版本，旧98a快照继续可用。


## 本次修复验收

- 产品 `0286ec5d64e97872df899aea9e04112057ca5691`，tree `d2c059ff7430ff80026acbb082530e21f2111734`。
- [Quality42747491212](https://github.com/colehank/khor/actions/runs/42747491212) 全绿：新Lab数据检查、GUI units/types、原通知手势等settings交互组；56项实际App Lab及热更新通过。
- 实际App已检查保留Plan后刷新仍通知其他会话、关闭后点击“再次触发”重新出现；手机／桌面截图 `app-notification-retrigger-chromium-mobile/desktop.png` 已检查。
- 当前地址 `http://127.0.0.1:23950/app/?scene=list&machines=3&notices=1&review=repeat-v2`。下方Lab工具只在通知演示页挂载；“再次触发／汇总”通过数据fixture发事件，正式App规则和通知UI不变。
- LIVE来源 `notification-demo-028-transfer/extracted-fast/developer-lab`；此前下载中断，最终从同一GitHub artifact13731688051分段续传，完整ZIP CRC、manifest源码/tree均核对。临时ZIP和分段已清理。
- 服务PID57328／exec92502；原98a及294／3a／394／ce0／269／pinned页和JS保留HTTP200，新工具chunk同样HTTP200。线上验证 `notification-demo-live-verification.json`。
- 本轮不发布；真实手机／native浏览器叠层仍待验收。

## 前次交付（98a）

已实现并在 Lab 交付，等待用户视觉／真机验收；发布继续暂停。

- 产品来源：`98a3d2e9c8440159ee72c2abb14176fdd2357fdc`，tree `1ea106af015d7582cd30ee85949b8e7d89256134`。
- [公开托管 Quality42722300514](https://github.com/colehank/khor/actions/runs/42722300514) 全绿：frontend；GUI units/types/static；notification-ui／key-row-ui／appearance／dialogs；56项完整App Lab和热更新。
- 通知6组（1100／390／320 × 浅／深）检查上滑、短拖复位、触摸取消与暂停、不误点、同源合并、汇总、内容摘要、静音、减少动态效果及不保存收件内容。
- 第二行使用与 SessionsList 共用的 `sessionContentPreview`；正在交互时保留当前内容，新状态结束交互后再更新。关闭不调用会话已读／已处理接口。
- 桌面最多3条、手机1条；排队不计时、32来源上限、5及以上可合并来源突发汇总。悬停／触摸／焦点／失焦暂停；窗口恢复等待实际新snapshot安静消费，不被旧缓存提前放行。
- 手机／桌面普通通知及汇总的实际App截图已检查（CJK字体与正式头像）。`native_verified=false`／`user_accepted=false` 保持真实。
- LIVE：[普通通知](http://127.0.0.1:23950/app/?scene=list&machines=3&notices=1)（约2秒后出现）；[突发汇总](http://127.0.0.1:23950/app/?scene=list&machines=3&notices=burst)（约20秒后出现）。
- 服务PID22365／exec session34521；只替换核对过argv的旧Lab PID36315，没有重启真实App或宿主。保留294／3a／394／ce0／269／pinned旧快照及JS；新页与旧资源HTTP200。
- 可信产物 `/Volumes/disk0/Khor/evidence/more-network-gui-20261109/notification-98a-review/developer-lab`；浏览器证据 `notification-98a-gui/quality-notifications/results.json`；线上版本校验 `notification-interaction-live-verification.json`。

原生浏览器／PDF上方叠层、物理iPhone安全区／系统手势仍未作真机验证；全轮发行回归、main合并和ship继续等待后续授权与验收。本轮没有声称整个仓库／所有平台发行检查均通过。

## 实施记录

用户已授权实现成熟通知行为；本轮只提交开发分支和更新 Lab，发布继续暂停。单智能体串行。本机只处理源码与可信 CI 静态产物，测试／构建走公开 GitHub 托管 runner。

## 目标

- 顶部滑入，上滑跟手关闭，未达阈值回弹；拖动不得误点进入会话。
- 手机一条、桌面最多三条；真正展示才开始计时，触摸／悬停／焦点及窗口失焦暂停。
- 同机器同会话合并最新状态，正在交互的内容保持稳定。
- 有限等待队列，丢弃陈旧事件；五个及以上会话突发汇总，点击进入会话列表。
- 第二行显示与会话列表一致的最近内容摘要；无内容才回退到事件状态，内容只在内存中保留。
- 关闭不修改会话未读／等待／失败状态；仅水位和静音偏好落盘。

## 执行计划

1. 分开等待与展示状态，补全合并、暂停、过期和汇总的模型检查。
2. 复用正式 NotificationBanners 实现手势、入场、可访问性和减少动态效果。
3. 扩充通知 Lab 场景与真实组件交互检查，运行公开 Quality settings。
4. 检查可信 App Lab 产物和截图，保留旧快照，在 23950 原地址切换新版本供用户验收。

## 当前

基线 HEAD 0a982e4368c2e0b9d7caef25bd1ef4e2203dcf45。线上 Lab 仍为 294848eb955263132bf334b88c4029a35513030a，暂无本任务的新验证结果。
原生浏览器／PDF上方叠层及物理设备手势仍需真机验收；不能以浏览器证据宣称已验证。

## 第一轮远端验证

产品 d56a1e9687e4a5211b9c08c6562f89b44a0db8ec，Quality42719318147：前端构建、GUI模型与类型通过；notification-ui 在短拖后绘制矩阵断言失败（-9.3位移）。测试原先只推进JS模拟时钟，修订为分别验证生产拖动状态已清除，以及等待浏览器实际CSS动画结束；保持最终零偏移断言，不放宽阈值。其他设置交互组通过。完整App Lab仍在运行。

补充源码审查修复：首次打开／后台／锁屏恢复先消费水位，避免补弹；通知挂载及visibility/focus恢复同步窗口状态；跨窗口静音更新只移除明确被静音的来源，不能用缺少机器元数据的半行推断默认订阅；屏幕阅读器同时朗读最近内容。

## 定位到的实际拖动状态问题

b0a1a89f1ed425024f046b481d00c0a6398e1832／Quality42720920071：前端、模型／类型通过；更严格的短拖检查复现 `{dragging:true, inline:"", paint:"none"}`。真实问题为 JSX 从可变ref读取拖动标记，渲染／编译缓存无法可靠追踪该改变。改用独立React状态，松手和系统取消都明确复位；保留状态与最终绘制的双重断言。首次回弹矩阵失败不能仅归因于模拟时钟。

恢复水位进一步以真实新snapshot边界保护：前台恢复时缓存行不解除抑制，首个新机器快照安静消费，随后新事件才提示；模型增加对应检查。
