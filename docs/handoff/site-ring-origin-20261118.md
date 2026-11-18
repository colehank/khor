# Single visible outer ring — 2026-11-18

Local worktree: `/Volumes/disk0/Khor/worktrees/current/khor-site-story`, branch `feat/site-story`.
Published from source `2fd88d05` to gh-pages `503c998` in [Pages run 44281610286](https://github.com/colehank/khor/actions/runs/44281610286).

## Report and cause

The user rejected two complete glass rings in `/Users/zgh/khor-drop/IMG_5728.png`.
The previous local attempt preserved the full outer ring in the traveling core,
then rendered a second perimeter to become KhorNet. A circular cutout hid their
shared pixels, but could not prevent two rings once they separated. Early width
and origin-alignment checks passed while missing this structural visual defect.
That copied-ring approach is superseded by this correction.

## Current implementation

- Only the perimeter owns the large outer glass band. The source core no longer
  retains a visible outer ring, rim contours, shadow or annulus reflections.
- The annulus light group is reparented from the source core into the perimeter.
  Shadow, refractive edges and band share its changing contour. There is no
  overlapping-ring cutout and no fading between two complete large rings.
- The core travels with its diamond, inner ring and center. Its outer stroke is
  fully withdrawn while traveling. Once it reaches the final small title scale
  (72%), that small mark's outer stroke draws and four small glyph tips grow.
  In reverse, those details withdraw before the core enlarges.
- The single perimeter begins at the canonical outer circle, retains thickness
  through 14%, then narrows to the existing green KhorNet line by 68%.
  The title clearance opens before the caption appears.
- The same four original outer pearls travel on outward cubic paths and become
  devices. On return they become the final icon's four pearls. No original-size
  pearl set remains hidden in the moving core.
- Native scrolling, chapter layout, themes, desktop GUI, mobile poster and
  download behavior are unchanged.

## Focused verification

Evidence: `/Volumes/disk0/Khor/evidence/site-single-perimeter-20261118/`.
Headless installed Chrome captures six positions in each direction at 390×844
light and 1440×1000 dark (24 frames). Visually inspected at least three intermediate
light-phone and dark-wide frames, including the formerly duplicated-ring region,
and the phone return sequence. No two large complete glass rings remain.

Checks require one main perimeter, zero core outer-rim/annulus layers, four original
traveling pearls, valid closed geometry and core opacity 1. Any visible core outer
stroke must already be at the target small mark's scale; before 72% it is fully
withdrawn. No page errors occur. Targeted TypeScript, Astro static build and
`git diff --check` pass. No real Lab, native build, visible browser or WebKit matrix.
