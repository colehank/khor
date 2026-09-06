# new-api 的线上形状

> ## ⚠️ 这是一份**快照**,它会过期,而且过期时不会有人告诉你
>
> **读的是:`QuantumNous/new-api`,commit
> `0c76e4dae77a279e015329b7478e6f02d6b62edd`**(该仓 2026-09-06T14:49:02Z 的默认分支),
> 2026-09-07 03:48–03:55 用 `gh api repos/QuantumNous/new-api/contents/<路径>`
> 远程读的——**没有 clone**(调研代码不落盘)。
>
> **上游一改,这份就不准了,而以那时的源码为准,不是以这份为准。** 下面每一处
> `file.go:NN` 都只在那个 commit 上成立;分支往前走之后,同一个行号会指向别的东西
> ——**而它坏的时候不报错**,读的人拿到的是一个精确到行、指着不相干代码的指路牌。
>
> 复核的办法和当初取它的办法是同一条:
> `gh api repos/QuantumNous/new-api/contents/<路径>?ref=0c76e4da…`
>
> ## 为什么它在仓里
>
> **这仓没有 new-api 的真样本,这份文件就是 `khor_node::accounts` 里那一支解析的
> 全部依据。** 依据和实现要一起活着:一份活在别处的依据等于没有依据——下一个人要么
> 把源码重读一遍,要么照着代码猜当初为什么那么写。
>
> 代码那边不指这份文件,指的是**上游仓库 + 路径 + 行 + sha**(`accounts.rs` 模块头
> 有那个锚点):那样的引用不依赖这份文件还在,而这份文件只是把同一批出处摊开讲清楚。

> ⚠️ **以代码为准:new-api 自己的机读文档和它的代码不一致,两处。** 下面说的
> `docs/openapi/api.json` 是 **new-api 仓里那一份**,不是 khor 的;拿它来对 khor
> 的实现,会以为我们写错了,所以先说在这儿:
>
> | api.json 说 | 代码说 | 出处 |
> |---|---|---|
> | `/api/log/token` 参数是 `key`(query) | 只读 `Authorization` 头,**没有 query 兜底** | `router/api-router.go:340` → `middleware/auth.go:288-305`;`c.Query("key")` 在**另一个**中间件 `TokenAuth()`,auth.go:390 |
> | `/api/log/token`「🔓 无需鉴权（通过令牌查询）」 | 挂着 `TokenAuthReadOnly()`,无头即 401 | 同上 |
>
> 两个端点的 `responses` 在它那份 api.json 里都是空的(只有 `description: 成功`),
> 所以**响应形状只能从 Go 源取**——下面每一条都是这么来的。
>
> ⚠️ **证据等级:源码,不是真站捕获。** 源码说的是它**应该**返回什么。真站可能跑着
> 更老的版本,或者被改过。这仓已经吃过一次这种亏(老站尾斜杠 404)。**所以解析必须
> 对缺字段宽容**(`Option` / `serde default`),并且这份文件的地位是「有出处的形状」,
> 不是「验证过的形状」。

---

## 一、`GET /api/usage/token/` —— 令牌自己的额度

### 路由与鉴权

`router/api-router.go:282-290`

```go
usageRoute := apiRouter.Group("/usage")
usageRoute.Use(middleware.CORS(), middleware.CriticalRateLimit())
{
    tokenUsageRoute := usageRoute.Group("/token")
    tokenUsageRoute.Use(middleware.TokenAuthReadOnly())
    { tokenUsageRoute.GET("/", controller.GetTokenUsage) }
}
```

**尾斜杠是路由的一部分**(`Group("/token")` + `GET("/")`)。khor 现在发的就是
`/api/usage/token/`,对。

### 响应

`controller/token.go:229-276`(handler 自己还再读一次 `Authorization` 头)

```go
expiredAt := token.ExpiredTime
if expiredAt == -1 { expiredAt = 0 }

c.JSON(http.StatusOK, gin.H{
    "code": true, "message": "ok",
    "data": gin.H{
        "object":               "token_usage",
        "name":                 token.Name,
        "total_granted":        token.RemainQuota + token.UsedQuota,
        "total_used":           token.UsedQuota,
        "total_available":      token.RemainQuota,
        "unlimited_quota":      token.UnlimitedQuota,
        "model_limits":         token.GetModelLimitsMap(),
        "model_limits_enabled": token.ModelLimitsEnabled,
        "expires_at":           expiredAt,
    },
})
```

| 字段 | 类型 | 可空性 | 出处 |
|---|---|---|---|
| `data.object` | string,常量 `"token_usage"` | 总有 | token.go:265 |
| `data.name` | string | 总有,可能是 `""` | `Token.Name`,model/token.go:19 |
| `data.total_granted` | int(网关原始单位) | 总有 | = remain + used |
| `data.total_used` | int | 总有,default 0 | `Token.UsedQuota`,model/token.go:28 |
| `data.total_available` | int | 总有,default 0 | `Token.RemainQuota`,model/token.go:23 |
| `data.unlimited_quota` | bool | 总有 | model/token.go:24 |
| **`data.model_limits`** | **`map[string]bool`**,不是数组 | 总有,可能是 `{}` | `GetModelLimitsMap()`,model/token.go:345-352 |
| **`data.model_limits_enabled`** | bool | 总有 | model/token.go:25 |
| **`data.expires_at`** | **int64 秒**,`0` = 永不过期 | 总有 | 见下 |

