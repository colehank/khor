# Q039 current inventory候选：部分独立源审交还

状态：partial-review-handed-back。按root最新指令停止扩审，四个生产源freeze已解除/交回。**不是完整独立审查完成，不是编译或运行证明**；root后续补丁由另一非作者复核。

审查者native_helpers，只写本卡；生产作者root。现场wave1 HEAD **34ef67c32f47d6b6aef44824452634f2dafd9ca9**，dirty四源。接审帧SHA：link **cbbd59b4b89376d21b92ba5424a90c862864293a292eeac7f5a7e395c975065b**，proto **aa6e422b671662ad1a84e37e1bb7db0ede9a4a7c6f28838045af3f709d473a9e**，en **98a9af17113f2444dcefce75e67cebeafa251e18f2716f41f1bef9979f860fd9**，zh **fde6aaf894139bfee096611327d9fc402585d2bb5a99f06e3017c74b8ddfc549**；inventory test仍effbb。初始指纹与link diff在`Q039/inventory-root-review`，解除冻结后的新源不属于该帧。

采用graph-first minimal/detect/flows/impact/tests_for；索引仍built fe265/HEAD34ef不匹配、0flow/缺符号，不当作无风险证明。随后读取实际diff、相关调用方与宏。未Cargo、未gen、未改生产或测试；没有本任务PID或槽占用。

## 已确认需修：新增错误字段漏本地化

`proto::answer`只对`Response::Refused.why`调用`khor_catalog::reword`。新增`InventoryNotAdmitted.why`和`Response::Versions.docs`内`DocumentInventory::Failed.why`走原样返回，跨语言peer的认证/文档失败不再按本端语言重述；这属于当前功能，不是旧端兼容要求。root已接收，准备补相应codec/helper分支和实际unit。该后续实现**本卡未审、未运行**。

## 已读部分与目前边界

- opening/answer_versions的独立闭包使已知doc宏arm内load `?`在单doc边界返回，并形成local failures/远端Failed，后续doc仍能继续；unknown name的宏最终Err也进入Failed，未见旧elsecontinue吞名路径残留。
- client将local failures并入远端结果，按原selected names排序后sync_documents逐项继续、保留首错；sync_document的宏`?`只退出该函数。epoch Apart显式partial描述，devices/broadcast业务副作用仍以成功且bytes>0为条件。此为所读源结论，仍待真实4父/协议门验证。
- 仅dispatch真实admit失败且请求为Versions时产生typed InventoryNotAdmitted；其他API仍为一般Refused。client只对typed结果尝试一次既有Vouch，失败/一般Refused不落回sync_everything。既有签名、root/revocation校验未见在所读段被放宽。
- 已核current类型/Versions主要callsite与Pair专用rounds唯一caller仍保留；没有据此宣称所有协议/Pair/readings路径已全面审完。未知op、旧TopRow形状等历史fixture未在本轮全面清理，不宣称全compat已清空。

root将修本地化后安排normal check/current_inventory/proto/真实Node4/table_lost，并另派非作者复核。本卡只交还已有部分发现，不继续扩大到full-transfer/Pair其它缺口。main未改/未ship；STATE由root维护。

## 最终限定证据核对（2026-10-26，交还root）

本节是追加的独立证据审，未将前述部分源审改称全仓/全协议完整审。只读核对`Q039/current-inventory-green-01`的final/results、实际各log、原`inventory-run-01`两红、candidate-02源快照和真实catalog producer；没有Cargo/gen、没有修改生产/测试，没有占用槽或新PID。graph-first minimal查询仍遇built fe265/HEAD34ef旧索引，不据此推断现场安全。核对结束现场HEAD为**1d2640d2**。

### 实际运行与数量

`current-inventory-green-01/final.json`为complete=true/source_stable=true，12个step均exit0、stopped=null；1060项source-before/source-after逐项相等。实际日志确认normal `cargo check --locked -j2 -p khor-node --lib`完成、当前inventory 6、proto 20、真实Node父4、table_lost 1通过；Q045相关shared14/acp12/host25/pi29/join20/satellite35，共135。因此不同Rust父用例总数为**166**，没有把Node四个dedicated worker或join十一worker再次计入。日志中的忽略worker/平台用例未算通过。实际编译log有本树catalog/detect/core/a2a/sync/acp/net/node等第一方编译，非仅0测试或旧顶层binary摘要。

