// The long reply `lab/md.tsx` streams, on its own so a script can lex
// the same bytes outside the browser.
/** ~50 000 characters of what a long answer looks like: prose that is
    mostly Chinese, lists, a table, and a code block every few paragraphs
    in the languages agents write most. Deterministic, so two builds are
    measured on the same bytes. */
export function longReply(): string {
  const para = (i: number) =>
    `第 ${i} 段：这一段在说明一个改动为什么要这样做，**先量再修**，其中提到 \`openFenceAt\` 和 ` +
    `[一个链接](https://example.com/${i})，还有一句 *斜体的补充*。这里继续写一些中文，让行能折起来，` +
    `读起来像一段真正的回复：改动影响到的文件、要验证的地方、下一步要做什么，以及一个不太确定的判断。`;
  const code = (i: number) => {
    const langs = ["ts", "rust", "python", "bash"];
    const lang = langs[i % langs.length];
    const body = Array.from({ length: 18 }, (_, k) =>
      lang === "rust"
        ? `    let v${k} = items.iter().filter(|x| x.id > ${k}).count(); // ${i}`
        : lang === "python"
          ? `    v${k} = [x for x in items if x.id > ${k}]  # ${i}`
          : lang === "bash"
            ? `grep -rn "pattern${k}" src | head -${k + 1}  # ${i}`
            : `  const v${k} = items.filter((x) => x.id > ${k}).length; // ${i}`,
    ).join("\n");
    return "```" + lang + "\n" + body + "\n```";
  };
  const out: string[] = ["# 一份很长的回复", ""];
  let i = 0;
  while (out.join("\n").length < 50_000) {
    i += 1;
    out.push(`## 第 ${i} 节`, "", para(i), "", para(i + 1000), "");
    if (i % 2 === 0) out.push(code(i), "");
    if (i % 3 === 0) out.push(`- 第一项 ${i}`, `- 第二项 **重点**`, `  - 嵌套 \`x\``, `- 第三项`, "");
    if (i % 5 === 0) out.push("| 列 | 值 | 说明 |", "|:--|--:|:-:|", `| a | ${i} | 左对齐 |`, `| b | ${i * 2} | 右对齐 |`, "");
    if (i % 7 === 0) out.push(`> 引用一句话 ${i}`, "");
  }
  return out.join("\n");
}

