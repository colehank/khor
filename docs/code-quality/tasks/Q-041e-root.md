# Q041e：完整且失败安全的 bindings 生成

状态：candidate02源码独审B1已闭，作者既定定向执行全通过，待root非作者收据审；实现owner `node_test_review`（root已正式移交写权）。全仓治理用户授权范围，未合main/ship。

实现树复用 /Volumes/disk0/Khor/worktrees/current/khor-quality-native-io，已从clean库交付8ffe6aa2切到fix/quality-generated-bindings@686f2e42；交接时clean；当前仅下列三份授权Rust源变化，不新建worktree/target。旧库作者写权已归还root。

契约以Q-041e-plan.md为输入：保71根与TS形状、runtime Cargo路径；单一导出函数，生成到owned sibling完整stage；管理清单确定目录归属，显式任意非空无归属目录拒绝；旧默认generated仅严格核generated头/regular flat TS后接管；短OS锁+精确backup/最小publish恢复记录，失败保live或精确旧backup，不按目录前缀或mtime清理。catalog不在本片。

允许源码：apps/gui/core/src/bin/gen_bindings.rs，其私有managed-output helper和定向测试/脚本；不改依赖、锁、公共package/CI、root类型、UI或产品流程。

原producer已由root在owned scratch获得下述真实行为红；本轮已完成最小实现源码，定向行为验证待独立源审后排槽。不能以字面grep或复制导出算法代替。

本任务当前未获Cargo/gen槽，不编译/运行生成器；只完成授权实现与候选冻结。生成门交接后再用同一quality-target/jobs1/incremental0/lockedoffline，双盘至少5GiB。

## 原producer真实红（root执行，不重跑/不冒为作者自跑）

`Q041/bindings-red-01` 的实际旧generator build成功（115.717s）；同一binary fresh/reused均正常exit0，fresh154 types而reused157，恰PlaceRow/QuotaWindowKind/Wheel遗留，真实inventory相等断言失败。source1920稳定，原红证据实际21文件；本实现未改该原红、未动wave1原158 gen输出或旧3类型。当前不以root旧红替代新实现绿。

## Candidate01实现边界（未编译/未执行）

恰授权三Rust源：`apps/gui/core/src/bin/gen_bindings.rs`、私有 `gen_bindings/managed.rs` 与 `managed_tests.rs`。依赖/Cargo.lock/package/CI/公开root类型/UI未改。原generator SHA777ddadb…；独立静态提取证明原71根顺序/全限定类型与新单一roots宏逐项相同；path preflight和export都走此宏，不复制名单。ts-rs依赖visitor在创建stage/导出前拒绝absolute、parent escape、嵌套/非TS路径；当前平铺TS形状仍由原export_all_to生成。

- 标准no-arg Cargo路径只取本次runtime CARGO_MANIFEST_DIR，不再退回编译时checkout；direct binary可给一个明确output arg，缺env且无arg报错。输出祖先和target/static control files拒绝symlink/非目录/非regular；系统临时根允许先canonicalize一次（macOS /var别名），其下仍逐层拒绝链接，绝不canonicalize目标bindings来追随链接。
- 仅bindings专属目录。显式existing非空无inventory拒绝；空/新目录可建立管理。默认canonical路径既有输出仅在全部regular flat `.ts`且ts-rs generated header完整时一次接管。已有inventory必须与实际全目录name/blake3逐项相同；默认路径也不能靠generated header吞未列条目。catalog在外，完全不触碰。
- 私有sibling stage完整export/检查全planned inventory并同步后才建立发布record；export/校验失败清理仅本次freshstage的完整planned-file whitelist，unknown/symlink先报错保全部。记录schema/producer/target及精确stage/backup nonce与old/new inventory；不按prefix/mtime扫GC，不借oldfile header清任意用户目录。
- std OS try_lock串行同输出，独立于执行器/Node锁。完整record同步后才old→backup、stage→live；第二步失败可恢复原完整old；发现foreign destination/坏record则保live/backup/record报明确路径，不为rollback删除陌生目录。下次同锁下先核record恢复或完成已发布cleanup。两个目录rename间存在短路径空窗，不能称portable非空目录atomic swap/断电保证。
- cleanup先验证整个剩余stage+backup whitelist，才unlink。中途unlink IO失败保持完整新live、记录与精确剩余备份以供下次继续；不能谎称旧backup仍完整或已清完。发布后cleanup错误明确返回“published; cleanup incomplete”，不回滚新live或删未知内容。invalid/partial journal fail closed且旧live/backup保留，需确切人工检查；不猜nonce重建。
- 预算只约束generator目录/小record：4096类型、单TS16MiB、inventory/journal2MiB；不是产品协议预算。没有新crate/通用事务服务/长驻daemon/产品故障env开关。

