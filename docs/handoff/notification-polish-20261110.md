# LIVE · 通知外观优化完成 · 2026-11-10

实现与验证完成，等待用户 Lab 审美验收；保持不合并／不发布。有效源 `294848eb955263132bf334b88c4029a35513030a`，tree `c34d32a6bc6c744550b37dd5cae8654683d1665e`，工作分支 `integrate/settings-gui-20261109`。294 相对产品 b10 仅改测试预览的窄屏启动标记与断言；8d 为规范补充。

- 精确 [Quality42708174149](https://github.com/colehank/khor/actions/runs/42708174149) 全部选中检查 **SUCCESS**：frontend、GUI unit/types/static、notification-ui／key-row-ui／appearance／dialogs、56 项真实 App Lab、保留草稿的热更新。
- 通知交互额外覆盖1100／390／320，浅深两主题共六组：到期、悬停暂停、关闭、静音、准确会话跳转、不保存历史，全部通过。已看六组中的浅390／深320／深1100几何截图，以及中文字体齐全的实际 App 手机／桌面通知截图。隔离 GUI job 无中文字体的方框不是产品文字缺失；用户预览使用实际 App 产物。
- LIVE：`http://127.0.0.1:23950/app/?scene=list&machines=3&notices=1`。初始快照静默，随后约18秒发示例事件；真实通知控制器和正式 UI 渲染。人工刷新当前页面获取新版本。
- 可信产物：`/Volumes/disk0/Khor/evidence/more-network-gui-20261109/notification-294-review/developer-lab`。Python exec20376／PID36315；换前再次核对 argv/PID。只替换已验证的旧 Lab Python97389，安装版、宿主、用户浏览器导航与刷新未动。
- 新页面／JS与旧3a／394／ce0／269／pinned快照页面／JS全HTTP200。核验记录 `notification-live-verification.json`。未用 CUA。

外观：现有20圆角 token、12留白、列表规格头像、分开的标题／状态、小圆关闭钮配完整热区；厚玻璃底色配公共模糊／边缘／阴影，正文不再明显透上来干扰。无模糊能力或减少透明效果有实色兜底。没有改变通知控制器、持久化与事件规则；原生浏览器／PDF上的层级覆盖问题仍未闭环，不因本次外观验收宣称已解决。

以下为执行历史，以此顶部为准。

---

# 通知横幅外观优化 · 2026-11-10

用户反馈现有通知的圆角与材质不好看，已授权优化并更新真实 App Lab。根 agent 串行；本机源码／Python 静态检查与可信产物服务，构建／浏览器验证走公开 GitHub-hosted Quality。继续保持不合并／不发布／不重启用户 App 或宿主。

起点：`integrate/settings-gui-20261109@19a14171`；既有 Lab23950 仍是已验证3a203ac3，Pythonsession92335/PID97389，换前必须核对 PID/argv 并保留 snapshots。

实现：通知复用现有 radius-2xl／gap-row／列表头像，glass-menu 厚底配公共玻璃边缘／模糊／阴影；取消与公共玻璃冲突的第二套 float 阴影。主次文字留白、小圆关闭键与独立触摸热区；无 backdrop-filter／减少透明时实色底兜底。只调整外观，不改通知事件、水位、静音、有效期、点击跳转或持久化。

下一步：核对浅深主题与忙碌正文背景、桌面／390及320窄屏、长标题／多行文案，沿用 notification-ui 的真实控制器行为检查；可信 CI 成功后看实际 App 截图，再更新23950。原生浏览器／PDF上方通知的承载问题仍是单独未闭环项，不因本次 CSS 优化宣称已修。
