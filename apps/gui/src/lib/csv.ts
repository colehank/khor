// Delimited text as rows of cells (工作区批⑤).
//
// **On this side of the wire, because the thing being drawn is the
// buffer.** The node used to turn a csv into a markdown table and hand
// over the HTML; that answer is about the bytes on the far disk, and
// the preview a person wants while typing is about the text in front of
// them. Moving it here also gave the `csv` crate back — two packages,
// counted off the Cargo.lock diff.
//
// **RFC 4180, and no more than that.** Quoted fields, delimiters and
// newlines inside quotes, `""` for a literal quote, CR LF or LF. What
// it deliberately does not do is guess: no header detection (a first
// line is a header often and data sometimes, and khor cannot tell — the
// node's table said the same), no type inference, no delimiter
// sniffing. The caller passes the separator, which it knows from the
// extension.
//
// A malformed file is still a file to look at: an unterminated quote
// runs to the end of the text and becomes one long cell, which is what
// it looks like, rather than an error over a pane that could have shown
// something.

/** `a,"b,c",d` → `[["a", "b,c", "d"]]`. Rows in order, cells in order.

    A trailing newline does not make an extra row — every text file ends
    with one and a blank last row would be in every table khor draws. A
    blank line in the *middle* is a row with one empty cell, because it
    is one: the file says so. */
export function parseDelimited(text: string, sep: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let cell = "";
  let quoted = false;
  let i = 0;
  const push = () => {
    row.push(cell);
    cell = "";
  };
  const endRow = () => {
    push();
    rows.push(row);
    row = [];
  };
  while (i < text.length) {
    const c = text[i];
    if (quoted) {
      if (c === '"') {
        // `""` inside a quoted field is one quote; a lone one ends it.
        if (text[i + 1] === '"') {
          cell += '"';
          i += 2;
          continue;
        }
        quoted = false;
        i += 1;
        continue;
      }
      cell += c;
      i += 1;
      continue;
    }
    if (c === '"' && cell === "") {
      quoted = true;
      i += 1;
      continue;
    }
    if (c === sep) {
      push();
      i += 1;
      continue;
    }
    if (c === "\r" && text[i + 1] === "\n") {
      endRow();
      i += 2;
      continue;
    }
    if (c === "\n" || c === "\r") {
      endRow();
      i += 1;
      continue;
    }
    cell += c;
    i += 1;
  }
  // Whatever is left is a last row, unless the file simply ended after
  // its final newline.
  if (cell !== "" || row.length > 0) endRow();
  return rows;
}
