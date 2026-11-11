const version = 'v0.2.0-rc.12';
const releaseUrl = `https://github.com/colehank/khor/releases/tag/${version}`;
const asset = (name: string) => `https://github.com/colehank/khor/releases/download/${version}/${name}`;
// Only assets present in the published RC12 release are offered.
export const release = {
  version, channel: 'preview', url: releaseUrl,
  downloads: [
    {platform:'macOS App', arch:'Apple silicon', file:'khor-macos-arm64.app.tar.gz'},
    {platform:'macOS App', arch:'Intel', file:'khor-macos-x86_64.app.tar.gz'},
    {platform:'Linux CLI', arch:'x86_64', file:'khor-linux-x86_64.gz'},
    {platform:'Linux CLI', arch:'ARM64', file:'khor-linux-aarch64.gz'},
    {platform:'macOS CLI', arch:'Apple silicon', file:'khor-macos-arm64.gz'},
    {platform:'macOS CLI', arch:'Intel', file:'khor-macos-x86_64.gz'},
  ].map(item => ({...item, url:asset(item.file)})),
};
