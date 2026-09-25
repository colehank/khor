# Avatar

身份。默认是 `lq1 → lq3` 的 135° 渐变——这三档橄榄绿是 `crates/core/src/avatar.rs` 那套调色板的镜像，也是橄榄绿在这套系统里保留的第一个岗位。外沿一圈 `avatar-edge`，让浅色头像从浅色底上分得出来。

三档尺寸：`avatar` 32 在列表行里，`avatar-lg` 72 在详情页顶部，`avatar-xl` 96 在总览里。永远是 `radius-full`。

角标（`mark`）说的是**这是哪一类**：`chat` 对话、`terminal` 终端、`link` 借来的连接。它坐在右下角，底是 `card`，外面一圈 1.5px 的 `side` 把它从头像上切开。

## 用

```jsx
<Avatar size={32} mark="terminal" />
<Avatar size={72} from="#9cc19a" to="#c9523f" />
```

消费方负责：尺寸、两端颜色（从后端那套调色板来，不要现编）、角标。

## Do / Don't

- 颜色从 `avatar.rs` 那套里取，**不要**当场算一个 hash 出来的颜色——同一个身份在两处必须同色。
- 头像上不写字。要缩写的场合说明这个东西还没有身份，应该用一个字形而不是首字母。
- 标签条上那个 16 的脸是同一套渐变，但它不是 `Avatar`，它是 `Tab` 自己的 `k-tab-face`。
