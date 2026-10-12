// **The file's symbols** (cm-d item 5): what a file defines — its
// functions, classes, methods, types, impls, and a document's headings —
// read off the syntax tree, for 「符号」 in the file's 「…」 (GitHub's file
// page has 「Symbols」 at its top right). A press goes to the line.
//
// **Only what the tree marks as a definition.** The lezer grammars name
// their definitions (`FunctionDeclaration`, Rust's `FunctionItem`,
// Python's `FunctionDefinition`, Go's `FunctionDecl`, Markdown's
// `ATXHeading1` …); a legacy stream mode has no tree and so no symbols,
// and the item is not offered there — nothing is guessed from the text.
//
// Walked when the list is asked for, not on every change: the list is a
// dialog, and nothing types into the file while it is up.
import { ensureSyntaxTree, syntaxTree } from "@codemirror/language";
import type { EditorState } from "@codemirror/state";
import type { EditorView } from "@codemirror/view";

export type CodeSymbol = { name: string; line: number; depth: number };

/** The tree's definitions, by node name, across the lezer grammars of
    `@codemirror/language-data` that mark them. */
const DEFS = new Set([
  // JavaScript / TypeScript
  "FunctionDeclaration",
  "ClassDeclaration",
  "MethodDeclaration",
  "InterfaceDeclaration",
  "TypeAliasDeclaration",
  "EnumDeclaration",
  // Rust
  "FunctionItem",
  "StructItem",
  "EnumItem",
  "TraitItem",
  "ImplItem",
  "ModItem",
  // Python
  "FunctionDefinition",
  "ClassDefinition",
  // Go
  "FunctionDecl",
  "MethodDecl",
  "TypeSpec",
]);

/** A definition's name is its first child of one of these. */
const NAMES = new Set(["VariableDefinition", "PropertyDefinition", "TypeDefinition", "BoundIdentifier", "TypeIdentifier", "VariableName", "DefName", "FieldName"]);

const HEADING = /^(ATXHeading[1-6]|SetextHeading[12])$/;

/** How long the list may spend parsing what the editor has not parsed
    yet (the part of a long file never scrolled to). A parse that does
    not finish in it hands back nothing, so the list is then what the
    editor had parsed already. A 50 000-line `.ts` parses from cold in
    about 400 ms (cm-d §5). */
const PARSE_MS = 1000;

/** The symbols of what `view` shows, the whole file parsed first if
    `PARSE_MS` allows — and that tree kept in the view, so folding and
    the next list do not parse it again. */
export function symbolsOfView(view: EditorView): CodeSymbol[] {
  const tree = ensureSyntaxTree(view.state, view.state.doc.length, PARSE_MS);
  if (tree && tree !== syntaxTree(view.state)) view.dispatch({});
  return symbolsOf(view.state);
}

/** Every definition in `state`'s document, in order, each with the
    number of definitions it stands inside (a heading: its level less
    one). Empty for a file whose language marks none (or has no tree). */
export function symbolsOf(state: EditorState): CodeSymbol[] {
  return walk(state, false);
}

/** Whether what the editor has parsed so far holds one definition — for
    「…」, which asks as it opens and must not wait on a parse: the
    first one found ends the walk. */
export function hasSymbols(state: EditorState): boolean {
  return walk(state, true).length > 0;
}

function walk(state: EditorState, one: boolean): CodeSymbol[] {
  const tree = syntaxTree(state);
  const out: CodeSymbol[] = [];
  let depth = 0;
  tree.iterate({
    enter(n) {
      if (one && out.length) return false;
      const heading = HEADING.test(n.name);
      if (!heading && !DEFS.has(n.name)) return;
      let name: string | null = null;
      if (heading) {
        // The heading's own words, its marks left off.
        name = state.doc.lineAt(n.from).text.replace(/^\s{0,3}#+\s*/, "").replace(/\s+#+\s*$/, "").trim();
      } else if (n.name === "ImplItem") {
        // `impl Display for Entry`: everything before the body.
        const body = n.node.getChild("DeclarationList");
        name = state.doc.sliceString(n.from, body ? body.from : n.to).replace(/\s+/g, " ").trim();
      } else {
        for (let c = n.node.firstChild; c; c = c.nextSibling) {
          if (NAMES.has(c.name)) {
            name = state.doc.sliceString(c.from, c.to);
            break;
          }
        }
      }
      const level = heading ? Number(n.name.slice(-1)) - 1 : depth;
      if (name) out.push({ name, line: state.doc.lineAt(n.from).number, depth: level });
      // A heading holds no definitions; a definition's own may nest.
      if (!heading) depth += 1;
    },
    leave(n) {
      if (DEFS.has(n.name)) depth -= 1;
    },
  });
  return out;
}
