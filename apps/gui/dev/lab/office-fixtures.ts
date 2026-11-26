import generated from './data/world.generated.json';

const previews: Record<string, { total: number; html: string }> = generated.office_previews;
export const officeFixtures = Object.keys(previews);

export async function officeFixturePreview(file: string, raw: boolean) {
  const {total, html} = previews[file];
  const preview = {kind:'doc',total,cut:false,empty:!html,media:'application/octet-stream',data:'',html};
  if (!raw) return preview;
  const version = document.querySelector<HTMLMetaElement>('meta[name="khor-lab-version"]')?.content;
  const base = version ? '/versions/'+version : '';
  const response = await fetch(base+'/assets/preview-fixtures/'+file.split('/').map(encodeURIComponent).join('/'));
  if (!response.ok) throw new Error('Office Lab sample is missing');
  const bytes = new Uint8Array(await response.arrayBuffer());
  return {...preview, empty:bytes.length===0, data:btoa(Array.from(bytes, byte => String.fromCharCode(byte)).join('')), html:''};
}
