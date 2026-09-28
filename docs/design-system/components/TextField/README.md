# TextField

要打字的那一格。macOS 的 `NSTextField` 放在表单里的样子：**和搜索框是一家**——同一种 `fill` 底、`radius-sm` 6、`ctl-md` 28 高，没有描边，没有阴影；字是 `body` 13 配 `ink`，占位文字 `tertiary`。有焦点时外面一圈 2 px 的 `ring`，贴着框画（offset 0），圆角跟着框走——和开关、分段控件的焦点环同一种画法（2026-09-28）。

**为什么不是网页那种框。** 此前 `ui/input.tsx` 是 shadcn 原样：`rounded-md` 8 的角、一圈 `input` 描边、Tailwind 自己的 `shadow-xs`，焦点时一圈半透明的 3 px 光晕，窄一档就把字降到 `callout` 12。一张 sheet 里，弹出按钮（`field`）、分段控件的槽、搜索框都是 `fill` 的底，只有要打字的那两格穿着另一身衣服，读起来像从别的 app 借来的。

```
名称   [ 我的会话                        ]   ← fill 底，没有线
目录   [ ~/work/khor                     ]
       ╭─────────────────────────────────╮
PIN    │ ••••••                          │   ← 有焦点：2 px ring
       ╰─────────────────────────────────╯
```

## 用

```jsx
<Input value={name} onChange={…} placeholder="名称" />
<Input type="password" aria-label="PIN" />
```

桌面端真实实现：`apps/gui/src/components/ui/input.tsx`（`Input`）。sheet、对话框、设置页、锁屏里的每一格都是它；搜索框是同一个 `Input` 加左端的放大镜（`pl-7`），见 `SearchField`。原地改名（`EditableName`，标签、会话头、行里双击）是同一种底和焦点环，只是字号跟着它替换的那个名字。

| 状态 | 长什么样 |
|---|---|
| 平常 | `fill` 底，`ink` 字，`tertiary` 占位 |
| 有焦点 | 外面一圈 2 px `ring`（offset 0） |
| 不可用 | 整格 0.4 透明度（和按钮一样），**光标还是箭头**——AppKit 在灰掉的控件上不换光标，灰已经说了「不能」 |
| 值不对（`aria-invalid`） | 那一圈换成 `destructive` |

热区：高度下限是 `--hit`（鼠标时就是 `ctl-md`，手指时涨到 `touch` 44），样子不变。

## Do / Don't

- 标签在左一栏、右对齐（`sheet-form`），框在右一栏；**不要**把标签压在框上面做成一摞。
- 说明写在框下面，`callout` 或 `caption` 配 `secondary`，不写进占位文字——占位文字一打字就没了。
- **不要**给框加描边或阴影来「显得像输入框」；`fill` 的底就是它的形状。
- **不要**写 `cursor-not-allowed`，也不要给框换手指光标。
- 选项是一串固定的值时，它就不是文字框了：用弹出按钮或分段控件。

## 不是它的

- **对话的输入框**（`ChatView` 的 `Textarea`、机器之间的留言框）是写字的面，不是表单里的一格：它站在阅读栏里、字号跟着阅读栏（`read` 16），保留一圈 `hairline` 的框。它和这里是不是一家，留给负责人在真 app 里看了再定（2026-09-28 记下，没动）。
- **终端里的查找条**是浮层里的一条（20-states 浮层），框本身透明、坐在 `popover` 上。
