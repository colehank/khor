import {flushSync} from 'react-dom';

/** iOS needs the newly mounted input to take focus in the initiating tap. */
export function focusPageAddress(begin:()=>void, container:Element|null):void {
  flushSync(begin);
  const input = container?.querySelector<HTMLInputElement>('[data-page-address]');
  if (!input || input.closest('[hidden],[inert],[aria-hidden="true"]')) return;
  input.focus({preventScroll:true});
  input.select();
}
