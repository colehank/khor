import type {TabRow} from '../../src/gen/bindings/TabRow';
import type {WorkspaceRow} from '../../src/gen/bindings/WorkspaceRow';
import type {FileText} from '../../src/gen/bindings/FileText';
import type {RecentRow} from '../../src/gen/bindings/RecentRow';
import type {DeviceRow} from '../../src/gen/bindings/DeviceRow';
import type {GitCommitShow} from '../../src/gen/bindings/GitCommitShow';
import {labCommits,labDirectory,labRoot,labText,labWorkspaces} from './world';

/** A valid transport refusal represented by the finite fixture. */
export class FixtureRefusal extends Error {}

const root = '/preview/project';
export function navigationFixture(devices: DeviceRow[], origin: string, seeded: boolean, count=3, page=false) {
  const device=devices[0]?.id??'e'.repeat(64);
  let revision = 1, sequence = 0;
  const documents = new Map<string,FileText>();
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
    const file = ['file', root + '/README.md', 'README.md'], web = ['page', origin + '/lab/example', 'Lab page'];
    const destinations = [['workspace', root, 'project'],...(page?[web,file]:[file,web]),['file',root+'/notes.md','notes.md'],['file',root+'/design.md','design.md']];
    for (const [view, url, title] of destinations.slice(0,count)) {
      open('content', {session: 'gui/ai-elements-review', view, url, title});
    }
  }
  const workspaces: WorkspaceRow[] = labWorkspaces(devices);
  const histories = new Map<string,ReturnType<typeof labCommits>>();
  const history = (path: string) => {if(!histories.has(path))histories.set(path,labCommits(path));return histories.get(path)!;};
  const owner = (input: Record<string,unknown>) => devices.find(device=>device.id===input.machine||device.name===input.machine)??devices[0];
  const repository = (input: Record<string,unknown>) => {
    const path=String(input.dir||root).replace(/\\/g,'/').replace(/\/$/,'');
    return workspaces.find(workspace=>workspace.device===owner(input)?.id&&workspace.git&&(path===workspace.path||path.startsWith(workspace.path+'/')));
  };
  const requireRepository = (input: Record<string,unknown>) => {
    const workspace=repository(input);
    if(!workspace)throw new FixtureRefusal('No Git repository: '+String(input.dir||root));
    return workspace;
  };
  const pathOf = (input: Record<string,unknown>) => String(input.path||labRoot(owner(input))).replace(/\\/g,'/').replace(/\/$/,'');
  const relative = (input: Record<string,unknown>) => {
    const path=pathOf(input),base=labRoot(owner(input));
    if(path===base)return '';
    if(!path.startsWith(base+'/'))throw new Error('Path is outside the generated Lab filesystem: '+path);
    return path.slice(base.length+1);
  };
  const documentKey = (input: Record<string,unknown>) => (owner(input)?.id??device)+':'+pathOf(input);
  // Mutable copies of the finite generated snapshot. This is a frontend Lab
  // transport, never a filesystem implementation or evidence of OS rename.
  const directories = new Map<string,ReturnType<typeof labDirectory>>();
  const originals = new Map<string,string>();
  const seededDevices = new Set<string>();
  const seedFiles = (input: Record<string,unknown>) => {
    const machine = owner(input), id = machine?.id??device, base = labRoot(machine);
    if (seededDevices.has(id)) return;
    const walk = (relative: string) => {
      const path = base + (relative ? '/'+relative : '');
      const listing = labDirectory(relative);
      directories.set(id+':'+path,{...listing,entries:listing.entries.map(entry=>({...entry}))});
      for (const entry of listing.entries) {
        const child = (relative ? relative+'/' : '')+entry.name;
        if (entry.dir) walk(child);
        else originals.set(id+':'+base+'/'+child,child);
      }
    };
    walk('');
    seededDevices.add(id);
  };
  const directory = (input: Record<string,unknown>) => {
    relative(input);
    seedFiles(input);
    const listing = directories.get(documentKey(input));
    if (!listing) throw new FixtureRefusal('No fixture directory: '+pathOf(input));
    return listing;
  };
  const file = (input: Record<string,unknown> = {}): FileText => {
    relative(input);
    seedFiles(input);
    const key=documentKey(input), original=originals.get(key);
    if (original===undefined) throw new Error('No fixture file: '+pathOf(input));
    if (!documents.has(key)) {
      const text=labText(original);
      documents.set(key,{text,total:new TextEncoder().encode(text).length,at_ms:at,digest:'fixture-'+revision,cut:false,binary:false});
    }
    return documents.get(key)!;
  };
  const handles = new Set(['tabs_list','tabs_open','tabs_go','tabs_focus','tabs_close','tabs_pin','tabs_reorder',
    'recents','recent_visit','git_diff','dirs_begin','dirs_wait','dirs_leave','git_history','git_log','git_commit','git_blame','git_show','workspaces','ls','mkdir','rename_path','read_text','stat_text','write_text','preview','touched','checkout','git_status','code_here','trash_here','git_windows','code_windows']);
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
      if (command === 'git_history') {
        requireRepository(input);
        const commits=history(String(input.dir||root)),start=Number(input.cursor||0),limit=Number(input.limit)||40;
        return {head:commits[0].id,commits:commits.slice(start,start+limit),next:start+limit<commits.length ? String(start+limit) : null};
      }
      if (command === 'git_log') return {commits:history(String(input.path||root)).slice(0,20).map(commit=>({...commit,body:commit.subject})),more:false};
      if (command === 'git_blame') return {commits:[],runs:[]};
      if (command === 'git_show') return {text:file(input).text,cut:false,diff:{hunks:[],more:false}};
      if (command === 'git_commit') {
        const commits=history(String(input.dir||root)),commit=commits.find(commit=>commit.id===input.rev)??commits[0];
        return {commit:{...commit,body:commit.subject+'\n\n完整 App Lab 的提交示例。\n\n保留中文、长路径、文件折叠与阅读位置。'},total:2,more:false,files:[
          {path:'src/navigation.ts',from:'',status:'M',added:90,removed:90,binary:false,big:false,capped:false,
            before:Array.from({length:90},(_,i)=>`const item${i} = "before";`).join('\n'),after:Array.from({length:90},(_,i)=>`const item${i} = "after";`).join('\n')},
          {path:'docs/中文说明.md',from:'',status:'A',added:3,removed:0,binary:false,big:false,capped:false,before:'',after:'# 导航说明\n\n轻推预选，松手确认。\n'},
        ]} satisfies GitCommitShow;
      }
      if (command === 'workspaces') return workspaces;
      if (command === 'ls') return {path:pathOf(input),...directory(input)};
      if (command === 'mkdir') {
        const path=pathOf(input);relative({...input,path});
        if(path.split('/').some(part=>part==='.'||part==='..'))throw new Error('Invalid fixture path');
        const split=path.lastIndexOf('/'),parent=path.slice(0,split),name=path.slice(split+1);
        const listing=directory({...input,path:parent}),key=documentKey({...input,path});
        if(directories.has(key)||listing.entries.some(entry=>entry.name===name))throw new Error('Fixture name already exists: '+path);
        listing.entries.push({name,dir:true,size:0,at_ms:at,from:null,received_ms:null});
        listing.entries.sort((a,b)=>Number(a.name.startsWith('.'))-Number(b.name.startsWith('.'))||Number(b.dir)-Number(a.dir)||(a.name.toLowerCase()<b.name.toLowerCase()?-1:a.name.toLowerCase()>b.name.toLowerCase()?1:0));
        directories.set(key,{entries:[],truncated:false});revision++;return null;
      }
      if (command === 'rename_path') {
        const from=pathOf({...input,path:input.from}),to=pathOf({...input,path:input.to});
        if (from===to||to.startsWith(from+'/')||[from,to].some(path=>path.split('/').some(part=>part==='.'||part==='..'))) {
          throw new Error('Invalid fixture move');
        }
        relative({...input,path:from});relative({...input,path:to});
        const parent=(path:string)=>path.slice(0,path.lastIndexOf('/'));
        const name=(path:string)=>path.slice(path.lastIndexOf('/')+1);
        const source=directory({...input,path:parent(from)}),target=directory({...input,path:parent(to)});
        const entry=source.entries.find(entry=>entry.name===name(from));
        if (!entry) throw new Error('No fixture entry: '+from);
        if (target.entries.some(entry=>entry.name===name(to))) throw new Error('Fixture name already exists: '+to);
        const prefix=documentKey({...input,path:from}),next=documentKey({...input,path:to});
        const relocate=<T,>(map:Map<string,T>)=>{for(const [key,value] of [...map])if(key===prefix||key.startsWith(prefix+'/')){map.delete(key);map.set(next+key.slice(prefix.length),value);}};
        source.entries=source.entries.filter(row=>row!==entry);
        target.entries.push({...entry,name:name(to)});
        target.entries.sort((a,b)=>Number(b.dir)-Number(a.dir)||a.name.localeCompare(b.name));
        relocate(directories);relocate(originals);relocate(documents);
        revision++;
        return null;
      }
      if (command === 'read_text' || command === 'stat_text') return file(input);
      if (command === 'write_text') {
        if (input.was !== file(input).digest) throw new Error('Fixture file changed; read it again before saving');
        const text=String(input.text);revision++;
        const next={...file(input),text,total:new TextEncoder().encode(text).length,digest:'fixture-'+revision,at_ms:at+revision};
        documents.set(documentKey(input),next);
        return {digest:next.digest,at_ms:next.at_ms};
      }
      if (command === 'preview') return {kind:'text',total:file(input).total,data:'',media:'',html:'',cut:false,empty:!file(input).text};
      if (command === 'touched') return at;
      if (command === 'checkout') return !!repository(input);
      if (command === 'git_status') return {branch:requireRepository(input).branch,rows:[{path:'README.md',mark:'M'},{path:'src/navigation.ts',mark:'M'},{path:'docs/中文说明.md',mark:'A'},{path:'notes.md',mark:'?'}],more:false,ahead:2,behind:1,fetched_ms:at};
      if (command === 'trash_here') return false;
      if (command === 'code_here') return {version: '', ready: false};
      if (command === 'git_windows' || command === 'code_windows') return [];
      throw new Error('Unknown navigation fixture command');
    },
  };
}
