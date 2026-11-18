# Scroll recoil correction — 2026-11-18

Local worktree: `/Volumes/disk0/Khor/worktrees/current/khor-site-story`, branch `feat/site-story`.
Published after the user's deployment request: UI `f8ee7108` / gh-pages `5affba22`.
Pages run `44238238098` succeeded; ordinary khor.cloud touch/wheel checks report 0px recoil.

## Cause and change

The user reported strong drag resistance and seeing the second caption before being pulled
back. The root had `scroll-snap-type:y proximity` with nine `scroll-snap-stop:always` markers.
Captions switch when a segment starts, while snap points represented completed poses. A short
gesture could therefore reveal the next caption but still end within the previous snap point's
catchment. The browser then moved backward through the caption boundary.

Remove the snap controller, root snap CSS and invisible markers. All manual scrolling now uses
the browser's native displacement/momentum. No wheel/touch interception or corrective scrolling
is introduced. Existing reading distances, 40% completed-pose portions, local sticky morphs,
bookmarks and explicit #connect navigation are retained.
The #connect target rounds forward with a one-pixel margin: without snap correction,
floating-point subtraction at an exact integer endpoint could otherwise leave the
network infinitesimally below its live pose. This is only an explicit navigation
target; manual scroll positions are never adjusted.

## Reproduction and verification

Evidence: `/Volumes/disk0/Khor/evidence/site-scroll-recoil-20261118/`.
Tests use headless installed Chrome with actual CDP touch scrolling / Playwright wheel input;
they do not disable snap in the test or use programmatic movement for the tested gesture.

| Input | Before | After |
| --- | --- | --- |
| 390×844 touch, 250px drag | Advanced 251px, revealed agents, snapped back 251px | Advanced 251px, 0px recoil |
| 1440×1000 wheel, 290px | Advanced 283px, revealed agents, snapped back 283px | Advanced 290px, 0px recoil |

Forward gesture regression was red before the change and green afterward. Reverse input,
chapter bookmarks, #connect and reduced-motion CTA were also checked. Targeted TypeScript,
Astro static build and `git diff --check` pass. No native/GUI rebuild or visible test windows.
Chrome's synthesized touch is not an iPhone Safari real-device verification.
