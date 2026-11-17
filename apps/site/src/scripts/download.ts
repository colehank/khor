import {release} from '../releases';
import {sitePath} from '../paths';
import {translate} from '../i18n';

type Platform='macos'|'linux'|'windows'|'mobile'|'unknown';
export type DownloadDevice={platform:Platform;arch?:'arm64'|'x86_64';mobileSystem?:'ios'|'android'};
type BrowserNavigator=Navigator&{userAgentData?:{platform:string;getHighEntropyValues?:(hints:string[])=>Promise<{architecture?:string;bitness?:string;platform?:string}>}};

export function recommendDownload(device:DownloadDevice) {
  const label=device.platform==='macos'?translate('Download for macOS','下载 macOS 版'):device.platform==='linux'?translate('Download the Linux CLI','下载 Linux 命令行'):device.platform==='mobile'?translate('Get the desktop app','获取桌面应用'):translate('See download options','查看下载选项');
  const platform=device.platform==='macos'?'macOS App':device.platform==='linux'?'Linux CLI':null;
  const arch=device.arch==='arm64'?(device.platform==='macos'?'Apple silicon':'ARM64'):device.arch==='x86_64'?'x86_64':null;
  const exact=platform&&arch?release.downloads.find(item=>item.platform===platform&&(item.arch===arch||(arch==='x86_64'&&item.arch==='Intel'))):undefined;
  const icon=device.platform==='mobile'?device.mobileSystem??'unknown':device.platform;
  return {label,icon,url:exact?.url??sitePath(platform?`/download/?platform=${device.platform}`:'/download/'),file:exact?.file,
    note:exact?`${exact.platform} · ${exact.arch} · ${release.version}`:platform?`${platform} · ${translate('Choose your chip','选择芯片版本')}`:translate('Available for macOS, with a CLI for Linux and macOS','提供 macOS 应用及 Linux / macOS 命令行客户端')};
}

export async function detectDownloadDevice(nav:BrowserNavigator):Promise<DownloadDevice> {
  const ua=nav.userAgent;
  const os=nav.userAgentData?.platform??nav.platform;
  if(/Android/i.test(ua))return {platform:'mobile',mobileSystem:'android'};
  if(/iPhone|iPad|iPod/i.test(ua)||(/Mac/i.test(os)&&nav.maxTouchPoints>1))return {platform:'mobile',mobileSystem:'ios'};
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
  const render=()=>{
  const choice=recommendDownload(selected===device.platform?device:{platform:selected});
  document.querySelectorAll<HTMLAnchorElement>('[data-device-download]').forEach(link=>{
    link.href=choice.url;
    const label=link.querySelector('[data-download-label]');
    if(label)label.textContent=choice.label;
    link.querySelectorAll<HTMLElement>('[data-download-mark]').forEach(mark=>mark.hidden=mark.dataset.downloadMark!==choice.icon);
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
  };
  render();window.addEventListener('khor:localechange',render);
}
