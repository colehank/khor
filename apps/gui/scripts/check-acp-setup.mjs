import assert from 'node:assert/strict';
import {agentPresentation,agentSetupFailure} from '../src/lib/acp-session.ts';

assert.equal(agentPresentation('codex').name,'Codex');
assert.equal(agentPresentation('claude').native,true);
assert.equal(agentPresentation('codex-acp').name,'Codex（ACP 扩展）');
assert.equal(agentPresentation('claude-acp').native,false);
assert.equal(agentPresentation('custom-acp').name,'custom-acp','custom registrations retain their own names');
for(const reason of ['npm_cache_unavailable','npm_package_unavailable','npm_download_failed','node_unavailable']){
  const message=agentSetupFailure(reason,'transport closed token=private');
  assert.ok(message&&!message.includes('private'));
}
assert.equal(agentSetupFailure(null,'Error: Incoming transport closed: {"method":"initialize"}'),'未能连接智能体，请重试');
assert.equal(agentSetupFailure(null,'Permission denied'),'Permission denied');
assert.equal(agentSetupFailure(null,null),'无法读取智能体能力');
console.log('ACP setup: native/extension presentation, finite startup causes, private stderr isolation and fallback passed');
