# Q041-b 不可变镜像与站点发布独立审查

状态：固定源审查与scratch脚本回归完成；已证目录目标、静态权限和FIFO阻断均已修复，无本范围剩余已知阻断。真实SSH/rsync/Caddy部署及集成验收未做，不能据此关闭整个Q041。

## 范围与所有权

- 审查者native_helpers，生产作者root。树`/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-store-owner-recovery`，分支`fix/quality-install-integrity`。起审a1之前HEAD c9f2962b，最终运行启动时HEAD **59da27746e56693e7c3d95df9bae73edb03c1171**，root随后将相同b源保存为 **a723bbba67ac8ed3d8b0fbd12bbc69089d5f7871**。不将各修复阶段或提交时点误写为同一个不变HEAD。
- 范围：mirror-sync/mirror-lib、site push/Caddy注释/public bootstrap、两份新测试及两份文档。a1生产另见[Q-041-a1-review.md](Q-041-a1-review.md)。没有写STATE或其他生产文件。
- 采用review-changes，graph-first minimal context/detect/affected flows/impact/tests_for后核实际diff。图谱初次built c371，最后仍存在built59da/HEADa723不匹配，shell覆盖不足；0 affected flows不代替现场。实际读取生产脚本、调用链、测试adapter、文档和固定源SHA。
- 父agent只临时交付两份测试的窄新增写权；每次新增后明确交回，所有生产修复由root实施。当前两份测试写权均已交回。无Cargo、真实SSH/rsync、installer main、npm真实build、真实HOME/store或服务变更。

## 固定源与调用路径

最终20项运行的source-before/after逐项一致；运行后现场读取仍匹配：

| 文件 | SHA256 |
| --- | --- |
| scripts/mirror-sync.sh | b44274ee32de608ac0d3dffa897b15e86ebc224b18a0cb9aab312966b3a1eebe |
| scripts/mirror-lib.sh | e1232726e373fb464393d33626ae6fb7f167497b0a995019b13b019aede97584 |
| scripts/tests/test_release_mirror.py | 671c9bdf812576377b1a141787b81f60b55f6e0f3d1682f99745307d59498285 |
| apps/site/deploy/push.sh | f5f2cd2249fd279be14f64d7c8fc63383c1e9c1b67f43d47469fa7d1a25441bd |
| scripts/tests/test_site_publication.py | f968106e4ca74d069617a192d0f993bd60a936406b76ce35716a9e73d8e7f04d |
| apps/site/deploy/Caddyfile | d069b7c904896cc527eab5103c01e0b712cb5cfdb09660031c88d15a6b369f72 |
| apps/site/public/install.sh | 589d4088d241990628ef8ea7cd7853fc847f3222f535a1a12937e657ae487e99 |
| apps/site/README.md | c7d68a4d093a2ef4bd9956c5ecf69ef3361e1025cf25eede4d7c3faad375497f |
| scripts/release-mirror.md | 0c0c908ab43b0147ae61dbe7de7fc80d15d6b972945066bb7b4383a3fb9a4c54 |

支持源也纳入运行指纹：install.sh d303a0c2、canonical a28542ca、onset-lib 2d3c2946（完整指纹见`Q041/b-final-review/source-after.json`）。

源审确认：

- `mirror-sync`加载可信canonical共享resolver，原子mkdir锁后只拥有自己的mktemp stage；版本只选择一次，固定四平台归档/SHA和同tag installer，检查迁移marker与语法。未混入main installer、旧flat覆盖或任意asset名。
- `validate → commit`先验证完整代的regular/non-symlink文件、严格压缩字节SHA、完整gzip/魔数及installer语法。已有tag逐项比较九个文件，拒绝内容差异；最终`-f`阻止FIFO进入cmp。新代完整落位后设置公开模式，再发布bootstrap，最后原子rename VERSION。目标install.sh/VERSION非regular或symlink、releases symlink明确拒绝。
- site先准备完整本地代，页面rsync排除整个`/dl/`，保留旧reader。远端唯一incoming receipt严格验证，再只上传选中release；发送本地可信canonical+mirror-lib bundle执行相同校验/发布，未执行下载的installer。锁和清理均只针对本次stage，路径space/quote保持literal。
- push清理在本次接审阶段由root从吞掉SSH失败改为明确stderr告警，之后重新冻结；其前后不冒称同源。copy失败仍保留原非零状态，cleanup失败不覆盖首因。
- Caddy只改缓存契约注释，未改路由/header逻辑；保留不可变旧代支持缓存中的旧VERSION reader。文档明确不自动GC/不更新旧flat，锁须确认writer退出后人工处理。
- public bootstrap最后一行改动把可预测`khor-install.$$`换成`mktemp ...XXXXXXXX`，trap仍精确清自己的路径，下载成功后才sh执行；只读核对，无完整installer运行证明。

