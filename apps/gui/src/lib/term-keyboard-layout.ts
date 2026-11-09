import { BASE_KEYS, type BaseKey, type KeyStroke } from "./term-key-model";

export type KeyboardPlatform = "mac" | "pc";
export function keyboardPlatform(platform: string): KeyboardPlatform { return /Mac|iPhone|iPad|iPod/.test(platform)?"mac":"pc"; }
const token=(code:string)=>`{${code}}`;
export const modifierKeys = {ControlLeft:"ctrl",ControlRight:"ctrl",AltLeft:"alt",AltRight:"alt",ShiftLeft:"shift",ShiftRight:"shift"} as const;
export const systemKeys = ["CapsLock","MetaLeft","MetaRight","ContextMenu"] as const;
export function keyboardRows(platform:KeyboardPlatform): string[] {
  return [
    ["Escape",...Array.from({length:12},(_,i)=>`F${i+1}`),"Insert","Delete"],
    ["Backquote",...Array.from({length:10},(_,i)=>`Digit${(i+1)%10}`),"Minus","Equal","Backspace"],
    ["Tab",..."QWERTYUIOP".split("").map(k=>`Key${k}`),"BracketLeft","BracketRight","Backslash"],
    ["CapsLock",..."ASDFGHJKL".split("").map(k=>`Key${k}`),"Semicolon","Quote","Enter"],
    ["ShiftLeft",..."ZXCVBNM".split("").map(k=>`Key${k}`),"Comma","Period","Slash","ShiftRight"],
    platform==="mac" ? ["ControlLeft","AltLeft","MetaLeft","Space","MetaRight","AltRight","ControlRight"] :
      ["ControlLeft","MetaLeft","AltLeft","Space","AltRight","ContextMenu","ControlRight"],
    ["Home","End","PageUp","PageDown","ArrowLeft","ArrowUp","ArrowDown","ArrowRight"],
  ].map(row=>row.map(token).join(" "));
}
const punctuation:Record<string,[string,string]>={Backquote:["`","~"],Minus:["-","_"],Equal:["=","+"],BracketLeft:["[","{"],BracketRight:["]","}"],Backslash:["\\","|"],Semicolon:[";",":"],Quote:["'",'"'],Comma:[",","<"],Period:[".",">"],Slash:["/","?"]};
export function keyboardLabels(platform:KeyboardPlatform,shift:boolean):Record<string,string> {
  const labels:Record<string,string>={Escape:"Esc",Tab:"Tab",Enter:platform==="mac"?"Return":"Enter",Backspace:platform==="mac"?"Delete":"Backspace",Delete:platform==="mac"?"⌦":"Delete",Insert:"Ins",Space:"Space",CapsLock:"Caps Lock",Home:"Home",End:"End",PageUp:"PgUp",PageDown:"PgDn",ArrowLeft:"←",ArrowRight:"→",ArrowUp:"↑",ArrowDown:"↓",MetaLeft:platform==="mac"?"⌘":"Win",MetaRight:"⌘",ContextMenu:"Menu",ControlLeft:platform==="mac"?"⌃":"Ctrl",ControlRight:platform==="mac"?"⌃":"Ctrl",AltLeft:platform==="mac"?"⌥":"Alt",AltRight:platform==="mac"?"⌥":"Alt",ShiftLeft:"Shift",ShiftRight:"Shift"};
  for(const code of BASE_KEYS){
    if(/^Key[A-Z]$/.test(code))labels[code]=code.slice(3);
    else if(/^Digit/.test(code))labels[code]=shift?")!@#$%^&*("[Number(code.slice(5))]:code.slice(5);
    else if(punctuation[code])labels[code]=punctuation[code][shift?1:0];
    else labels[code]??=code;
  }
  return Object.fromEntries(Object.entries(labels).map(([code,label])=>[token(code),label]));
}
export type KeyboardStroke=Omit<KeyStroke,"key">&{key:BaseKey|null};
export const emptyKeyboardStroke=():KeyboardStroke=>({key:null,ctrl:false,alt:false,shift:false});
export function completeKeyboardStroke(stroke:KeyboardStroke):KeyStroke|null {return stroke.key?{...stroke,key:stroke.key}:null;}
export function chooseKeyboardKey(stroke:KeyboardStroke,key:string):KeyboardStroke {
  const code=key.replace(/^\{|\}$/g,"");
  const modifier=modifierKeys[code as keyof typeof modifierKeys];
  if(modifier)return {...stroke,[modifier]:!stroke[modifier]};
  return (BASE_KEYS as readonly string[]).includes(code)?{...stroke,key:stroke.key===code?null:code as BaseKey}:stroke;
}

export function keyboardGroup(key:string|null):"navigation"|"letters"|"symbols"|"function" {
  return !key?"navigation":/^Key[A-Z]$/.test(key)?"letters":/^F\d+$/.test(key)?"function":/^(Digit|Backquote|Minus|Equal|Bracket|Backslash|Semicolon|Quote|Comma|Period|Slash)/.test(key)?"symbols":"navigation";
}
