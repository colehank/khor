export const officeFixtures = ['阶段汇报.pptx', '项目周报.docx', '验收台账.xlsx'].map(name => 'office/'+name);

export async function officeFixturePreview(file: string, raw: boolean) {
  const version = document.querySelector<HTMLMetaElement>('meta[name="khor-lab-version"]')?.content;
  const base = version ? '/versions/'+version : '';
  const response = await fetch(base+'/assets/preview-fixtures/'+file.split('/').map(encodeURIComponent).join('/'));
  if (!response.ok) throw new Error('Office Lab sample is missing');
  const bytes = new Uint8Array(await response.arrayBuffer());
  const title = file.replace(/^office\//,'').replace(/\.\w+$/,'');
  return {
    kind: 'doc', total: bytes.length, cut: false, empty: false,
    media: 'application/octet-stream',
    data: raw ? btoa(Array.from(bytes, byte => String.fromCharCode(byte)).join('')) : '',
    html: raw ? '' : '<h1>'+title+'</h1><p>Office 文档的文本预览。</p>',
  };
}
