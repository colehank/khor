# Q031 root 独立审查

对象：d464f6be361f0124fc9066a677166055a81b34f2；审查者 root，非作者。

审查实际response/handle_async/ledger来源。仅验证成功保存的Message响应context或Task receipt授予continuation，未完成intent和原请求中的context不授予权限；归属同时限定caller、agent别名和原card_url。接口/token刷新不等同于换服务；返回Message的context/task opaque标识先验证再保存。上下文没有通过伪造Task补齐，当前持久ledger无TTL淘汰，32MiB上限拒绝新增而非静默丢失授权。

真实 Q031/green/status.json exit0；cargo.log为4通过，1个fresh-process worker由父用例执行。JSONRPC/REST父测试包含重启恢复；失败来源、unknown/pending context、别名/endpoint隔离在专用用例中有断言。

结论：无本提交阻断，可集成。该结论不代表对外部A2A服务真实性另作保证；精确集成版本回归仍待执行，main/发布未授权。
