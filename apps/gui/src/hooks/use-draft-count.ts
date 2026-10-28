import {useLayoutEffect,useRef} from 'react';
import {reportDrafts} from '@/lib/drafts';

/** The buffer's owner reports until that owner is actually unmounted.
 * A private symbol keeps parallel panes and React roots independent. */
export function useDraftCount(count:number) {
  const owner=useRef(Symbol('draft-owner'));
  useLayoutEffect(()=>{reportDrafts(owner.current,count);},[count]);
  useLayoutEffect(()=>()=>reportDrafts(owner.current,0),[]);
}
