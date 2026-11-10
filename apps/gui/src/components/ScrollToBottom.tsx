import type {ComponentProps} from 'react';
import {GlassCircle} from '@/components/Glass';
import {IconArrow} from '@/components/icons';
import {gui} from '@/gen/catalog';
import {cn} from '@/lib/utils';

/** The same floating return control for agent chat, Khor chat and terminal history. */
export function ScrollToBottom({fresh=false,className,...props}:Omit<ComponentProps<typeof GlassCircle>,'children'>&{fresh?:boolean}){
  return <GlassCircle {...props} aria-label={props['aria-label']??(fresh?gui.new_below:gui.to_bottom)} tipSide="top"
    data-scroll-to-bottom data-fresh={fresh} className={cn('absolute bottom-in right-row z-10',className)}>
    <IconArrow to="down"/>
  </GlassCircle>;
}
