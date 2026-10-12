---
title: 会话
summary: 开会话、接上去、看状态
order: 3
---

shell、智能体、对话、文件传输都是会话，在同一个列表里：

```sh
khor sessions
```

## 开一个

```sh
khor open                 # 持久会话，本机的 shell
khor open -- claude       # 跑一条命令
khor open --on <机器>     # 开在另一台机器上
khor run -- <命令>        # 临时会话，跟着这个终端活
```

持久会话在终端关掉之后还在。接上去、关掉：

```sh
khor attach <session>     # Ctrl-\ 断开
khor close <session>
```

## 六个状态词

全网、每一种界面，同一个状态同一个词。

| 词 | 意思 |
|---|---|
| 忙碌 | 在跑 |
| 待批 | 有动作等你准许。排在最前 |
| 完成 | 这一轮完了，还没看 |
| 中断 | 出错了，进程还在 |
| 失败 | 进程以非零状态退出 |
| 空闲 | 没在跑，或者看过了 |

## 智能体

Claude Code、Codex 的会话在 khor 装好后自己出现在列表里，不用配置。给 Claude Code 装上钩子，才有「完成」这一档：

```sh
khor hooks install
```

在 khor 之外开的智能体会话和 tmux 会话，可以导进列表：

```sh
khor import               # 列出可导入的
khor import <序号>
```

## 文件

```sh
khor send <机器> <文件>   # 摘要先到，对方许可后才传
khor accept <传输号>      # 在收文件的那台上
khor pull <机器> <文件>   # 把那台机器上的文件拉过来
khor ls <机器> [路径]
```
