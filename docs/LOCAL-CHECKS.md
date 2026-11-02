# 本地提交与推送检查

每个克隆执行一次：

```sh
uv tool install pre-commit==4.5.1
pre-commit install --hook-type pre-commit --hook-type pre-push --allow-missing-config
```

使用默认迁移模式，不加`--overwrite`；已有code-review-graph hook保留并继续运行。Git worktree共享hooks；`--allow-missing-config`允许旧的纯规划分支正常提交。配置纳入Git，新克隆仍需上述安装步骤。

| 阶段 | 按改动范围执行 |
| --- | --- |
| pre-commit | Python/JSON/TOML/JS/Shell语法、现有Rust格式检查、相关GUI/agent类型检查、actionlint |
| pre-push | 干净且与待推送提交一致的工作树、脚本测试及安装器同步、GUI单元/类型/设计规则/mobile static、agent测试 |
| GitHub | 独立重跑质量检查；真实浏览器交互、原生进程/SSH/Windows、签名、公证和发行构建 |

提交前检查由pre-commit暂存未提交的部分修改，只验证暂存内容并恢复用户编辑；检查不自动修文件。推送前需先提交或自行妥善保存未完成修改，避免拿不同的工作树冒充待推送版本。

使用现有Node22.19、Python3.11+、Rust1.95/rustfmt1.9、actionlint1.7.12与ShellCheck0.11.0。只改某部分时，仅要求该部分依赖。GUI/agent先`npm ci --prefix apps/gui`或`apps/agent`；GUI生成类型沿正常开发准备流程执行`npm run gen --prefix apps/gui`。hooks不会自动安装依赖、生成代码或编译原生App，也不自动下载浏览器。actionlint和ShellCheck放PATH；已有独立二进制也可用`git config khor.actionlint /绝对路径/actionlint`、`git config khor.shellcheck /绝对路径/shellcheck`指定。ShellCheck是必需依赖，不能因本机缺失而减少检查范围。

本地推送指向旧提交的发布tag时，应在该候选提交的干净工作树执行；避免检查当前main却推送另一个版本。通过GitHub创建tag不经过本地hook，仍受GitHub发布检查约束。

Windows运行可移植语法/格式/类型及release/API测试；依赖POSIX shell与进程的脚本测试在macOS/Linux运行。真实Windows进程检查继续由Windows runner承担。GitHub不会信任“本地已经通过”来省掉自己的检查。

手动复核与CI使用相同入口：

```sh
pre-commit run --all-files --hook-stage pre-commit
pre-commit run --all-files --hook-stage pre-push
```

首次推送或修改hook配置会运行更多检查；平时按改动文件分流。不要通过`--no-verify`或`SKIP`把失败当作通过，缺依赖应先补齐，失败应保留并修复。

机制参考：[pre-commit官方文档](https://pre-commit.com/#install)。
