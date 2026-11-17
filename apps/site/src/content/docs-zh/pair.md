---
title: 配对
summary: 与一台设备配对，加入整个网络
order: 2
---

第一台设备会建立自己的网络。每台新增设备只需配对一次，即可加入。

## 添加设备

在任意已入网的设备上创建邀请：

```sh
khor invite
```

邀请有效期为 15 分钟，只能使用一次。在新设备上运行：

```sh
khor pair <ticket>
```

配对后，设备信息会在网络中同步：

```sh
khor devices
```

## 根密钥与恢复词

第一台设备持有网络的根密钥，用于签发设备身份证书。请记下 24 个恢复词，并单独妥善保存：

```sh
khor root words <local-device-name>
```

若丢失持有根密钥的设备，可在另一台设备上恢复：

```sh
khor root recover <24-recovery-words>
```

`khor root show` 可查看当前设备信任的根，以及是否持有根密钥。

## 等待审批

新设备收到根密钥持有者签发的证书后才会入网。若邀请设备不持有根密钥，请在持有者上批准：

```sh
khor pending
khor approve <device>
```

## 仅支持 SSH 的主机

无法运行 Khor 的主机也可接入，由本地设备代为操作。需要先配置免密 SSH：

```sh
khor adopt <ssh-host> [name]
```

## 移除设备或离网

| 命令 | 作用 |
|---|---|
| `khor forget <device>` | 从网络移除设备；再次加入需重新配对 |
| `khor revoke <device>` | 由根密钥持有者撤销设备；其他设备收到更新列表后会拒绝它 |
| `khor leave <local-device-name>` | 离开网络，保留本地会话、工作区、身份与设置 |
| `khor pause`、`khor resume` | 暂停或恢复网络访问 |

移除设备无法抹除已经存储在它上面的凭据。设备丢失后，请在相关服务中轮换这些凭据。
