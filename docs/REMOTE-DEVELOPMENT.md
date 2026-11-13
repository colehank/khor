# 轻量本机与远端开发

本机保存源码、Git、Python 3.11+、GitHub CLI、SSH和必要的编辑器索引。Node/Bun/Rust、npm依赖、原生SDK、浏览器测试与增量缓存放在开发机。离线可以编辑和提交，网络恢复后再提交远端检查。系统级工具可能被其他项目使用，不由Khor的清理过程卸载。

## 配置本地hook

```sh
python3 scripts/dev.py configure-local --install-command
# 需要回到完整本机开发时：
python3 scripts/dev.py configure-local --restore
```

这会为当前克隆的所有worktree安装轻量hook，原有hook文件保留；现有code-review-graph索引hook继续调用，恢复命令还原原来的core.hooksPath。轻量hook只检查**暂存内容**的空白错误、Python/JSON/TOML/Shell结构，不联网、不安装依赖、不编译。JS/TS、Rust格式、工作流和测试没有在本地通过；hook会明确提示这一点。不要把轻量hook的成功当作完整检查成功。

`--install-command`还会把这两个小型源码脚本保存在Git公共目录，提供`~/.local/bin/khor-dev`命令。它不依赖当前分支是否包含脚本；以后可直接在任意Khor工作树运行`khor-dev check`。重新安装即可更新脚本；恢复hook不会删除这一工具。

GitHub `.pre-commit-config.yaml` 和Quality/发布流程继续完整运行。合并前必须等待对应提交的可信GitHub Quality通过；租机的开发结果是加速反馈，不能替代可信CI，更不能作为发行来源。

## 提交准确的开发源码

```sh
python3 scripts/dev.py check                 # 默认fast：格式、脚本、GUI/Agent类型及单元/静态检查
python3 scripts/dev.py check --task native   # 现有完整Mac原生回归
python3 scripts/dev.py check --task lab      # 预编译Mac原生Lab
python3 scripts/dev.py status RUN_ID
python3 scripts/dev.py result RUN_ID --output /path/to/evidence
```

首次需要`gh auth login`，该账号须能向`colehank/khor-build`写入并手动dispatch。2026-11-13仓库所有者明确将该构建库公开，源码快照、历史、日志与artifact按公开仓库处理。CLI与Lab共同核对请求仓库名和GitHub实际`full_name`，只为这个已授权builder允许公开提交；其他公开库在生成或上传源码前拒绝。仍可用`--builder-repo owner/private-repo`指定同协议的私有控制仓库。个人凭据只留在本机/GitHub托管准备任务；租机不接收个人token、SSH私钥、签名或模型凭据。公共PR不能自动触发这些手动工作流。

快照包含已跟踪文件的当前修改，含已暂存新增文件；**不自动上传未跟踪或忽略文件**。新源码先`git add path`再提交检查。快照不改当前分支、暂存区或stash列表，不要求先做一个WIP提交。它是无父提交的独立源码树，避免每次传输全部历史；基线commit、tree、dirty和snapshot commit写入回执。

提交后立即返回Actions链接和run ID，记录保存在Git公共目录`khor-development/runs`。SSH/iroh短时断线不会打断GH runner；无需让本地终端一直等待。源码准备、便携格式与脚本回归在现有CI使用的GH托管Linux上执行；Mac生成绑定并检查GUI/Agent，重型任务使用同一个槽串行，增量缓存保留。GitHub concurrency只保留一个pending任务；被后续请求替换的pending显示cancelled，不能当成已检查。用run ID核对结果，不盲目重发。

构建工作流使用控制仓库中的可信driver，不执行源码快照内的workflow。源码任务当然会执行开发代码，因此这台租机仍然不能承载敏感凭据。任务结束后退休该请求的临时源码分支；准备失败也会清理，未成功dispatch的请求可根据本机回执，在确认没有active任务后删除对应`dev-snapshots/<request_id>`。源码artifact保留3天，证据7天。

