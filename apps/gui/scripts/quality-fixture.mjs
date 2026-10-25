// Isolated data for the production commit page; no repository is modified.
export const qualityCommit = {
  id: '1'.repeat(40), author: 'Khor Lab', when_ms: 1792900800000,
  subject: '滚动与焦点验收', body: '选择不同文件，检查菜单关闭后页面停留的位置。',
};
export const qualityCommitShow = {
  commit: qualityCommit, total: 8, more: false,
  files: Array.from({ length: 8 }, (_, file) => {
    const lines = Array.from({ length: 120 }, (_, line) => `export const value${line} = ${file * 120 + line};`);
    return {
      path: `src/module-${file + 1}.ts`, from: '', status: 'M', added: 3, removed: 3,
      binary: false, big: false, capped: false,
      before: lines.join('\n'),
      after: lines.map((value, line) => [10, 60, 110].includes(line) ? `${value} // updated` : value).join('\n'),
    };
  }),
};
