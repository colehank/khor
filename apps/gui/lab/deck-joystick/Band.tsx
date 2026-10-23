/** Lab controls wrap the production component; no second gesture implementation. */
import {createContext,useContext,type ComponentProps} from 'react';
import {DeckBand as ProductionDeckBand} from '../../src/components/DeckBand';
import {DECK_JOYSTICK_DEFAULTS} from '../../src/lib/deck-joystick';

export const JoystickContext=createContext({onOverview:()=>{},tuning:DECK_JOYSTICK_DEFAULTS});
export function DeckBand(props:ComponentProps<typeof ProductionDeckBand>){
  const {onOverview,tuning}=useContext(JoystickContext);
  return <ProductionDeckBand {...props} onOverview={props.onOverview??onOverview} tuning={tuning}/>;
}
