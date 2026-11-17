import {execFileSync} from 'node:child_process';
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';

// Optional macOS-only regeneration; ordinary website builds use committed SVGs.
const output=fileURLToPath(new URL('../node_modules/.cache/sf-symbols-preview/',import.meta.url));
const navigation=process.argv.includes('--navigation');
const assets=fileURLToPath(new URL(navigation?'../src/assets/navigation/':'../src/assets/network/',import.meta.url));
mkdirSync(output,{recursive:true});
mkdirSync(assets,{recursive:true});
const symbols=navigation?{docs:'book.closed',download:'arrow.down',appearance:'circle.lefthalf.filled',language:'globe'}:{a:'cable.connector',b:'wifi',c:'cellularbars'};
const renderer=`
ObjC.import('AppKit');
var output=${JSON.stringify(output)};
var symbols=${JSON.stringify(symbols)};
var bounds={};
Object.keys(symbols).forEach(function(id){
  var symbol=$.NSImage.imageWithSystemSymbolNameAccessibilityDescription(symbols[id],'');
  if(!symbol)throw new Error('Missing system symbol: '+symbols[id]);
  symbol=symbol.imageWithSymbolConfiguration($.NSImageSymbolConfiguration.configurationWithPaletteColors($.NSArray.arrayWithObject($.NSColor.whiteColor)));
  var original=symbol.size,scale=${navigation?86:80}/Math.max(original.width,original.height);
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
  if(${navigation}){
    var w=Number(bitmap.pixelsWide),h=Number(bitmap.pixelsHigh),left=w,top=h,right=0,bottom=0;
    for(var y=0;y<h;y++)for(var x=0;x<w;x++)if(Number(bitmap.colorAtXY(x,y).alphaComponent)>.02){left=Math.min(left,x);top=Math.min(top,y);right=Math.max(right,x+1);bottom=Math.max(bottom,y+1);}
    var side=Math.max(right-left,bottom-top)/.9;
    bounds[id]={width:w,height:h,x:(left+right-side)/2,y:(top+bottom-side)/2,side:side};
  }
});
JSON.stringify(bounds);
`;
const bounds=JSON.parse(execFileSync('osascript',['-l','JavaScript'],{input:renderer,encoding:'utf8'}));
const marks=Object.fromEntries(Object.keys(symbols).map(id=>{
  const png=readFileSync(output+id+'.png').toString('base64');
  if(navigation){
    const b=bounds[id];
    return [id,`<svg xmlns="http://www.w3.org/2000/svg" viewBox="${b.x} ${b.y} ${b.side} ${b.side}" fill="currentColor" aria-hidden="true"><defs><mask id="sf-nav-${id}" maskUnits="userSpaceOnUse" x="0" y="0" width="${b.width}" height="${b.height}" style="mask-type:alpha"><image width="${b.width}" height="${b.height}" href="data:image/png;base64,${png}"/></mask></defs><rect width="${b.width}" height="${b.height}" mask="url(#sf-nav-${id})"/></svg>`];
  }
  return [id,`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor"><defs><mask id="sf-${id}" maskUnits="userSpaceOnUse" x="0" y="0" width="24" height="24" style="mask-type:alpha"><image width="24" height="24" href="data:image/png;base64,${png}"/></mask></defs><rect width="24" height="24" mask="url(#sf-${id})"/></svg>`];
}));
for(const [id,name] of Object.entries(navigation?{docs:'docs',download:'download',appearance:'appearance',language:'language'}:{a:'lan',b:'wifi',c:'cellular'}))writeFileSync(assets+name+'.svg',marks[id]+'\n');
console.log(navigation?'Navigation symbol assets regenerated':'Network symbol assets regenerated');
