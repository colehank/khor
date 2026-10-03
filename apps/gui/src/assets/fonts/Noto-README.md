# NotoSansSymbols-subset.ttf

A subset of two Noto faces, merged into one file (SIL Open Font License
1.1, `Noto-OFL.txt`):

- Noto Sans Symbols 2 v2.008 (`NotoSansSymbols2-Regular.ttf`, unhinted)
- Noto Sans Symbols v2.003 (`NotoSansSymbols-Regular.ttf`, unhinted),
  only the code points Symbols 2 lacks

Blocks kept: Arrows (U+2190–21FF), Miscellaneous Technical (U+2300–23FF),
Enclosed Alphanumerics (U+2460–24FF), Geometric Shapes (U+25A0–25FF),
Miscellaneous Symbols (U+2600–26FF), Dingbats (U+2700–27BF),
Supplemental Arrows-B (U+2900–297F), Miscellaneous Symbols and Arrows
(U+2B00–2BFF), and U+FE0E.

Why: these are the symbols a program prints as text (`⏺ ● ✔ ✗ ⚠ ★ …`)
that the system's monospace faces lack. Without a text face for them the
phone falls back to Apple Color Emoji and draws them as emoji; the Mac
falls back to Apple Symbols, which the phone does not have. Code points
whose default presentation is emoji (`✅ ❌ 🟢`) stay emoji.

Made with fontTools:

    pyftsubset NotoSansSymbols2-Regular.ttf --unicodes="U+2190-21FF,U+2300-23FF,U+2460-24FF,U+25A0-25FF,U+2600-26FF,U+2700-27BF,U+2900-297F,U+2B00-2BFF,U+FE0E" --layout-features='' --no-hinting --output-file=a.ttf
    pyftsubset NotoSansSymbols-Regular.ttf --unicodes-file=<those blocks minus a.ttf's cmap> --layout-features='' --no-hinting --output-file=b.ttf
    pyftmerge a.ttf b.ttf --output-file=NotoSansSymbols-subset.ttf
