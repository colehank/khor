import type {ComponentType, ReactNode} from "react";
import {NetworkMark} from "@/components/NetworkMark";
import {IconAppearance, IconChevronRight, IconFiles, IconGear, IconNetwork, IconPulse, IconRecent, IconSessions, IconShield, IconStore, IconTerminal} from "@/components/icons";
import {NarrowHead,SideSeat} from "@/components/SideHead";
import type {NavSeat} from "@/components/FloatNav";
import {settingsPages} from "@/lib/settings-navigation";
import type {MorePage} from "@/lib/more-navigation";
import {walkList} from "@/lib/shortcuts";
import {cn} from "@/lib/utils";
import {gui} from "@/gen/catalog";
import {moreName} from "./MorePage";

const glyphs:Record<Exclude<MorePage,"me">,ComponentType<{className?:string}>> = {
  appearance:IconAppearance,terminal:IconTerminal,session:IconSessions,workspace:IconFiles,
  connection:IconNetwork,security:IconShield,about:IconGear,
  store:IconStore,notifications:IconRecent,doctor:IconPulse,
};
const utilities = ["store","notifications","doctor"] as const;

/** The same grouped navigation stays beside every More page. */
export function MoreNavigation({page,narrow,overflow,unread,onPage,onLanding}: {
  page:MorePage|null;narrow:boolean;overflow:NavSeat[];unread:number;
  onPage:(page:MorePage)=>void;onLanding:(key:string)=>void;
}) {
  const row = (key:MorePage) => {
    const Glyph = key === "me" ? null : glyphs[key];
    return <button key={key} type="button" data-setting-row="nav" data-more-open={key}
      data-more-me={key === "me" ? "" : undefined} data-on={page === key}
      aria-current={page === key ? "page" : undefined} tabIndex={page === key ? 0 : -1}
      onClick={()=>onPage(key)}
      className={cn("flex min-h-touch min-w-0 items-center gap-row text-start focus-visible:-outline-offset-2",
        narrow ? "px-row text-body" : "mx-in rounded-md px-3 text-headline",
        key === "me" && "min-h-row-2",page === key ? "bg-fill-strong" : "hover:bg-fill")}
    >
      <span data-more-glyph={key} className="flex w-list-face flex-none items-center justify-center text-rail-ink [&>svg]:size-nav-glyph">
        {Glyph ? <Glyph/> : <NetworkMark/>}
      </span>
      <span className="flex min-w-0 flex-1 flex-col py-1">
        <span className="truncate">{moreName(key)}</span>
        {key === "me" && <span className="truncate text-callout text-muted-foreground">{gui.khor_network}</span>}
      </span>
      {key === "notifications" && unread > 0 && <span className="text-caption tabular-nums text-muted-foreground">{unread}</span>}
      {narrow && <IconChevronRight className="size-affordance flex-none text-tertiary"/>}
    </button>;
  };
  const group = (children:ReactNode) => <div data-more-card data-settings-card={narrow ? "" : undefined}
    className={cn("flex min-w-0 flex-col",narrow ? "mx-page" : "gap-px")}>{children}</div>;
  return <nav data-more-list aria-label={gui.rail_more}
    onKeyDown={event=>walkList(event,"[data-more-open], [data-more-landing]")}
    className={cn("flex min-h-0 flex-1 flex-col overflow-y-auto bg-side py-1",narrow && "pb-[var(--nav-room)]")}>
    {narrow ? <NarrowHead>{()=><h1 data-list-title className="min-w-0 flex-1 truncate text-title-lg">{gui.rail_more}</h1>}</NarrowHead>
      : <SideSeat><h1 data-list-title className="min-w-0 flex-1 truncate text-title-lg">{gui.rail_more}</h1></SideSeat>}
    {group(row("me"))}
    {overflow.length > 0 && <div className="mt-in">{group(overflow.map(seat=><button type="button" key={seat.key}
      data-setting-row="nav" data-more-landing={seat.key} tabIndex={-1} onClick={()=>onLanding(seat.key)}
      className={cn("flex min-h-touch items-center gap-row text-start hover:bg-fill focus-visible:-outline-offset-2",narrow ? "px-row text-body" : "mx-in rounded-md px-3 text-headline")}>
      <span className="flex w-list-face flex-none items-center justify-center text-rail-ink [&>svg]:size-nav-glyph">{seat.glyph}</span>
      <span className="min-w-0 flex-1 truncate">{seat.name}</span>
      {narrow && <IconChevronRight className="size-affordance flex-none text-tertiary"/>}
    </button>))}</div>}
    <section data-settings-navigation data-more-group="settings" aria-label={gui.nav_settings} className="mt-group">
      <h2 className="px-row pb-in text-caption text-muted-foreground">{gui.nav_settings}</h2>
      {group(settingsPages.map(row))}
    </section>
    <div data-more-utilities className="mt-group mb-pane">{group(utilities.map(row))}</div>
  </nav>;
}
