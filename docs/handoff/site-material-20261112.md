# 首屏材质与下载修订 · 2026-11-12

在 `feat/site-story` 本地修改，预览仍为 `http://127.0.0.1:23820/`。用户要求去掉首屏推荐版本说明，窗口透明底面从 50% 改到 40%，标题改成 “Work across your devices and agents.”，修复固定回复的下载链接与玻璃控件黑底；随后明确取消 GUI 放大功能。

## 实现

- 首屏不再有 `product-availability`，标题、页面 title 和页脚短语同步。下载页保留安装包平台、架构和版本，首屏按钮仍使用原有自适应推荐。
- 放大按钮、查看层、横向滑块、焦点循环和相关 Fog 暂停逻辑全部移除。手机和桌面都在可见且空闲时启用原位 GUI，布局始终为 1360px，再由 transform 缩放。
- 网站 bootstrap 的 `--window-opacity` 为 40%，不改变前景 opacity。深浅 WebP 重新从真实 App 截图；空白底面 alpha 为 102/255。透明首图没有可取样的场景，捕获时仅关闭内层 blur，保留玻璃实际 tint / rim，由父窗口提供背景模糊；交互界面继续使用原有玻璃滤镜。
- 下载失败已在旧代码复现：`download it here.` 是带 `data-md-link` 的 button，被网站限制 `[data-khor-agent-turn] button` 统一设成 disabled。现在只放行指向本网站 `/download/` 的固定按钮，原有其他有限交互范围保留。点击交由父页面进入同一自适应推荐，链接被测试路由接住，没有实际下载安装包。
- 黑底已在 Chrome 同一静态 Fog 下对照：窗口普通底面为 .5 alpha，但内层 backdrop-filter 取样透明 iframe 时又叠加一份暗底；取消饱和度或 root isolation 不能解决。`product-background.ts` 将同一个 Fog 场景放在 iframe 底层，先绘制场景，再由生产窗口绘制唯一 40% 底面，原玻璃控件正常取样。没有更改生产 App 或全局设计 token。
- 动态场景来自现有 Fog canvas 的 `captureStream(15)`，不创建第二个 WebGL 渲染器，不调用 getImageData / toDataURL 逐帧取图。流仅在 GUI 可见、页面前台、非减少动态效果时工作；离屏 / 页面隐藏时停止 tracks。无能力或失败时显示同一参数的静态帧。尺寸、位置、渐变和模糊按原 hero 的几何换算，避免独立背景错位。API 依据：[MDN captureStream](https://developer.mozilla.org/en-US/docs/Web/API/HTMLCanvasElement/captureStream)。

## 定向验证

证据目录 `/Volumes/disk0/Khor/evidence/site-story-20261112/hero-material-40`。

- `before.json` / `before-hero.png`：旧下载按钮 disabled、无 parent 下载消息；原控件背景。
- `frost-original.png`、`frost-blur-only.png`、`frost-isolation.png`、`frost-no-inner-backdrop.png`：同一场景的材质隔离对照，只有关闭取样会消除额外暗底；正式修复仍保留内层取样。
- `after.json`：窗口底面 .4，玻璃保留 `blur(28px) saturate(1.85)`，共享画布流可播放且主页面只有一个 Fog canvas；下载按钮可点击并到达已有 arm64 App 下载地址。Chrome 手机尺寸下无放大入口、无页面横向溢出，桌面布局仍为 1360；减少动态效果用静态背景。页面错误为空。
- `after-dark.png`、`after-light.png`、`after-phone.png`：内嵌 GUI、标题与无版本提示的截图。类型检查、演示打包和 Astro 静态构建为本地网站检查；未执行原生构建、远端构建或部署。
- `poster-alpha.json`：深浅首图均为 2720×1700，空白背景 alpha=102。`lifecycle.json`：离屏轨道 ended，返回可恢复，减少动态效果停止共享流；1050px 调整尺寸后背景坐标换算误差小于 1px。`fresh-light.png` 为新访问的浅色界面，正文使用真实浅色墨。

脚本 `/private/tmp/playwright-khor-material-before.js`、`playwright-khor-frost-probe.js`、`playwright-khor-material-after.js` 和 `playwright-khor-posters-40.js`。手机尺寸截图不作真机 FPS 结论，画布流兼容性也未扩大成浏览器矩阵。