私有test hooks仅test binary读取测试env，生产generate永远传noop hook。9普通父+1由父真实spawn的ignored worker源码已写：

1. 真正同71根export fresh/reused name+bytes（含managed inventory）一致；模拟旧3条目时同步更新原inventory以代表合法前一代，否则应拒绝未知。
2. stage首文件写后Err，完整旧live与catalog sentinel不变。
3. 显式unmanaged拒绝/default严格接管、unknown file及managed unlisted generated TS拒绝。
4. unknown partial stage/backup在任何清理前保留，旧TS/metadata/sentinel不删。
5. publication callback Err后完整旧目录恢复，后续可正常更新。
6. target/child/control/祖先静态symlink拒绝，外部sentinel不变。
7. 根内dependency export_to逃逸在导出前由同visitor拒绝。
8. 第二真实process在OS锁持有时得到Busy，原live不动；释放后正常生产helper成功。
9. 真实process到old已移backup的barrier后SIGKILL，下一生产helper先恢复旧live，再故意export Err验证旧bytes，后续正常更新。

截至候选冻结只执行rustfmt解析/检查、git diff-check、root名单静态等价证明；未Cargo/typecheck/gen/test/standard producer/types，任何上列case均不能算绿。一次新增断言的rustfmt --check先报告格式差异，已重新rustfmt并用Python check=True顺序执行两项检查通过；没有把shell最后命令0掩盖该中间格式结果。root将做非作者源审，再另排唯一Cargo/gen槽；本作者不审自己实现、不提交。

证据冻结至external `Q041/bindings-candidate-01` 与 internal recovery `Q041/bindings-candidate-01`：完整3源、baseline generator、完整tracked+new patch、当前全部tracked/new selected输入hash、candidate/71根proof。精确SHA与文件数以冻结candidate.json为准。当前无本任务运行PID/端口/scratch/生成产物。

正常producer验证计划须显式区分：oldred的“fresh后塞旧3而不更新清单”在新实现上应安全拒绝；新fresh/reused相等正例必须代表合法旧managed generation。默认旧工作树接管可在owned scratch对应runtime crate布局先验，不能为测试删除wave1原3。标准npm gen还会触catalog生成，仍须root授同源生成/构建槽。

## Candidate01冻结回执

external `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q041/bindings-candidate-01` 与 internal `/Users/zgh/Library/Caches/khor-code-quality/recovery/Q041/bindings-candidate-01` 共9文件逐SHA相同；1922 tracked/new输入。证据卡快照为本回执写入前版本。完整源SHA：generator `595b9a3e08dd99287e726c8eabd65a5e45e92c01908c5320e479e06fdf40f761`（6011B）、managed `137df2ed33485b3207e239cd277ac41d2fb4bf970d1a7b47a8ed804a4f400768`（19227B）、tests `2084080e21b1ac8398a3d84edb4706f399397cef61d3a285ef1d8f03f1913cda`（14405B）。candidate.json SHA `8d92c766860e8c3dca74275c150d873b5c0eca1598e1d5ccd7cab43451c32c1f`。冻结后free外6390894592B、内14748188672B，均≥5GiB。源码保持冻结，root接独审；未Cargo/gen/测试/提交。

