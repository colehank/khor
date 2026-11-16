# 恢复下滑叙事（2026-11-16）

用户要求恢复下滑切换，同时澄清 Meet Khor 是固定的左上光源逐渐打开。本轮取代同日的自动播放尝试，已随 2026-11-16 网站快照发布，详情见 [Pages 收据](site-pages-20261112.md)。

- 恢复原生 WAAPI ScrollTimeline：图标随滚动连续变化，停滑即停，倒滑可逆。字幕在每段起点用既有短时渐变交接；没有计时播放、滚动吸附或手势拦截。
- 有效演化 3.6 屏，首句桌面 .5 屏 / 手机 .7 屏，总高 5.1 / 5.3 屏。保留段末 6% 的短暂落定与 smoothstep 归位。
- Meet 文案与光照在同一滚动章节起动。固定左上光场与完整玻璃图稿使用同一个 smoothstep 强度渐亮，平面构图同步淡出；不再以移动光带或裁切边界解释光源。
- 保留完整原始 SVG 图稿中的光位、玻璃材质与静态滤镜；仅改变三层 HTML 的 opacity，删除所有扫光、旋转裁切和反向补偿包装，不逐帧改写 SVG 遮罩或渐变。
- 旧浏览器沿用可见范围内的缓存 RAF 后备。移动端范围使用稳定 sticky 高度，不混用动态 innerHeight。减少动态效果保留紧凑静态章节，五个书签恢复滚动到对应姿态。
- 后续 Continue 的 6 秒去程／回程、Agent 的 12 秒四段依存，以及 You 整条超边亮起均保持。

Headless Chrome 在 1440×900 与 390×844 验证五幕及玻璃中段、正倒滚、停滑后图形不自走、无页面错误或横向溢出。强制禁用 ScrollTimeline 后的倒滚及减少动态效果检查通过；没有打开桌面测试窗口或运行其他浏览器。网站脚本 TypeScript、detector 和 diff 检查通过。

证据：`/Volumes/disk0/Khor/evidence/site-scroll-return-20261116/`。临时脚本 `/tmp/playwright-test-khor-scroll-return.js` 与 `/tmp/playwright-test-khor-story-pace-paths.js`。


固定光源补充验证：Headless Chrome 在桌面、手机尺寸与强制 RAF 后备中采样起点／中段／终点及倒滑，确认光场与材质同步增强、图稿位置固定且没有移动遮罩；减少动态效果显示完整材质，平面层隐藏。TypeScript、detector、diff 检查通过。新证据 `/Volumes/disk0/Khor/evidence/site-light-source-20261116/`，脚本 `/tmp/playwright-test-khor-light-source.js`；已随 2026-11-16 网站快照发布。
