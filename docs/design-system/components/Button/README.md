# Button

普通按钮，`fill-strong` 的底、`ink` 的字、`radius-sm` 6 的角。三档高度：`ctl-sm` 22 给密排的行，`ctl-md` 28 是默认，`ctl-lg` 32 只给对话框。指针是手指时热区涨到 `touch` 44，样子不变。

三种变体，用途是排他的：

- **push**（默认）——设置里的动作行、页面级操作。
- **primary**——`brand-strong` 的底、`on-brand` 的字。**一个对话框里最多一个**，而且只在对话框里。列表行里、板里都不许出现。**唯一的例外**：新标签页上的「创建」（`data-new-session-create`，2026-10-07 负责人定）——那一页就是原来的新建会话对话框，`check-face` 第十七轮 b 只放过这一颗。
- **destructive**——`destructive` 的底、`on-destructive` 的字。**只在确认窗里**，别处的删除走右键菜单。

## 用

```jsx
<Button>管理…</Button>
<Button kind="primary" size="lg">删除</Button>
<Button kind="plain" icon="plus" aria-label="新建" />
```

消费方负责：文案、`onClick`、以及只有图标时的 `aria-label`。

## Do / Don't

- 还会再问一次的动作，文案带省略号：`管理…`、`生成邀请…`、`清除…`。当场就做完的不带。
- **一个动作一个家**（D9，2026-09-30）：打开的那一样东西的动词放 R0 右端，会让新东西出现的动作只在 L0 的 ＋，行的动作在右键菜单；不要在板里摆一排按钮，也不要让一个动作在两处都有。R0 的文字按钮是默认的 `ctl-md` 28、`body` 13，不是 `sm`——`sm` 只给密排的行（见 `Toolbar`）。
- 按下时 `scale(--press)` 0.97，`dur-120` `ease-out`；开了「减弱动态效果」就没有。
- 别给按钮加阴影。`float` 只给浮起来的东西。
