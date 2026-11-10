import type {ComponentType, ReactNode} from "react";
import {IconAppearance, IconChevronRight, IconFiles, IconGear, IconNetwork, IconPulse, IconSessions, IconShield, IconStore} from "@/components/icons";
import {NarrowHead,SideSeat} from "@/components/SideHead";
import type {MorePage} from "@/lib/more-navigation";
import {walkList} from "@/lib/shortcuts";
import {cn} from "@/lib/utils";
import {menu} from "@/gen/catalog";
import {moreName} from "./MorePage";

type NavigationPage = Exclude<MorePage,"me"|"terminal">;
const glyphs:Record<NavigationPage,ComponentType<{className?:string}>> = {
  appearance:IconAppearance,session:IconSessions,workspace:IconFiles,
  connection:IconNetwork,security:IconShield,about:IconGear,
  store:IconStore,doctor:IconPulse,
};
const groups = [["session","workspace","appearance"],["connection","security","store","doctor"],["about"]] as const;

/** The same grouped navigation stays beside every More page. */
export function MoreNavigation({page,narrow,onPage}: {
  page:MorePage|null;narrow:boolean;
  onPage:(page:MorePage)=>void;
}) {
  const selected = narrow ? null : page === "terminal" ? "session" : page;
  const tabStop = selected ?? groups[0][0];
  const row = (key:NavigationPage) => {
    const Glyph = glyphs[key];
    return <button key={key} type="button" data-setting-row="nav" data-more-open={key}
      data-on={selected === key}
      aria-current={selected === key ? "page" : undefined} tabIndex={tabStop === key ? 0 : -1}
      onClick={()=>onPage(key)}
      className={cn("flex min-h-touch min-w-0 items-center gap-in text-start focus-visible:-outline-offset-2",
        narrow ? "px-row text-body" : "mx-in rounded-md px-3 text-headline",
        selected === key ? "bg-fill-strong" : "hover:bg-fill")}
    >
      <span data-more-glyph={key} className="flex w-nav-glyph flex-none items-center justify-center text-rail-ink [&>svg]:size-nav-glyph">
        <Glyph/>
      </span>
      <span className="flex min-w-0 flex-1 flex-col py-1">
        <span className="truncate">{moreName(key)}</span>
      </span>
      {narrow && <IconChevronRight className="size-affordance flex-none text-tertiary"/>}
    </button>;
  };
  const group = (children:ReactNode) => <div data-more-card data-settings-card={narrow ? "" : undefined}
    className={cn("flex min-w-0 flex-col",narrow ? "mx-page" : "gap-px")}>{children}</div>;
  return <nav data-more-list aria-label={menu.settings}
    onKeyDown={event=>walkList(event,"[data-more-open]")}
    className={cn("flex min-h-0 flex-1 flex-col overflow-y-auto bg-side py-1",narrow && "pb-[var(--nav-room)]")}>
    {narrow ? <NarrowHead>{()=><h1 data-list-title className="min-w-0 flex-1 truncate text-title-lg">{menu.settings}</h1>}</NarrowHead>
      : <SideSeat><h1 data-list-title className="min-w-0 flex-1 truncate text-title-lg">{menu.settings}</h1></SideSeat>}
    <div data-settings-navigation className="flex min-w-0 flex-col gap-group pb-pane">
      {groups.map((items,index)=><section key={index} data-more-group={index}>{group(items.map(row))}</section>)}
    </div>
  </nav>;
}
