# Q041e candidate01 root 独立源码审

审查者root，非三源实现作者；2026-10-27。graph先minimal，图3e46旧/未覆盖新generator，依据实际完整三源与锁定ts-rs10.1.0实际递归export源码，不把0flow/testgap当覆盖。当前尚未编译/运行候选9父。

静态已核：71根单一宏同时preflight/export；TypeVisitor递归规则与ts-rs匹配（不可导出wrapper由derive自己的visit_generics依赖暴露）；flat path先于export校验；默认runtime Cargo路径与显式参数、管理归属、whole-stage、OS try_lock、完整journal先sync再移动live、exact old/new recovery、完整剩余白名单后cleanup、partialcleanup可恢复；tests有actual71rootsfresh/reused而不是抄根名单，有真实OS子进程Busy和SIGKILL窗口，producer无env故障开关。

## B1：类型文件数量预算可在已发布后才被发现

managed.rs entries允许MAX_FILES+1总项是为了容纳inventory；但inspect构造Inventory后未valid_inventory就返回unmanaged/adopt tree，generate_with也没有先限制planned.len。于是4097个生成TS的旧默认目录可被接管，或4097 planned/staged TS可完成stage与journal；layout.journal只在最终recover才valid_inventory，可能在已移动/发布live后发现old/new.files超4096，导致生成失败、已更新live与保留无效journal，后续gen永久报unknown inventory。它不是当前154类型业务已观察崩溃，但属于确定的预算/失败边界错误。

最小修复：inspect构造出的Inventory立刻valid_inventory再认可/接管；planned数量在export/stage之前同一MAX_FILES拒绝，不能等publish后才发现。增加真实generate行为边界：超预算planned在export回调前拒绝并保原live、无journal；旧默认无marker超预算生成目录也须在任何live移动前拒绝（可用有限owned scratch测试，不改真实目录），不要只直接测valid_inventory函数形状。原候选01与基线真red证据保留，改后candidate02独审delta再跑定向组。

## 边界与后续

静态symlink拒绝，不把检查+open夸成防同uid恶意竞态保证；普通两rename不是连续reader原子快照/整机断电事务。默认现目录一次接管只允许generated flat files；显式非空无marker拒绝是有意接口收敛。temp根允许std::env::temp_dir的OS别名，其它symlink路径当前明确拒绝；如标准脚本实际传另一OS alias须定向验证，不据macOS成功称Windows runtime通过。完整CLI默认/显式运行与同runtime crate两上下文、标准producer/types均待真实运行。

## candidate02 delta闭合（未执行）

实际三源与候选副本逐字一致，external/internal全13file SHA一致。生产相对01仅inspect构造值立刻valid_inventory和generate_with在Layout/export前planned.len限额，gen主文件与71根名单未改。两个新增generate行为父分别保护超planned的原live/journal/siblings、4097旧默认无marker文件不export/不移动/无stage或backup；有明确正向生成测试而非一律拒绝假绿。B1源码修订已闭合。当前11普通父+1由父调用的ignored worker尚未编译/运行，不称green。源码审查本片可进入定向验证；唯一槽先R1，作者无编译权。

## 作者执行与集成后验证通过

Root独立核bindings-green-01原始11父log、7phase exit0/guardnone、当前1922输入及两fresh=false实际binary完整SHA/size/mtime、190双盘file、A旧154+3接管/Bfresh/explicit/standard全inventory一致，154TS+1marker。真实Busy/kill父代码已先源审，不将内部worker另计。root收据bindings-root-green01-audit。作者c89aa135三源=受测候选，root仅cherry为wave1 **670347d7**。

集成后重新normal build当前wave1 generator64.098s，再标准npmgen6.729s与types5.164s全部exit0，1923tracked输入前后当前一致；三组均ESRCH。真实默认旧157TS由producer接管为154TS+1marker，恰清原PlaceRow/QuotaWindowKind/Wheel，和作者standard inventory逐SHA相同；旧全部输出原件保bindings-before，不靠手删通过。完整Q041/bindings-integration-670347d7已双盘。原runner只清空TMP时留下94个npm Node compile-cache小文件，原result的tmp_absent=false保留；后以exclusiveTMP/全组退/lsof0/regular uid nlink inode SHA完整清单精确删除，post-cleanup-result证明scratch absent，未动其它缓存或用户App。

该生成器子任务实现/独审/集成/适用真实producer/types已闭环。全Q041的fresh onset、远程/Linux/musl/Rust嵌入与最终App仍另验，不据本片关闭。
