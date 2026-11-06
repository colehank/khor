export type DeckIntent = 'keep' | 'switch' | 'new' | 'overview';
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
  bottom: {neutral: .5, neutralHold: .5, create: 1.8, overview: 3.8},
  right: {neutral: .3, neutralHold: .45, create: 3.2, overview: 4.8},
  hysteresis: .3,
});

/** Directions differ; action distances and release semantics belong to one contact. */
export function deckIntent(contact: DeckIntentContact): DeckIntent {
  const {dx, up, placement, previous} = contact;
  const touch = Math.max(1, contact.touch);
  const reach = DECK_ACTION_REACH[placement];
  const home = Math.min(Math.hypot(dx, up), contact.homeDistance ?? Infinity);
  if (home <= touch * (previous === 'keep' ? reach.neutralHold : reach.neutral)) return 'keep';
  const forward = placement === 'bottom' ? up : (dx + up) * Math.SQRT1_2;
  const sideways = placement === 'bottom' ? Math.abs(dx) : Math.abs(dx - up) * Math.SQRT1_2;
  if (forward <= 0 || sideways > forward * .7) return 'switch';
  const retreat = touch * DECK_ACTION_REACH.hysteresis;
  if (forward >= touch * reach.overview - (previous === 'overview' ? retreat : 0)) return 'overview';
  if (forward >= touch * reach.create - (previous === 'new' || previous === 'overview' ? retreat : 0)) return 'new';
  return 'switch';
}