## Candidate02：root独审B1预算前置

root非作者完整独审见Q-041e-review.md；candidate01及原red证据完整保留。只改managed与其私有tests，generator和71根宏保持candidate01字节不变。inspect构造实际Inventory后立刻valid_inventory，任何旧默认无marker的4097 TS也不能获得接管资格；generate_with在Layout/stage/export之前按同一MAX_FILES拒绝超planned，避免发布后才发现无效journal。无其它协议/输出归属/发布算法变化。

新增2普通行为父，总11普通+1ignored真正process worker；未执行。其一真实generate以4097 planned调用，export panic guard确保不执行，原live bytes、siblings、无journal及既存journal sentinel均保持；其二owned scratch真实4097 generated TS旧默认目录，无marker，generate拒绝前不执行export，旧全部bytes/无marker/无journal保持，未产生stage/backup。不是单测validator内部返回值。静态rustfmt --check三源及diff-check通过，未Cargo/gen/test/typecheck；R1持下一编译槽。root将delta复审，本作者不能独审自己修订。

Candidate02源码：generator SHA `595b9a3e08dd99287e726c8eabd65a5e45e92c01908c5320e479e06fdf40f761`，managed SHA `e7586caec527f96f6dfb6f982751636fe0eca497d49028edf634409772949b53`，tests SHA `6878b4a022b3658f544bbc0a736c6ced5a4c1c0e72677de5da88a237b337c522`。将另冻external/internal recovery `Q041/bindings-candidate-02`，含相对01精确delta和基线完整patch。

Candidate02冻结已完成：双盘各13文件逐SHA一致、1922输入；candidate.json SHA `f27e69bf327893e9bdf296b753bd169518fea3b7c9399fe0bde31d641c14fef7`。最终free外6310547456B/内14736818176B。证据卡快照不含本回执，源码保持上述三SHA，待root独审delta。

## 待授槽执行准备

root已完成candidate02 delta独审，B1闭合；不代表11父已运行。执行准备external/internal recovery `Q041/bindings-execution-plan-01` 各4文件逐SHA一致，只有证据目录写入，worktree初始仍1922源frame。run.py SHA `7235d697df61002ce8e777a0d3d930473c91d4f888881f344c31178050f1fbbe`，data SHA `59c633b785c4b3869f6be9efac28ef10ec969ffdc39c7937210e82146952ddcc`。AST/语法与具名STEP300/TOTAL720/MIN5GiB、own PGID/guard/有限drain已静态核；控制参考旧producer红与Q033 host03真实runner，新runner未启动，未Cargo/gen/types/native。

计划先cargo test --bin gen_bindings的11普通+1ignored实际worker，再cargo build正常generator并核current manifest/fresh=false/actualbinary；两个scratch runtime Cargo上下文默认A旧154+3接管/Bfresh、显式arg优先、最后native-io标准npm gen/test:types。执行前保native默认154实际无marker输出原件+inventory/catalog及只读wave1旧3原件/SHA，不预删输出，不动wave1。标准gen输出单记outputs，不混入1922输入。错误按compile/setup/business/normal-generation/types/resource分开并停后续。实际启动仍须root再次授唯一槽；准备脚本不自发执行，无本任务PID。

## Gen02真实执行完成（作者记录，独审归root）

root正式再次授唯一Cargo/native/gen槽后执行，先实际读最新STATE/PLAN/root Q-041e-review与准备计划。外层launcher独立AST核runner SHA `7235d697df61002ce8e777a0d3d930473c91d4f888881f344c31178050f1fbbe`、STEP300/TOTAL720/MIN5368709120；candidate02双盘13文件和1922完整input逐项匹配，HEAD686f/branch正确且无cargo/rustc/gen_bindings后才启动。launchavailable外6148276224B/内13273280512B。runner内部再次核AST/source；没有改冻结三源或修改测试oracle。

实际顺序7阶段全部exit0/guardnull/无forced group drain：

