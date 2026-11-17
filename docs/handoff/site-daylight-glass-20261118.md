# Daylight glass — 2026-11-18

Local worktree: `/Volumes/disk0/Khor/worktrees/current/khor-site-story`, branch `feat/site-story`.
Implemented after the user approved the light-material study. Not committed or deployed.

## Design and scope

The website's light surface is neutral mist white `#f6f7f5`, using the explicitly approved
`--site-daylight-canvas`. Shared product tokens and the embedded GUI's theme remain independent.
The homepage, docs and download surface use this same canvas; the unchanged Fog parameters fade
into it through the existing hero gradient.

The icon uses daylight transparent glass rather than a dense graphite diamond:

- A clear diamond face with restrained transmitted color and paired bright/shaded edges.
- Glass-band reflections, refractive edges and low-opacity directional shadows.
- A broad upper-left white environment and restrained green transmission.
- Canonical inner-ring and pearl colors, source geometry, node ownership and transition paths.

`src/brand-artwork.ts` derives the light paints from the shared white/neutral/brand tokens.
Each light ramp is merged with the canonical ramp at their union of offsets. Sampling the
source's linear color/alpha ramp at new offsets retains its interpolation in dark appearance.
The live CSS theme expressions survive fragment isolation and morph cloning; the same elements
change paint without a second light icon or theme crossfade. Existing mask/gap timing is unchanged.
Light-background screen compositing was removed: its environment now has a transparent ground
and its own daylight paints. Dark continues using the original source environment.

## Verification

Evidence: `/Volumes/disk0/Khor/evidence/site-daylight-glass-20261118/`.
Study: `/Volumes/disk0/Khor/evidence/site-light-material-study-20261118/`.

- Headless installed Chrome: 390×844 light, 1440×1000 light and dark.
- Unfold and fold sampled at 20%, 50%, 80% for each size/theme; mobile intermediate frames
  inspected in both directions, plus complete Meet and Build compositions.
- Body canvas resolves to RGB(246,247,245) in light and RGB(0,0,0) in dark. Light pane base
  opacity is 0.08; dark remains 0.28. First light band stop is 0.95; dark remains 0.5864.
- No page errors or invalid morph geometry. Existing command centering remains intact.
- Targeted TypeScript check, Astro-only static build, `git diff --check`.

No GUI rebuild, native build, real Lab, visible browser windows or deployment.
This supersedes the graphite-light material recorded in `site-navigation-installations-20261118.md`.

## Copied perimeter correction

The user found a broad olive ribbon during Meet → Stay. The copied gradient was theme-aware,
but its color/alpha were flattened to brand ink using the same front-loaded progress as width.
The ink appeared while the band was still wide. `story-morph.ts` now retains refractive source
colors/alpha until 76% of thickness has contracted, then transitions to the final green wire.
Reverse folding follows the same reversible mapping. The gradient coordinate frame also follows
the interpolated perimeter bounds instead of retaining the original icon's small paint bounds.
This uses already-computed path coordinates, without per-frame DOM layout reads.

Evidence: `/Volumes/disk0/Khor/evidence/site-perimeter-material-20261118/`.
Headless Chrome checks five positions in both directions at phone/wide/light/dark sizes:
the broad perimeter retains varied refractive colors and alpha, the thin endpoint is green,
and no invalid geometry or page errors occur. Three early unfold frames and reverse frames
were inspected. Targeted TypeScript, Astro static build and whitespace checks pass.
