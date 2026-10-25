/** Provider-owned setup; credential files are account-scoped on the execution home. */
import { ModelRuntime } from "@earendil-works/pi-coding-agent";
import type { AuthPrompt, AuthEvent } from "@earendil-works/pi-ai";
import { openModels, seatCredentials, type AccountSeat } from "./models.ts";
import {configOptions,type SessionOptions} from './session-state.ts';
import {pickModel} from './models.ts';
export interface Management {
  type: "manage"; operation: "models" | "login" | "session_settings" | "session_set_option";
  account: AccountSeat; credentialPath: string; authType?: "api_key" | "oauth";
  deviceId: string;
  model?:string;options?:SessionOptions;
  configId?:string;value?:string|boolean;
}
export async function manage(command: Management, prompt: (p: AuthPrompt) => Promise<string>, notify: (e: AuthEvent) => void, signal?: AbortSignal) {
  const seated = openModels(command.account);
  const runtime = await ModelRuntime.create({ ...(command.account.key ? { credentials: seatCredentials(seated.providerId, command.account) } : { authPath: command.credentialPath }), modelsPath: null, modelsStorePath: command.credentialPath + ".models", refreshOnCreate: false });
  runtime.registerNativeProvider(seated.models.getProvider(seated.providerId)!);
  if (command.operation === "login") {
    if (!command.authType) throw new Error("Missing authentication method");
    const credential = await runtime.login(seated.providerId, command.authType, { prompt, notify, signal }, { agentName: "khor", getDeviceId: () => command.deviceId });
    if (!(await runtime.listCredentials()).some((c) => c.providerId === seated.providerId)) throw new Error("Credential was not saved");
    return { credentialType: credential.type };
  }
  const refreshed = await runtime.refresh({ providers: [seated.providerId], force: true, signal: signal?AbortSignal.any([signal,AbortSignal.timeout(12000)]):AbortSignal.timeout(12000) });
  const models = runtime.getModels(seated.providerId), trouble = refreshed.errors.get(seated.providerId);
  if(command.operation==='session_settings'||command.operation==='session_set_option'){
    if(!command.model)throw new Error('Missing model');
    const options={...command.options};let model=command.model;
    if(command.operation==='session_set_option'){
      if(command.configId==='model'){
        if(typeof command.value!=='string'||!command.value.trim()||command.value.length>256)throw new Error('Invalid model');
        model=command.value.trim();delete options.thought_level;
      }else{
        const selected=models.find(m=>m.id===model)??pickModel(seated,model).model;
        const option=configOptions(selected,models,options,models.some(m=>m.id===model)||pickModel(seated,model).listed).find(o=>o.id===command.configId);
        if(!option)throw new Error('Unsupported session option');
        if(option.type==='boolean'?typeof command.value!=='boolean':!('options' in option&&option.options?.some(o=>o.value===command.value)))throw new Error('Unsupported session option value');
        options[command.configId!]=command.value!;
      }
    }
    const selected=models.find(m=>m.id===model)??pickModel(seated,model).model;
    const config=configOptions(selected,models,options,models.some(m=>m.id===model)||pickModel(seated,model).listed);
    const normalized=Object.fromEntries(config.filter(o=>o.id!=='model').map(o=>[o.id,o.currentValue!]));
    return {model:selected.id,options:normalized,configOptions:config,trouble:trouble?.message??null};
  }
  return { models: models.map((m) => ({ id: m.id, name: m.name, contextWindow: m.contextWindow, reasoning: m.reasoning, image: m.input.includes("image") })), trouble: trouble?.message ?? null };
}
