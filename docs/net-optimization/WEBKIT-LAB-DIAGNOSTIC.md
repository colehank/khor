# Mac Lab 浏览器崩溃诊断

准确产品集成I3为6f4c8c39/treec651b2fb；完整Quality43507369283首轮13/13通过，GUI原27组及附件9场全部成功。原生Mac Lab43507423100的native/frontend成功、真实arm64 host与来源已核，但两次完整browser checks分别在popup动画5秒、菜单点击15秒失败，不能宣告完整Lab通过。

保留两次失败的完整log/ZIP；两轮各完成62个无errors场景，原77人口均不完整。随后固定四场诊断43515609775，仅测试脚本变化，比较资源预览后与全新浏览器，顺序after_resources/fresh/fresh/after_resources。实际失败/成功/失败/成功，各条件一成一败；失败后DOM/RAF/JS timer均含Target crashed。已证实WebKit页面崩溃，尚未定位具体产品或引擎缺陷。资源预览唯一原因假设不受支持。数据与log逐场一致，ZIP SHA/CRC已核：E/public-webkit-menu-diagnostic-v1/four-case-crash-verification-v1.json。

原引擎由锁文件实际固定playwright-core1.62.1/WebKit26.5。官方npm stable与[v1.65.1发布页](https://github.com/microsoft/playwright/releases/tag/v1.65.1)于本轮核对为1.65.1，内置WebKit27.2 revision2388；原元数据E/playwright-stable-metadata-20261114-v1.json。旧版本相关公开crash报告仅作为实验线索，不当作本项目根因。

冻结E/webkit-stable-diagnostic-design-v1.json后，独立版本对照仅修改package-lock的playwright-core三字段；无产品代码、断言、动画、点击稳定性或超时变化。固定四场一次，无替代/原样重复。准确dd56cb30/treea9c639cb，run43520578187已首轮success。4/4原settings通过，两组资源前置均无errors，实际WebKit27.2；原log和ZIP14083711309 SHA c2a880fd/CRC已核、逐场人口相等，E/public-webkit-stable-diagnostic-v1/four-case-stable-verification-v1.json。该版完整manager/unit/其他既有前后置步骤亦success。全四场通过只允许后续完整同源Lab验证；不从四场推断总体稳定性，不将仅版本变化归因某个具体上游修复，也不替代网络/真机/N5。

本地仅源码和证据操作，无产品执行；其他任务Lab23961、main和现装App保持。

基于该固定四场证据，I4完整候选准备中：在DB原两端夹具上只带入锁文件更新和本计划最新记录，恢复原完整77case runner，重新跑准确源Quality all与native Mac Lab；不带入诊断替代runner。E/product-integration-v4-intent-v1.json于集成前冻结。
