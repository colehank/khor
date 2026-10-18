// Stands in for elkjs (lane md-render, the owner's ruling 2026-10-19):
// mermaid's `elk` layout is 1.5 MB of EPL-2.0 code for a layout a diagram
// has to ask for by name. A diagram that does ask fails to lay out here,
// and khor shows it as its source (`components/Mermaid`), as it does any
// diagram that will not draw. Wired in by `overrides` in package.json, so
// the real package is never installed.
export default class ELK {
  layout() {
    return Promise.reject(new Error("the elk layout is not part of this build"));
  }
}
