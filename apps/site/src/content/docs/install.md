---
title: 安装
summary: 安装桌面 App，或使用命令行客户端
order: 1
---

<h2 id="desktop">桌面 App</h2>

从 [下载页](/download/) 选择适合这台 Mac 的版本：Apple silicon 或 Intel。「关于本机」可以查看芯片信息。

1. 解压下载的文件，将 **Khor.app** 放入「应用程序」，然后打开。
2. 在 **机器** 页确认本机已经出现。一台设备就可以开始使用，之后再加入其他设备。
3. 切到 **会话**，点列表旁的 **＋**。新标签页中选择 **目标机器**、**目录** 和可用的 **启动方式**，再点 **创建**。

### 连接另一台设备

另一台设备也需要运行 Khor：有桌面的 Mac 可以安装 App，没有桌面的 Linux / Mac 主机可以使用下方的命令行安装。

在 **机器** 页点 **＋**，选择 **Khor 主机**：

- 在已有设备选择 **邀请机器**，复制生成的凭据。
- 在新设备的同一入口选择 **加入网络**，填写凭据并加入。
- 回到机器列表，确认对方已经出现，再在新会话中选择它。若界面显示待批，先按提示在持根设备上批准。

已有会话可以从列表中继续打开。连接、凭据与远端启动方式以实际设备状态为准；网页上的示例界面使用本地演示数据。

更多命令行配对方式见 [入网](/docs/pair/)。

<h2 id="cli">命令行客户端</h2>

在终端中使用 Khor，或为没有桌面的主机安装常驻进程：

```sh
curl -fsSL https://khor.cloud/install.sh | sh
```

不需要 root，不需要工具链。支持 Linux（x86_64、aarch64）和 macOS（Apple 芯片、Intel）。

## 它做了什么

- 把 `khor` 放进 `~/.local/bin`。这个目录不在 `PATH` 里的话，往 `~/.profile` 加一行
- 生成这台机器的身份
- 启动常驻进程 `khor serve`，并登记开机自启：macOS 用 LaunchAgent，Linux 用 systemd，没有 systemd 的机器在登录时拉起

装完新开一个终端，检查一遍：

```sh
khor version
khor doctor
```

`khor doctor` 一行一项：名字、状态、一句话。「未量」不是「正常」，是没量到。

## 选项

环境变量写在 `sh` 前面：

```sh
curl -fsSL https://khor.cloud/install.sh | KHOR_NO_SERVE=1 sh
```

| 变量 | 作用 |
|---|---|
| `KHOR_NO_SERVE` | 只装，不启动常驻进程 |
| `KHOR_MIRROR` | 指定安装镜像；未设置时从 GitHub Releases 下载 |

## 从 GitHub 装

```sh
curl -fsSL https://github.com/colehank/khor/releases/latest/download/install.sh | sh
```

这条路可以用 `KHOR_VERSION` 指定版本。

## 升级

再跑一遍安装命令。它换掉二进制，重启常驻进程。
