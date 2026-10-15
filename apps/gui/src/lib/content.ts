import type { ContentTab, ContentView } from '@/hooks/use-session-content';
import { gui } from '@/gen/catalog';
export const contentView = (tab: ContentTab): ContentView => tab.view || 'workspace';
export const contentName = (tab: ContentTab) => tab.title || (contentView(tab)==='workspace' ? tab.url.split(/[\\/]/).filter(Boolean).pop() || gui.workspaces_tab : contentView(tab)==='page' ? tab.url : tab.url.split(/[\\/]/).pop() || tab.url);
