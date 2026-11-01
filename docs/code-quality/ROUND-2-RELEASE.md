# 第二轮正式交付记录

2026-11-01。用户明确授权“先直接commit到main并ship”，替代旧发布前真人验收限制。产品源a228，已合并推送的正式构建源为 `main@eca881f2b30c2893b7e73f81ae1e0f5cefbe8c6c`；后继提交仅补发布文档。

| 目标 | 实际结果 |
| --- | --- |
| main | 产品与12任务卡／交付记录已合并并推送 |
| Mac GUI＋CLI | release构建、完整UI／Pi、strict签名核验和正式切换均通过；GUI91072运行；原8宿主身份及8会话ID不变 |
| hinton／turing／lecun | 一个Linux构建、一次共享NFS安装；三实际listener exe SHA、installed SHA与完整UI全部采用新版 |
| iOS正式归档 | 通过；真实arm64 IOS、签名／team／profile／新UI一致；旧60可回退，当前archive不变 |
| TestFlight | **0.1.14（61）已上传成功**；用户处理协议后沿原归档upload-2，xcodebuild0／119.853s，两成功标记与Apple真实build均核；Apple处理可用性及手机安装未查询 |

共同完整UI：`khor-ui-assets:/assets/index-B55HQiJV.js`。Mac GUI SHA `7af048369a2fbe2833d870e835550168735b26274e52e4c83665be3cbc3884e6`；Mac CLI `6e162eaae715c9b468a9c2479eac934963998a07aefb3743a7fa1fc181cc29a3`；Linux CLI `9117de4af9fc07b0bff2c660c12fe42c27bc6085da67f9cbee3640cea68a6c7a`。所有原source/gen/dist在发布文档修改前最终核等。

证据唯一目录 `/Volumes/disk0/Khor/evidence/releases/20261101-round2`。非作者最终审查`final-release-review.json` SHA `4a4f8a0705f319192e255cbf5dad6be86b06559f7c9413f7f243086f270f7aa5`。核心入口：`final-products-before-docs.json`、`mac-qualified.json`／`mac-cutover-result.json`、`ios/result.json`／`product-verification.json`／`upload-1/result.json`、`cluster/stage-validation.json`／`adoption-summary.json`／`session-comparison.json`。

原失败均保留：Mac／Linux Cargo均0，原wrapper用了错误的多字节mmap成员检查；补验以明确find全字节核验，不重编、不改旧false。原onset1来自serve.sh空日志末AND-list返回1，三节点实际采用另证，不冒onset0。lecun的sessions读取发布前后均busy，不能算会话验收通过。main types、既有unit、Vite、Pi43测试通过；额外gates索引有35孤立脚本＋chat-recovery未登记，记NEXT，不声称所有门绿或unit覆盖35项。

Apple接续见本次外部`ios-upload-handoff.md`。用户本人／团队Account Holder应先审阅处理Apple更新协议；Root未代接受条款、变更账号／keychain或自动重试。当前只读有效本地签名development1／distribution0，协议处理后再判断签名修复。归档已保存，后继纯文档提交不会要求重编产品；续传仍须核实际产品输入等同，成功后才记录Apple分配build。

原发布后未验仍保留：Mac系统中文IME与真实iPhone交互，见[真人清单](ROUND-2-HUMAN-ACCEPTANCE.md)。本地构建／签名资格和正式发布不能替代真人操作；**不宣布34项整轮完成**。


## Apple协议处理后的实际接续

用户回复“通过了”后，新upload-2实际exec31607／Xcode10898，exit0、无forced、group_remaining空；Upload succeeded与EXPORT SUCCEEDED俱全。ContentDelivery原日志数字字段确认版本0.1.14／build61，上传收据`ios/upload-2/result.json` SHA `3083addf77e2a6f26678325dfff6a6a86c76d46e48da6e398e97ae5754e4bf91`，版本收据 `92bf22f9aa1612385cccdb42d0600d4cfd5b14d78ba2481fb3f25b1b44c648e8`。当次文档HEAD2a与built eca仅4docs，实际非文档源码／gen188／dist278、App全树／.a与导出选项首尾同，复用归档、未重编或再次部署Mac／三节点。

上文upload-1及Apple接续是原时点失败，保留而不回填；现在本次SHIP已完成，**不再次上传**。未查询Apple处理／TestFlight可用状态，未安装手机，也不把用户对Apple协议的回复当作中文IME或手机交互验收通过。

非作者本次上传尾核 `ios-upload-2-review.json` SHA `8476f4e3b973c69dfd7e1279a39802991b9ab7892703f48d3111c4c4d42461a7`，核完整本次成功标记、同归档与原位版本日志。
