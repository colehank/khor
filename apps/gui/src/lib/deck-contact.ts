/** A contact stays with its deck while page-entry motion finishes on a
 * subpixel. Real geometry changes still invalidate the pending selection. */
export type DeckContactLayout=readonly number[];
export function readDeckContactLayout(element:HTMLElement):DeckContactLayout{
  const r=element.getBoundingClientRect(),v=window.visualViewport;
  return [r.x,r.y,r.width,r.height,innerWidth,innerHeight,v?.width??innerWidth,v?.height??innerHeight,v?.offsetLeft??0,v?.offsetTop??0];
}
export function sameDeckContactLayout(before:DeckContactLayout,after:DeckContactLayout){
  return before.length===after.length&&before.every((value,i)=>Number.isFinite(value)&&Number.isFinite(after[i])&&Math.abs(value-after[i])<=1);
}
