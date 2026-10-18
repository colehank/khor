// Markdown to React, with the processor made once (lane md-render).
//
// **Why not `react-markdown` any more.** It builds a fresh unified
// processor on every render — every plugin's attacher runs again, gfm
// builds its parser extensions again. For one text that is nothing; for a
// turn drawn as a few dozen blocks it is a few dozen processors made and
// thrown away per draw. Here a processor is made and frozen once per face
// (`Markdown`'s `kitOf`) and every block runs through the same one. What
// `react-markdown` did after the parse is kept as it was, line for line,
// because each line is a safety:
//
// - raw HTML that no plugin turned into elements becomes its own text
//   (the conversation never parses it; on the file page `rehype-raw` has
//   already turned the allowed part into elements);
// - every address-bearing attribute goes through `safeUrl`, which drops
//   any scheme but the web's, mail's and chat's (`javascript:` above all).
import { toJsxRuntime, type Components } from "hast-util-to-jsx-runtime";
import { urlAttributes } from "html-url-attributes";
import type { ReactNode } from "react";
import { Fragment, jsx, jsxs } from "react/jsx-runtime";
import remarkParse from "remark-parse";
import remarkRehype, { type Options as Bridge } from "remark-rehype";
import { unified, type PluggableList } from "unified";
import { visit } from "unist-util-visit";

export type { Bridge, Components, PluggableList };

/** A frozen remark → rehype processor: parse and run, never configured
    again. */
export type Pipeline = ReturnType<typeof pipeline>;

export function pipeline(remark: PluggableList, bridge: Bridge, rehype: PluggableList) {
  return unified()
    .use(remarkParse)
    .use(remark)
    .use(remarkRehype, { ...bridge, allowDangerousHtml: true })
    .use(rehype)
    .freeze();
}

type Hast = {
  type: string;
  tagName?: string;
  value?: string;
  properties?: Record<string, unknown>;
  children?: Hast[];
};

const SAFE = /^(https?|ircs?|mailto|xmpp)$/i;

/** `react-markdown`'s `defaultUrlTransform`: an address with no scheme is
    relative and kept; one with a scheme is kept only for those above. */
export function safeUrl(value: string): string {
  const colon = value.indexOf(":");
  const question = value.indexOf("?");
  const hash = value.indexOf("#");
  const slash = value.indexOf("/");
  if (
    colon === -1 ||
    (slash !== -1 && colon > slash) ||
    (question !== -1 && colon > question) ||
    (hash !== -1 && colon > hash) ||
    SAFE.test(value.slice(0, colon))
  ) {
    return value;
  }
  return "";
}

export function render(p: Pipeline, text: string, components: Components): ReactNode {
  const tree = p.runSync(p.parse(text), text) as unknown as Hast;
  visit(tree as never, (node: Hast, index: number | undefined, parent: Hast | undefined) => {
    if (node.type === "raw" && parent?.children && typeof index === "number") {
      parent.children[index] = { type: "text", value: node.value ?? "" };
      return index;
    }
    if (node.type === "element" && node.properties) {
      for (const key in urlAttributes) {
        if (!Object.hasOwn(urlAttributes, key) || !Object.hasOwn(node.properties, key)) continue;
        const only = urlAttributes[key];
        if (only === null || only.includes(node.tagName ?? "")) {
          node.properties[key] = safeUrl(String(node.properties[key] || ""));
        }
      }
    }
    return undefined;
  });
  return toJsxRuntime(tree as never, {
    Fragment,
    components,
    ignoreInvalidStyle: true,
    jsx,
    jsxs,
    passKeys: true,
    passNode: true,
  });
}
