// GitHub's alerts (lane md-render): a quote whose first line is
// `[!NOTE]`, `[!TIP]`, `[!IMPORTANT]`, `[!WARNING]` or `[!CAUTION]` is
// not a quotation, it is the writer flagging a passage — so it is drawn
// as a passage with its kind named above it, not as a quote.
//
// **A rehype pass, after the clean.** On the file page the HTML is
// cleaned with GitHub's schema, which drops attributes it does not know;
// running after it means the mark this adds is this build's and is not
// at the mercy of the allow-list. Both faces run it.
//
// **The kind is a word, not a colour.** GitHub paints the five in blue,
// green, purple, amber and red; in this app colour has jobs already (the
// brand's three, the six state words, git's seven), and none of those
// is「this paragraph is a tip」. The word is set in the title weight; the
// rule down the left is the quote's, because it is still a passage set
// apart.
type Hast = {
  type: string;
  tagName?: string;
  value?: string;
  properties?: Record<string, unknown>;
  children?: Hast[];
  position?: unknown;
};

export type AlertKind = "note" | "tip" | "important" | "warning" | "caution";

const MARK = /^\s*\[!(note|tip|important|warning|caution)\][ \t]*(\r?\n)?/i;

export function rehypeAlerts(titles: Record<AlertKind, string>) {
  const walk = (node: Hast) => {
    for (const kid of node.children ?? []) {
      if (kid.type !== "element") continue;
      if (kid.tagName === "blockquote") alert(kid);
      walk(kid);
    }
  };
  const alert = (quote: Hast) => {
    const first = quote.children?.find((k) => k.type === "element");
    if (first?.tagName !== "p") return;
    const text = first.children?.[0];
    if (text?.type !== "text" || text.value === undefined) return;
    const m = MARK.exec(text.value);
    if (m === null) return;
    // GitHub's rule: the marker is the whole of its line.
    const rest = text.value.slice(m[0].length);
    if (m[2] === undefined && rest !== "") return;
    const kids = first.children ?? [];
    text.value = rest;
    // The line break the conversation's parser turned the marker's
    // newline into goes with it.
    if (rest === "") {
      kids.shift();
      if (kids[0]?.type === "element" && kids[0].tagName === "br") kids.shift();
      if (kids[0]?.type === "text") kids[0].value = kids[0].value?.replace(/^\r?\n/, "");
    }
    const kind = m[1].toLowerCase() as AlertKind;
    const body = (quote.children ?? []).filter((k) => !(k === first && kids.length === 0));
    quote.tagName = "div";
    quote.properties = { ...quote.properties, dataAlert: kind };
    quote.children = [
      { type: "element", tagName: "div", properties: { dataAlertTitle: "" }, children: [{ type: "text", value: titles[kind] }] },
      ...body,
    ];
  };
  return () => (tree: Hast) => walk(tree);
}
