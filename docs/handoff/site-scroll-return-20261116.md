# 恢复下滑叙事（2026-11-16）

用户要求恢复下滑切换，同时 Meet Khor 的光从左上扫到右下。本轮取代同日的自动播放尝试，仅本地修改，未部署。

- 恢复原生 WAAPI ScrollTimeline：图标随滚动连续变化，停滑即停，倒滑可逆。字幕在每段起点用既有短时渐变交接；没有计时播放、滚动吸附或手势拦截。
- 有效演化 3.6 屏，首句桌面 .5 屏 / 手机 .7 屏，总高 5.1 / 5.3 屏。保留段末 6% 的短暂落定与 smoothstep 归位。
- Meet 文案与光照在同一滚动章节起动。材质和背景用 45° 的固定羽化裁切从左上到右下揭示，内部图稿反向旋转保持原位。平面构图使用匹配的反向裁切，确保光到达的位置才玻璃化。
- 保留完整原始 SVG 图稿、静态滤镜；只移动 HTML 裁切与反向补偿层，不逐帧改写 SVG 遮罩或渐变。
- 旧浏览器沿用可见范围内的缓存 RAF 后备。移动端范围使用稳定 sticky 高度，不混用动态 innerHeight。减少动态效果保留紧凑静态章节，五个书签恢复滚动到对应姿态。
- 后续 Continue 的 6 秒去程／回程、Agent 的 12 秒四段依存，以及 You 整条超边亮起均保持。

Headless Chrome 在 1440×900 与 390×844 验证五幕及玻璃中段、正倒滚、停滑后图形不自走、无页面错误或横向溢出。强制禁用 ScrollTimeline 后的倒滚及减少动态效果检查通过；没有打开桌面测试窗口或运行其他浏览器。网站脚本 TypeScript、detector 和 diff 检查通过。

证据：`/Volumes/disk0/Khor/evidence/site-scroll-return-20261116/`。临时脚本 `/tmp/playwright-test-khor-scroll-return.js` 与 `/tmp/playwright-test-khor-story-pace-paths.js`。
