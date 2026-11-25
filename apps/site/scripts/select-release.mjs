import {createHash} from 'node:crypto';
import {readFile, writeFile} from 'node:fs/promises';

const repository = 'https://github.com/colehank/khor';
const manifestName = 'khor-release.json';
const installerName = 'install.sh';
const windowsMsi = 'khor-windows-x86_64.msi';
const releasePattern = /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-rc\.(0|[1-9]\d*))?$/;
const catalog = JSON.parse(await readFile(new URL('../src/download-assets.json', import.meta.url), 'utf8'));

function compareVersions(a, b) {
  for (let i = 0; i < a.version.length; i++) {
    if (a.version[i] !== b.version[i]) return a.version[i] > b.version[i] ? -1 : 1;
  }
  return 0;
}

function selectReleases(releases) {
  const candidates = [];
  for (const item of releases) {
    const match = releasePattern.exec(item.tag_name);
    if (item.draft || !item.published_at || !match || item.prerelease !== Boolean(match[4])) continue;
    candidates.push({item, version: match.slice(1).map(part => BigInt(part ?? 0))});
  }
  candidates.sort(compareVersions);
  const stable = candidates.find(({item}) => !item.prerelease)?.item;
  const preview = candidates.find(({item}) => item.prerelease)?.item;
  if (!stable && !preview) throw new Error('No published Khor release found');
  return {primary: stable ?? preview, preview: stable && preview};
}

async function readAsset(asset) {
  const response = await fetch(asset.browser_download_url, {signal: AbortSignal.timeout(15_000)});
  if (!response.ok) throw new Error(asset.name + ' returned HTTP ' + response.status);
  const bytes = Buffer.from(await response.arrayBuffer());
  const digest = 'sha256:' + createHash('sha256').update(bytes).digest('hex');
  if (bytes.length !== asset.size || digest !== asset.digest) {
    throw new Error(asset.name + ' failed the GitHub digest check');
  }
  return bytes;
}

async function verifyRelease(item) {
  if (!item.immutable) throw new Error(item.tag_name + ' is missing GitHub release immutability');
  const base = repository + '/releases/download/' + item.tag_name + '/';
  const assets = new Map(item.assets.map(asset => [asset.name, asset]));
  function requireAsset(name) {
    const asset = assets.get(name);
    if (!asset || asset.state !== 'uploaded' || asset.size <= 0 || asset.browser_download_url !== base + name
      || !/^sha256:[0-9a-f]{64}$/.test(asset.digest ?? '')) {
      throw new Error(item.tag_name + ' is missing a complete ' + name + ' asset');
    }
    return asset;
  }

  requireAsset(manifestName + '.sig');
  const manifest = JSON.parse(await readAsset(requireAsset(manifestName)));
  if (manifest.schema !== 1 || manifest.identity?.tag !== item.tag_name || !Array.isArray(manifest.artifacts)) {
    throw new Error(item.tag_name + ' has an invalid release manifest');
  }
  const products = new Map(manifest.artifacts.map(product => [product.file, product]));
  const files = [];
  for (const {file, platform} of catalog) {
    if (file === windowsMsi && !assets.has(file) && !products.has(file)) continue;
    const asset = requireAsset(file);
    const product = products.get(file);
    if (product?.url !== asset.browser_download_url || product.bytes !== asset.size || 'sha256:' + product.sha256 !== asset.digest) {
      throw new Error(item.tag_name + ' release manifest disagrees with GitHub about ' + file);
    }
    if (platform.endsWith(' CLI')) requireAsset(file + '.sha256');
    files.push(file);
  }
  return {version: item.tag_name, channel: item.prerelease ? 'preview' : 'stable', files, installer: requireAsset(installerName)};
}

const [releaseListPath, installerPath] = process.argv.slice(2);
if (!releaseListPath || !installerPath) throw new Error('Usage: node select-release.mjs <GitHub release API pages> <installer output>');
const pages = JSON.parse(await readFile(releaseListPath, 'utf8'));
const {primary, preview} = selectReleases(pages.flat());
const {version, channel, files, installer} = await verifyRelease(primary);
const previewVersion = preview ? (await verifyRelease(preview)).version : null;
await writeFile(installerPath, await readAsset(installer));
await writeFile(new URL('../src/releases.json', import.meta.url), JSON.stringify({version, channel, files, previewVersion}, null, 2) + '\n');
console.log('Selected ' + version + ' (' + channel + '), with ' + files.length + ' verified download links');