**顶层的键叫 `code` 而且是 `true`(布尔),不是 `success`。** khor 现在不看这一层
(只看 HTTP 状态再取 `data`),所以没事——但别有人以后照着别的端点写 `success` 检查。

### ⚠️ `expires_at` 的哨兵:`0` = 永不过期

`model/token.go:22`:

```go
ExpiredTime int64 `json:"expired_time" gorm:"bigint;default:-1"` // -1 means never expired
```

而 handler **把 `-1` 改写成 `0`** 再吐出来(token.go:256-259)。所以:

- **单位:秒**(`common.GetTimestamp()` 这一族都是秒)。
- **`0` 是哨兵,不是 1970-01-01。** 照直解析会画出「1970 年 1 月 1 日到期」。
- `-1` 在这个端点上**看不到**(被改写了),但**防它**:别的路径(比如管理端的
  `expired_time`)仍然发 `-1`,而一个负数时间戳同样会被解成一个真日期。
- 判据:`expires_at <= 0` → 不到期,不画日期。

### ⚠️ `model_limits` 是 map 不是数组

`{"gpt-4": true, "claude-3": true}`,不是 `["gpt-4", "claude-3"]`。要画「模型」那一栏,
取的是**键**;`model_limits_enabled` 为 false 时这个 map 的内容**无意义**——
`DisableModelLimits` 只把 enabled 置 false(model/token.go:354-360),
**不清空 `ModelLimits` 字段**,所以关掉限制的令牌可能仍带着一串旧模型名。
**必须先看 `model_limits_enabled`,再看 `model_limits`。**

### ⚠️ 「分组」不在这个端点里

`Token` 结构体**有** `Group`(model/token.go:29),但 `GetTokenUsage` 的响应
**没有把它放进去**。所以样稿右栏那个「分组」:

- 从 `/api/usage/token/` **拿不到**;
- 每条**日志**里有 `group`(见下),那是「这一次请求走的分组」,不是「这个令牌的分组」;
- 要令牌自己的分组,得走管理端(要 admin key),不在这条路上。

**这一条要么撤掉,要么改口径成「最近用过的分组」。** 别拿日志里的 group 冒充令牌的
group ——它们是两个东西。

---

## 二、`GET /api/log/token` —— 这个令牌最近的调用记录

### 路由与鉴权

`router/api-router.go:340`

```go
logRoute.GET("/token", middleware.TokenAuthReadOnly(), controller.GetLogByKey)
```

**没有尾斜杠。**(和上一个端点不一样,别顺手加。)

### ⚠️ 项目自己的两份出处在这里打架

`docs/openapi/api.json` 把这个端点写成:

```
/api/log/token  get   参数: key (query, string)   "🔓 无需鉴权（通过令牌查询）"
```

**而代码不是这样。** `TokenAuthReadOnly`(middleware/auth.go:288-305)只读
`Authorization` 头:

```go
key := c.Request.Header.Get("Authorization")
if key == "" { 401 }
if strings.HasPrefix(key, "Bearer ") || strings.HasPrefix(key, "bearer ") { key = strings.TrimSpace(key[7:]) }
key = strings.TrimPrefix(key, "sk-")
parts := strings.Split(key, "-"); key = parts[0]
```

`c.Query("key")` 那条兜底在 **`TokenAuth()`**(auth.go:390),是另一个中间件,
**这条路由没用它**。

**照 OpenAPI 写会 401。** 走头,和 `/api/usage/token/` 同一种。

顺带一条 khor 该知道的:**key 只取第一个 `-` 之前的那一段**(`Split(key,"-")[0]`,
在剥掉 `sk-` 之后)。所以 `sk-abc-def` 认到的是 `abc`。这是网关的事,khor 原样发就行,
但排查「密钥被拒」时这一行能省一趟。

### 鉴权中间件还会做的两件事(都会变成 401/403 而不是空列表)

- 令牌 `Status == TokenStatusDisabled` → 401(auth.go:326)。**注意:其它状态
  (比如耗尽)是放行的**,注释明写着 read-only 要继续能查。
- 用户 `Status != UserStatusEnabled` → **403**(auth.go:346)。

khor 的 `get_json` 把 401/403 都映成 `KEY_REFUSED`,对这两种都说得过去。

### 响应

`controller/log.go:79-101`

```go
tokenId := c.GetInt("token_id")          // 中间件设的
if tokenId == 0 { 200 + success:false + "无效的令牌" }   // ⚠️ 200,不是 4xx
logs, err := model.GetLogByTokenId(tokenId)
if err != nil { 200 + success:false + err.Error() }      // ⚠️ 同样 200
c.JSON(200, gin.H{"success": true, "message": "", "data": logs})
```

