# 弹窗与任务底栏统一（2026-11-10）

状态：执行中。用户已同意按 Apple HIG 与 Khor 的任务语义统一整个 app 的弹层。
工作副本：`integrate/settings-gui-20261109`；基线 `39db2fe1`。发布继续暂停；不合并、不 ship、不重启用户 App。根 agent 串行执行，不新增智能体。

## 约定

- 草稿任务：取消 + 保存／添加／连接。桌面右下，手机顶部左右；标题和操作固定，正文单独滚动。
- 即时管理：关闭，不用不能回滚的取消／完成。子步骤返回；编辑子项保存后回父页。
- 只读预览：标准关闭；危险决定：取消 + 明确动作。就近选择菜单不加无意义的完成。
- 会话／文件／浏览器的持续浏览甲板保留自己的导航，不改成表单。
- Escape、遮罩和下拉关闭走同一个退出策略；保护草稿和进行中的操作；不堆叠任务 sheet。
- 平台差异由公共组件处理。桌面右下是 Khor 约定，并非 HIG 对所有 macOS 弹窗的硬性要求。

## 计划与完成条件

1. 已完成：读设计规范、官方 HIG；盘点 38 处弹层，图谱没有覆盖后使用定向源码读取。
2. 实现公共标题与语义操作槽位，统一平台摆放、正文滚动和退出策略。
3. 逐一迁移任务弹层；短确认保留公共确认布局；明确记录持续浏览例外，修正混合即时／草稿语义。
4. 更新设计规范，删除旧的“任何弹窗都没有关闭”和手机底部提交等冲突规则。
5. GitHub 托管 CI 验证公共组件与真实 App Lab：桌面／窄屏、长标题、滚动、键盘、退出、子步骤、异步失败与草稿。
6. 提供可信构建的实时 Lab，等待用户验收，不发布。

## 恢复入口

- 盘点：`/Volumes/disk0/Khor/evidence/more-network-gui-20261109/overlay-inventory-20261110.json`。
- 实现：`apps/gui/src/components/ui/dialog.tsx`、`lib/overlay-policy.ts`、`styles/app.css`；所有任务调用点来自上述盘点。
- 既有 Lab：`http://127.0.0.1:23950/app/?scene=devices&machines=3`，目前仍是 `394afa05` 旧产物；新代码未构建前不要声称已上线。
- 本地只改源码／静态检查，Node/Rust/GUI 执行通过公开 GitHub 托管 CI。gh 路径见旧 handoff。
- 旧任务记录：`docs/handoff/settings-terminal-redesign-20261109.md`。

官方依据：[Sheets](https://developer.apple.com/design/human-interface-guidelines/sheets)、[Modality](https://developer.apple.com/design/human-interface-guidelines/modality)、[Popovers](https://developer.apple.com/design/human-interface-guidelines/popovers)、[Toolbars](https://developer.apple.com/design/human-interface-guidelines/toolbars)、[Alerts](https://developer.apple.com/design/human-interface-guidelines/alerts)。
