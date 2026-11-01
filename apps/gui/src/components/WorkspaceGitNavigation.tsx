import { GlassCapsule, GlassItem } from '@/components/Glass';
import { IconBranch, IconFileTray, IconHistory } from '@/components/icons';
import { gui } from '@/gen/catalog';
import '@/styles/workspace-navigation.css';

export function WorkspaceGitNavigation({branch,repo,history,onHistory,seat='workspace'}:{
  branch:string;repo:boolean;history:boolean;onHistory:(history:boolean)=>void;seat?:'workspace'|'session';
}) {
  if(!repo)return null;
  return <div data-workspace-navigation data-seat={seat} className="min-h-12 pb-1.5">
    <span data-workspace-branch-icon><IconBranch/></span>
    <span data-workspace-branch title={branch}>{branch}</span>
    <GlassCapsule role="group" aria-label={gui.workspaces_tab}>
      <GlassItem aria-label={gui.workspace_tree} aria-pressed={!history} on={!history} onClick={()=>onHistory(false)}><IconFileTray/></GlassItem>
      <GlassItem aria-label={gui.commit_history} aria-pressed={history} on={history} onClick={()=>onHistory(true)}><IconHistory/></GlassItem>
    </GlassCapsule>
  </div>;
}
