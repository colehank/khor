# Shell

六个区域拼起来的那副骨架：**L0** 第一行 · **L1** 竖栏 · **L2** 列表 · **R0** 工具栏 · **R1** 板 · 窗口本身。页面去填它，不改它。

这个组件在系统里的用处是：**骨架变一次，只在一个地方看得见变化**，而不用在二十个预览里各画一遍。真正的 app 里有它自己的那一份。

尺寸：两条第一行都是 `pane-head` 48；竖栏 `rail-w` 64；列表默认 `list-w` 288，能拖到 `list-w-min` 180；板离四边 `gap-in` 8、圆角 `radius-xl` 12。

**宽度只有一个主人。** L 右缘那道缝是唯一的把手，L0 跟着它走，搜索框在溢出之前先压成一枚放大镜。拖到三个宽度，量出来的四个宽度必须一致——这是一道门。

**拖动区域由外壳声明一次**，在 L0、L1、R0 上；L2 的空白不拖（D3）。组件内部永远不自己标拖动区域。

## 用

```jsx
<Shell
  head={<SideHead placeholder="搜索会话" />}
  rail={<Rail items={landings} />}
  list={rows}
  toolbar={<Toolbar variant="tabs">{tabs}</Toolbar>}
  board={<Board joined day={day}>{page}</Board>}
/>
```

## Do / Don't

- **R0 和 R1 之间什么都不许加**，只有 4 px 的「一天」是例外。
- macOS 上左块用 `sidebar` 材质（D4），半透明透出桌面；Windows、Linux、网页端用 `side` 实色。这个预览画的是实色那一版。
- Windows 的窗口按钮在 **R0 右端**，L0 从搜索框开始。
- 窄档（< `narrow` 760）变成「列表一屏 ⇄ 详情一屏」，竖栏变底部栏，板铺满、不留边距和圆角。
