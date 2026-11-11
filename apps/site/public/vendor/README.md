# Local background dependencies

The marketing hero uses the unmodified Vanta Fog preset selected by the user.
These pinned browser distributions are served locally; no CDN is needed at runtime.

- `three-r134.min.js`: three.js r134, https://cdnjs.cloudflare.com/ajax/libs/three.js/r134/three.min.js
- `vanta-fog-0.5.24.min.js`: Vanta 0.5.24 Fog, https://cdn.jsdelivr.net/npm/vanta@0.5.24/dist/vanta.fog.min.js

MIT licenses are included alongside the distributions. Vanta skips rendering while
its element is outside the viewport. Reduced motion uses a captured frame of the
same preset, with no speed or colour substitution.
