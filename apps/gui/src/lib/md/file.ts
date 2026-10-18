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
// Two additions to the schema, both this app's own elements: the path
// link `remarkPaths` makes, and nothing else.
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
  attributes: { ...defaultSchema.attributes, "khor-path": ["written"] },
};

export { rehypeRaw, rehypeSanitize, rehypeSlug };
