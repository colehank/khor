# Agents 文案与页尾精修（2026-11-16）

用户要求 harnesses 改为 agents，并优化单调的页尾。已随 2026-11-16 网站快照发布，详情见 [Pages 收据](site-pages-20261112.md)。

- 开场第二句、同源书签标签及屏幕阅读器叙事统一为 With many agents；原来的滚动演化、图形与任务动画保持。
- 收尾标题与主操作放在左侧，原始透明玻璃图标放大至最多 336px 放在右侧。保持 Build with Khor 和既有说明，不增加新宣传主张。
- 安装方式以细线与主区分开，继续共用 ProductActions / ProductInstallations，保留设备推荐、五平台、Coming soon 提示和 CLI 复制；不重新实现控件。
- 手机按图标、标题、操作、安装组纵排。团队署名分为 Presented by 与团队名称两层，KapeStack 和斜杠作为一组换行；底部补充 Back to top，指向实际的 khor-hero。
- GitHub 仓库入口仍只出现在顶栏；两个已有团队链接继续指向已确认的 org 和公司网站。

Headless Chrome 在 1440 / 820 / 390px 宽度检查布局，在 320px 补查窄屏换行；无横向溢出、重复 ID 或页面错误。验证 Agents 书签、下载与首屏一致、提示层范围、复制后仅勾反馈、团队链接及返回顶部。复制检查在隔离浏览器中替换 clipboard.writeText 验证传值与反馈，没有改用户系统剪贴板。Astro 开发渲染、detector 与 diff 检查通过；无原生构建或其他浏览器。

证据：`/Volumes/disk0/Khor/evidence/site-footer-20261116/`，临时脚本 `/tmp/playwright-test-khor-footer.js`、`/tmp/playwright-test-khor-footer-controls.js`。
