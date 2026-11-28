import generated from './data/world.generated.json';

const previews: Record<string, { total: number; html: string }> = generated.office_previews;
export const officeFixtures = Object.keys(previews);

const media: Record<string, { kind:"pdf"|"image"; media:string }> = {
  'media/preview-check.pdf': {kind:'pdf',media:'application/pdf'},
  'media/khor-icon.png': {kind:'image',media:'image/png'},
};
export const visualFixtures = Object.keys(media);

async function fixtureBytes(file: string) {
  const version = document.querySelector<HTMLMetaElement>('meta[name="khor-lab-version"]')?.content;
  const base = version ? '/versions/'+version : '';
  const response = await fetch(base+'/assets/preview-fixtures/'+file.split('/').map(encodeURIComponent).join('/'));
  if (!response.ok) throw new Error('Lab preview sample is missing');
  const bytes = new Uint8Array(await response.arrayBuffer());
  const data = btoa(Array.from(bytes, byte => String.fromCharCode(byte)).join(''));
  return {total:bytes.length,cut:false,empty:bytes.length===0,data,html:''};
}

export async function visualFixturePreview(file: string) {
  return {...media[file],...await fixtureBytes(file)};
}

export async function officeFixturePreview(file: string, raw: boolean) {
  const {total, html} = previews[file];
  const preview = {kind:'doc',total,cut:false,empty:!html,media:'application/octet-stream',data:'',html};
  if (!raw) return preview;
  return {...preview,...await fixtureBytes(file)};
}