脚本实际`python3 -m unittest discover -s scripts/tests -p test_*.py`exit0，log内层组件回归为`Ran 31 tests ... OK`，外层为`Ran 52 tests ... OK (skipped=1)`：外层51执行通过、1跳过，不能称52项全部实际执行。final记录双盘minimum_free分别8,357,208,064/5,834,117,120 bytes，均高于5GiB；本审只核既有记录，未另运行或借用新的运行结果。

### 同fixture真实红→绿

原prepared、原红run-01、新green及当前树的`sync_doc_isolation.rs`均为**effbb7aa0a86dd2e2b82b6ceab888c804774bfe1926a89818ce65a5ffa022574**，未弱化原oversize/epoch及新增inventory断言。实际对照：

- local原红为具体accounts `Not a directory (os error 20)` Err，但Seen=0/Pin=false；现绿仍为同类完整owned accounts路径错误Err，Seen=123/Pin=true。
- remote原红为Ok(传输了2160字节)、Seen=123/Pin=true，违反必须保留错误；现绿为完整remote accounts路径错误Err，Seen=123/Pin=true。并未把原始remote情况误写为单纯未推进。
- 同一完整green运行保留oversize合法25,448,186-byte差集、24851/4096KiB明确Err，Seen=123/Pin=true；epoch控制为Ok且明确“部分同步/数据版本不同、未合并”，Seen=123/Pin=true。四ordinary父确实各调一次worker并通过，不是ignored入口计绿。

### 新typed payload与本地化

实际`current-inventory.log`包含并通过`inventory_failure_crosses_the_wire_with_its_document_and_reason`：读取源确认它将accounts/Failed/原why真正encode→decode→versions_answer，并逐字段assert；不是仅构造enum。另typed admission、一般Refused保留、首错、apart描述等五项均有真实ok。

此前确认的本地化缺口现已实质修正：当前`proto::answer`同时重述Refused/InventoryNotAdmitted，并遍历Versions内Failed；Version与未知诊断不变。源中`inventory_errors_use_local_words_and_keep_unknown_diagnostics`对中英文catalog句分别走真实answer(encode(...))，断言typed admission及Failed重述、Version和未知OS文字保留；该测试在实际proto20日志中ok。此处只接收该局部后续修复及已运行测试，不倒填初始aa6帧为已审新实现。

### 源帧与生成物

已运行帧link为**cbbd59b4b89376d21b92ba5424a90c862864293a292eeac7f5a7e395c975065b**、proto为**f67c8bf54cc5ce308d1b6e40c36b29e305c3c76855ddf67e41a3fa7ce483dea2**；en98a9、zhfde6、fixture effbb与当前逐字相同。当前proto为**e0716489b4e9eebb7c88984415be44a4884ba9aa9c1e59b024fc615959225fa3**。为避免由不同SHA推断内容，使用candidate-02原源中Watched和Versions两段旧注释，**仅在内存中**还原当前对应两段，得到SHA精确等于已运行f67；故green后到current确实只有这两注释，无逻辑/测试字节变化。初审aa6→f67包含本地化修复和新增局部测试，不能混称aa6→current也只有注释。

`current-inventory-catalog-gen/result.json`实际命令为`cargo run --locked --offline -j2 -q -p khor-catalog --bin gen_ts`，exit0/stopped=null；promotion记录成功后提升。独立读取producer `catalog.ts`与ignored `apps/gui/src/gen/catalog.ts`原始bytes，**355,959 bytes逐字相同**，SHA均为**a91e9c415983999703867dc7bacc9960e0be1a755f8e0a0eed1a705d4ae914d4**。没有用手写生成片段代替producer。

本限定证据审未发现新的阻断，可以据上述精确帧进入root收口。root另排的doc-only最终normalcheck/proto组/types属于另一运行帧，本节未冒领其结果；不重复无关135。完整bounded transfer/Pair大表/strict Applied仍非本批交付，不由inventory绿宣称全部解决。审查卡写权和所有freeze现交回root；生产/测试全程未写。

### 交还前补核最终源码运行

root通知doc-only最终组完成后，追加读取`current-inventory-final-01`的final/results及各log，确认4step全部exit0/stopped=null、complete/source_stable=true；1063项before/after完全一致，明确包含ignored生成catalog a91e。当前proto e071已实际完成normal lib check、current_inventory6+proto20，以及`npm --prefix apps/gui run test:types`，不再仅依赖逆向注释字节证明；此26是166中同名用例重验，不新增计数。重建方法在`current-inventory-doc-only-proof/receipt.json`诚实标为reconstruction、不是当时snapshot，与本审独立内存还原得出的f67一致。

本卡与写权最终交回root，无待保持freeze；没有新阻断。只读证据审止于本批，不展开其它治理事项。
