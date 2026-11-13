# GUI 手机导航验收修复（2026-11-13）

用户 IMG_5633 中，普通 Khor 会话的模型与会话导航出现在输入框下面。App 主入口只判断 `kind=gui` 或旧固定 ID `wide/all`；独立 Khor 会话的 `broadcast` 类型未进入顶部导航。工作区的 SessionFrame 有另一份判断，已经包含该类型。

正式组件现共用 `isConversationKind(gui|broadcast)`，主入口的顶部页头、甲板位置及会话列表入口使用同一个判断。终端底部导航保持原规则。Lab 新增 `scene=khor`，使用独立 `wide/agent-*` ID，避免只检查旧全局会话。

旧实现加新场景/检查的 43348081799 在 Chromium、WebKit 的320/390宽度均明确失败：主 Khor 顶部页头0而应为1；ACP对照通过。320工作区另有两次检查点中子会话展开入口，截图确认后改为点击会话标题，布局断言未改。

修复源码7b560f49的完整Lab 43348681012：88项全部无错误，热更新通过。其ZIP artifact14002424823 SHA256 `82ea969a9e46b4f3317fb581b2583997fd3cac975a170ff54a83cd7a0f805cf0`、CRC、源码和tree均核对。后续dcaf72f4仅修改上述测试点击目标，正式UI源码逐字一致。

dcaf72f4的专项43349243986：共享unit/gates/mobile static、16项导航交互和热更新全通过。覆盖主会话/工作区、Khor/ACP、320/390、Chromium/WebKit，以及顶部模型/返回/新建、输入框独立头像、设置菜单和缩短视口。ZIP artifact14003201778 SHA256 `85969b80fb4064ba6abe12c3491f9290c822820659ba718ef9dc813fa11c0f98`已核验，主Khor手机和工作区320截图已实际查看。缩短视口不冒充物理手机系统键盘。

旧底栏清理阶段的验收地址：http://127.0.0.1:23961/app/?scene=khor&machines=3 ，手机通过MacLab借网。当时静态源db3229cf/tree1abc52e6，Python PID42122 / exec81619，16个版本、48个JS的HTTP及字节摘要通过。该地址现已继续更新，最新现场见 [桌面拖放与固定标题材质](desktop-glass-drop-20261113.md)；没有重启用户App或宿主。

修复在 `fix/gui-mobile-navigation-20261113`，RC.16保持不可变，安装包尚不包含此项改动。重型验证全在远端，本机仅源码与Python静态服务。完整证据在 `/Volumes/disk0/Khor/evidence/gui-mobile-navigation-20261113/`。临时验证workflow f79123ec仅补选择专项及刷新apt索引，不合入产品；此前两次检查因Ubuntu旧包索引404中止，均保留失败记录。

用户随后要求清除旧布局。db3229cf 已删除无调用的 `combinedDock`、单独的 `deckAt` 参数、底部组合版专用 CSS/检查，以及旧 Lab 的比较残留。导航位置与输入区槽位统一由对话头决定，不再允许对话头与底部导航的矛盾组合。终端、浏览器和文件页继续使用共享的有效底栏。清理后的远端专项43352507780已通过共享unit/gates/mobile static、16项导航及热更新。产物14004821679的SHA256 `84569051f0f6ee6f03b3ae742c9cf019eb764fcc29799d60fdce95671af9fc29`、327项ZIP CRC及source/tree均核对，主Khor390 WebKit和工作区320 Chromium截图已查看。同一源码的完整Lab43352520613也已通过：88项全部无错误及热更新。产物14004479171的SHA256 `cfba945e674583cc941635b7ee0ff3421656cc5c5ddbb463c7dad056726048af`、694项ZIP CRC及source/tree均核对。
