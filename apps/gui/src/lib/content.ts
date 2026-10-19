import type { ContentTab, ContentView } from '@/hooks/use-session-content';
import { gui } from '@/gen/catalog';
export const contentView = (tab: ContentTab): ContentView => tab.view || 'workspace';
export const contentName = (tab: ContentTab) => tab.title || (contentView(tab)==='workspace' ? tab.url.split(/[\\/]/).filter(Boolean).pop() || gui.content_new : contentView(tab)==='page' ? tab.url : tab.url.split(/[\\/]/).pop() || tab.url);

/** A rendered choice and its action share one identity snapshot. The live
 * handle resolver may change during an async node/optimistic-ID handoff. */
export function contentTabTargets(tabs:ContentTab[],handle:(tab:ContentTab)=>string):Map<string,ContentTab> {
  return new Map(tabs.map(tab=>[handle(tab),tab]));
}
