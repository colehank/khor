import {createRoot} from 'react-dom/client';
import {useLayoutEffect} from 'react';
import {HistorySheet} from '@/views/FileHistory';
import {CommitPage} from '@/views/CommitPage';
import {TooltipProvider} from '@/components/ui/tooltip';
import {Button} from '@/components/ui/button';
import {useRepositoryHistory} from '@/hooks/use-repository-history';
import {useWindowNarrow} from '@/hooks/use-narrow';
import {speakLang} from '@/lang';
import '@/styles/tokens.css';import '@/styles/app.css';import './history-deck.css';
speakLang('zh');
function Lab(){const{historyOpen,setHistoryOpen,commit,setCommit,backToHistory}=useRepositoryHistory('fixture','/repo');const narrow=useWindowNarrow();useLayoutEffect(()=>{document.documentElement.toggleAttribute('data-narrow',narrow);},[narrow]);return <TooltipProvider><main className="history-proof" data-history-proof>{commit?<CommitPage machine="fixture" root="/repo" commit={commit} edgeToEdge onBack={backToHistory}/>:<div className="history-proof-home"><h1>工作区</h1><Button onClick={()=>setHistoryOpen(true)}>提交历史</Button></div>}<HistorySheet open={historyOpen} onOpenChange={setHistoryOpen} machine="fixture" path="/repo" repo onPick={setCommit}/><div className="history-proof-system" aria-hidden="true">系统安全区（模拟）</div></main></TooltipProvider>}
const root=createRoot(document.getElementById('root')!);root.render(<Lab/>);if(import.meta.hot)import.meta.hot.dispose(()=>root.unmount());
