# LIVE · 弹窗与任务底栏统一 · 2026-11-10

实现完成，等待用户 Lab 验收；发布继续暂停。用户未授权本轮 merge/ship。根 agent 串行；本机仅源码和可信静态产物服务，所有构建与浏览器测试都在公开 GitHub 托管 CI。

## 权威状态

- 工作副本：`/Volumes/disk0/Khor/worktrees/current/khor-settings-gui-20261109`，分支 `integrate/settings-gui-20261109`。
- 已验证并 LIVE 的源：`3a203ac38e9b1dc852efdec88596b6814022d8bf`，tree `e49c98e004f5341004fcec5193368a62204b0baa`。3a 相对产品 df021ca6 只修测试选择器。
- 精确 Quality：[42700929380](https://github.com/colehank/khor/actions/runs/42700929380) **SUCCESS**。frontend、GUI unit/types/static、通知／键行／外观／dialogs 全部通过；真实 App Lab **56 项无错误**，热更新保留草稿通过。旧失败与取消记录不算当前验证。
- Lab：`http://127.0.0.1:23950/app/?scene=devices&machines=3`。可信产物 `evidence/more-network-gui-20261109/overlay-3a-review/developer-lab`；Python exec session **92335**。新页面和 JS、旧394／ce0／269／pinned 的 snapshot 页面和 JS 均已 HTTP200 核验。
- 已看真实 App 的手机／桌面键行、手机配色、390×400 深色配对结果截图。标题现在落在屏幕中心；正文滚动，操作固定。
- 只替换已核对 PID 与 argv 的旧 Lab Python15607；安装版、宿主、用户浏览器导航与刷新没有操作。未用 CUA。

## 最终行为

`TaskOverlay` 管标题、正文及 `TaskActions` 的 cancel/back/close/primary/tools 槽位。编辑操作桌面右下、手机顶部；即时管理和只读内容用标准关闭；子步骤返回／保存。手机左右轨取真实动作宽度的最大值，标题精确居中。跨断点保持同一份 DOM；未保存内容在隐式退出时保护，进行中的不可取消写入冻结草稿；正文出错保留标题和关闭入口。

已迁移机器添加各步骤、机器资料与改名、额度／Pi 认证、网络、导入、目录、文件预览、重命名、启动方式、键行与配色。短确认保留公共确认布局；持续浏览的会话标签总览、历史甲板和网络图机器卡显式 `layout="content"`。机器头像即时写，名称为同一个容器中的草稿子步骤；启动方式保存返回管理页。

## 用户验收入口

1. 更多 → 会话 → 快捷键行：管理关闭、添加／编辑返回与保存，两种窗口宽度的位置与标题。
2. 会话 → 配色 → 自定义 → 设置：取消／保存配色、长表单滚动、Escape 草稿保留／丢弃。
3. 机器 ＋ → Khor／SSH：标题与动作一致；配对已提交／已加入后只有关闭，不能伪装成取消或再提交。

浏览器 Lab 不冒充 WKWebView 原生键盘、惯性与安全区实测；这些仍需真机验收。规范已写入 `docs/design-system/components/Dialog/README.md`，检查已加入 Quality 的 settings/all 范围。

以下为执行历史（以此顶部状态为准）。

---

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

`42697107254` 的真实 App Lab：设置桌面／手机、GUI、工作区、机器列表、多种设备数量、导航与创建手势全部通过；仅配对结果关闭时测试仍寻找“完成”，已改为明确验证标准关闭角色及“关闭”。视觉检查发现手机左右动作宽度不同会使标题偏移，公共 frame 已改为按真实按钮尺寸建立对称边轨，补了中心线断言。下一次验证须覆盖全部这些最终改动。
