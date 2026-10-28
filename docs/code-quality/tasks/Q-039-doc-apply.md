# Q039 strict Doc apply：接口与真实 fence 方案（只读设计）

状态：方案交root确认；未生产改/未建分支/未Cargo/未执行新测试。用户当前Khor协议统一开发，不main merge/ship。读取wave1集成 `9543b23349f2a23796e83506604b93d87c978752` sync基座；root inventory的Node/catalog候选并行写权不触碰。overwrite旧树HEAD1c76保持不切分支。

## 依据与既有路径

已读refactor-safely技能，graph minimal仍main8704，不能覆盖集成merge_report；回退actual store/glue/wire/transfer与pending_cache真实测试源码。

| 现有输入 | SHA256 |
| --- | --- |
| crates/sync/src/store.rs | 6c906fe18b80cb24ea45586d7c07b3d81f6eeb8ba063f112274331faece8b943 |
| crates/sync/src/glue.rs | 5f7fb8df853531b900b33f4cae4466d0c896e55ddc5c8fa0ea6877686fdae428 |
| crates/sync/src/wire.rs | 110ae0c702c1d7070432bebabd1d156432fbd5db3f1a93b31ea0ca2aeea19a7d |
| crates/sync/src/transfer.rs | f010cac088a3648e0e33b859560a68aa4258d7155122623a60523e6be56d7567 |
| crates/sync/tests/pending_cache.rs | 097bb693dd5e7afa07a9aacb57a0c5388e17f9715c428aa1d0844b4616cdc585 |

锁Loro1.16.2。13 Doc已有真实merge_report，glue将Loro import.success/import.pending转换成writer半开区间，不能当完整VV或精确本批byte计数。record_import记pending最高end，compact在unresolved时拒绝；load缓存不再丢pending（ed460已集成）。这些不是strict apply已经完成的证据。

现路径：Lease::take先尝试shared .pile（10s有界），失败/无目录允许pile=None，仍能借pen/spare；load读block构造store.epoch；load_pile可能在空pile乐观设epoch=EPOCH/stamp=true而物理.epoch尚未写；flush→make_dir→stamp→block write_atomic→ledger→on_disk推进。read_epoch把missing/IO/非法都当0，不可直接当strict检查器。

## 已确认契约与未采纳替代

root确认接口方向 `Loaded<D>::apply_checked(spec, bytes) -> Result<FlushedImport, ApplyError>`，成功代表当前live真实导入及BlockStore.flush完成，**不是网络Applied ACK或跨平台掉电durable receipt**。

spec至少含expected_epoch、baseVV、targetVV；文档/source/recipient/完整body摘要与canonical binding由上层已认证transfer验证并绑定所选Loaded，不从bytes猜Doc类型，也不让网络提供任意本机路径。

root已明确：**before.includes_vv(base)是StaleBase前置**。base是接收者先前声明已持久具备的版本；现在缺base即重新规划，即使body碰巧携带base也拒绝，live/数据块不动。审查者曾提出“候选后covers base即可”的通用merge替代，root未采纳，本卡不将其写成当前方案；显式full-snapshot repair是另种spec，不在本切片。

base与target可以并发，不要求互含；target不必包含receiver before。candidate与actual允许超出target：都应覆盖before、base、target的逐peer max/union，不能用相等比较把本地并发删掉。由于本spec先要求before含base，union仍显式保留base断言便于诊断。base必须来自已flush声明；建议实现同时查store.on_disk covers base（或至少将这条作为不可绕过的调用前提），待root确认，不允许调用者把未flush VV伪称已持久。

## 建议最小类型/方法

- ApplySpec借用/拥有已decode的VV与expected_epoch；VV解析/负counter校验在边界完成。支持并发VV，不新增target>=base约束。
- FlushedImport由apply_checked唯一构造，不derive Deserialize/不允许外部随意组装；含实际applied VV、epoch、是否写新block及retired_pen（存储路径若保留只供内部诊断，不自动进入wire）。duplicate可返回无新block，但仍必须证实目标已在on_disk覆盖。
- Rejected类：WrongEpoch、StaleBase、SeedFailure、PendingImport、MissingTarget/CoverageFailure、坏body；都在live merge前，不能写Doc数据/pen/epoch/ledger。
- Fence类：缺fence/锁超时/epoch元数据IO或格式错/lock路径替换/初始化冲突；live和Doc数据不改，可能创建owner-only控制目录/.pile，必须与“候选验证无数据写”区分，不谎称全文件系统无任何metadata变化。
- CommitFailed类：真实导入、pen退休、block/ledger写失败；可能已有live/部分disk变化，绝不构造FlushedImport或清stage当成功，保留重试/诊断。不能在API文案声称这些错误已回滚。

