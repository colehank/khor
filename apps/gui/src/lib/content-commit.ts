export type ContentCommit = { root:string; id:string };
export const commitAddress = ({root,id}:ContentCommit):string => JSON.stringify([root,id]);
export function readCommitAddress(address:string):ContentCommit|null {
  try {
    const value:unknown=JSON.parse(address);
    return Array.isArray(value)&&value.length===2&&typeof value[0]==='string'&&typeof value[1]==='string'&&/^(?:[0-9a-f]{40}|[0-9a-f]{64})$/i.test(value[1])
      ? {root:value[0],id:value[1]} : null;
  } catch { return null; }
}