## 先红后绿与覆盖有效性

证据均在`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q041`。

1. root原镜像6项实际旧源结果`b-mirror-red/tests.log`为4 failures + 1 error；其余不是目标失败。后续扩大到10项绿。site第一次`b-site-red.log`的fake rsync未递归模拟`--delete`，曾误绿，**不算故障红或安全证明**。修adapter后`b-site-red-02.log`真实脚本删除旧pinned-reader，2 tests中1 error为实际FileNotFoundError，才是有效旧reader红。
2. cleanup未确认：授权新增`test_failed_copy_retains_failure_when_remote_cleanup_is_unconfirmed`，只让adapter对`rm `命令exit255。实际push保留copy exit23、明确warning、旧VERSION/reader不变，无新代，确实残留一个owned incoming。`b-site-cleanup-owner`1/1绿；没有把无法确认写成“零残留”。
3. 目录目标P2：实际probe中install.sh为目录，旧writer exit0、VERSION推进v1.2.3，却把`.installer.*`搬进目录，bootstrap仍不可用（`b-review-probes/directory-bootstrap.json`）。授权永久目录/指针四子例与公开模式断言，在`b-target-modes-red`得到**2 tests/5 failures**，源稳定：root/gen0700、bootstrap/VERSION0600而预期0755/0644。root补regular/symlink目标守卫、releases symlink拒绝，以及公开files0644/dirs0755，后续完整组覆盖修复。
4. 已有代FIFO P2：真实旧源**9ae4fc7c68f1ed48bf1582ecb094099d96c92e08cd710b8c5a645dc2bf55055d**的installer FIFO使cmp等待；自有PGID98928的3秒guard观察到shell+cmp后只终止该组，旧VERSION保留，source稳定（`b-review-probes/existing-generation-fifo.json`）。root在cmp前加`-f`，未回退源重取红。永久`test_existing_generation_special_files_are_rejected_without_orphans`覆盖installer和archive FIFO，要求及时明确immutable拒绝、旧pointer/reader/FIFO保留，进程组已不存在；超时只清己组并判失败，不能当成功拒绝。`b-generation-fifo-green`1/1通过。

## 最终独立运行与交还

`Q041/b-final-review`实际执行：

```
PYTHONPATH=scripts/tests PYTHONDONTWRITEBYTECODE=1 python3 -m unittest test_release_mirror test_site_publication -v
```

专用scratch TMPDIR，实际生产脚本由private curl/ssh/rsync/npm适配器提供固定数据/本地远端副本；没有联网部署。**13 mirror + 7 site = 20/20，exit0，unittest13.873秒，总14.107秒，source_stable=true**。runner PID5959、子PGID5970，自有90秒墙钟/5GiB护栏；最低external9,259,925,504B/internal8,941,821,952B。结束后`killpg(5970,0)`确认组不存在；证据包含run/process/result和source-before/after，不再重复全套或追加新探针。

结论限于固定脚本和scratch覆盖：当前无剩余已知阻断，可交root分提交/集成。权限仅以真实stat.S_IMODE核0755/0644，未切独立webserver用户；未真实SSH/rsync/Caddy运行、完整installer、断电耐久、release签名或自动GC。传输/copy或校验在pointer前失败已验证旧pointer保留；**最终远端发布已发生而SSH ACK丢失时，客户端仍可能无法判断提交状态**。cleanup拒绝可能残留stage，真实回收/ACK协议归Q045，不将所有SSH失败概括为事务回滚。

作者已交还两份测试写权；当前无本任务活动PGID，不占Cargo/native槽。没有更改main或发布。
