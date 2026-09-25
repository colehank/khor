# ListRow

L2 列表里的一行。有副标题就是两行、高 `row-2` 48；没有就是一行、高 `row-1` 32。左右内边距 `gap-in` 8，各部分之间 `gap-row` 12，选中块圆角 `radius-md` 8。

三个槽：**leading**（头像或字形块）· **标题 / 副标题** · **meta**（右端的时间、计数、状态词）。标题 `body`，选中时换成 `headline`；副标题 `callout` 配 `secondary`；meta `caption` 配 `tertiary`。

## 选中有两种（D6）

- **窗口在前**：底 `brand-strong` 实心，**三行字全部满值 `on-brand`**。
- **窗口不在前**：底 `fill-strong`，字回到 `ink`。

## 用

```jsx
<ListRow
  selected
  leading={<Avatar size={32} mark="chat" />}
  title="khor"
  subtitle="把侧边栏的搜索和窗口按钮并到一行"
  meta={<><StatusWord word="busy" /><br/>1 分钟</>}
/>
```

消费方负责：三个槽的内容、`selected`、`onClick`。行**不提供**自己的动作按钮。

## Do / Don't

- **选中行上不许用透明度分主次。** 白在 `brand-strong` 上是 4.89，降到 0.92 就只剩 4.43。主次靠字号和字重。
- **行的动作只在右键菜单里**（头两项：置顶、删除），不要在行里内嵌一排按钮——那会让行的宽度变成一个可以争的东西。
- meta 是 `tertiary`，它低于 4.5，**只放扫一眼的东西**。必须读到的（比如「等你」）用 `StatusWord`，它自己带颜色。
- 列表空白处**不拖窗口**（D3）。
