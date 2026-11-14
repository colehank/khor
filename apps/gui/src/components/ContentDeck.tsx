/** The session content deck's visual slots, without navigation state. */
import type {ComponentProps} from 'react';
import {cn} from '@/lib/utils';

export function ContentDeck({className,...props}:ComponentProps<'div'>){
 return <div data-content-deck className={cn('mobile-content-deck',className)} {...props}/>;
}
export function ContentPill({className,...props}:ComponentProps<'div'>){
 return <div className={cn('glass mobile-content-pill',className)} {...props}/>;
}
/** A tab face is centered in a full pill seat, including room for its halo. */
export function PillFace({className,...props}:ComponentProps<'span'>){
 return <span className={cn('grid size-glass-item-h flex-none place-items-center',className)} {...props}/>;
}
