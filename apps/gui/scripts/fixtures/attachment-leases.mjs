// Transport-only lease validation. It models no sockets, shared resources or queues.
export function installAttachmentLeases() {
  const leases = new Map();
  let serial = 0;
  const kinds = new Map([
    ...['open','poll','wait','say','answer','respond','control','set_mode','set_option','stop','replay','put','put_paths','leave'].map(name => [`chat_${name}`, 'chat']),
    ...['open','attach','stream','wait','key','resize','ready','forget','prompt','scroll','pane_history','drop','drop_items','drop_progress','drop_cancel','leave'].map(name => [`term_${name}`, 'terminal']),
  ]);
  window.__attachmentLeases = (command, args) => {
    if (command === 'attachment_begin') {
      if (typeof args.id !== 'string' || !args.id || !['chat','terminal'].includes(args.kind)) throw new Error('invalid attachment Begin');
      const key = (++serial).toString(16).padStart(32,'0');
      leases.set(key,{id:args.id,kind:args.kind,claimed:false});
      return {handled:true,value:key};
    }
    const kind = kinds.get(command);
    if (!kind) return {handled:false};
    const lease = leases.get(args.lease);
    if (!lease || lease.id !== args.id || lease.kind !== kind) throw new Error('attachment fixture scope mismatch');
    if (command.endsWith('_leave')) {
      lease.closed = true;
      return {handled:true,value:null};
    }
    if (lease.closed) throw new Error('attachment fixture is closed');
    if (command.endsWith('_open')) {
      if (lease.claimed) throw new Error('attachment fixture was already claimed');
      lease.claimed = true;
    } else if (!lease.claimed) throw new Error('attachment fixture is not open');
    return {handled:false};
  };
}
