# 弹窗与任务底栏统一（2026-11-10）

状态：实现与验证中。用户已同意按 Apple HIG 与 Khor 的任务语义统一整个 app 的弹层。
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
2. 已实现第一版公共标题与语义操作槽位，统一平台摆放、正文滚动和退出策略。
3. 逐一迁移任务弹层；短确认保留公共确认布局；明确记录持续浏览例外，修正混合即时／草稿语义。
4. 更新设计规范，删除旧的“任何弹窗都没有关闭”和手机底部提交等冲突规则。
5. GitHub 托管 CI 验证（首版 d2b68522 / 42696470999；当前正补充子步骤与遗漏的订阅导入）公共组件与真实 App Lab：桌面／窄屏、长标题、滚动、键盘、退出、子步骤、异步失败与草稿。
6. 提供可信构建的实时 Lab，等待用户验收，不发布。

## 恢复入口

- 盘点：`/Volumes/disk0/Khor/evidence/more-network-gui-20261109/overlay-inventory-20261110.json`。
- 实现：`apps/gui/src/components/ui/dialog.tsx`、`lib/overlay-policy.ts`、`styles/app.css`；所有任务调用点来自上述盘点。
- 既有 Lab：`http://127.0.0.1:23950/app/?scene=devices&machines=3`，目前仍是 `394afa05` 旧产物；新代码未构建前不要声称已上线。
- 本地只改源码／静态检查，Node/Rust/GUI 执行通过公开 GitHub 托管 CI。gh 路径见旧 handoff。
- 旧任务记录：`docs/handoff/settings-terminal-redesign-20261109.md`。

官方依据：[Sheets](https://developer.apple.com/design/human-interface-guidelines/sheets)、[Modality](https://developer.apple.com/design/human-interface-guidelines/modality)、[Popovers](https://developer.apple.com/design/human-interface-guidelines/popovers)、[Toolbars](https://developer.apple.com/design/human-interface-guidelines/toolbars)、[Alerts](https://developer.apple.com/design/human-interface-guidelines/alerts)。

## 当前实现与验证注意

- 公共 `TaskActions` 按明确 cancel/back/close/primary/tools 槽位摆放，不根据文案猜。真实字段跨断点不重挂载。长内容固定标题／操作，一个正文 scroller。
- 键行管理即时变化用关闭；子键稿返回／保存；机器头像管理即时变化，改名为同容器子步骤。配色、重命名、额度等保护草稿；进行中的不可取消写入禁止重复退出。
- 已迁移账户、网络、添加机器各步骤、预览、导入、启动方式、目录、键行。短确认沿用共享确认布局。
- `dialogs` 已加入 settings/all 的 GUI 交互组，探针保留原视口／手势断言，并新增平台动作位置、长标题、草稿退出、失败保留、断点 DOM 身份检查。
- 上轮 087a272c 的 Quality 42686283647 已确认全部成功；不能作为本轮改动的验证结果。

### 验证进行中

`b8a88808` / Quality `42697107254` 前端构建及 TypeScript 已成功，GUI 与真实 App Lab 正在运行。未提交补充包括正文错误时保留关闭入口、预览边距、写入中冻结草稿、交互截图上传；需一起提交并验证最终 SHA，不能把 b8 的结果算给新代码。当前 Lab 仍是旧394，发布保持暂停。

`42697107254` 的 GUI 已读日志：所有 unit/types/static、通知、键行（桌面＋320/390 手机＋真实 xterm）、外观均通过；新增启用的 dialogs 首先卡在旧夹具 `devices=[]`，真实新会话按规则禁用启动方式选择。正在给该隔离夹具补一台可宿主机器，保留全部断言，不改生产规则绕过它。
