# Q039 current inventory 最终候选非作者源审

结论：受审功能源码未见新增阻断，首轮两项边界已按B与per-doc结果方案修复；2项相关注释待顺手收口。可继续root正在进行的固定候选Cargo/真实回归，**本审查本身未Cargo/gen/测试，不将前一帧green当本帧通过**。最终运行及ignored生成物由root另交来源后再核。

## 固定来源与权限

wave1 HEAD `34ef67c32f47d6b6aef44824452634f2dafd9ca9`，dirty仅5文件，root唯一源写权。图优先minimal/detect仍main8704未覆盖，回退actual diff、全部Versions构造/consumer、normal/0RTT codec、Pair/Vouch与fixture调用。5源审前后SHA一致：

| 文件 | SHA256 |
| --- | --- |
| crates/node/src/link.rs | cbbd59b4b89376d21b92ba5424a90c862864293a292eeac7f5a7e395c975065b |
| crates/node/src/proto.rs | f67c8bf54cc5ce308d1b6e40c36b29e305c3c76855ddf67e41a3fa7ce483dea2 |
| crates/catalog/en.toml | 98a9af17113f2444dcefce75e67cebeafa251e18f2716f41f1bef9979f860fd9 |
| crates/catalog/zh.toml | fde6aaf894139bfee096611327d9fc402585d2bb5a99f06e3017c74b8ddfc549 |
| crates/node/tests/table_lost.rs | a37c271734d83400c5f693012a4a4bb1571d1a5a6bc751768ef558a6f4ba3bfd |

strict Doc proposal已交root后暂放，本次未恢复其生产写权；不修改其他审查者卡、不动STATE/main/ship。

## 对首轮问题的复核

### 仅inventory的admission类型

真实dispatch admit失败只在req=Versions返回InventoryNotAdmitted；其余返回Err由dispatch转原Refused，Watch流式独立门不变。sync_device只对该typed响应调用既有限频签名Vouch，成功后重发同opening.request；普通Refused、Vouch失败、第二次仍拒绝都显式Err，不退到删除的sync_everything。

take_vouch的same root signature/freshness/双方admission/newest seq/gone ids校验没有变，vouch_offer与VOUCH_EVERY逻辑保留。当前普通API的why解释与原陌生caller auth回归无需因本切片改成新类型，避免首轮A引入的大调用面变化。table_lost真实自愈仍应由root本帧实际验证；源审不能替代运行。

### 完整per-doc inventory边界

opening返回RoundOpening，原selected names保留；每doc的with_doc!包在独立Result闭包中，已知arm的load ?不再返回整个opening，失败保留为local failures且其余digest继续收集。answer_versions同样闭包，每个错误形成DocumentInventory::Failed，unknown名也显式失败而非旧continue。

Response::Versions docs类型明确为(name,DocumentInventory::Version/Failed)，其构造/consumer在link/proto当前Rust调用处均已适配；没有留下inventory回读旧String tuple的生产消费者。client合并remote结果与本地failures，按original names索引稳定排序，再逐项sync_documents，保证首错误遵循selected顺序，后续doc继续。synced doc成功有bytes才触发devices follow/broadcast take_arrived；失败不触发。所有失败按doc写journal并保first_failure，epoch Apart不merge且给显式partial描述，不能读成already in sync。

有数据移动仍刷新readings，即使别的doc失败；刷新失败成为错误而非假称完整成功。Pair专用rounds“两轮”唯一剩余caller仍是Pair bootstrap的devices，不是拒绝inventory后的旧路径，未误删现行初次配对。

## 新错误本地化复核

native_helpers部分审指出的缺口在本帧已补：proto::answer解码后对Refused/InventoryNotAdmitted why，以及Versions内每个Failed why调用同一个khor_catalog::reword；未知OS诊断原样保留，Version字串/readings不改。

真实normal exchange约11265、0RTT早期答复约3342都用proto::answer，因此不是仅某一路被翻译。嵌套readings仍沿原proto::decode/reading处理，不把它误算为此次新doc错误路径。

新增当前codec unit实际构造英文与中文not_paired两种remote文本，通过encode→answer；核typed admission与doc Failed都本地化、合法Version不改、未知诊断不改。已逐字核两literal与当前catalog一致。此unit尚待root Cargo；本源审没有执行它。新6 current_inventory pure unit与当前roundtrip均检查typed形状，但不以它们替代真实Node循环/自愈测试。

## table_lost fixture调整

diff只有4处Node::open_as改为open_hosting(...,false)，原pair/Trusted/table lost/round Ok业务断言未改。实际lib.rs明确hosts=false将vendor/tmux/procs Discovery设为空，并在自己device row记录hosts=false；这不是完全没有capability影响的“任意同义替换”，但本test不测host会话创建，Pair/Vouch/Versions/Sync不经过refuse_unless_hosting，认证/名单行为不被替身绕开。用于避免真实vendor discovery进入fixture适当，不冒称验证桌面host所有功能。

## 已实际核对的原始红与本帧边界

只读核 `Q039/inventory-run-01/cargo.log`：local known accounts目录被owned普通文件替换，真实load错误，outcome Err且Seen=0/Pin=false；remote同种真实load错误后旧inventory Refused触发旧full-round fallback，optional accounts错误被旧策略吞掉，outcome Ok(2160 bytes)且Seen=123/Pin=true。两普通parent实际失败，原literal日志保留。这解释了初轮宏纠正：remote的成功来自后续legacy吞错，并非answer_versions把已知load ?静默continue。

当前sync_doc_isolation新fixture同时assert真实load错误原因、Seen123/Pins true、最终round Err、损坏/原始backup数据都未删；这些oracle对应本次修复，未用额外无关transport失败充当目标红。新帧source红绿结果由root正在跑，审查未提前声明绿。

## 注释待收口（非运行阻断，已报root）

1. proto.rs约961–967 Response::Versions.docs仍说unknown doc name“不命名”，现契约是Failed显式命名；应写清same-version省略、changed为Version、unavailable/unknown为Failed。
2. proto.rs约132 Watched仍引用“head of this file measures tail-append”；header政策已更换，这处因果失效。可仅留当前watched目录与volume由目标设备报告的语义。

不要求在当前Cargo运行中改冻结源。其余全proto历史叙述/旧fixture继续按后续清理，不在此批扩为全协议重构。

## 交回

source功能审通过，等待root提供本帧正常lib/current_inventory/proto/真实Node4/table_lost的实际结果与source stable证明，再作运行结论。catalog新增sync-partly-apart en/zh双placeholder{0,1}已在首轮TOML解析核过且此帧SHA未变；新Rust msg/GUI生成输出未由本审查执行，需按root来源另核，不用旧ignored生成物冒充新key覆盖。

本轮未改变五源/未运行Cargo/gen/未占native槽。root可回收只读审查槽；main不合并/发布。
