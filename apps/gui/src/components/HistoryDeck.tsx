/** Read-only history keeps one reachable way back and one compact identity. */
import {GlassCircle} from '@/components/Glass';
import {IconBack} from '@/components/icons';
import {gui} from '@/gen/catalog';
export function HistoryDeck({title,subtitle,onBack,commit=false}: {title:string;subtitle:string;onBack:()=>void;commit?:boolean}) {
  return <nav data-history-deck className="history-deck" aria-label={gui.commit_history}>
    <GlassCircle aria-label={gui.back} data-commit-back={commit?'':undefined} data-history-back={!commit?'':undefined} onClick={onBack}><IconBack/></GlassCircle>
    <div data-history-pill className="glass history-pill"><span className="min-w-0 flex-1"><span className="block truncate text-body">{title}</span><span className="block truncate text-caption text-muted-foreground">{subtitle}</span></span></div>
  </nav>;
}
