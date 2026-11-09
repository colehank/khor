import type {ReactNode} from "react";
import {cn} from "@/lib/utils";

/** Settings group titles are peers. Containers align them with their rows. */
export function SectionHeading({children,className}: {children:ReactNode;className?:string}) {
  return <h2 data-settings-heading className={cn("flex-none text-title-sm font-title text-foreground",className)}>{children}</h2>;
}
