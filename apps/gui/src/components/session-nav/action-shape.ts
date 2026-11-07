export type ActionCell={x:number;y:number;width:number;height:number;grow:number};
type Point={x:number;y:number};
type ShapeBounds={left:number;top:number;width:number;height:number};
const xy=(p:Point)=>`${Number(p.x.toFixed(3))} ${Number(p.y.toFixed(3))}`;
const offset=(p:Point,d:Point,length:number):Point=>({x:p.x+d.x*length,y:p.y+d.y*length});

/** Cut only the facing cap. The rest of each capsule keeps its original outline. */
function cap(cell:ActionCell,vertical:boolean,facing:number,pull:number,left:number,top:number){
  const u=(vertical?cell.width:cell.height)/2,v=(vertical?cell.height:cell.width)/2,r=Math.min(u,v);
  const flat=u-r,a=Math.min(u*.72,r*.9),delta=Math.max(0,a-flat),edge=Math.sqrt(r*r-delta*delta);
  const facingEdge=v-r+edge,handle=a*.55+pull*.25;
  const vector=(x:number,y:number):Point=>vertical?{x:x*facing,y:y*facing}:{x:y*facing,y:x*facing};
  const at=(x:number,y:number):Point=>{const d=vector(x,y);return {x:cell.x-left+d.x,y:cell.y-top+d.y};};
  const arc=(x:number,y:number)=>`A ${r} ${r} 0 0 ${vertical?1:0} ${xy(at(x,y))}`;
  const line=(x:number,y:number)=>`L ${xy(at(x,y))}`;
  const start=at(-a,facingEdge),end=at(a,facingEdge),tip=at(0,v+pull);
  const incoming=vector(-edge/r,delta/r),outgoing=vector(-edge/r,-delta/r);
  const trace=[a<=flat?line(-flat,v):'',arc(-u,v-r),line(-u,-v+r),arc(-flat,-v),line(flat,-v),arc(u,-v+r),line(u,v-r),arc(a>flat?a:flat,a>flat?facingEdge:v),a<=flat?line(a,v):''].join(' ');
  const tail=`C ${xy(offset(end,incoming,handle))} ${xy(at(a*.3,v+pull))} ${xy(tip)} C ${xy(at(-a*.3,v+pull))} ${xy(offset(start,outgoing,-handle))} ${xy(start)}`;
  return {start,end,tip,trace,tail,incoming,outgoing,handle,a};
}

export function actionShapeBounds(cells:ActionCell[],touch:number):ShapeBounds{
  const pad=touch*.025;
  const left=Math.min(...cells.map(c=>c.x-c.width/2))-pad,top=Math.min(...cells.map(c=>c.y-c.height/2))-pad;
  const width=Math.max(...cells.map(c=>c.x+c.width/2))+pad-left,height=Math.max(...cells.map(c=>c.y+c.height/2))+pad-top;
  return {left,top,width,height};
}
/** One glass silhouette: caps stretch into tips, meet, grow a waist, then split.
 * The shared outline has no overlapping rims or a separately painted connector. */
export function liquidActionShape(overview:ActionCell,newTab:ActionCell|null,together:number,touch:number,space?:ShapeBounds){
  const {left,top,width,height}=space??actionShapeBounds(newTab?[overview,newTab]:[overview],touch);
  const vertical=!newTab||Math.abs(newTab.y-overview.y)>=Math.abs(newTab.x-overview.x);
  const facing=!newTab?1:(vertical?Math.sign(newTab.y-overview.y):Math.sign(newTab.x-overview.x))||1;
  const pull=newTab?touch*.42*together:0;
  const a=cap(overview,vertical,facing,pull,left,top),b=newTab?cap(newTab,vertical,-facing,pull,left,top):null;
  const separate=(c:typeof a)=>`M ${xy(c.start)} ${c.trace} ${c.tail} Z`;
  let path=separate(a),neck=0;
  if(b&&newTab){
    const axis=vertical?{x:0,y:facing}:{x:facing,y:0};
    const overlap=(a.tip.x-b.tip.x)*axis.x+(a.tip.y-b.tip.y)*axis.y;
    neck=Math.min(a.a,b.a)*.42*Math.sqrt(Math.max(0,Math.min(1,overlap/(touch*.45))));
    if(neck>0){
      const middle={x:(a.tip.x+b.tip.x)/2,y:(a.tip.y+b.tip.y)/2};
      const cross=vertical?{x:facing,y:0}:{x:0,y:facing};
      const span=Math.abs((b.start.x-a.end.x)*axis.x+(b.start.y-a.end.y)*axis.y);
      const stem=Math.min(Math.max(touch*.08,span*.22),span*.4);
      const crossAt=(p:Point)=>vertical?p.x:p.y;
      const join=(from:typeof a,to:typeof a,waist:Point,d:Point)=>{
        // Keep cap tangents, but never overshoot the narrower waist. This
        // makes each shoulder curve in once, without an extra lobe or kink.
        const handle=(p:Point,tangent:Point,limit:number)=>Math.min(limit,Math.abs(crossAt(p)-crossAt(waist))*.7/Math.max(.001,Math.abs(crossAt(tangent))));
        const first=offset(from.end,from.incoming,handle(from.end,from.incoming,from.handle));
        const last=offset(to.start,to.outgoing,-handle(to.start,to.outgoing,to.handle));
        return `C ${xy(first)} ${xy(offset(waist,d,-stem))} ${xy(waist)} C ${xy(offset(waist,d,stem))} ${xy(last)} ${xy(to.start)}`;
      };
      path=`M ${xy(a.start)} ${a.trace} ${join(a,b,offset(middle,cross,neck),axis)} ${b.trace} ${join(b,a,offset(middle,cross,-neck),{x:-axis.x,y:-axis.y})} Z`;
    }else path+=' '+separate(b);
  }
  return {path,left,top,width,height,joined:neck>0,neck,pull};
}
