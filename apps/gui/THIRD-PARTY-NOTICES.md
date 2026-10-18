# Third-party notices

Files khor's interface ships that were made by others, with their licences.

## Nerd Fonts — Symbols Only (mono face)

- File: `src/assets/fonts/SymbolsNerdFontMono-Regular.ttf`, from the
  official release v3.5.1 (`NerdFontsSymbolsOnly.zip`, 2026-08-21),
  unchanged. SHA-256
  `fe471e538392f51910faab985fa8e192a39dd3426125edd15b71b3680df0e749`.
- Licence: MIT, Copyright (c) 2014 Ryan L McIntyre —
  `src/assets/fonts/NerdFonts-LICENSE.txt`.
- The glyphs come from icon sets under their own licences, listed in the
  release's README (`src/assets/fonts/NerdFonts-README.md`): MIT
  (Devicons, Font Awesome Extension, Octicons, Seti, Powerline, Power
  Symbols, Hack extras), CC BY 4.0 (Codicons — © Microsoft; Font Awesome
  — © Fonticons, Inc.), Apache 2.0 (Material Design Icons), SIL OFL 1.1
  (Pomicons, Weather Icons) and the Unlicense (Font Logos). No copyleft
  licence of the GPL or AGPL kind is among them.

## Seti UI file icons

- Files: `src/vendor/seti/` (definitions and icons).
- Licence: MIT — `src/vendor/seti/LICENSE`.

## KaTeX fonts

- Files: the `KaTeX_*.woff2` faces of `katex` 0.18.10
  (`node_modules/katex/dist/fonts/`), bundled unchanged into the math
  chunk the first time a text holds a formula; the woff and ttf copies are
  left out (`vite.config.ts`).
- Licence: MIT, Copyright (c) 2013-2020 Khan Academy and other
  contributors — `node_modules/katex/LICENSE`.

## Diagram sanitiser (DOMPurify)

- Bundled from npm into the mermaid chunks, which load only when a
  closed ```` ```mermaid ```` block is drawn: `dompurify` 3.4.17, licence
  MPL-2.0 OR Apache-2.0, used here under Apache-2.0. (mermaid's `elk`
  layout and its `elkjs` are not shipped: `vendor/no-elk` stands in.)
