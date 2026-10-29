# G 共用滚动材质

2026-10-29 working，用户“开始吧”授权本轮实施，不main/ship。owner round2_input从C稳定checkpoint轮换，作者树仍/Volumes/disk0/Khor/worktrees/current/khor-round2-content，fix/round2-content，G基线e58f21a1（C生产+隔离Lab已固定，root负责生产集成；Lab不进产品）。中央仅本卡由G作者写，其它总账root负责。

权威范围：ROUND-2-EXECUTION、STICKY-GLASS、SESSION-CONTENT，R2-013菜单材质、R2-022差异sticky标题、R2-024所有滚动固定组块。最终已验参考4d541e5eb83c242304c96a4aeb07682082a59a45，旧浓度方案只历史。不把首片段通过冒整G完成。

## 唯一写权与顺序

root批准首片段：apps/gui/src/lib/scroll-edge.ts现有band owner；app.css仅band/material/transparent相关块；必要新纯几何与真实定向检查；独立Lab入口复用正式实现。先交主标题/正文渐隐契约，再接分组/commit/sticky表头等消费者；新增具体consumer路径先报root协调。禁止第二全页observer；不凭空多拆geometry模块；不改tokens/native/ChatView/WorkspacePane/TerminalPane结构。设计系统规则先读，采用现有token/icon；真机/native仍root唯一槽。

R013先在真实ContentPathField上下文采样最终菜单背景/叠层，不以band透明度修菜单。网页地址推荐继续关闭。R022/R024共享同一材质所有权，W后续仅消费稳定属性/契约，不建第二套。

## 目标与约束

- 固定主标题、分组、提交文件标题、表头无底线或下沿阴影，整体自然过渡；普通内容边界保留。
- 标题文字/控件不透明。正文自下而上模糊/淡出，在最上端完全不可见；需保留二级sticky标题自己的满值文字，不把整scroller全局mask误伤内部sticky。
- 窗口底与正文分开：solid回主题实色，clear保用户opacity，frost保原生模糊；窗口底只画一次，不给标题叠固定tint/黑白块。沿既有设置，不加生产外观滑杆。
- 无blur/reduced-transparency时主题实色兜底；主题/透明切换不能改变阅读位置、首行/scroll range、键盘定位。
- 复用band当前scroller发现、测高、settleBands、observer与清理；真实终端scroller不进入band。迟到/卸载要移除仅自身样式，保留其它owner。

## 当前准备与证据

已只读核设计README/30-behavior及参考Lab，图谱对scroll-edge/PaneBar缺覆盖后才读源。当前band已拥有scroller、测高、Mutation/Resize observer；app.css现有scrolled tint+hairline及transparent取消overlap，后者与新契约冲突。参考Lab按局部正文/二级标题分别mask，不能直接复制sg-*全页扫描。

C3与终端Lab唯一23835/PID92646/exec53679已完成root当前批次；首片段会在不动旧入口的独立Lab复用产品实现。G evidence根/Volumes/disk0/Khor/evidence/round2/glass，片段按source SHA/ref、red/green/真实DOM、原生限制记录；非作者reviewer独审。当前尚无G产品修改或完成声明。
