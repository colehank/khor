# 桌面标签与窗口精修（2026-10-17）

## 范围

本轮用户确认的 Lab 全部落地，见同日 pending 的「桌面与来源补齐」。路径默认家目录和新建 session 的新话题按用户要求留待下一轮。

## 验收

- 修复前真实内容回归在空组展开后等不到侧区，记录 /Volumes/disk0/khor-desktop-tab-polish-red.log。
- TypeScript、前端生产构建、boot、px、type、icons、select、gates 通过。
- content-model 通过，包括节点内容排序不改变选中和其它 session。
- 节点原子排序测试通过：并发新增／关闭、其它 session、重复 id、置顶、元数据与 active 保留；配置相关 Rust 测试 7 项通过。
- session-content 完整 Chrome 桌面＋手机通过；额外桌面段覆盖六个长标题、窄宽度、两层连续关闭与鼠标排序、重载、Esc、预览保持和单 ＋。
- appearance 通过：预设、DIY、JSON、缓冲与 PTY 保留、透明 0%–100%、保留／隐藏终端、WebGL／DOM、窗口轮廓默认与开关、点击穿透及背景 alpha 不变。
- content-ui 13 步通过：来源图标与箭头、输入空间、历史删除、浏览机器选择、320／390／430 宽度、键盘与安全区。
- 全量 palette 未通过：标签、玻璃和选中材质前段测量通过，但头像抽样后段得到部分对比度 1.00；第二次独立运行也在 light 头像段失败。Avatar／FaceMark／配色 token 本轮未改动，尚未证明是既有产品问题还是抽样器问题，不能记为通过；第二次已失败的长扫描提前停止。记录 palette.log、palette-final.log，后续定位，不通过放宽阈值来消除。
- macOS 隔离原生 app 验收：空组恢复工作区、文件侧区、session 原玻璃与内部半透明同时高亮；系统红黄绿按钮、窗口透明度调节和独立轮廓。发现并修复外观中轮廓开关重复出现的问题。最终设置页只出现一个开关，关闭／开启不改变 98% 背景透明度。
- 最终原生拖动验收补齐了 Tauri 的 tabs_reorder 命令注册；实际鼠标拖动文件标签成功，并经私有节点 tabs_list 确认 example.ts、README.md 顺序持久化，当前文件未改变。会话层 Kapelntro 从第三位拖到首位，当前 session 保持不变。
- Windows／Linux 缺少实机环境，本轮是通用前端轮廓＋目标分支代码检查，不能称作平台真机通过；iPhone 原生键盘及网页叠层也没有新增真机结论。

## 证据

- 详细日志：/Volumes/disk0/khor-desktop-tab-polish-*.log
- 完整流程截图：/Volumes/disk0/khor-checks/desktop-tab-polish-full
- 最终桌面回归：/Volumes/disk0/khor-checks/desktop-tab-polish-final
- 手机操作：/Volumes/disk0/khor-checks/desktop-tab-polish-mobile
- 原生截图：/Users/zgh/khor-drop/desktop-tab-polish（native-final.png、session-order-final.png、appearance-final.png）

## 交付状态

实现位于外挂盘独立分支 desktop-tab-polish；未合并 main、未 ship。新增构建不替换当前运行 app；私有节点、会话与临时验收 app 已清理。提交前图工具在该 worktree 没有符号覆盖；commit hook 后已增量索引，复查识别 65 个变化符号、风险 0.60、63 项静态覆盖缺口（含测试自身），不能以 0 affected flows 作为通过；已按实际 diff、类型检查、模型与真实操作补做检查。
