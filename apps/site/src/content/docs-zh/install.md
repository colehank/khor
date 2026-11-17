---
title: 安装
summary: 获取桌面应用，或从终端使用 Khor
order: 1
---

<h2 id="desktop-zh">桌面应用</h2>

在[下载页面](/download/)选择 Apple silicon 或 Intel 版本。不确定芯片型号时，可查看 **关于本机**。

1. 解压下载文件，将 **Khor.app** 移到 **应用程序**，然后打开。
2. 打开 **机器**，确认当前设备已出现在列表中。一台设备就能开始使用。
3. 打开 **会话**，点击列表旁的 **＋**。在新标签页选择目标机器、目录和可用的启动方式，再点击 **创建**。

### 连接其他设备

每台设备都需要安装 Khor。有桌面环境的 Mac 可以安装应用；无桌面的 Linux 或 Mac 主机可以使用下面的 CLI。

在 **机器** 中点击 **＋**，选择 **Khor 主机**：

- 在已有设备上选择 **邀请设备**，复制邀请。
- 在新设备的同一菜单中选择 **加入网络**，输入邀请。
- 确认两台设备都出现在机器列表中。创建会话时，即可选择另一台设备。若请求正在等待审批，请在持有根密钥的设备上批准。

打开列表中的已有会话即可继续工作。连接、凭据和启动方式取决于实际设备；官网预览使用本地演示数据。

命令行配对流程见[配对](/docs/pair/)。

<h2 id="cli-zh">命令行客户端</h2>

从终端使用 Khor，或在无桌面的主机上安装后台服务：

```sh
curl -fsSL https://khor.cloud/install.sh | sh
```

无需 root 权限或编译工具链。支持 Linux（x86_64、aarch64）及 macOS（Apple silicon、Intel）。

## 安装器会做什么

- 将 `khor` 安装到 `~/.local/bin`；若该目录不在 `PATH` 中，则补充到 `~/.profile`。
- 为当前设备创建身份。
- 启动 `khor serve` 并注册自动启动：macOS 使用 LaunchAgent，Linux 使用 systemd；没有 systemd 时使用登录钩子。

打开新的终端检查安装：

```sh
khor version
khor doctor
```

`khor doctor` 会逐项显示检查名称、状态和说明。未测量不代表正常，只表示当前没有读数。

## 安装选项

将环境变量放在 `sh` 前面：

```sh
curl -fsSL https://khor.cloud/install.sh | KHOR_NO_SERVE=1 sh
```

| 变量 | 作用 |
|---|---|
| `KHOR_NO_SERVE` | 只安装，不启动后台服务 |
| `KHOR_MIRROR` | 设置下载镜像；默认使用 GitHub Releases |

## 从 GitHub 安装

```sh
curl -fsSL https://github.com/colehank/khor/releases/latest/download/install.sh | sh
```

可配合 `KHOR_VERSION` 选择指定版本。

## 更新

再次运行安装命令即可替换可执行文件，并重启后台服务。
