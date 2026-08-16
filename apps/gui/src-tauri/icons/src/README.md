# 应用图标源文件 · 琉璃城 V3「辉心」(Liquid Glass)

这里是**设计源**,不是构建产物。`icons/` 根目录下的 PNG / ICNS / ICO 全部由这些 SVG 生成。

## 设计概要

坛城结构:**外环(结界)· 内方(宫墙,四顶点与外环相接)· 四门珠 · 心珠**。
几何全部闭合——不带进度弧,圆满是这个图标的前提。

材质原则(V3「辉心」):**明度推向两极**——玻璃要么几乎看不见,要么在边上亮起来,
不停留在中间调。

- 内方是玻璃平面:本体几乎全透明(白 5% → 2%),草绿晕染只剩中心 30% 且尾段归零
  (晕染圆心即坛城圆心,面像被心珠照亮);棱光 90% 是全图最亮的线,内缘拾光 70%
  做出玻璃厚度
- 底沉到 `#141410 → #080807`,两团失焦光斑(14%/9%)与中心辉光(16%)自己亮起来,
  不加颜料
- 落影紧:35% / blur 14 / dy 12——悬浮是「浮起 12px」,不是罩一圈黑晕
- 外环是三层玻璃管(6% 白管体 / 34% 液态色 / 顶部受光 30%),不画 specular 弧线
- 心珠 r66、bloom 55%;珠上白色高光点统一 70%,避免小尺寸读成噪点

外形按 Apple 官方 Liquid Glass 规范:**全出血 1024 方形画布**,
角半径 229(22.4%)、曲率连续,直边笔直。

配色沿用 `apps/desktop/src/styles.css` 的 `--liquid` 与暗底,未引入新色。

## 文件

| 文件 | 用途 |
| --- | --- |
| `mandala-glass.svg` | **主稿(带掩模)**。传统 tauri / icns / ico 流程用这份 |
| `mandala-glass-bleed.svg` | 全出血无掩模版。Xcode 26 / Icon Composer 用这份,圆角与折射交给系统;文件头注释里有四层拆分建议 |
| `mandala-16.svg` | 16px 专稿。只留环与心,用于替换生成结果里的最小尺寸 |
| `tray.svg` | 托盘模板版。纯黑 + alpha,系统自动着色 |
| `mandala-glyph.svg` | 抽象符号版。只留骨干(环 / 方 / 四门珠 / 心珠),`currentColor` 纯线条,app 内当通用符号用,不是应用图标 |

## 生成全套图标

主稿含 `feGaussianBlur`,**必须用 resvg / rsvg-convert 这类完整 SVG 渲染器**导出,
简易转换器会丢滤镜。

```sh
cd apps/desktop/src-tauri/icons

# 1. 主稿导出 1024 PNG
resvg --width 1024 --height 1024 src/mandala-glass.svg icon.png

# 2. 生成全平台图标(覆盖 icons/ 下的 PNG / icns / ico / android / ios)
cd ../..                 # 到 apps/desktop
pnpm tauri icon src-tauri/icons/icon.png

# 3. 最小尺寸换成专稿(Windows ico 内的 16px 同理,需要时用 icotool 重打包)
cd src-tauri/icons
resvg --width 32 --height 32 src/mandala-16.svg 32x32.png

# 4. 托盘图标(@2x 与 1x 同源,只是导出尺寸不同)
resvg --width 44 --height 44 src/tray.svg tray.png
resvg --width 88 --height 88 src/tray.svg tray@2x.png
```

已在本机用 resvg 试渲过 512 / 64 / 32 / 44,滤镜与掩模均正常。

`tauri.conf.json` 的 `bundle.icon` 数组无需改动——路径不变,只是内容被替换。

`icon.svg`(根目录那份)是**上一代设计**,替换时一并用 `src/mandala-glass.svg` 覆盖或删除,
避免两份源并存。

## 验收

- Dock / 启动台:32 / 64 / 128 / 256 各看一眼,玻璃细节在 64px 以下应自然融化成辉光,剪影不变
- 菜单栏托盘:亮色与暗色壁纸各切一次,模板图标应自动反色
- 若走 Icon Composer:确认系统折射生效后,`mandala-glass.svg` 里手绘的拾光边可以撤掉,避免与系统效果叠加
