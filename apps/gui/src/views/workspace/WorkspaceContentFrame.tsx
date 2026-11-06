import type {ComponentProps} from 'react';
import {Tabs} from '@/browse/Tabs';
import {TabFrontAct} from '@/browse/TabStrip';
import {BarSeat,SessionScreen} from '@/components/SessionBar';

export function WorkspaceContentFrame({narrow,onBack,children,...tabs}:ComponentProps<typeof Tabs>&{narrow:boolean;onBack:()=>void}) {
  if(!narrow)return <Tabs {...tabs}>{children}</Tabs>;
  const front=tabs.faces.find(face=>face.id===tabs.active);
  return <SessionScreen faces={tabs.faces} active={tabs.active} onFocus={tabs.onFocus} onClose={tabs.onClose} onBack={onBack} onTitle={onBack}>
    <BarSeat at="lead"><TabFrontAct face={front} tipSide="top"/></BarSeat>
    <BarSeat at="trail">{tabs.tail}</BarSeat>
    {children}
  </SessionScreen>;
}
