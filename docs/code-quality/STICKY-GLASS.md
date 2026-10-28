# 滚动固定区域毛玻璃 Lab

2026-10-28，R2-024。用户提供 IMG_5541.png，要求所有随滚动保持的组块（如标题）与下方内容不画分割线，以毛玻璃区分，并先做 Lab。状态：**Lab 就绪，待用户验收视觉和体感；未接入正式产品。**

## 入口与版本

- 入口：<http://localhost:23832/lab/sticky-glass.html>。
- 工作副本：`/Volumes/disk0/Khor/worktrees/current/khor-workspace-layout-lab`，分支 `lab/workspace-layout-20261028`。
- 源码：`3791fd70ac83937a3512715237aede5893a02df1`；仅新增 `apps/gui/lab/sticky-glass.{tsx,css,html,md,tsconfig.json}` 五个文件。
- 基线：main `e7c335c8dd81de091bd12b92cd704a63f25d03f3`。既有工作区参考版 `c501029490bf5997a271e252941582911d421a8f` 保持不变，新材质只在新路由生效。
- 服务沿用隔离 Vite，在 `apps/gui` 执行 `node_modules/.bin/vite --config lab/workspace-layout.vite.ts`；按实际端口与工作目录核现场，不按旧 PID 清理服务。

## 验收内容

顶部可选择会话、列表与分组、提交与文件、表格四个场景，切换深浅色；更多菜单可对比原有边线、回到顶部。

1. 固定主标题、分组标题、提交文件标题和表头统一去除底线与下沿阴影。滚过背后的内容被模糊，标题文字和按钮保持清楚。
2. 复用已有 band / band-blur token；底部用 gap-row 范围柔和淡出。二级固定标题停在主标题下方，不重复加入 band 高度。
3. 普通列表、代码与静态内容的原有边界保留；本需求只改变滚动保持区域的材质。
4. 无 backdrop-filter 或减少透明时回退同主题实色，维持可读性。

样式仅在 `.sg-lab` 内覆盖。复用真实 PaneBar、SideList、Glass、Composer、Tabs、CommitPage、CodeMirror；数据与操作是隔离样例。默认浏览器入口由 workspaceLayoutFixtures 拦截 API，不带 bridge 参数运行；不通过原生 bridge 运行此 Lab。消息、新建、搜索、排序只操作样例状态，不写用户文件。

## 已完成验证与限制

- `node_modules/.bin/tsc --noEmit -p lab/sticky-glass.tsconfig.json` 通过。
- 1280×900 桌面深色、390×844 窄屏深浅色 × 四场景，共 12 项通过。检查真实滚动、固定位置、二级标题位置、零底线/阴影、实际 blur、无横向溢出及运行错误；另外验证列表搜索、新建和原样对比。
- 证据：`/Volumes/disk0/Khor/evidence/labs/sticky-glass-20261028/final/report.json`，同目录截图；主智能体已目视核对截图。
- 独立源码审查完成，无阻断发现。源码新增范围与 CSS 局部作用域确认；默认浏览器入口的隔离限制已记录。
- 原失败保留：run1 暴露重复 band 偏移和样例头像根元素尺寸，已修正；run2 剩余两项是移动提交说明较长，测试固定滚动量尚未跨过吸顶阈值。按首文件实际几何计算滚动量后通过，没有为测试再改产品样式；diagnostic 保留。

未进行原生 App、物理 iPhone 或全产品页面验收；浏览器检查不替代用户视觉和体感验收。main/App 未改动或重启，无合并/ship；S/C/B 与工作区正式接入仍待下一轮授权。用户验收后，将此材质 Lab 与既有工作区布局参考分别交给下一轮。
