import { GlassCapsule, GlassItem } from '@/components/Glass';
import { IconBranch, IconFileTray, IconHistory, IconSessions } from '@/components/icons';
import { gui } from '@/gen/catalog';
import type { WorkspaceSidebarView } from '@/views/workspace/workspace-memory';
import '@/styles/workspace-navigation.css';

import type { ReactNode } from "react";

export function WorkspaceNavigation({branch,repo,view,onView,withSessions=false,seat='workspace',branchPicker}:{
  branchPicker?:ReactNode;
  branch:string;repo:boolean;view:WorkspaceSidebarView;onView:(view:WorkspaceSidebarView)=>void;withSessions?:boolean;seat?:'workspace'|'session';
}) {
  if(!repo&&!withSessions)return null;
  return <div data-workspace-navigation data-seat={seat} data-view={view} data-with-sessions={withSessions || undefined} className="min-h-12 pb-1.5">
    {!branchPicker&&<span data-workspace-branch-icon aria-hidden>{repo&&<IconBranch/>}</span>}
    <div data-workspace-branch>{branchPicker ?? <span title={branch}>{branch}</span>}</div>
    <GlassCapsule role="group" aria-label={gui.workspaces_tab}>
      <GlassItem data-workspace-view="files" aria-label={gui.workspace_tree} aria-pressed={view==='files'} on={view==='files'} onClick={()=>onView('files')}><IconFileTray/></GlassItem>
      {repo&&<GlassItem data-workspace-view="history" aria-label={gui.commit_history} aria-pressed={view==='history'} on={view==='history'} onClick={()=>onView('history')}><IconHistory/></GlassItem>}
      {withSessions&&<GlassItem data-workspace-view="sessions" aria-label={gui.workspace_sessions} aria-pressed={view==='sessions'} on={view==='sessions'} onClick={()=>onView('sessions')}><IconSessions/></GlassItem>}
    </GlassCapsule>
  </div>;
}
