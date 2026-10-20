/** Only the Lab injects persisted tuning; all behavior lives in the production engine. */
import {deckScrubber as productionScrubber,type DeckClock,type DeckScrubState} from '../src/lib/deck-scrub';
import {getTuning} from './deck-tuning';
export * from '../src/lib/deck-scrub';
export {DEFAULT_TUNING,getTuning,setTuning} from './deck-tuning';
export function deckScrubber(count:number,initial:number,changed:(s:DeckScrubState)=>void,commit:(index:number)=>void,clock?:DeckClock){
  return productionScrubber(count,initial,changed,commit,clock,getTuning);
}