**⚠️ 失败也是 HTTP 200,靠 `success` 字段区分。** khor 的 `get_json` 只看 HTTP 状态,
所以这两种失败会被当成成功、然后 `data` 解不出来 →`UNREADABLE`。
**这条要显式判 `success`**,否则「无效的令牌」会被报成「读不懂它的回答」——
一档接不上落进旁边那一档的活样本。

### 每条记录

`model/log.go:59-81`(`Log`),经 `formatUserLogs` 加工(log.go:116-122)

| 字段 | 类型 | 说明 |
|---|---|---|
| `id` | int | |
| `user_id` | int | |
| **`created_at`** | **int64 秒** | 索引字段;`RecordLog` 用 `common.GetTimestamp()` |
| `type` | int | **2 = 消费**(`LogTypeConsume`,log.go:87)。1=充值 3=管理 4=系统 5=错误 6=退款。**算花费只能数 type==2** |
| `content` | string | |
| `username` | string | default `""` |
| `token_name` | string | default `""` |
| **`model_name`** | string | default `""` |
| **`quota`** | int | default 0,网关原始单位——**要除 `quota_per_unit` 才是钱**,和 `/api/usage/token/` 一样 |
| **`prompt_tokens`** | int | default 0 |
| **`completion_tokens`** | int | default 0 |
| `use_time` | int | 秒 |
| `is_stream` | bool | |
| `channel` | int | (Go 字段名 `ChannelId`,JSON 键是 `channel`) |
| `channel_name` | string | **总是 `""`**——`formatUserLogs` 显式清空它(log.go:118) |
| `token_id` | int | |
| **`group`** | string | **这一次请求走的分组**,不是令牌的分组 |
| `ip` | string | |
| `request_id` / `upstream_request_id` | string | **`omitempty`——空时整个键不出现** |
| `other` | string | 一段 JSON 字符串,按可见性裁过(`logOtherVisibilityUser`) |

### 条数与顺序

`model/log.go:140-148`

```go
order := "id desc"
LOG_DB.Model(&Log{}).Where("token_id = ?", tokenId).Order(order).Limit(common.MaxRecentItems).Find(&logs)
```

- **`MaxRecentItems = 1000`**(`common/constants.go:60,` 是 `var` 不是 `const`——
  理论上可被改,但没有配置项指向它)。
- **`id desc`,最新在前。** 不分页、没有时间窗参数——**要按天分桶只能自己在客户端切**,
  而且**只覆盖最近 1000 条**:一个忙的令牌可能一天就用掉 1000 条,那时候「7 天趋势」
  会只画出最近半天。**这是这条路的天花板,不是实现的疏忽,要说出来。**

---

## 三、上面这些落成了代码里的哪几条判据

每一条都在 `crates/node/src/accounts.rs`,后面括号里是它住在哪个函数。
**这一节是给「为什么这里要多绕一下」找答案用的**,不是待办清单。

1. **`expires_at <= 0` → 不到期,不画日期**(`expiry_of`)。两种哨兵拼法一起挡。
2. **`model_limits_enabled == false` → 忽略 `model_limits` 的内容**,哪怕它非空
   (`model_limits_of`)。
3. **`model_limits` 按 map 的键取**,而且**排序**——JSON 对象没有顺序,而这串东西
   要画在屏幕上,两次读数之间不能换位置(`model_limits_of`)。
4. **令牌自己的「分组」这两条路都拿不到**,所以界面上没有这一项(2026-09-07 裁定:
   撤掉,不改口径)。日志里的 `group` 是**那一次请求走的分组**,是另一件事。
5. **`/api/log/token` 走 `Authorization` 头**,不走 `key` query。
6. **失败是 HTTP 200 + `success:false`,必须显式判**(`refused_in_body`,在
   `get_json` 里)。不判的话「密钥被拒」会变成「读不懂它的回答」——**指着一个没坏的
   东西**。
7. **日志只有最近 1000 条、`id desc`、无时间窗**(`NEWAPI_LOG_MAX`),所以趋势图的
   标题写**实际覆盖了几天**,并在真截断时说出条数。
8. **花费只数 `type == 2`**;`quota` 要除 `quota_per_unit`(`read_log`)。
9. **所有字段按 `Option` / 默认值解**——见页首那句:这份形状没有经过真站验证。

## 四、⚠️ 拿到真站样本之后,先对这四处

**到今天为止,这一支实现从来没有对着一个真的 new-api 站跑过。** 这不是待办,
是这份文档的状态;拿到一次真原文之后,**先对这四处,不一致本身就是发现**:

- `expires_at` 真站是不是 `0`(或者有没有出现负数);
- `model_limits` 真站是 map,还是被某个版本改成了数组;
- 顶层是 `code`(布尔)还是 `success`;
- `/api/log/token` 真站认不认 `?key=`(更老的版本可能认)。

对完之后**把这一节改成读数**(哪年哪月对的、对上了哪几条),而不是把它删掉:
删掉之后,下一个人无从知道它是验过了还是没人验。
