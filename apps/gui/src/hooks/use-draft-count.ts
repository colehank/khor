import {useCallback,useLayoutEffect,useRef,useState} from 'react';
import type {Dispatch,SetStateAction} from 'react';
import {reportDrafts} from '@/lib/drafts';

function useDraftRegistration(initialCount:number) {
  const registration=useRef({owner:Symbol('draft-owner'),active:false,count:initialCount});
  useLayoutEffect(()=>{
    const state=registration.current;
    state.active=true;
    reportDrafts(state.owner,state.count);
    return ()=>{state.active=false;reportDrafts(state.owner,0);};
  },[]);
  return registration.current;
}

/** The buffer's owner reports until that owner is actually unmounted.
 * A private symbol keeps parallel panes and React roots independent. */
export function useDraftCount(count:number) {
  const state=useDraftRegistration(count);
  useLayoutEffect(()=>{
    state.count=count;
    if(state.active)reportDrafts(state.owner,count);
  },[count,state]);
}

/** Quit can run before React commits the editor's next render. */
export function useDraftState<T>(initial:T|(()=>T),countOf:(value:T)=>number):[T,Dispatch<SetStateAction<T>>] {
  const [value,project]=useState(initial);
  const accepted=useRef(value), counter=useRef(countOf);
  counter.current=countOf;
  const state=useDraftRegistration(countOf(value));
  const setValue=useCallback((action:SetStateAction<T>)=>{
    if(!state.active)return;
    const next=typeof action==='function'?(action as (previous:T)=>T)(accepted.current):action;
    if(Object.is(next,accepted.current))return;
    accepted.current=next;
    state.count=counter.current(next);
    reportDrafts(state.owner,state.count);
    project(next);
  },[state]);
  return [value,setValue];
}
