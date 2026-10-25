import json,sys,os
from pathlib import Path
Path(__file__).with_suffix(".pid").write_text(str(os.getpid()))
pending=None;cancelled=False

def send(value): print(json.dumps({'jsonrpc':'2.0',**value}),flush=True)
for line in sys.stdin:
 m=json.loads(line);method=m.get('method');id=m.get('id')
 if method=='initialize':send({'id':id,'result':{'protocolVersion':1,'agentCapabilities':{},'authMethods':[],'agentInfo':{'name':'owned-process-fixture','version':'1'}}})
 elif method=='session/new':
  if 'refuse' in sys.argv:send({'id':id,'error':{'code':-32603,'message':'synthetic open refused'}})
  else:send({'id':id,'result':{'sessionId':'owned-process-fixture-session'}})
 elif method=='session/prompt':
  pending=id;cancelled=False
  send({'id':'synthetic-permission','method':'session/request_permission','params':{'sessionId':'owned-process-fixture-session','toolCall':{'toolCallId':'synthetic-tool'},'options':[{'optionId':'allow','name':'Allow','kind':'allow_once'}]}})
 elif method=='session/cancel':cancelled=True
 elif id=='synthetic-permission' and 'result' in m:
  assert m['result']['outcome']['outcome']=='cancelled', 'permission dismissal did not cross the actual protocol'
  send({'id':pending,'result':{'stopReason':'cancelled' if cancelled else 'end_turn'}});pending=None
 elif id is not None:send({'id':id,'error':{'code':-32601,'message':'synthetic method unavailable'}})
