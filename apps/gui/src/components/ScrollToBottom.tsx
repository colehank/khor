import type {ComponentProps} from 'react';
import {GlassCircle} from '@/components/Glass';
import {IconArrow} from '@/components/icons';
import {gui} from '@/gen/catalog';
import {cn} from '@/lib/utils';

type ReplyStatus = 'replying' | 'complete';

/** The same floating return control for agent chat, Khor chat and terminal history. */
export function ScrollToBottom({fresh=false,replyStatus,className,...props}:Omit<ComponentProps<typeof GlassCircle>,'children'>&{fresh?:boolean;replyStatus?:ReplyStatus}){
  const label = replyStatus === 'replying' ? gui.chat_bottom_replying
    : replyStatus === 'complete' ? gui.chat_bottom_complete : fresh ? gui.new_below : gui.to_bottom;
  return <GlassCircle {...props} aria-label={props['aria-label'] ?? label} tipSide="top"
    data-scroll-to-bottom data-fresh={fresh} className={cn('absolute bottom-in right-row z-10',className)}>
    <IconArrow to="down"/>
    {replyStatus === 'replying' && <span aria-hidden="true" data-chat-bottom-ring
      className="pointer-events-none absolute -inset-xxs rounded-full border-(length:--badge-stroke) border-transparent border-t-state-busy motion-safe:animate-spin" />}
    {replyStatus === 'complete' && <span aria-hidden="true" data-chat-bottom-dot
      className="pointer-events-none absolute right-0 top-0 size-in rounded-full bg-badge" />}
  </GlassCircle>;
}
