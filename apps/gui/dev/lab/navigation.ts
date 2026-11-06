import type {TabRow} from '../../src/gen/bindings/TabRow';
import type {WorkspaceRow} from '../../src/gen/bindings/WorkspaceRow';
import type {FileText} from '../../src/gen/bindings/FileText';
import type {RecentRow} from '../../src/gen/bindings/RecentRow';

const root = '/preview/project';
const initialText = '# Khor Lab\n\nThis file is fixture data. The App uses its production editor and Markdown renderer.\n\nInline: \\(a+b\\).\n';
export function navigationFixture(device: string, origin: string, seeded: boolean) {
  let text = initialText, revision = 1, sequence = 0;
  const at = Date.now();
  const recents = new Map<string, RecentRow[]>([['page', seeded ? [{device, name:'验收环境', place:origin+'/lab/example', at_ms:at, session:'gui/ai-elements-review'}] : []]]);
  const tabs = new Map<string, {tabs: TabRow[]; active: string}>();
  const collection = (kind: string) => {
    if (!tabs.has(kind)) tabs.set(kind, {tabs: [], active: ''});
    return tabs.get(kind)!;
  };
  const open = (kind: string, input: Record<string, unknown>) => {
    const state = collection(kind), id = 'lab-tab-' + ++sequence;
    state.tabs.push({id, device: String(input.device || device), url: String(input.url || ''), title: String(input.title || ''),
      pinned: false, session: String(input.session || ''), view: String(input.view || 'workspace'), touched: at,
      ...(typeof input.request === 'string' ? {request: input.request} : {})});
    state.active = id;
    return id;
  };
  if (seeded) {
    for (const [view, url, title] of [['workspace', root, 'project'], ['file', root + '/README.md', 'README.md'], ['page', origin + '/lab/example', 'Lab page']]) {
      open('content', {session: 'gui/ai-elements-review', view, url, title});
    }
  }
  const workspace: WorkspaceRow = {device, machine: '验收环境', path: root, name: 'project', pinned: false, branch: null,
    changed: 0, changed_capped: false, changed_ms: at, git: null, newest: null};
  const file = (): FileText => ({text, total: new TextEncoder().encode(text).length, at_ms: at + revision,
    digest: 'fixture-' + revision, cut: false, binary: false});
  const handles = new Set(['tabs_list','tabs_open','tabs_go','tabs_focus','tabs_close','tabs_pin','tabs_reorder',
    'recents','recent_visit','git_diff','dirs_begin','dirs_wait','dirs_leave','git_history','git_log','workspaces','ls','read_text','stat_text','write_text','preview','touched','checkout','git_status','code_here']);
  return {
    handles: (command: string) => handles.has(command),
    request(command: string, input: Record<string, unknown>): unknown {
      if (command.startsWith('tabs_')) {
        const state = collection(String(input.kind));
        if (command === 'tabs_list') return state;
        if (command === 'tabs_open') return open(String(input.kind), input);
        if (command === 'tabs_reorder') {
          const ids = input.ids;
          if (!Array.isArray(ids) || new Set(ids).size !== ids.length || ids.some(id => !state.tabs.some(t => t.id === id && t.session === input.session))) {
            throw new Error('Invalid fixture tab order');
          }
          const ordered = ids.map(id => state.tabs.find(t => t.id === id)!);
          let cursor = 0;
          state.tabs = state.tabs.map(t => ids.includes(t.id) ? ordered[cursor++] : t);
          return state;
        }
        const tab = state.tabs.find(t => t.id === input.id);
        if (!tab && command !== 'tabs_reorder') throw new Error('Unknown fixture tab');
        if (command === 'tabs_go') Object.assign(tab!, {url: input.url, device: input.device, title: input.title, view: input.view, touched: Date.now()});
        if (command === 'tabs_pin') tab!.pinned = input.on === true;
        if (command === 'tabs_close') {
          if (tab!.pinned) throw new Error('Unpin the fixture tab before closing it');
          state.tabs = state.tabs.filter(t => t !== tab);
        }
        state.active = command === 'tabs_close' ? state.tabs[0]?.id || '' : tab?.id || state.active;
        return state;
      }
      if (command === 'dirs_begin' || command === 'dirs_wait') {
        const batch = {cursor: String(revision), reset: false, dirs: [], workspace: false, coverage: [], retry_ms: 1000};
        return command === 'dirs_begin' ? {id: 'fixture-watch', machine_id: device, batch} : batch;
      }
      if (command === 'dirs_leave') return null;
      if (command === 'recents') return (recents.get(String(input.kind))??[]).slice(0,Number(input.limit)||100);
      if (command === 'recent_visit') {
        const kind=String(input.kind),place=String(input.place);
        recents.set(kind,[{device,name:'验收环境',place,at_ms:Date.now(),session:typeof input.session==='string'?input.session:null},...(recents.get(kind)??[]).filter(row=>row.place!==place)]);
        return null;
      }
      if (command === 'git_diff') return {hunks: [], more: false};
      if (command === 'git_history') return {head: null, commits: [], next: null};
      if (command === 'git_log') return {commits: [], more: false};
      if (command === 'workspaces') return [workspace];
      if (command === 'ls') return {path: String(input.path || root), entries: [
        {name: 'README.md', dir: false, size: file().total, at_ms: at, from: null, received_ms: null},
      ], truncated: false};
      if (command === 'read_text' || command === 'stat_text') return file();
      if (command === 'write_text') {
        if (input.was !== file().digest) throw new Error('Fixture file changed; read it again before saving');
        text = String(input.text);revision++;
        return {digest: file().digest, at_ms: file().at_ms};
      }
      if (command === 'preview') return {kind: 'text', total: file().total, data: '', media: '', html: '', cut: false, empty: !text};
      if (command === 'touched') return at;
      if (command === 'checkout') return false;
      if (command === 'git_status') return {branch: '', rows: [], more: false, ahead: 0, behind: 0, fetched_ms: 0};
      if (command === 'code_here') return {version: '', ready: false};
      throw new Error('Unknown navigation fixture command');
    },
  };
}