优先新方法位于store.rs，复用Doc/glue，不新建一个并行Doc抽象或直接替换D。strict checkpoint/fence helper应是private Result接口；旧宽松open_owner_only/Lease::take读取行为不要在此误变成无限等待。

## 候选预检与提交步骤

1. 独占`&mut Loaded`使用期间，不允许其他alias并发变更同一D；现有Doc业务写入会commit后再version。捕获before VV与纯应用态snapshot；先expected_epoch对store metadata、before covers base等便宜检查，失败不创建目录/锁文件。
2. D::open同peer_id建立仅内存candidate，不拿第二pen、不调用load/load_pile（后者会reseed或写ledger/epoch）。只import应用态snapshot；不能clone包含旧pending队列的Loro状态。seed report须无pending且覆盖before；导入body，候选report.pending必须为空，candidate VV覆盖before/base/target。
3. candidate不替换live。即使原live已有pending，预检也不能以“live存在pending”一律拒绝；strict本批不得引入新pending，既存pending要留在原Doc/BlockStore。将来本批body恰好提供旧pending依赖，actual可以多应用旧ops而超candidate/target。
4. 取得/验证以下真实pile fence，再strict读磁盘epoch，与store/spec重核。还须保证新空pile初始化已合法完成；验证完后持fence至真实merge、pen退休与flush全部返回，不在中途drop。
5. 在原live D上merge_report(body)，record_import真实report，执行checked pen退休，flush。不要将candidate report冒充live report（live可能解锁旧pending）；不要替换D或清原pending map。实际VV必须覆盖before/base/target。
6. flush成功后再核store.on_disk覆盖实际完成VV及target，才构造FlushedImport。flush(None)不等于天然成功：既有代码在delta空时也可提前None，应以on_disk覆盖确认。输入duplicate但live有未flush本地ops仍需flush，不能仅按candidate==before跳过。

### pen退休的确切缺口

当前took_in返回Option，`write_atomic(.pen,...).ok()?`把退休IO错误混成“无需退休”。strict必须有Result版本（可保留普通wrapper），区分NotNeeded、Retired、Failed；不能看None就ACK。当前成功顺序真实merge→record_import→took_in→flush可保留，但退休失败时live已可能变化，后续处理必须是CommitFailed而非Rejected/no-mutation。

实施前需root确认该失败如何安全保留后续可恢复写入：若retire失败后直接丢Loaded，可能放回未更新pen而未flush入场的同writer ops。最小保守方案是失败路径仍尝试flush使实际已应用ops可重读，保留原退休错误并不发receipt；若flush也失败则明确poison/retain/retry而非假称可任意丢弃安全。另一方案在已验证candidate阶段预退休会前移控制写，而且旧live pending激活的同writer ops还须考虑；不能只靠candidate简单预测代替真实took_in。此项只提出安全决策点，未擅自改root确认的成功路径。

## 真实 epoch/pile fence 状态表

严格epoch读取：缺文件是Missing，不是IOError/Invalid的别名。使用Result区分NotFound、权限/读取失败、非法/溢出内容；不能调用当前read_epoch().unwrap_or(0)。只有已确认epoch0的文档才将Missing解释为合法0；stamp=true的current空pile是初始化状态，不是已验证磁盘EPOCH。

