// One cadence for the network's roaming and the fixed capability diagrams.
const travel=1700,settle=1300,hold=2000;
export const tourMotion={travel,settle,hold,turn:travel+settle+hold,cycle:(travel+settle+hold)*2} as const;
// With x control points at 1/3 and 2/3, this is the same smoothstep as below.
export const tourEasing='cubic-bezier(.333333,0,.666667,1)';
export const tourProgress=(t:number)=>t*t*(3-2*t);
