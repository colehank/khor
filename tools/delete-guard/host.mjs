/**
 * Refuse a tool call before it reaches any executor or sandbox.
 *
 * This is the layer that also runs under full access, where the ACL sandbox is
 * not consulted. It answers one question: may this call name that path?
 */
import fs from 'node:fs';
import path from 'node:path';
import { decide } from './guard.mjs';

function audit(exec, verdict) {
  const home = process.env.DSH_HOME;
  if (!home) return;
  try {
    const file = path.join(home, 'logs', 'delete-guard.jsonl');
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.appendFileSync(file, JSON.stringify({ at: new Date().toISOString(), tool: exec.name, reason: verdict.reason }) + '\n');
  } catch {
    // Logging must never turn a denial into an execution.
  }
}

function argumentsOf(exec) {
  let args = exec ? exec.arguments : undefined;
  if (typeof args === 'string') {
    try {
      args = JSON.parse(args);
    } catch {
      return null;
    }
  }
  return args && typeof args === 'object' ? args : null;
}

export function apply(ctx) {
  ctx.on('tools/pre-execute', async (exec, next) => {
    const args = argumentsOf(exec);
    if (args === null) {
      // An unexpected wire shape must not brick the shell: the ACL sandbox still
      // owns writes, and the record says the guard could not read the call.
      audit(exec, { reason: 'unreadable arguments' });
      return next();
    }
    const verdict = decide(exec.name, args);
    if (verdict.allow) return next();
    audit(exec, verdict);
    return { kind: 'deny', reason: verdict.reason };
  });
}
