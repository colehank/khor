# 终端附件浏览与目录传输

2026-10-15：沿用已确认的附件 lab，入口仍是软键行的附件按钮。甲板、键行和终端的相对位置不变。

## 交互

- 文件点开只读原文，显式切换预览；目录点开浏览。勾选是独立动作，不会发送。
- 文件和目录可以跨机器选择。选择父目录后，其子项已包含，不再重复选送。
- 「已选择」可核对来源、路径和已知体积。默认接收位置使用目标机器的存储设置；也可选择会话当前目录。
- 选择会话所在机器的路径时，默认直接插入原路径；选择其他来源或指定当前目录时，目标机器先接收。全部成功后把落地路径一次性 shell quoting 后粘贴，不发 Enter。
- 设备文件使用系统文件选择。原生桌面选择目录使用系统目录选择器；iOS 选择目录期间保持 security scope，并通过 NSFileCoordinator 在后台读取为 app 自有暂存副本。关闭面板保留选择；取消传输保留可重试的原生副本；移除选择、完成发送或关闭终端时释放。
- 网页目录选择使用 `webkitdirectory`，该浏览器接口不提供空目录；浏览器拖入目录的 entries API 与原生目录选择均保留空目录。
- 传输进度仍属于药丸，面板也提供取消。失败仍可从药丸或选择面板重试；新发送会清掉被替代的失败提示和暂存副本。取消不粘贴；整批中已经落地的其他项目保留。

## 数据与协议

`term-attachments.ts` 管来源、父子包含和设备文件分组；`TerminalPane` 持有选择生命周期及唯一发送器；`TermFiles` 只负责浏览和操作。`FilePreview` 复用工作区的只读 `FileView`，不另造预览渲染器。

单一来源保留 `term_drop`；多来源或需保留的原生暂存用 `term_drop_items`，每项携带自己的来源。`term_drop_folder` 创建独立目录 token，`term_drop_stage_into` 用相对路径上传子文件，`term_drop_forget` 只删除受管 token，不取消其他传输。完成或取消才清理普通上传源，失败保留重试输入；原生目录副本由选择持有。

目录传输在 node 共用 `tree::Manifest` / `Landing`：递归快照、原有文件切片传输、隐藏目录落地、结束前重读快照、独占重命名发布。`FetchTree` / `SatFetchTree` / `Tree` 是新增协议变体，未修改已有 frame 字段。SSH 卫星通过 sponsor 获取 NUL 分隔快照，沿用原有 SSH 文件读取。旧版节点仍可传单文件，目录两端需要支持新增操作的版本。

每个目录最多 20,000 项、64 层、路径合计 2 MiB、内容 200 MiB；单文件沿用 200 MiB。拒绝路径穿越、重复项、缺失父目录。目录中的符号链接和特殊文件明确报错，不跟随、不静默跳过。保留目录结构和空目录、普通文件内容及 Unix 文件可执行位；目录权限和时间戳不承诺原样复制。源快照变化则不发布。最终目录名占用时加后缀，发布时再次原子拒绝覆盖。

## 验证

- `npm run attachments`：生产选择/暂存函数，跨来源父子包含、Windows 路径、空目录、取消和 token 归属
- `FACE_ONLY=term npm run face`：正式 App、TermKeys、TermFiles、TerminalPane，读取不发送、只读原文、跨机器选择、目标目录、目录暂存、取消重试、失败重试清理、设备文件及已有拖入/粘贴路径
- `FACE_ONLY=bar npm run face`：现有甲板、键行位置及交互回归
- `cargo test -p khor-node --lib tree`、`cargo test -p khor-node --lib a_directory_drop`：相对路径验证、快照、目录、重名和取消清理
- `cargo test -p khor-node --test files_wire`：真实配对连接拉取嵌套目录和空目录；未配对请求拒绝对照
- `cargo test -p khor-gui-core --lib term::`：暂存、取消、原生选择重试与失败批次源保留

真机验收仍需：iOS Files / iCloud 文件夹授权与取消、切后台后重试；桌面原生目录选择；Linux / Windows 实机间目录传输。编译与浏览器测试不能替代这些平台的系统文件选择器验收。

## 本轮结果

`npm test`、`npm run build`、`npm run attachments`、`FACE_ONLY=term` 和 `FACE_ONLY=bar` 均通过；node 的目录单测、真实 wire 测试和 gui-core 的 27 条 term 测试通过，Mac / iOS 编译检查通过。修复前已见单文件复制函数拒绝目录的失败测试，修复后同一目录复制用例通过。

仓库已有两组独立门失败，在 main 上也能复现：`npm run gates` 缺少 cm-measure 入口以及 indent/revert 的文档登记；`npm run bytes` 扫到既有终端录制 fixture 和 vendor 压缩包中的原始控制字符。本轮不修改这些文件。
