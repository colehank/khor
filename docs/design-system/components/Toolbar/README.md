# Toolbar

右边的第一行（R0），高 `pane-head` 48，**每一页都有**。两种形态：

- **`variant="title"`**（页型 A、C）——后退/前进 + 页面标题（`title-sm` 15）。这对箭头走的是**在这个落地页里做过的选择**，和「系统设置」一样；无处可去时它不出现（D2）。
- **`variant="tabs"`**（页型 B）——标签条 + ＋。

页型 C 用 `address` 顶掉标题：那一行本身就是地址栏。

**页面级按钮只在这里，只在右端**（D9）。行的动作在右键菜单。不许两边都有。

## 用

```jsx
<Toolbar title="外观" />
<Toolbar title="turing" trailing={<><Button size="sm">唤醒</Button>…</>} />
<Toolbar address="lan.cibol.khor.cloud/plans" />
<Toolbar variant="tabs" trailing={<Button kind="plain" icon="plus" />}>{tabs}</Toolbar>
```

消费方负责：标题或地址、历史能不能走、右端按钮。

## Do / Don't

- **身份只出现一次**：R0 的标题（A、C）或者标签（B）。板里**不许**再有第二个标题。
- **R0 和 R1 之间什么都不许加**：不加副标题行，不加状态句。会话页那条 4 px 的「一天」是唯一的例外，而且它没有文字。
- 「过期 / 离线」那行字挂在 R0 **右端**，用 `caption`，不是板里的横幅。
- 这一行的空白处拖窗口。
