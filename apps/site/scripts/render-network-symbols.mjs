import {execFileSync} from 'node:child_process';
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';

// Optional macOS-only regeneration; ordinary website builds use committed SVGs.
const output=fileURLToPath(new URL('../node_modules/.cache/sf-symbols-preview/',import.meta.url));
const assets=fileURLToPath(new URL('../src/assets/network/',import.meta.url));
mkdirSync(output,{recursive:true});
mkdirSync(assets,{recursive:true});
const symbols={a:'cable.connector',b:'wifi',c:'cellularbars'};
const renderer=`
ObjC.import('AppKit');
var output=${JSON.stringify(output)};
var symbols=${JSON.stringify(symbols)};
Object.keys(symbols).forEach(function(id){
  var symbol=$.NSImage.imageWithSystemSymbolNameAccessibilityDescription(symbols[id],'');
  symbol=symbol.imageWithSymbolConfiguration($.NSImageSymbolConfiguration.configurationWithPaletteColors($.NSArray.arrayWithObject($.NSColor.whiteColor)));
  var original=symbol.size,scale=80/Math.max(original.width,original.height);
  var width=original.width*scale,height=original.height*scale;
  var canvas=$.NSImage.alloc.initWithSize({width:96,height:96});
  canvas.lockFocus;
  $.NSGraphicsContext.currentContext.imageInterpolation=$.NSImageInterpolationHigh;
  symbol.drawInRectFromRectOperationFraction(
    {origin:{x:(96-width)/2,y:(96-height)/2},size:{width:width,height:height}},
    {origin:{x:0,y:0},size:original},$.NSCompositingOperationSourceOver,1);
  canvas.unlockFocus;
  var bitmap=$.NSBitmapImageRep.imageRepWithData(canvas.TIFFRepresentation);
  bitmap.representationUsingTypeProperties($.NSBitmapImageFileTypePNG,$.NSDictionary.dictionary).writeToFileAtomically(output+id+'.png',true);
});
'Rendered local SF Symbols';
`;
execFileSync('osascript',['-l','JavaScript'],{input:renderer,encoding:'utf8'});
const marks=Object.fromEntries(Object.keys(symbols).map(id=>{
  const png=readFileSync(output+id+'.png').toString('base64');
  return [id,`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor"><defs><mask id="sf-${id}" maskUnits="userSpaceOnUse" x="0" y="0" width="24" height="24" style="mask-type:alpha"><image width="24" height="24" href="data:image/png;base64,${png}"/></mask></defs><rect width="24" height="24" mask="url(#sf-${id})"/></svg>`];
}));
for(const [id,name] of Object.entries({a:'lan',b:'wifi',c:'cellular'}))writeFileSync(assets+name+'.svg',marks[id]+'\n');
console.log('Network symbol assets regenerated');
