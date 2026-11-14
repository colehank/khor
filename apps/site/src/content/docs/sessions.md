---
title: Sessions
summary: Start sessions, reconnect, and follow their progress
order: 3
---

Shells, agents, conversations, and file transfers share one session list:

```sh
khor sessions
```

## Start a session

```sh
khor open                 # Persistent local shell
khor open -- claude       # Run a command
khor open --on <device>    # Start on another device
khor run -- <command>     # Temporary session tied to this terminal
```

Persistent sessions keep running after you close the terminal. Reconnect or close them:

```sh
khor attach <session>     # Ctrl-\ to detach
khor close <session>
```

## Session states

The same states apply across your network and every Khor interface.

| State | Meaning |
|---|---|
| Busy | Running |
| Blocked | Waiting for your permission; shown first |
| Done | This turn has finished and you have not viewed it yet |
| Errored | An error occurred, but the process is still alive |
| Failed | The process exited with a nonzero status |
| Idle | Not running, or the result has already been viewed |

## Agents

Claude Code and Codex sessions appear automatically after you install Khor. Install the Claude Code hooks to enable the completed-turn state:

```sh
khor hooks install
```

Import agent or tmux sessions started outside Khor:

```sh
khor import               # List available sessions
khor import <number>
```

## Files

```sh
khor send <device> <file>  # Send metadata first; transfer after approval
khor accept <transfer-id> # Run on the receiving device
khor pull <device> <file> # Fetch a remote file
khor ls <device> [path]
```
