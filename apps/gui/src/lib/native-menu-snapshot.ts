/** A native child WKWebView gets the *rendered* shared menu, not another
 * spelling of its rows. It has no app IPC and receives no app scripts.
 * Freeze computed styles so theme, touch sizing and SVG faces keep the
 * same source as the React menu. The native host supplies its tiny,
 * lease-scoped pick/measure bridge as a private WKUserScript. */
const properties = [
  "display", "position", "top", "right", "bottom", "left", "z-index", "isolation",
  "box-sizing", "width", "height", "min-width", "min-height", "max-width", "max-height",
  "margin-top", "margin-right", "margin-bottom", "margin-left",
  "padding-top", "padding-right", "padding-bottom", "padding-left",
  "overflow-x", "overflow-y", "overscroll-behavior-x", "overscroll-behavior-y",
  "flex-direction", "flex-wrap", "flex-grow", "flex-shrink", "flex-basis",
  "align-items", "align-self", "justify-content", "row-gap", "column-gap",
  "grid-template-columns", "grid-template-rows", "place-items",
  "font-family", "font-size", "font-weight", "font-style", "font-variant-numeric",
  "line-height", "letter-spacing", "white-space", "text-overflow", "text-align",
  "text-transform", "text-decoration", "word-break", "overflow-wrap", "color", "color-scheme",
  "background-color", "background-image", "background-clip", "background-origin", "background-size", "background-position", "background-repeat",
  "border-top", "border-right", "border-bottom", "border-left", "border-radius",
  "box-shadow", "opacity", "visibility", "filter", "backdrop-filter", "-webkit-backdrop-filter",
  "transform", "transform-origin", "transform-box", "clip-path",
  "mask-image", "mask-clip", "mask-origin", "mask-size", "mask-position", "mask-repeat", "mask-mode", "mask-composite",
  "-webkit-mask-image", "-webkit-mask-clip", "-webkit-mask-origin", "-webkit-mask-size", "-webkit-mask-position", "-webkit-mask-repeat", "-webkit-mask-composite",
  "mix-blend-mode", "fill", "fill-opacity", "fill-rule", "stroke", "stroke-width", "stroke-opacity",
  "stroke-linecap", "stroke-linejoin", "stroke-dasharray", "stroke-dashoffset", "vector-effect",
  "flood-color", "flood-opacity", "color-interpolation-filters", "r", "cx", "cy",
  "pointer-events", "touch-action", "user-select", "-webkit-user-select", "-webkit-appearance",
];

function copyStyle(style: CSSStyleDeclaration, to: HTMLElement | SVGElement) {
  // Computed values already resolve theme variables. Bound the property
  // set rather than copying hundreds of inherited custom properties on
  // every SVG shape (which could exceed the native payload budget).
  for (const property of properties) {
    let value = style.getPropertyValue(property);
    // WebKit may resolve an SVG fragment against the app's full URL.
    // Only document-local references can cross this boundary.
    value = value.replace(/url\(["']?[^)"']*#([^)'"\s]+)["']?\)/g, 'url("#$1")');
    if (/url\(/i.test(value) && !/^url\(["']?#[^)]*\)$/i.test(value.trim())) continue;
    to.style.setProperty(property, value);
  }
  to.style.setProperty("animation", "none");
  to.style.setProperty("transition", "none");
}

export function nativeMenuSnapshot(menu: HTMLElement): string {
  const clone = menu.cloneNode(true) as HTMLElement;
  const originals = [menu, ...menu.querySelectorAll<HTMLElement | SVGElement>("*")];
  const copies = [clone, ...clone.querySelectorAll<HTMLElement | SVGElement>("*")];
  for (let i = 0; i < originals.length; i++) {
    const from = originals[i], to = copies[i];
    if (/^(SCRIPT|STYLE|LINK|META|BASE|IFRAME|OBJECT|EMBED|FORM)$/i.test(to.tagName)) {
      to.remove();
      continue;
    }
    for (const attribute of [...to.attributes]) {
      if (/^on/i.test(attribute.name) || /^(src|srcset|action|formaction|nonce|integrity)$/i.test(attribute.name) ||
          (/^(href|xlink:href)$/i.test(attribute.name) && !attribute.value.startsWith("#"))) {
        to.removeAttribute(attribute.name);
      }
    }
    to.removeAttribute("style");
    copyStyle(getComputedStyle(from), to);
    // Decorative generated boxes (glass rims) are not cloneNode children.
    // Freeze their actual resolved styles into ordinary inert boxes, in
    // the same before/after paint order. No second copy of the rim CSS.
    if (from instanceof HTMLElement) for (const side of ["before", "after"] as const) {
      const style = getComputedStyle(from, `::${side}`);
      if (!['""', "''"].includes(style.content) || style.position !== "absolute" || style.pointerEvents !== "none") continue;
      const decoration = document.createElement("span");
      decoration.dataset.nativeMenuPseudo = side;
      decoration.setAttribute("aria-hidden", "true");
      copyStyle(style, decoration);
      if (side === "before") to.prepend(decoration);
      else to.append(decoration);
    }
  }
  clone.style.position = "relative";
  clone.style.left = "auto";
  clone.style.top = "auto";
  clone.style.margin = "0";
  clone.style.width = "100%";
  const scheme = getComputedStyle(menu).colorScheme;
  const safeScheme = scheme === "dark" ? "dark" : "light";
  return `<!doctype html><html style="color-scheme:${safeScheme}"><head><meta name="viewport" content="width=device-width,initial-scale=1,maximum-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; img-src data:; script-src 'none'"><style>html,body{margin:0;padding:0;background:transparent;overflow:hidden}*{box-sizing:border-box}</style></head><body>${clone.outerHTML}</body></html>`;
}