| Loaded状态 | 允许动作/重核 | 拒绝与边界 |
| --- | --- | --- |
| lease.pile=Some，disk epoch有效且等store/spec | 复用已持shared；无需再次lock/升级；持有期间reseed拿不到exclusive | 不能为了“更强”在同File盲取exclusive；不能drop现有shared制造空窗 |
| lease.pile=None，现有dir，epoch有效 | Result打开.pile、真正有界try_lock_shared，取得后strict重读epoch；匹配才commit | 超时/打开/锁错误都拒绝，不按历史宽松fallback继续 |
| 无目录或确实新空pile，stamp=true/store EPOCH | candidate全过后owner-only建dir/.pile；在没有已有shared时取exclusive初始化fence；重核确无.loro、物理epoch Missing/一致，再原子写本store EPOCH，清stamp；保留exclusive至flush | 别把别的copy已写的epoch0/blocks当新pile覆盖。发现blocks/epoch冲突则StaleEpoch/NeedsReload，live不替换 |
| 已有shared、stamp=true而磁盘Missing | 建议本切片明确UninitializedEpoch/NeedsReload并立即返回，保留原shared/live；常规已存在空dir load_pile通常先reseed写epoch，不会到此特殊态 | 不可升级已持shared、也不可shared下盲写epoch来盖过其他writer。若root要求无重开自动初始化，需先统一整个empty-pile初始化协议，再另审 |
| 正常chat/load epoch0且Missing | 真shared fence后Missing可按该store合法0处理；不写EPOCH1 | 不把所有Missing都当EPOCH1，也不强制chat进入key-value epoch |
| 取得锁后disk epoch与store/spec不符 | StaleEpoch、保原D/旧pending，不发receipt；由上层重规划 | 不信只在创建Loaded时读过的store.epoch，不自动reseed/换Doc掩盖冲突 |

shared/exclusive初始化取得后应归Lease持续持有，而不是临时unlock后靠store.epoch。无需为stamp=false普通路径争exclusive；锁等待应有调用deadline/PILE_WAIT上界，CPU/大body工作由caller放blocking任务而非GUI reactor。

真实fence依赖协作协议的reseed只在.pile exclusive下改epoch，且.pile不被协作代码unlink替换。失锁/路径替换须fail closed；Unix可用File/path metadata dev+ino校验受锁的是当前.pile，Windows需对应稳定handle身份核验（实现API/CI能力先核，不臆造std字段）。仅锁内存bool不能证明路径仍同inode；恶意外部删除/非协作旧普通writer不是此方法可伪称完全防御的文件系统沙箱。

## 写路径、权限与失败结果

| 阶段 | 实际可写路径 | 权限/顺序与承诺 |
| --- | --- | --- |
| candidate验证 | 无 | candidate纯内存；不读load缓存构造另一个Loaded，不reseed/flush live |
| fence准备 | 必要的新目录、.pile | 新目录0700/新文件0600，沿既有make_dir/open规则，不静默chmod既有共享目录；路径由已选Loaded决定，非wire用户任意路径 |
| 空pile初始化 | .epoch.tmp.pid.seq→.epoch | 有真实exclusive新pile fence、确认空后写；这是commit控制metadata，失败可留temp，不发receipt |
| pen退休 | .pen-k.tmp.*→.pen-k | checked Result，失败不能当“不需退休”；Lease仍持旧inode写名锁是既有策略，新pen内容给下一持有者新writer |
| flush | u-writer-seq.loro.tmp.*→block，然后.merged.tmp.*→ledger | write_atomic f.sync_all+rename，但没有目录fsync/跨多文件事务；ledger失败时block可已存在，on_disk还未推进，重试可冗余块但不能报Applied |

现有write_atomic create owner-only temp、sync_all后rename；不新增全目录扫描删除或垃圾回收。保留broken文件/held ledger语义，不能用candidate替换live清掉无法读取的旧块。FlushedImport只证明声明target/current applied状态按当前flush路径完成，不宣称所有旧pending已持久化、所有broken已修复或断电后目录项绝对durable。

## 可控真实 Loro 与锁测试矩阵（待授权实现/运行）

用真PinDoc为主、DeviceDoc至少一条同peer笔退休；必要时参数化13 Doc的基础snapshot/import合同。全scratch，真实Loro1.16.2，不手造MergeReport/VV来假证明import。测试oracle同时看Doc逻辑值、真实VV、pending可后续激活、目录文件字节清单/epoch/ledger，以及重开后target/local并发内容。权限测试不用chmod在root环境假通过，可用确定的路径类型冲突/独占子进程锁与barrier；故障hook仅在写阶段最小cfg(test)点而非重写apply算法。

