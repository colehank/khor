// The file page's chunk (lane md-render): what a README needs and a
// conversation must never have.
//
// **Raw HTML, through GitHub's own allow-list.** A README is written for
// GitHub, and GitHub renders `<details>`, `<p align=center>`, `<kbd>`,
// `<img width>` — a preview that showed them as source would be a worse
// reading of the file than the one a browser gives. So it is parsed
// (`rehype-raw`) and then cleaned with the schema GitHub publishes
// (`rehype-sanitize`'s default): a `<script>` is stripped with its
// contents, an `on*` attribute or a `javascript:` address never reaches
// the DOM, and that is measured rather than believed
// (`scripts/check-md.mjs`). The conversation does not load this chunk at
// all: its text is a remote agent's, and there it stays text.
//
// Two changes to the schema. One addition: the path link `remarkPaths`
// makes. One narrowing: **a picture's address is `https` or a path, never
// `http`** (the owner's ruling, 2026-10-19, VS Code's Strict default). The
// app's window has no content policy to fall back on, so the rule lives
// here: an `http` picture loses its `src` in the clean and is drawn as its
// alt, and `<source srcset>` — an address the clean would not otherwise
// check — is dropped. A path is read from the file's machine
// (`Markdown`'s `FileImage`), an `https` picture is fetched by this one.
//
// **Heading anchors** (`rehype-slug`, GitHub's slugger) come after the
// clean, so a heading's `id` is the one a `#章节` link in the same file
// was written against; ids the *author* wrote go through the clean and
// come out prefixed `user-content-`, which is GitHub's rule too. The link
// handler looks for both (`Markdown`).
import rehypeRaw from "rehype-raw";
import rehypeSanitize, { defaultSchema } from "rehype-sanitize";
import rehypeSlug from "rehype-slug";

export const schema = {
  ...defaultSchema,
  tagNames: [...(defaultSchema.tagNames ?? []), "khor-path"],
  attributes: { ...defaultSchema.attributes, "khor-path": ["written"], source: [] },
  protocols: { ...defaultSchema.protocols, src: ["https"] },
};

export { rehypeRaw, rehypeSanitize, rehypeSlug };
