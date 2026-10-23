# shadcn/ui AI Elements

Source-vendored presentation components. ACP and khor's existing incremental
history/live fold remain the source of truth.

- Upstream: https://github.com/shadcn-ui/ui
- License: MIT, reproduced in [LICENSE](LICENSE)
- Registry style: `radix-nova`, retrieved 2026-10-23
- Exact registry URLs and original source SHA-256: [registry.json](registry.json)
- Composition examples: https://ui.shadcn.com/docs/ai-elements
  (`base/reasoning`, `base/tool`, `base/plan`, `base/confirmation`)

## Components and compatibility changes

`src/components/ui/ai/` contains the registry's Trace, Message, Bubble, Item,
CodeBlock, Button and Separator. Their slot structure and default visual
composition are retained. `ui/collapsible.tsx` provides the Radix primitive.

Local changes resolve imports to khor's `cn` and individual Radix packages,
translate registry icon placeholders to Lucide, map interface typography to
khor tokens, and use Radix's actual state attributes. Disclosure controls add
the existing touch minimum and reduced-motion treatment. Code copy actions
use khor's translated accessible label. Existing app controls and theme are
not overwritten by the registry.

Disclosures retain the animation package's 200 ms default. `--time-yield`
is a two-second outcome-label hold and must not set disclosure duration.

The user explicitly requested upstream icons after rejecting khor's diagnostics
icon for reasoning. Only `components/ai-elements/icons.ts` imports Lucide;
the icon gate permits that exact adapter. App navigation retains its own family.
Reasoning uses the upstream text-and-disclosure composition without a decorative
icon.

`src/components/ai-elements/agent.tsx` adapts ACP data to these components:

- Reasoning opens while streaming and closes on completion; manual choice wins.
- Tools retain original titles, raw input, bounded output and written-file
  actions. Edits use the existing CodeMirror diff engine without mounting an
  editor. CodeBlock shows only actual removed/added lines, independent old/new
  line numbers, three lines of context, revealable gaps and full-content mode.
- Plans retain protocol order and update in place.
- Confirmation uses Item and Button; original option IDs and labels survive.
  Submitting and waiting-for-session states retain the request and disable
  choices. Transport success alone does not settle the card; the ACP answer
  does. Failures can retry. Settled requests become compact Trace rows.

ChatView and khor’s Pi-backed task receipts share `lib/acp-view.ts` and
`ai-elements/content.tsx`. Pi events become ACP v1 display snapshots after
Rust SDK validation; its private MCP transport and task authority stay intact.
Codex unified patches retain their source hunk coordinates and are not
interpreted as complete after-files. Tool results retain all textual blocks.

ChatView uses Message and Bubble for conversation structure. Existing Markdown,
composer, attachments, transport and scroll ownership remain in use.

`use-chat-scroll.ts` owns the conversation scroll position. It follows resized
streaming content only at the tail, anchors a manually expanded disclosure,
and keeps a visible history item in place when content above it changes.
The explicit bottom action resumes following. ResizeObserver is scoped to the
visible conversation column and scroll viewport and disconnects on unmount.

## Local acceptance

From `apps/gui`, run `npm run ai-elements:preview`, then open
http://localhost:23776/?bridge=23776 . Add `&theme=light` or `&theme=dark`
to select appearance. The production App and ChatView render three isolated
example sessions: component overview, plan/confirmation and streaming. The
dev-only Vite bridge executes no system commands and reads no real sessions.
This configuration is not part of the normal application build.

## Verification and limits

- `npm run ai-elements`: production App with controlled ACP boundary, desktop
  and mobile widths in both themes, realistic and long-output fixtures,
  keyboard/touch targets, real diff rows/context, streaming, permission
  submitting/waiting/retry and identity, disclosure/read anchors and animated
  tail following. The same gate checks the pure diff model's insertion,
  deletion, creation, EOF, blank/repeated lines, CRLF and gap reconstruction.
- `npm test --ignore-scripts` and `npm run build --ignore-scripts` pass.
- The older `npm run acp` suite has attachment-paste fixture failures and a
  wizard selector for the removed `data-new-session-add-agent` control. Both
  reproduce on unmodified main (`069b1271`); they are not suppressed here.
- Latest-main integration (base `09c01676`): Pi's real local-provider tests
  pass (36 tests, one packaged-runtime-only skip); Rust/Pi/MCP durable task,
  approval, stop/restart and seen-snapshot tests pass (17 cases). The two-device
  credential-home test passes. Claude/Codex stdio ACP fixtures pass; the paid
  real-Codex test stays opt-in. All four bounded-update/translation unit cases
  pass. Shared GUI and console browser suites, default checks and build pass.
- The older ACP suite was rerun: the same three clipboard fixtures report zero
  attachments, then the removed `data-new-session-add-agent` selector times out.
  This gate is not reported as passing.
- Native iOS / Android / Windows / Linux interactions were not exercised.
- The graph index targets main, not this worktree; zero affected flows is not
  coverage evidence. The actual component source and diff were reviewed.

## Updating

Fetch the recorded registry URLs, compare source hashes and composition docs,
then port changes selectively. Update the manifest and license together. Do
not run a registry installer over khor's existing shared controls or theme.
