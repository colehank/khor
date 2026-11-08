export type DeckIntent = 'keep' | 'switch' | 'handoff' | 'new' | 'overview';
export type DeckPlacement = 'bottom' | 'right';

export type DeckIntentContact = {
  dx: number;
  up: number;
  touch: number;
  placement: DeckPlacement;
  previous: DeckIntent;
  homeDistance?: number;
};

export const DECK_ACTION_REACH = Object.freeze({
  bottom: {neutral: .5, neutralHold: .5, create: 1.8, overview: 4.8},
  right: {neutral: .3, neutralHold: .45, create: 3.2, overview: 5.8},
});
export const DECK_HANDOFF = Object.freeze({entry:1.2,exchange:1.3});
const ease=(value:number)=>{if(value<=1e-5)return 0;if(value>=1-1e-5)return 1;return value*value*(3-2*value);};

/** Forward travel is shared by action recognition and its continuous preview. */
export function deckActionTravel(dx:number,up:number,placement:DeckPlacement){
  const forward=placement==='bottom'?up:(dx+up)*Math.SQRT1_2;
  const sideways=placement==='bottom'?Math.abs(dx):Math.abs(dx-up)*Math.SQRT1_2;
  if(forward<=0)return 0;
  return forward*ease((forward*.7-sideways)/(forward*.6));
}

/** One contact progress owns the material and release destination together. */
export function deckControl(contact:DeckIntentContact,create=true){
  const {dx, up, placement, previous} = contact;
  const touch = Math.max(1, contact.touch);
  const reach = DECK_ACTION_REACH[placement];
  const home = Math.min(Math.hypot(dx, up), contact.homeDistance ?? Infinity);
  const homeActive=home<=touch*(previous==='keep'?reach.neutralHold:reach.neutral);
  const travel=homeActive?0:deckActionTravel(dx,up,placement)/touch;
  const entry=create?ease((travel-reach.create+DECK_HANDOFF.entry)/DECK_HANDOFF.entry):0;
  const transfer=ease((travel-reach.overview+DECK_HANDOFF.exchange)/DECK_HANDOFF.exchange);
  const progress=create?entry:transfer;
  const intent:DeckIntent=homeActive?'keep':progress===0?'switch':progress<1||transfer>0&&transfer<1?'handoff':transfer===1?'overview':'new';
  return {intent,entry,transfer,travel,parked:progress>0,progress};
}
export function deckIntent(contact:DeckIntentContact):DeckIntent{return deckControl(contact).intent;}
