/** The Khor Pi adapter's public state uses ACP v1 shapes throughout. */
import {getSupportedThinkingLevels,type Model,type Api} from '@earendil-works/pi-ai';
import type {AgentSession} from '@earendil-works/pi-coding-agent';

export type SessionOptionValue=string|boolean;
export type SessionOptions=Record<string,SessionOptionValue>;
export function configOptions(model:Model<Api>,models:readonly Model<Api>[],options:SessionOptions={},metadataKnown=true){
  const levels=getSupportedThinkingLevels(model);
  const requested=typeof options.thought_level==='string'?options.thought_level:undefined;
  const thinking=requested&&levels.some(v=>v===requested)?requested:levels.includes('medium')?'medium':levels.includes('off')?'off':levels[0];
  const choices=[...models];if(!choices.some(v=>v.id===model.id))choices.unshift(model);
  return [
    {id:'model',name:'Model',category:'model',type:'select',currentValue:model.id,options:choices.map(v=>({value:v.id,name:v.name}))},
    ...(metadataKnown&&model.reasoning&&levels.length?[{id:'thought_level',name:'Thinking',category:'thought_level',type:'select',currentValue:thinking,options:levels.map(value=>({value,name:value}))}]:[]),
    {id:'auto_compaction',name:'Auto compaction',category:'model_config',type:'boolean',currentValue:options.auto_compaction!==false},
  ];
}
export function applySessionOptions(session:AgentSession,options:SessionOptions={}){
  if(options.thought_level!==undefined){
    const level=session.getAvailableThinkingLevels().find(v=>v===options.thought_level);
    if(!level)throw new Error('Thinking level is not supported by the selected model');
    session.setThinkingLevel(level);
  }
  if(options.auto_compaction!==undefined){
    if(typeof options.auto_compaction!=='boolean')throw new Error('Auto compaction must be a boolean');
    session.setAutoCompactionEnabled(options.auto_compaction);
  }
}
/** A context reading is a snapshot, never cumulative billing tokens.
 * Pi explicitly returns null after compaction until a trustworthy count exists.
 * An unlisted gateway model has guessed metadata, so do not publish its limit. */
export function usageUpdate(session:Pick<AgentSession,'getContextUsage'|'getSessionStats'>,knownWindow=true){
  if(!knownWindow)return null;
  const usage=session.getContextUsage();
  if(!usage||usage.tokens===null||!Number.isFinite(usage.tokens)||!Number.isFinite(usage.contextWindow)||usage.contextWindow<=0)return null;
  const cost=session.getSessionStats().cost;
  return {sessionUpdate:'usage_update',used:Math.max(0,Math.round(usage.tokens)),size:usage.contextWindow,
    ...(Number.isFinite(cost)&&cost>=0?{cost:{amount:cost,currency:'USD'}}:{})};
}