| 场景 | 刺激 | 必须观察 |
| --- | --- | --- |
| pending新import | 真实source先create再edit，只送缺依赖edit | Rejected Pending；live值/VV与所有数据文件字节不变，无receipt |
| 谎报target | body合法但target多声明一个真实source未发送op | MissingTarget，无live/数据写；不能仅看report.applied非空 |
| missing base | 当前before缺声明base，body甚至可补齐 | 明确StaleBase前置，body不导入；live/数据不变；另测source base与target并发但before含base合法 |
| 本地/远端并发 | 两个独立writer各有不互含op，before含声明base，body带远端增量 | 成功后包含本地和远端，VV覆盖union，重开也完整；不要求candidate==target |
| duplicate | 两次同body/spec合法，第二次当前before仍含base | 都只有通过flush/on_disk覆盖才receipt，第二次无无意义header块；另测本地有dirty ops仍需flush |
| candidate错误 | 非法bytes/seed导入异常/覆盖不足 | logical/value/VV/pen/epoch/ledger/blocks不变，旧pending可后续提供依赖激活 |
| 保留live旧pending | live先park另一个依赖外edit，strict导入独立合法body，再给旧依赖 | strict成功不能清旧pending，后续旧edit实际出现；candidate只应用态seed，不能靠替Doc变绿 |
| 旧pending被解锁 | strict body恰给旧pending所需依赖 | actualVV可大于candidate/target；真实report记录额外applied，flush重开仍有旧edit |
| pen退休 | 真实构造远端同当前writer计数超过before | 退休发生且下个lease拿不同writer，当前Doc不被换；退休写失败必须报checked CommitFailed，不假NoOp |
| flush block失败 | 候选过后制造可控block写失败 | 无FlushedImport；明确live可能已变，stage可重试，不声称回滚 |
| flush ledger失败 | block已rename后.merged写失败 | 无receipt且重开能读已落block；修障碍重试可flush成功，不误删已写数据 |
| empty new pile | load_pile不存在目录→stamp1，无fence | 候选失败不建目录；合法body后真实exclusive初始化.epoch1，block后重开epoch1/内容完整 |
| shared stamp Missing | 特意构造已有shared且stamp1/未物理epoch | 有界明确拒绝、shared未丢、无deadlock/盲stamp；root若选不同初始化协议则对应真实并发证明 |
| stale epoch | unfencedLoaded之后由自有另进程持exclusive改变epoch，再apply | 取真实fence后StaleEpoch，无旧history混入；不是只测spec!=store的早期分支 |
| fence失败/失锁 | 自有子进程持exclusive/barrier，或.pile路径被换inode | 有界FenceUnavailable/Lost，不能退回None成功；不杀非owned进程 |
| chat epoch0 | 正常load、Missing .epoch、合法body | fenced成功仍epoch0，不写EPOCH1；与key-value初始化分开 |

lock子进程必须有release+join/进程fuse，失败时释放自有文件锁与PID；不跑用户HOME/服务、不SSH/真实安装。flush故障不得靠不可控磁盘耗尽。每项开始/结束source/binary指纹与实际case数记录；尚无上述运行结果。

## 交root的确认点与后续切片

已确认：方法名/FlushedImport非Applied、live不替换、candidate no-pending+coverage、base StaleBase前置、保本地并发与旧live pending。

仍请root明确：1) shared+stamp/Missing特殊态本批fail-closed还是先增加统一初始化协议；2) checked pen退休IO失败后必须如何保留安全owner/flush重试，不可继续用Option吞错；3) base已持久的on_disk附加检查；4) strict文件身份检查Windows实现与最初运行平台边界。生产范围仍须下一授权，不能根据此卡自行建branch/改源码。

下一步建议先真实Doc候选/覆盖/old-pending测试与fence状态测试，再最小store.rs/glue所需helper，最后Node transport用FlushedImport构造应用回执，Pair/table_b64另批。VerifiedComplete→此方法只是下一层证据，远端Applied receipt持久化/重放owner归属仍不是本切片可抢先完成的工作。