1. cargo test --bin gen_bindings：74.508s（真实编译后11普通父全部通过，测试自身5.82s，1ignored仅由父实际spawn、不多计）。包含Busy独立process和old→backup窗口真实SIGKILL+wait(signal9)、actual71根managedobsolete→fresh全bytes一致、两预算前置、失败/未知/symlink保护。没有单独打印两个child PID，不虚构观察名单；两个普通父的成功必须满足真实worker marker/退出或SIGKILL断言，且其所在phase PGID整体ESRCH。
2. current normal gen_bindings build：4.615s；actual artifact fresh=false、manifest指向native-io，而非共享target另一树旧binary。
3. 同一normal binary runtime-A默认旧154+wave1旧3的严格generated无marker目录：1.296s；真正生成后去旧3。
4. runtime-B另一CARGO_MANIFEST_DIR上下文默认fresh：1.027s；与A所有names/SHA相等。
5. runtime-B下显式output arg优先：1.022s；与A/B全hash相等，B默认不变。三scratch输出期间真实native默认154原bytes完全不变。
6. native-io标准npm run gen：2.551s，正常catalog producer+same binary正常bindings默认路径接管；结果与上述fresh相等。
7. npm run test:types：4.917s，实际tsc --noEmit通过。未npm install、未全npm tests、未native/browser。

实际最终输出为**154 TS+1 managed marker=155文件**；不是155types。源码输入仍1922 before=after=current，标准gen新输出单记output inventory。预先保存了真实native默认154全原件/inventory、catalog原件及wave1旧3只读原件/SHA；没有预删默认目录。wave1 PlaceRow/QuotaWindowKind/Wheel原位bytes核未变，未清集成旧输出。

实际test binary `debug/deps/gen_bindings-c3867d218d0ba0b7` SHA `2c3c501f6f950a99fda879e3bb15ecd66c29643c58531689cd088c41f8e2e847`/13253872B；normal `debug/gen_bindings` SHA `070dbbac698e1275d62f118871dfaa0f5fabcfd1db8230875c59d43194d0bda2`/12052128B。两者fresh=false/current manifest，全部phase后及最终SHA稳定。

总91.485455834s，min外6000975872B/内13267976192B，均≥5GiB。实际7phase PGID58496/59558/59589/59598/59606/59620/59743结束后再次killpg0全ESRCH，无cargo/rustc/gen_bindings；owned process清单空。cleanup完整校验562个本次scratch regular/uid/nlink文件后精确unlink，容器最后移除，gbe-g1不存在。没有扩大清理到target或用户目录。

external `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q041/bindings-green-01` 与internal recovery/Q041同名 **190文件逐SHA一致**：含旧154原件与三旧TS、catalog、完整三候选源+patch、source前后、raw7阶段、actual artifacts/binary、输出inventory、AST/launcher、cleanup/final/final-audit。最终审后available外6019604480B/内13265858560B。本次unique槽已明确归还root；未commit/未改生产源，源码仍candidate02三SHA。作者不自审，等待root独立核收据与集成排程；不称Windows/runtime/App/P5或全仓goals完成。

## 作者提交与写权归还

root非作者真实190文件收据审通过，回执Q041/bindings-root-green01-audit；另明确只授权三冻结Rust源commit。已提交 `c89aa1351febe15002c11ffda269d2db61ad5f1e`，parent686f2e42，tree `f8cadfb82ea5cc8e04cd5ddde0fd30b2ca57fb1e`。diff-tree恰三授权源，candidate02=tested=commit blob=current三SHA全等，branch现场clean。没有generated/证据/catalog/依赖/其它路径入commit。正常仓库pre-commit仅执行CRG update/detect，未Cargo/gen/test；其图报告不当覆盖或新增受测父。commit双盘回执Q041/bindings-commit-c89aa135。

本卡及native-io源码写权现在明确交还root，可提交中央卡/cherry到wave1并做integration producer/types。本作者不自行集成，不再修改已交源/卡，不称全Q041或全仓完成；保持可复用角色。
