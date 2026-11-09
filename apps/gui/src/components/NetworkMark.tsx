import {IconNetwork} from "@/components/icons";
import {cn} from "@/lib/utils";

/** A shared network identity, distinct from every member machine's avatar. */
export function NetworkMark({large=false}: {large?:boolean}) {
  return <span data-network-mark aria-hidden="true" className={cn("glass flex flex-none items-center justify-center rounded-full",large ? "size-avatar-xl" : "size-avatar")}>
    <IconNetwork className={large ? "size-avatar" : "size-nav-glyph"}/>
  </span>;
}
