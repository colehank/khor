# Dialog 与任务底栏

任务先决定操作含义，屏幕再决定摆放。2026-11-10 用户确认采用下面的统一约定；它取代旧版“所有弹窗都没有关闭按钮”“手机提交在底部”“机器头像即时写、名字等完成”的规则。

## 类型与出口

| 类型 | 语义 | Desktop | Mobile |
| --- | --- | --- | --- |
| 草稿编辑、新建 | 取消丢弃草稿；保存／添加／连接提交；失败保留内容 | 右下取消＋唯一主操作 | 顶部左取消、右主操作、中央标题 |
| 即时管理 | 已生效的设置不伪装成可取消；关闭离开 | 标题旁关闭；任务动作在底部 | 顶部关闭；任务动作在右侧 |
| 多步骤 | 返回上一步；保存子项后回父页 | 返回＋保存按任务位置排列 | 顶部左返回、右保存 |
| 只读预览 | 关闭；打开文件／导出是独立工具 | 标题旁标准关闭 | 顶部标准关闭 |
| 危险决定 | 取消＋明确的删除／重置动作 | 公共短确认布局 | 公共短确认布局，不扩大成长表单 |
| 就近选择 | 选择后回原处；多选菜单支持连续选择 | PopupButton／就近菜单 | 同样的就近菜单 |
| 持续浏览 | 会话／文件／网页自己的返回、收起、关闭 | 侧区与标签栏 | 原有药丸与内容甲板 |

桌面右下提交是 **Khor 的应用约定**；Apple HIG 不要求所有 macOS 弹窗的按钮都在右下。`Back` 不是关闭整个任务，`Cancel` 不能承诺撤销已经落盘的操作。邀请凭据、认证与安装进度等已经产生副作用的流程用关闭。

## 公共组件

```tsx
<Dialog open={open} onOpenChange={setOpen}>
  <TaskOverlay purpose="edit">
    <DialogTitle>重命名</DialogTitle>
    <DialogBody>{/* 草稿内容 */}</DialogBody>
    <TaskActions dirty={name !== original} busy={saving}
      cancel={<Button onClick={onClose}>取消</Button>}
      primary={<Button onClick={save}>重命名</Button>}/>
  </TaskOverlay>
</Dialog>
```

- `TaskActions` 声明 `cancel`、`back`、`close` 之一，以及可选 `primary`、`tools`。可以放在持有状态的子表单里，但一个任务同时只有一组操作；不自行定位按钮、不读取按钮文字猜用途。
- `TaskOverlay` 负责标题与操作的固定布局；正文只有一个公共滚动区。`DialogBody` 在任务内只是内容分组；预览器自身的文档滚动仍由预览器负责。
- 同一组按钮跨断点移动位置，输入控件不重挂载。不要通过改 App 窗口大小获得空间。
- 一个任务一个主按钮；导出、复制、删除自定义配色等放 `tools`，不与提交混成一排同等强调操作。
- `dirty` 只表示未提交草稿。Escape／遮罩／下拉关闭统一请求退出：有草稿先询问保留编辑或丢弃；显式取消／返回是明确放弃当前草稿。`busy` 表示不可取消的写入，操作期间不退出、不重复提交；可取消认证用关闭主动中止等待。
- 没有操作槽位的只读／选择任务提供标准关闭。非任务内容需要明确 `layout="content"`，目前仅会话标签总览、文件／提交历史甲板、网络图机器卡；它们已有完整导航。
- 长编辑用 `initialHeight="large" maxHeight="large"`；选择默认中档到大档；短编辑适配内容。只有能拖动／改变高度的任务才显示抓手。
- 标题、面、圆角、间距和控件高度用现有 tokens。桌面浮层 `popover`＋`float`＋`hairline`；手机任务底栏沿用 `background`、上沿线与 `menu-r`。遮罩统一 `sheet-scrim`。

文件与目录这种持续浏览页面使用 `ContentOverlay` 组合现有 `TaskOverlay`：保留公共遮罩、抓手、焦点和手势，内容不再继承表单的横向与底部内边距。里面直接放正式 `FilePage`／`FilesPane`，由页面负责自己的内容边距和导航安全区。附件预览与机器目录使用同一个容器，避免预览再次缩窄。

`ContentDeck`／`ContentPill` 是现有会话内容底栏的公共行与药丸；工作区文件页也使用同一个药丸。头像放在 `PillFace` 的居中座位内，与 session 的 `DeckBand` 共用占位，不把头像本身直接贴到胶囊边缘。实际 App Lab 的附件流程检查预览完整内容宽度、头像座位、底栏对齐与终端尺寸保持。

## 边界与手势

所有弹窗以 `visualViewport` 的高度和偏移为边界，包括软键盘、安全区和低矮桌面窗口。长内容能滚到最后一项，主操作始终可达。菜单的手势属于菜单，不能拉动外层底栏；路径补全参与正文高度，不被裁掉。

`overlay-policy.ts` 决定呈现与停靠；`use-task-overlay` 处理内容高度；`use-sheet-pan` 处理拖动、滚动交接和取消。非输入类任务不自动唤起键盘。任务之间替换时，退出的任务不能抢走新任务焦点；真正关闭时回到打开控件。嵌套步骤使用同一容器或替换任务，不叠两张任务 sheet；放弃草稿是短确认。

## 验证

`npm run dialogs` 验证真实公共组件与表单的视口、操作位置、草稿退出、长列表、焦点和手势归属；`npm run dialogs -- --serve` 提供一次性数据的检查入口。真实 App Lab 同时覆盖键行、配色和设置，发布前另做 iOS 键盘／惯性／安全区与桌面窗口验收。合成 touch 探针不冒充 WKWebView 原生实测。

官方依据：[Sheets](https://developer.apple.com/design/human-interface-guidelines/sheets)、[Modality](https://developer.apple.com/design/human-interface-guidelines/modality)、[Popovers](https://developer.apple.com/design/human-interface-guidelines/popovers)、[Toolbars](https://developer.apple.com/design/human-interface-guidelines/toolbars)、[Alerts](https://developer.apple.com/design/human-interface-guidelines/alerts)。
