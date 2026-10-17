# 新建会话：正式实现与验收 Lab

工作树：`/Volumes/disk0/khor-session-launch-lab`，分支 `session-launch-lab`。

预览：http://127.0.0.1:1520/lab/session-launch.html ，窄屏可用 `?view=phone`。

启动：在 apps/gui 下运行 `./node_modules/.bin/vite --config lab/session-launch.vite.ts --host 127.0.0.1 --port 1520 --strictPort`。缓存独立，复用现有 node_modules，不安装第二份依赖。

## 最终交互

- 三个常驻输入框：目标机器、目录、启动方式。点击显示全部候选，输入后筛选，固定框内滚动，不截掉后面的项目。
- 机器框和候选均显示机器头像。启动方式与目标机器独立组合；不能托管终端的设备仍只提供留言。
- 目录展示真实默认值，有入口目录时保留入口目录，否则查询目标机器家目录；换机器保留各自输入。清空后不强行填回。共享 PathField 支持 Unix、Windows drive/UNC 与目标机家目录补全。
- 启动方式直接输入代码，也可以选择 Shell、Claude/Codex 终端、GUI 智能体、留言及自定义预设。多行代码保持原样；普通 Enter 换行，Ctrl/Cmd Enter 创建，Alt+方向键进入候选，组字时不提交。
- 候选末尾是「添加为启动方式…」「管理启动方式…」。添加带入当前代码，只需起名；已有命令可保存别名，重名明确报错。管理可添加、编辑、删除预设，并保留接入其他 ACP 智能体的入口。
- 空启动代码是普通 shell；启动代码只在新建时作为交互输入送入一次，定义、环境变量和 cd 留在同一个 shell。重试创建和重新连接都不会重复发送。没有改写 Claude/Codex 的执行路径。
- 启动预设存于操作者 store 的 `.khor/launch-presets.json`，独立于目标机器；本轮没有增加跨操作者设备同步。损坏的文件报错，不用空表覆盖。
- 新协议词追加在枚举尾部。旧节点拒绝带启动代码的请求，不退回一个会悄悄丢弃代码的旧创建操作；使用此功能的目标节点需要更新。

## Lab 边界

Lab 挂载正式 NewSessionPage、PathField、ChoiceField、FormGrid、Dialog、MachineAvatar。宽屏挂载正式 TabStrip；窄屏挂载正式 SessionScreen / 底部甲板及标签列表，不画顶部标签条。

所有 API 由 Lab 内的封闭假数据回答，不读取真实目录、不联系节点、不执行命令。包含 15 台机器、Windows 目标和超过一屏的目录候选。Lab 预设只存在 `khor.lab.launch-presets.v1`，与正式预设文件隔离。

## 已验证

- 正式表单实际操作：输入多行代码、Alt+方向键添加、保存、匹配内置命令后另存别名、重复名称报错及原稿保留、管理删除、机器搜索选择、Windows 家目录及切换机器后代码保留、宽窄布局。
- 独立菜单 Lab：`/lab/picker-scroll.html`，`?cut` 模拟服务端不完整目录。改前路径只有 8/36 项，改后 36/36；滚动、方向键到末项，固定玻璃边框不随内容滚动，服务端截断明确提示。操作菜单、右键菜单、会话搜索、Console 候选共享 MenuViewport。
- 正式表单候选原先参与布局、撑高表单；现用 Radix anchor/portal 浮动、按可用高度滚动，不移动其后的控件。
- TypeScript（包含 Lab）、前端常规 npm test、Vite 生产构建、macOS Tauri cargo check。
- Node/core 单元测试；真实跨机器 open_from_phone 集成测试：手机 resident 经网络在另一节点创建 shell，同一 token 重试得到同一 ID，启动副作用仅一次，断开重连两次不重复，后续 PTY 输入读到启动设置的环境变量。
- check-launch-paths 纳入 npm test：Unix、Windows drive/UNC、根目录保留、家目录拼接。
- 旧 check-face 的目录默认值、机器选择、启动方式选择和 handoff 断言已适配；本轮未重跑完整浏览器 check-face。

未宣称验收：iPhone 真实键盘/触摸、原生补全浮窗，以及 Windows/Linux 实机启动。macOS 上的窄屏 Lab 和路径语法测试不能替代这些平台验收。
