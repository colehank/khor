/**
 * Decide whether a tool call may touch the path it names.
 *
 * The rule is about where, not which verb: anything that mentions the system
 * drive or a user profile path is refused before the command exists, whatever
 * language it is written in. Destructive commands aimed elsewhere must name a
 * literal path; a variable or glob operand is refused because it is exactly how
 * `Remove-Item $home` deleted a profile.
 */

const C_REFERENCE = /(^|[^A-Za-z0-9_])([Cc]:([\\/]|$)|%USERPROFILE%|%APPDATA%|%LOCALAPPDATA%|%SystemRoot%|\$env:USERPROFILE\b|\$env:APPDATA\b|\$env:LOCALAPPDATA\b|\$env:SystemRoot\b|\$home\b|\$profile\b)/i;

// A read-only prefix is honoured only for a single, unchained command.
const READ_ONLY = [
  /^(Get-ChildItem|Get-Content|Test-Path|Get-Item|Get-ItemProperty|Resolve-Path|Select-String)\b/i,
  /^git\s+-C\s+\S+\s+(status|log|diff|show|rev-parse)\b/i,
];

const CHAINED = /[;|]|&&|\|\||>\s*\S|\n/;

const DESTRUCTIVE = /\b(Remove-Item|Remove-ItemProperty|Clear-Content|Clear-Item|RemoveDirectory|del|erase|rd|rmdir|rm|rmtree|rmSync|unlink|mv|move)\b/i;
const ROBOCOPY = /\brobocopy\b/i;
const ROBOCOPY_PURGE = /\/(mir|purge)\b/i;

const VARIABLE_OR_GLOB = /\$[A-Za-z_{(]|%[A-Za-z_]+%|\*/;
const ABSOLUTE = /(^|[\s"'=])([A-Za-z]:[\\/]|\\\\[^\\/\s]+[\\/])/;
const RELATIVE_ESCAPE = /(^|[\s"'=])\.\.[\\/]/;

const MUTATING_TOOLS = new Set(['pwsh', 'bash']);
const FILE_TOOLS = new Set(['write', 'edit', 'str_replace_editor']);

function readOnlyCommand(command) {
  if (CHAINED.test(command)) return false;
  return READ_ONLY.some((pattern) => pattern.test(command.trim()));
}

function isDestructive(command) {
  if (DESTRUCTIVE.test(command)) return true;
  return ROBOCOPY.test(command) && ROBOCOPY_PURGE.test(command);
}

export function decide(tool, args = {}) {
  if (tool === 'plugin_manager') {
    const action = String(args.action || '');
    if (!action.startsWith('list')) {
      return { allow: false, reason: 'plugin management is reserved for the human; the delete guard cannot be disabled in-session' };
    }
    return { allow: true };
  }

  if (FILE_TOOLS.has(tool)) {
    const target = String(args.file_path || args.path || '');
    if (C_REFERENCE.test(target) || /^[Cc]:/.test(target)) {
      return { allow: false, reason: `writes to the system drive are refused: ${target}` };
    }
    return { allow: true };
  }

  if (!MUTATING_TOOLS.has(tool)) return { allow: true };

  const command = String(args.command || '');
  if (!command.trim()) return { allow: true };

  if (C_REFERENCE.test(command) && !readOnlyCommand(command)) {
    return {
      allow: false,
      reason: 'the command names the system drive or a user profile path; those are never deleted or overwritten by the agent, even with full access',
    };
  }

  if (!isDestructive(command)) return { allow: true };

  if (VARIABLE_OR_GLOB.test(command)) {
    return {
      allow: false,
      reason: 'destructive commands must name a literal path, never a variable or glob (this is how $home was deleted)',
    };
  }

  if (RELATIVE_ESCAPE.test(command)) {
    return { allow: false, reason: 'destructive commands must not walk outside the working directory with ..' };
  }

  if (!ABSOLUTE.test(command) && !/[\s"'=]\.?[\\/]?[A-Za-z0-9_.-]+[\\/]/.test(command)) {
    return { allow: false, reason: 'destructive commands must name a path explicitly' };
  }

  return { allow: true };
}
