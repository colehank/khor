import {release} from '../releases';
import {sitePath} from '../paths';

type Platform='macos'|'linux'|'windows'|'mobile'|'unknown';
export type DownloadDevice={platform:Platform;arch?:'arm64'|'x86_64'};
type BrowserNavigator=Navigator&{userAgentData?:{platform:string;getHighEntropyValues?:(hints:string[])=>Promise<{architecture?:string;bitness?:string;platform?:string}>}};

export function recommendDownload(device:DownloadDevice) {
  const label=device.platform==='macos'?'下载 macOS 版':device.platform==='linux'?'下载 Linux CLI':device.platform==='mobile'?'获取桌面版':'查看下载选项';
  const platform=device.platform==='macos'?'macOS App':device.platform==='linux'?'Linux CLI':null;
  const arch=device.arch==='arm64'?(device.platform==='macos'?'Apple silicon':'ARM64'):device.arch==='x86_64'?'x86_64':null;
  const exact=platform&&arch?release.downloads.find(item=>item.platform===platform&&(item.arch===arch||(arch==='x86_64'&&item.arch==='Intel'))):undefined;
  return {label,url:exact?.url??sitePath(platform?`/download/?platform=${device.platform}`:'/download/'),file:exact?.file,
    note:exact?`${exact.platform} · ${exact.arch} · ${release.version}`:platform?`${platform} · 选择适合你的芯片版本`:'当前提供 macOS App 与 Linux / macOS CLI'};
}

export async function detectDownloadDevice(nav:BrowserNavigator):Promise<DownloadDevice> {
  const ua=nav.userAgent;
  const os=nav.userAgentData?.platform??nav.platform;
  if(/Android|iPhone|iPad|iPod/i.test(ua)||(/Mac/i.test(os)&&nav.maxTouchPoints>1))return {platform:'mobile'};
  const platform:Platform=/Mac/i.test(os)?'macos':/Win/i.test(os)?'windows':/Linux/i.test(os)?'linux':'unknown';
  let arch:DownloadDevice['arch'];
  try{
    const hints=await nav.userAgentData?.getHighEntropyValues?.(['architecture','bitness']);
    if(hints?.architecture==='arm'&&hints.bitness==='64')arch='arm64';
    if(hints?.architecture==='x86'&&hints.bitness==='64')arch='x86_64';
  }catch{/* Platform-only selection remains available when hints are denied. */}
  if(!arch&&platform==='linux'){
    if(/aarch64|arm64/i.test(ua))arch='arm64';
    else if(/x86_64|x64/i.test(ua))arch='x86_64';
  }
  // macOS's legacy UA reports Intel on Apple silicon too. Never infer it.
  return {platform,arch};
}

export async function mountDeviceDownloads() {
  const device=await detectDownloadDevice(navigator);
  const requested=new URLSearchParams(location.search).get('platform');
  const selected=requested==='macos'||requested==='linux'?requested:device.platform;
  const choice=recommendDownload(selected===device.platform?device:{platform:selected});
  document.querySelectorAll<HTMLAnchorElement>('[data-device-download]').forEach(link=>{
    link.href=choice.url;
    const label=link.querySelector('[data-download-label]');
    if(label)label.textContent=choice.label;
  });
  document.querySelectorAll('[data-download-note]').forEach(note=>note.textContent=choice.note);
  const options=document.querySelector<HTMLElement>('[data-download-options]');
  if(options){
    options.dataset.preferred=selected==='linux'?'cli':'app';
    const manual=options.querySelector<HTMLDetailsElement>('.manual-downloads');
    if(manual&&selected==='linux')manual.open=true;
  }
  document.querySelectorAll<HTMLElement>('[data-download-platform]').forEach(row=>{
    const recommended=choice.file?row.dataset.downloadFile===choice.file:row.dataset.downloadPlatform===selected;
    row.toggleAttribute('data-recommended',recommended);
    row.querySelector<HTMLElement>('[data-recommend-label]')!.hidden=!recommended;
  });
}
