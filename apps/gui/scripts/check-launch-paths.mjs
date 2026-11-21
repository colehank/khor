// Paths are interpreted on the target machine, including a Windows target
// controlled from an iPhone or Mac. No host OS path library belongs here.
import assert from 'node:assert/strict';
import { directoryPrefix, settledPath, splitPath, homeRelative, parentPath, pathLeaf } from '../src/lib/path-input.ts';

assert.equal(directoryPrefix('/home/alice'), '/home/alice/');
assert.equal(directoryPrefix('C:\\Users\\Alice'), 'C:\\Users\\Alice\\');
assert.equal(directoryPrefix('\\\\server\\share'), '\\\\server\\share\\');
assert.equal(directoryPrefix('C:/Users/Alice/'), 'C:/Users/Alice/');
assert.equal(directoryPrefix('~\\projects'), '~\\projects\\');
assert.equal(settledPath('/'), '/');
assert.equal(settledPath('C:\\'), 'C:\\');
assert.equal(settledPath('C:/'), 'C:/');
assert.equal(settledPath(' /home/alice/ '), '/home/alice');
assert.equal(settledPath('C:\\Users\\Alice\\'), 'C:\\Users\\Alice');
assert.equal(settledPath('~/'), '~');
assert.deepEqual(splitPath('/home/alice/wo'), {parent:'/home/alice/',leaf:'wo'});
assert.deepEqual(splitPath('C:\\Users\\Alice\\wo'), {parent:'C:\\Users\\Alice\\',leaf:'wo'});
assert.deepEqual(splitPath('C:/Users/Alice/wo'), {parent:'C:/Users/Alice/',leaf:'wo'});
assert.deepEqual(splitPath('\\\\server\\share\\wo'), {parent:'\\\\server\\share\\',leaf:'wo'});
assert.deepEqual(splitPath('~\\wo'), {parent:'~\\',leaf:'wo'});
assert.equal(splitPath('relative/file'), null);
assert.equal(splitPath('filename'), null);
assert.equal(homeRelative('C:\\Users\\Alice', 'projects/work'), 'C:\\Users\\Alice\\projects\\work');
assert.equal(homeRelative('/home/alice', 'projects/work'), '/home/alice/projects/work');
const drive = '\\\\?\\C:\\';
const project = `${drive}Khor 项目`;
const nested = homeRelative(project, '资料/readme.md');
assert.equal(nested, `${project}\\资料\\readme.md`);
assert.equal(parentPath(nested), `${project}\\资料`);
assert.equal(parentPath(project), drive);
assert.equal(parentPath(drive), null);
assert.equal(parentPath('\\\\?\\UNC\\server\\share'), null);
assert.equal(pathLeaf(nested), 'readme.md');
assert.equal(pathLeaf('/home/alice/a\\b'), 'a\\b', 'POSIX filenames can contain literal backslashes');
console.log('launch paths: Unix, Windows drive/UNC, home and roots passed');
