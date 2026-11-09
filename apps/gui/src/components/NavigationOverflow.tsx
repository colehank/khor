import type {ReactElement} from "react";
import type {NavSeat} from "@/components/FloatNav";
import {IconMore,IconNetwork} from "@/components/icons";
import {DropdownMenu,DropdownMenuContent,DropdownMenuItem,DropdownMenuTrigger} from "@/components/ui/dropdown-menu";
import {gui} from "@/gen/catalog";

/** Hidden landings and More are peers; each opens its own navigation. */
export function NavigationOverflow({seats,onGo,onMore,onMe,mobile=false,children}: {
  seats:NavSeat[];onGo:(key:string)=>void;onMore:()=>void;onMe:()=>void;mobile?:boolean;children:ReactElement;
}) {
  return <DropdownMenu>
    <DropdownMenuTrigger asChild>{children}</DropdownMenuTrigger>
    <DropdownMenuContent data-nav-menu align="end" side={mobile ? "top" : "bottom"}
      sideOffset={8} className="min-w-nav-menu" onCloseAutoFocus={event=>event.preventDefault()}>
      <DropdownMenuItem data-nav-me className="[&_svg]:size-nav-glyph" onSelect={onMe}>
        <IconNetwork/>{gui.my_khor}
      </DropdownMenuItem>
      {seats.map(seat=><DropdownMenuItem key={seat.key} data-more-landing={seat.key}
        className="[&_svg]:size-nav-glyph" onSelect={()=>onGo(seat.key)}>
        {seat.glyph}<span className="flex-1">{seat.name}</span>
        {!!seat.badge && <span className="text-caption tabular-nums text-muted-foreground">{seat.badge}</span>}
      </DropdownMenuItem>)}
      <DropdownMenuItem data-nav-more-page className="[&_svg]:size-nav-glyph" onSelect={onMore}>
        <IconMore/>{gui.rail_more}
      </DropdownMenuItem>
    </DropdownMenuContent>
  </DropdownMenu>;
}
