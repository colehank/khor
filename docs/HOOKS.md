# 接入:进程怎么把状态词交给 khor

session 状态不同步、home 现拿(NET.md);这页讲 home 上的进程怎么交词。
两条纪律:

- **智能体状态靠钩子,不靠抓屏。** 字符串匹配只留给非智能体程序兜底。
- **进程只许报活词**(忙碌/待批/完成/中断/空闲)。**失败由退出码推**,
  谁也不许自称失败——一个还能说话的进程说自己失败,必是谎话。

## 门:`khor state`

`khor run` 给孩子设 `KHOR_SESSION`;孩子或它的钩子跑 `khor state <词>`
就写进那个 session。没有这个环境变量就 `--session <号>`。

## Claude Code

settings 的每个事件都指同一条命令,词由 khor 从载荷里判
(映射收在 `khor state --hook` 的实现里,这页不抄第二遍):

```json
{
  "hooks": {
    "SessionStart":     [{"hooks": [{"type": "command", "command": "khor state --hook"}]}],
    "UserPromptSubmit": [{"hooks": [{"type": "command", "command": "khor state --hook"}]}],
    "Notification":     [{"hooks": [{"type": "command", "command": "khor state --hook"}]}],
    "Stop":             [{"hooks": [{"type": "command", "command": "khor state --hook"}]}],
    "SessionEnd":       [{"hooks": [{"type": "command", "command": "khor state --hook"}]}]
  }
}
```

一条判进映射时定过的界:**Notification 只有要许可的那种才是待批**。
「等你说下一句」永不进待批——那个角标归不了零,等于没有角标(SESSION.md)。

两种跑法:

- `khor run --tui -- claude`:一个 session、可靠 pid、真退出码。
- 直接在自己终端里跑:钩子自注册 `tui/<claude 的 session id>`,没有 pid
  ——被硬杀的 claude 会停在最后一个词上(词的时间戳还在老去,看得出来),
  close 清掉;观察 session 的 pid 探测挂账。
