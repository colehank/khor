export type NotificationInput = {type:string;key:string;action:"open"|"dismiss";x:number;y:number;pointerId:number;pointerType:string;button:number;isPrimary:boolean;detail:number};
export type NotificationSignal = {h:number;radius:number;dark:boolean} | {input:NotificationInput};
export function notificationDOM(scope: Window, emit:(value:NotificationSignal)=>void): {update(html:string):void;dispose():void};