## 本机验收Lab

```sh
python3 scripts/dev.py result RUN_ID --output /path/to/lab-evidence --lab
ditto -x -k /path/to/lab-evidence/lab/KhorLab.zip /path/to/lab-evidence/lab
open /path/to/lab-evidence/lab/KhorLab.app
```

Lab是租机生成、临时adhoc签名且未公证的开发程序；不是可信正式发行包。macOS首次可能要求通过系统“隐私与安全性”允许打开。不要替换`/Applications/Khor.app`或给它导入生产账户。Lab启动器固定使用`~/Library/Caches/KhorLab/<snapshot commit>`独立store，Tauri identifier及网页profile也与生产隔离；可在其中创建专用测试会话。不要直接执行内部`khor-gui-bin`，那会绕过store隔离启动器。

构建成功只说明产物可生成与签名校验通过。中文IME、窗口、拖放、iPhone手势仍需真实设备验收。开发机不生成正式iOS签名包；iPhone继续用可信GH生成的TestFlight。

## 清理与保留

确认远端检查与Lab构建可用后，可删除本机Khor项目的`node_modules`、`dist`、生成绑定、Vite/测试浏览器缓存及无占用的开发target。先核每个worktree的忽略/未跟踪内容与占用；保留独有源码、记录、正式App、会话数据及所有运行镜像和兼容链接。全局Node/Rust/Xcode不是Khor专属产物，不随此步骤删除。

增量缓存只在macserver复用；重型任务继续使用其35GiB准入、20GiB停止和target容量门。可信GitHub发行机器只从源码与锁文件重建，不接受这台租机的可执行文件或编译缓存。

架构参考：[VS Code Remote SSH](https://code.visualstudio.com/docs/remote/ssh)、[GitHub Actions安全边界](https://docs.github.com/en/actions/reference/security/secure-use)、[Tauri独立配置](https://v2.tauri.app/develop/configuration-files/)。


## 便携网络检查的Mac入口

feat/net控制分支提供无需托管prepare的便携网络入口：

```sh
python3 scripts/dev.py check --task net-vendor --executor mac --builder-ref feat/net
```

只接受net-unit、net-node、net-vendor、net-congestion；默认standard与旧命令兼容。netlab仍需要Linux，Mac入口在上传前拒绝它。
沿用已注册development.yml、既有khor-mac-mini/khor-dev runner、mac-mini-worker队列及run-build构建锁/现有空间门，不修改机器的准入或清理策略。
Mac入口先由可信控制步骤校验parentless源码容器，再运行源码；checkout只使用GitHub短期只读job token且persist-credentials=false，materialized Git去掉remote/认证header。
不向租机转发个人token、SSH私钥、签名、模型凭据或生产store。此路径不依赖托管prepare/retire，不新增中继、部署或正式签名。
源码快照、平台、run/control及原始输出仍各自留证；Mac单元/loopback测量不能替代Linux命名空间SLO、可信Quality或真机结论。
Mac路径不自动退休快照分支；确认任务终态及本地证据ref后，只清理本请求实际拥有且SHA匹配的分支。复用快照的请求不能删原请求的分支。

Mac客户端现会另传dev-bundles/<request_id>：只包含无父源码快照的Git bundle，64MiB上限。worker经GitHub API读bundle，并按工作流GITHUB_SHA读取可信控制脚本，核Git blob、SHA-256、snapshot/tree后复用原准备与构建锁；Git只访问本地bundle。短期只读token不继承到Git/准备或构建步骤。旧checkout通道仍可由省略bundle双输入使用。终态证据保留后，bundle分支同样须核本请求所有权和bundle_commit才可清理。接口依据：[Git blobs API](https://docs.github.com/en/rest/git/blobs#get-a-blob)、[固定ref读取内容](https://docs.github.com/en/rest/repos/contents#get-repository-content)。
