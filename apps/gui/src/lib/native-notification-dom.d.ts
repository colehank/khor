export type NotificationInput = {type:string;key:string;action:"open"|"dismiss";x:number;y:number;pointerId:number;pointerType:string;button:number;isPrimary:boolean;detail:number};
export type NotificationSignal = {h:number;radius:number;dark:boolean} | {input:NotificationInput};
export type NotificationDOMScope = {
  DOMParser: typeof DOMParser; ResizeObserver: typeof ResizeObserver;
  getComputedStyle: (element:Element)=>CSSStyleDeclaration;
  document: {
    documentElement: HTMLElement;head: ParentNode;body: ParentNode;
    querySelector: (selector:string)=>Element|null;
    addEventListener: (type:string,listener:EventListenerOrEventListenerObject)=>void;
    removeEventListener: (type:string,listener:EventListenerOrEventListenerObject)=>void;
  };
};
export function notificationDOM(scope: NotificationDOMScope, emit:(value:NotificationSignal)=>void): {update(html:string):void;dispose():void};
