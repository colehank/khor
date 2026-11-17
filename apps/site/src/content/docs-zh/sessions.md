---
title: 会话
summary: 创建、重连会话并查看运行状态
order: 3
---

Shell、智能体、对话和文件传输共用一个会话列表：

```sh
khor sessions
```

## 创建会话

```sh
khor open                 # 持久化本地 Shell
khor open -- claude       # 运行命令
khor open --on <device>   # 在另一台设备上启动
khor run -- <command>     # 与当前终端绑定的临时会话
```

关闭终端后，持久化会话仍会继续运行。可重新连接或关闭：

```sh
khor attach <session>     # Ctrl-\ 断开连接
khor close <session>
```

## 会话状态

整个网络和各个 Khor 界面使用相同的状态。

| 状态 | 含义 |
|---|---|
| 忙碌 | 正在运行 |
| 待批 | 等待授权，优先显示 |
| 完成 | 当前轮次已结束，结果尚未查看 |
| 出错 | 发生错误，但进程仍然存活 |
| 失败 | 进程以非零状态退出 |
| 空闲 | 未运行，或结果已经查看 |

## 智能体

安装 Khor 后，Claude Code 和 Codex 会话会自动出现在列表中。安装 Claude Code 钩子以启用轮次完成状态：

```sh
khor hooks install
```

导入在 Khor 外启动的智能体或 tmux 会话：

```sh
khor import               # 列出可导入的会话
khor import <number>
```

## 文件

```sh
khor send <device> <file>  # 先发送元数据，批准后传输
khor accept <transfer-id> # 在接收设备上运行
khor pull <device> <file> # 获取远程文件
khor ls <device> [path]
```
