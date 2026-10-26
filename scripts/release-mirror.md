# 版本化安装镜像

`scripts/mirror-sync.sh` 在可访问 GitHub 的机器上运行。默认选择一次 latest；
也可用 `KHOR_VERSION=vX.Y.Z` 固定版本。`KHOR_REPO` 是来源仓库，
`KHOR_MIRROR_DIR` 是本地镜像目录。完整安装器来自同一个 release，不再混入 main。

目录结构：

```
install.sh                     当前 bootstrap，仍是自包含单文件
VERSION                        一行完整 tag
releases/vX.Y.Z/install.sh
releases/vX.Y.Z/khor-<platform>.gz
releases/vX.Y.Z/khor-<platform>.gz.sha256
```

每代包含 Linux x86_64/aarch64、macOS arm64/x86_64 四种归档及其严格单条 SHA256 记录。
客户端先选择 tag，然后归档和摘要都从同一版本目录获取。SHA256 验证精确压缩字节，
不能当作发布签名或独立认证来源。

writer 用 `.sync.lock` 拒绝并发写入。下载、摘要与 gzip/程序头校验全部完成后才将目录落位，
最后通过同目录临时文件 rename 更新 VERSION。相同 tag 已存在时仅接受逐文件相同的内容；
不同内容明确失败，不覆盖已有代。无自动历史回收，不承诺断电持久性。

第一次迁移会保留既有平铺资产，但它们不会随新协议继续更新。新客户端绝不回退到平铺资产；
有 VERSION 却无匹配版本目录时会报错。新镜像需要包含版本化布局标记的发行版安装器，
遇到旧安装器会明确拒绝发布。正式发布前须先准备包含此契约的 release；本轮代码验证没有发布资产。

异常退出可能留下 `.sync.lock`。先确认原 writer 已退出，再处理它自己的锁和暂存目录；
脚本不根据 PID 或年龄猜测锁已经失效，也不删除其他尝试的目录。

站点 `apps/site/deploy/push.sh` 将页面与镜像分阶段传送。页面 rsync 保护 `/dl`；镜像先到独占
`.incoming.*` 目录，远端重新校验后才切换 VERSION，既有版本保留。传送失败不会推进版本指针。
远端或 SSH 无法确认清理时可能留下未发布暂存目录，不能因此删除整个 `/dl`。

共享安装函数唯一源码是 `scripts/install-components.sh`。修改后运行
`python3 scripts/sync-install-components.py --write` 更新自包含安装器的嵌入段；
不带参数只检查。onset 直接加载本地源码，安装组件测试同时检查嵌入段逐字一致。

回归入口：`python3 -m unittest discover -s scripts/tests -p 'test_*.py'`。
镜像/站点测试运行真实脚本，网络、SSH、rsync 与构建由私有 scratch adapter 接管；
这些结果不代表真实远端部署或完整 installer main 已验收。
