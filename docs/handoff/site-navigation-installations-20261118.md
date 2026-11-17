# Navigation, installation layout and light glass — 2026-11-18

Worktree: `/Volumes/disk0/Khor/worktrees/current/khor-site-story`, `feat/site-story`.
Local changes only; no commit, merge or deployment.

## Behavior

- Navigation order: Docs, Download, GitHub, appearance, language. Functional icons use Apple SF Symbols
  with measured optical bounds, a 22px SVG box and a 44px hit target. GitHub retains its official mark.
- Appearance toggles immediately, without a menu. Language uses a native HTML select with System,
  English and Simplified Chinese. Defaults follow the system; explicit choices persist in localStorage,
  apply before paint and synchronize across tabs. No Khor popup components are used on the site.
- Theme/locale changes update metadata, content, graph labels, mobile posters and desktop GUI parameters.
  Mobile remains poster-only; no interactive iframe is loaded on coarse-pointer devices.
- KhorNet title and symbol retain matching brand green in all three scenes, with the existing light/dark tokens.
- The complete platform/CLI group is inside the Build composition. Short screens reserve enough normal-flow
  space to scroll the remainder without covering it with the footer. Hidden installation controls are inert.
- Both installation labels share typography. The command box shrinks to its content, is centered beneath
  its label on narrow screens, and wraps only when necessary. Copy feedback remains a temporary checkmark.
- Light-canvas glass uses a denser graphite diamond, a visible graphite/olive ring and lighter shadows.
  `brand-artwork.ts` derives this from the canonical SVG and existing color tokens; the dark source paint,
  all geometry, green nodes and transparent background are retained. Both handoffs use these same paint
  expressions, including the perimeter copy, so theme changes work mid-transition without a crossfade.

## Repeated Wired-label overlap

Short viewports compressed the graph while SVG text remained at its readable CSS size. The first figure
now has a minimum reading canvas on narrow screens, with excess height reserved after the opening stage.
At layout time `fitWired` measures actual character extents, tests fixed anchors on the same title ellipse,
and prioritizes clearance from device bodies, KhorNet and the roaming Wi-Fi envelope. It does not run per frame.
The KhorNet outer path has an independent title/route mask, so another network's label padding cannot erase it.

## Verification

Evidence: `/Volumes/disk0/Khor/evidence/site-preference-nav-20261118/` and
`/Volumes/disk0/Khor/evidence/site-build-installations-20261118/`.

- Installed Chrome, headless only: direct appearance switching, native language selection, preference
  persistence, route navigation, metadata, poster/GUI sync and matching KhorNet colors at 1440, 1024, 390 and 320px.
- Build layout: shared label typography, no horizontal overflow or copy/icon/install overlap, footer clearance,
  accessible command and copy feedback at four viewport sizes. Short screens retain natural extra scroll space.
- CLI intrinsic sizing: 408px command within a 671px container at 746px viewport; command, box and container
  centers coincide. At 390px it wraps within the available width and remains centered.
- Light glass opacity is 0.9 at the diamond base and 0.38 at the first band stop; dark remains 0.28 / 0.5864.
  Unfold and fold each inspected at 20%, 50%, 80% on mobile. No invalid geometry or page errors.
- Wired label sampled at three roaming positions and four normal viewport sizes, plus short/landscape layouts.
  Normal-view glyph clearance exceeds 4px from devices and other boundaries; captions remain fixed during roaming.
- Targeted website TypeScript, Astro-only static build and `git diff --check`.

No real Lab, native build, visible browser windows or new Safari-on-device verification.
