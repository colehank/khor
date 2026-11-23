# Windows W1 诊断入口基线

## 命令

```text
khor doctor --json
```

- 可执行文件：`F:\khor-target-build\debug\khor.exe`
- 退出码：0
- `KHOR_HOME`：`F:\Temp\khor-home`
- 输出：14 行 JSON 检查

## 基线结果（Windows 11 26100）

| check | state | 说明 |
| --- | --- | --- |
| version | warn | 本机 PATH 中无 `khor` 命令（直接运行 exe） |
| identity | ok | 身份可读、权限受限 |
| store | ok | store 可打开、可写 |
| serve | fail | serve 未启动（预期） |
| web | warn | 5467 未监听 |
| relays | warn | 一个 relay 可达，另一个不可达 |
| peers | ok | 尚未配对 |
| roster | ok | roster 版本 1，签名新鲜 |
| roads | unmeasured | serve 未运行 |
| agents | ok | 无注册 agent |
| vscode | ok | 无 code |
| disk | unmeasured | 卷剩余空间未测 |
| launcher | warn | 无 opener |
| terminal | ok | 未打开终端 |

原始 JSON 证据保存在：

```text
F:\khor-w1-evidence\doctor.json
F:\khor-w1-evidence\doctor.exit.txt
```

## 说明

`serve fail`、`web warn` 等是未启动状态，不是构建或诊断入口失败。  
W1 只验证诊断入口可运行、JSON 可解析、退出码为 0；不把该输出当作网络/配对/全功能通过证据。
