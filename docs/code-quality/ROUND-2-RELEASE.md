# 第二轮正式交付记录

2026-11-01。用户明确授权“先直接commit到main并ship”，替代旧发布前真人验收限制。产品源a228，已合并推送的正式构建源为 `main@eca881f2b30c2893b7e73f81ae1e0f5cefbe8c6c`；后继提交仅补发布文档。

| 目标 | 实际结果 |
| --- | --- |
| main | 产品与12任务卡／交付记录已合并并推送 |
| Mac GUI＋CLI | release构建、完整UI／Pi、strict签名核验和正式切换均通过；GUI91072运行；原8宿主身份及8会话ID不变 |
| hinton／turing／lecun | 一个Linux构建、一次共享NFS安装；三实际listener exe SHA、installed SHA与完整UI全部采用新版 |
| iOS正式归档 | 通过；真实arm64 IOS、签名／team／profile／新UI一致；旧60可回退，当前archive不变 |
| TestFlight | **未上传成功**；单次xcodebuild70，Apple报PLA更新与缺iOS Distribution身份；待账号持有人处理新协议后复查签名并续传 |

共同完整UI：`khor-ui-assets:/assets/index-B55HQiJV.js`。Mac GUI SHA `7af048369a2fbe2833d870e835550168735b26274e52e4c83665be3cbc3884e6`；Mac CLI `6e162eaae715c9b468a9c2479eac934963998a07aefb3743a7fa1fc181cc29a3`；Linux CLI `9117de4af9fc07b0bff2c660c12fe42c27bc6085da67f9cbee3640cea68a6c7a`。所有原source/gen/dist在发布文档修改前最终核等。

证据唯一目录 `/Volumes/disk0/Khor/evidence/releases/20261101-round2`。非作者最终审查`final-release-review.json` SHA `4a4f8a0705f319192e255cbf5dad6be86b06559f7c9413f7f243086f270f7aa5`。核心入口：`final-products-before-docs.json`、`mac-qualified.json`／`mac-cutover-result.json`、`ios/result.json`／`product-verification.json`／`upload-1/result.json`、`cluster/stage-validation.json`／`adoption-summary.json`／`session-comparison.json`。

原失败均保留：Mac／Linux Cargo均0，原wrapper用了错误的多字节mmap成员检查；补验以明确find全字节核验，不重编、不改旧false。原onset1来自serve.sh空日志末AND-list返回1，三节点实际采用另证，不冒onset0。lecun的sessions读取发布前后均busy，不能算会话验收通过。main types、既有unit、Vite、Pi43测试通过；额外gates索引有35孤立脚本＋chat-recovery未登记，记NEXT，不声称所有门绿或unit覆盖35项。

Apple接续见本次外部`ios-upload-handoff.md`。用户本人／团队Account Holder应先审阅处理Apple更新协议；Root未代接受条款、变更账号／keychain或自动重试。当前只读有效本地签名development1／distribution0，协议处理后再判断签名修复。归档已保存，后继纯文档提交不会要求重编产品；续传仍须核实际产品输入等同，成功后才记录Apple分配build。

原发布后未验仍保留：Mac系统中文IME与真实iPhone交互，见[真人清单](ROUND-2-HUMAN-ACCEPTANCE.md)。本地构建／签名资格和正式发布不能替代真人操作；**不宣布34项整轮完成**。
