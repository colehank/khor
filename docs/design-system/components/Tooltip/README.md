# Tooltip

指过去时出现的一个名字。**只给没有字的控件用**——竖栏的字形、只有图标的按钮。控件上已经写着字的，不再配提示。

样子：和内容一样宽（`w-max`），最宽 256，超过才折行；`caption` 11 配 `ink`，底 `popover`，1 px `hairline` 描边，`radius-sm` 6，`float` 一档阴影；内边距上下 `gap-xxs` 4、左右 `gap-in` 8。出现在控件的右边（竖栏）或下边（工具栏），离控件 6。

竖栏的提示**不等待**：指过去立刻出现（系统的 `title` 要等 1.5 秒，这正是它不用系统提示的原因）。窄档没有指针，不出现，名字由 `aria-label` 给读屏。

「一天」那条横条指过去时的读数也是这个样子（`popover`、`float`、`radius-sm`，2026-09-28 之前是 `rounded-md` 配 `shadow-sm`）。

## 用

```jsx
<Tooltip>
  <TooltipTrigger asChild>{railItem}</TooltipTrigger>
  <TooltipContent side="right">机器</TooltipContent>
</Tooltip>
```

消费方负责：那一个名字、出现在哪一侧。

## Do / Don't

- **一个名词**，就是这个控件的名字（「机器」「更多」），不写句子、不写快捷键说明。
- 宽度只由字决定。**2026-09-26 的墓碑**：`app.css` 里给左块定宽的 `[data-side]` 同时选中了 Radix 挂在每个浮层上的 `data-side="right"`，「机器」两个字的提示被撑成 353 宽、还盖住了列表行。给区域打的属性名，别和组件库自己的属性撞名；撞了就把选择器收窄到那个元素（`aside[data-side]`）。
- 不放按钮、不放链接。要人去点的东西不能只在指针停着时才存在。
- 需要解释的设置，用设置行的 `note`，不用提示。
