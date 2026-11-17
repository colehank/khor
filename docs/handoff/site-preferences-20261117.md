# Website preferences and native scroll handoffs — 2026-11-17

Local worktree: `/Volumes/disk0/Khor/worktrees/current/khor-site-story`, branch `feat/site-story`.
Published baseline remains `7accd420` / UI source `ed0623a9`. This iteration has not been committed or deployed.

## Accepted behavior

The homepage follows browser language (English / Simplified Chinese) and system light / dark preference.
Both languages are static content on the same routes. The first-paint preferences script translates metadata and
attributes, sets document language, and selects the correct GUI poster before its source is requested.
Desktop uses the real App with synthetic local data. Coarse-pointer / mobile uses only the complete actual GUI poster;
it does not load a desktop iframe, and browser pinch zoom remains available. Fog parameters are unchanged.

The latest introduction reads Meet Khor / 试试 Khor. Other confirmed copy includes one mind / 一个心智,
wired / 有线网, Wi-Fi / 无线网, Build with Khor / 与 Khor 一起构建, and Presented by / 呈现.
Stay connected describes a decentralized peer-to-peer network. Continue anywhere explicitly mentions accessing
target-device files and web apps and tracing Git history. Every chapter has a single paragraph with normal punctuation.

## Layout and animation

- The three diagrams use the same initial device coordinates, fixed title arcs, responsive label/glyph sizes,
  and title clearances that mask all crossing edges. Only the non-title Wi-Fi boundary deforms during roaming.
- Hyperedge marks have no backplates. File and Web are icon-only circular badges attached at the devices' lower right;
  agent badges occupy their upper right. Pairwise curves use the central corridor instead of a large outer loop.
- Middle chapters scroll as ordinary document content. There is no shared pinned stage that consumes wheel motion
  while waiting for a chapter threshold. There is no fixed-to-body visual overlay crossing the text.
- Only icon-to-network and agent-to-icon handoffs use local sticky stages. The complete canonical glass icon continuously changes its original stroke widths, gradient stops, specular sizes,
  surface cutout and pearl radii, then becomes the KhorNet mark with the accepted small-symbol weights. Perimeter linework
  and the four device nodes unfold around it. The center does not grow a second full icon.
- The reverse handoff gathers the graph into the complete line icon before restoring the original glass and fixed
  upper-left light. Material and geometry phases are distinct and reversible with native scroll.
- Intro progression is 5.2 viewport heights plus the initial .5 desktop / .7 mobile hold. Every change uses the
  first 60% of its segment, followed by a 40% completed-pose hold. Unfold uses .85 screen, followed by .75 screen hold.
- Let Khor take over has a 1-screen reading hold before the .85-screen fold. Build then has a .8-screen completed hold.
  These are distances, not timers; no wheel/touch interception, forced paging or time lock is introduced.
- Reduced motion preserves all text and diagrams in static document flow. Loops pause offscreen and while hidden.

`story.ts` retains the existing native ScrollTimeline for the five-step introduction. `narrative-tour.ts` owns the
two local handoffs and scene visibility. `story-morph.ts` continuously changes the perimeter paths and device trajectories; `story-material.ts` retains one
copy of the canonical primary geometry and changes its material in place. The glass, wire and small-glyph crossfade
layers were removed. Primary geometry remains at opacity 1; the static endpoints hand off only when their geometry
matches. Native edges draw/retract before hosts move, and caption cutouts follow the final network mask. The obsolete standalone product-tour observer was removed.

## Preference coverage

Localized homepage, navigation, accessibility text, SVG labels/descriptions, download page and recommendation,
copy feedback, footer, 404, docs sidebar and all five documentation articles. GUI bootstrap accepts the website
language without changing product App behavior. Chinese light/dark posters were captured from the actual App;
no screenshot UI was recreated. Existing tokens supply both palettes.

## Evidence and practical limits

Evidence root: `/Volumes/disk0/Khor/evidence/site-preferences-20261117/`.

- Initial preference checks: en-US / zh-CN × light / dark, homepage + docs, no horizontal overflow or page errors.
- `native-flow/report.json`: 1440×1000, 390×844, 320×700; middle-chapter position moved by -120 CSS px for +120 px
  native scroll; no duplicate SVG IDs or page errors. Reduced-motion Chinese mobile retained all three paragraphs
  and an operable final CTA in normal flow.
- `final-motion/`: final material, reading-hold and normal-scroll checks, with headless installed Chrome only.
- Targeted website TypeScript check, `npm run demo:check --prefix apps/site`, and full static website build.
  The final Astro-only rebuild follows the last animation refinement; the GUI bundle is unchanged from the full build.

No visible test windows, no WebKit matrix, no Rust/native build, no real Lab/device connections, no deployment.
Headless mobile layouts do not substitute for Safari-on-iPhone performance verification.

## Geometry/material follow-up

The user rejected crossfades as a substitute for transformation. Both handoffs now reuse persistent primary SVG
nodes throughout the scroll. Ring thickness changes in displayed pixels while the complete mark shrinks; the glass
pane clears geometrically from its center, highlights and shadows contract, gradients flatten to the diagram's ink,
and pearl/ring proportions continuously match the existing 2026-11-10 session glyph at the endpoint. There is no
second wire artwork or generated small-icon layer. Source and endpoint artwork use a binary, matching-pose handoff.
The network perimeter is a cubic-path interpolation with an advancing drawn segment, and device circles/glyphs grow
along curved paths. Secondary routes are drawn only after the device positions have settled, and retract before the
hosts move on the return. Masks protect network captions. Theme changes remeasure the active material's paint.

`geometry-morph/report.json` and screenshots sample both directions at six intermediate positions on 1440×1000
dark English and 390×844 light Chinese headless Chrome. Primary art stayed at computed opacity 1, its DOM identity
was unchanged, material/geometry values changed, and no page errors, duplicate IDs or horizontal overflow occurred.
The follow-up polish check verified that the original source image is hidden during the live morph and all three
diagrams remain visible when Reduce Motion is enabled during a transition. Evidence is under `geometry-morph/`;
its `build.log` records the final Astro static build. Reading holds and ordinary middle-chapter scroll are unchanged.
