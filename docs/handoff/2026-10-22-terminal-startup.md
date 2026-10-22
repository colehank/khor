# 自动启动命令等待终端就绪

分支：`fix/terminal-startup-ready`。独立 worktree：
`/Volumes/disk0/khor-terminal-startup-ready`，基于 `a99b70dd`。

## 问题与修复

新建会话在宿主公布地址之前，就把启动命令写入 shell。Codex 0.163.0
此时可能还没有终端能应答 OSC 10/11，250 ms 后将未知底色缓存下来，输入框
与历史用户消息便没有背景。终端后来接入时，初始回放也不会重新回答旧查询。

宿主协议 5 追加 `Ready`，不修改 Hello、Welcome 或跨机器 ShellStart 的结构。
GUI 在 xterm 初始回放完成后确认，CLI 在原始输入和转发线程建立后确认。
宿主持有一次性的启动输入，早到按键排在其后，重复确认与重连不重复执行。
远端新连接的初始回放完成后再次确认，避免首次确认随断线丢失。

旧客户端在接入时释放输入；旧宿主不会收到未知操作。没有启动命令的 shell
与普通 `open -d -- command` 保持原有运行方式。shell 初始化时已有查询的宿主
应答不排在启动命令后；尚未就绪的按键缓冲上限为 1 MiB。

## 验证

先红：

- `cargo test -p khor-node --test open_from_phone`：新建后没有终端就已经执行命令。
- `npm run terminal-startup` 使用原始 bridge：同样在无终端时检测到启动执行。

修复后通过：

- `cargo test -p khor-node --lib shell_start::`：5 项，包括早到按键、重复确认、空命令、缓冲边界和 shell 应答。
- `cargo test -p khor-node --test open_from_phone`：跨节点新建、重试、接入等待、单次执行、保留 shell 环境和协议 4 客户端。
- `cargo test -p khor-cli --test attach_version --test open`：8 项，覆盖新旧协议与普通持久命令生命周期。
- `npm test`、前端生产构建与 `npm run gates`。
- `cargo build -p khor-cli -p khor-gui-core -p khor-catalog --bins`。
- `cargo check -p khor-gui`：原生 Tauri 命令注册和桌面代码编译通过。
- `STARTUP_CODEX=/Users/zgh/.local/bin/codex npm run terminal-startup`：真实 PTY
  的 250 ms 探测成功；只建 socket 不释放启动；实际 Codex 0.163.0 的输入框背景
  在延迟接入后仍存在，桌面与窄屏重新绘制均已检查。私有身份，无提示词提交。

构建目录：`/Volumes/disk0/khor-terminal-startup-target`，通过
`CARGO_TARGET_DIR` 指定。深色截图与探测结果在：
`/var/folders/ml/w_5l2vkn5ll_yhqyc9bhkrdr0000gn/T/khor-terminal-startup-v4fx7a`。
浅色验证在同目录前缀的 `xUwBNB` 运行；窄屏截图以最新深色运行为准。

界面验证使用正式 React 组件、真实后端与 PTY 的浏览器入口；未启动新的原生
App 窗口进行界面走查。没有合并 main、替换当前安装或重启用户会话。
