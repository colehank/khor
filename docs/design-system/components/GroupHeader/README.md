# GroupHeader

列表和设置里的分组标题。`group` 11/600 配 `tertiary`，左端一个可转的人字（收起时指右），右端是计数。可折叠。

**置顶那一组和其余之间有一条 `Rule`**——1 px `hairline`，左右各留 `gap-in`。整个列表只有这一条线；别的分组之间靠标题本身的留白分开。

## 用

```jsx
<GroupHeader count={2}>置顶</GroupHeader>
<GroupHeader open={false} count={12}>这台机器</GroupHeader>
<Rule />
```

消费方负责：标题文字、`count`、折叠状态和它的持久化。

## Do / Don't

- 计数是 `tertiary` 的常规字重，不是徽标。真要叫人的数字用竖栏图标上的 `k-badge`。
- 折叠状态要记住，跨重启也记住。
- 组名是名词（「置顶」「这台机器」），不是句子。
