export const scenes = {
  gui: {title: 'GUI 会话', session: 'gui/ai-elements-review', row: 'AI Elements · 组件总览'},
  codex: {title: 'Codex 会话', session: 'gui/codex-review', row: 'Codex · 工具与补丁'},
  terminal: {title: '终端会话', session: 'shell/layout-lab', row: '终端 · 布局试验'},
  stream: {title: '流式消息', session: 'gui/ai-elements-stream', row: 'AI Elements · 流式响应'},
  list: {title: '会话列表', session: null, row: null},
  live: {title: '隔离的真实后端', session: null, row: null},
} as const;
export type Scene = keyof typeof scenes;
export const sessionIds = ['gui/ai-elements-review', 'gui/ai-elements-plan', 'gui/ai-elements-stream', 'gui/codex-review', 'wide/all', 'shell/layout-lab'];
