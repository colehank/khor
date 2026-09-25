# 设计

**这份文件已被取代。现行的设计系统在 [`docs/design-system/`](./design-system/)。**

改任何界面之前，从那里的 `README.md` 开始；Claude Code 走 `.claude/skills/khor-design`。

## 去哪儿找原来那些内容

| 原来在 DESIGN.md 的 | 现在在 |
|---|---|
| 颜色、品牌绿的岗位、六个状态词 | `design-system/README.md` 的「颜色」，逐个 token 的值和用途在 `design-system/tokens.json` |
| 字号与文字样式 | `design-system/README.md` 的「字」，14 个样式在 `tokens.json` 的 `type` |
| 间距、圆角、材质与深度 | `design-system/README.md` 的「间距、形状、材质」 |
| 骨架、六个区域、三种页型 | `design-system/guidelines/10-layout.md` |
| 区域状态、浮层用哪一种 | `design-system/guidelines/20-states.md` |
| 行为约定与它们的门、Do/Don't | `design-system/guidelines/30-behavior.md` |
| 组件（竖栏、列表行、标签、设置组、对话框…） | `design-system/components/<Comp>/README.md`，22 份 |
| 图标 | `design-system/components/Icon/README.md`；字形本体在 `apps/gui/src/components/icons.tsx` |
| 什么必须走 token、有哪几道门（`npm run` 清单）、动了什么跑哪几道、怎么加新档 | `design-system/guidelines/50-gates.md`（`npm run gates` 读它） |
| 浏览型外壳的契约 | `design-system/guidelines/60-browse-shell.md` |
| D1–D10 那些待定决议 | `design-system/guidelines/40-migration.md`，连同 D11–D13 和它们的落地状态 |

## 为什么换掉而不是继续改

原来这份是**现状的描述**：它记录 `tokens.css` 今天是什么样。新系统记的是**规则和理由**
——每个 token 带用途和量到的对比度，每条规则后面跟着那道能让构建变红的门，每处走过的
弯路立一块墓碑。区别在于：描述会随代码漂移且没人发现，规则漂移了门会变红。

旧内容在 git 历史里，需要时 `git log --follow docs/DESIGN.md` 调得出来。
