---
title: 安装
summary: 一条命令装好，装完常驻
order: 1
---

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
| `KHOR_MIRROR` | 换一个镜像地址，默认是 `https://khor.cloud/dl` |

## 从 GitHub 装

```sh
curl -fsSL https://github.com/colehank/khor/releases/latest/download/install.sh | sh
```

这条路可以用 `KHOR_VERSION` 指定版本。

## 升级

再跑一遍安装命令。它换掉二进制，重启常驻进程。
