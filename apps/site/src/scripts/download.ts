import {release} from '../releases';
import {sitePath} from '../paths';
import {translate} from '../i18n';

type Platform='macos'|'linux'|'windows'|'mobile'|'unknown';
export type DownloadDevice={platform:Platform;arch?:'arm64'|'x86_64';mobileSystem?:'ios'|'android'};
type BrowserNavigator=Navigator&{userAgentData?:{platform:string;getHighEntropyValues?:(hints:string[])=>Promise<{architecture?:string;bitness?:string;platform?:string}>}};

const downloadPlatforms={
  macos:{name:'macOS App',label:['Download for macOS','下载 macOS 版'],arm64:'Apple silicon'},
  linux:{name:'Linux CLI',label:['Download the Linux CLI','下载 Linux 命令行'],arm64:'ARM64'},
  windows:{name:'Windows App',label:['Download for Windows','下载 Windows 版'],arm64:'ARM64'},
} as const;

export function recommendDownload(device:DownloadDevice) {
  const platform=device.platform in downloadPlatforms?downloadPlatforms[device.platform as keyof typeof downloadPlatforms]:undefined;
  const downloads=release.downloads.filter(item=>item.platform===platform?.name);
  const arch=device.arch==='arm64'?platform?.arm64:device.platform==='macos'&&device.arch==='x86_64'?'Intel':device.arch;
  const exact=downloads.find(item=>item.arch===arch);
  const label=platform&&downloads.length?translate(platform.label[0],platform.label[1]):device.platform==='mobile'?translate('Get the desktop app','获取桌面应用'):translate('See download options','查看下载选项');
  const icon=device.platform==='mobile'?device.mobileSystem??'unknown':device.platform;
  return {label,icon,url:exact?.url??sitePath(downloads.length?`/download/?platform=${device.platform}`:'/download/'),file:exact?.file,
    note:exact?`${exact.platform} · ${exact.arch} · ${release.version}`:translate('Choose your platform','选择你的平台')};
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
  const selected=requested==='macos'||requested==='linux'||requested==='windows'?requested:device.platform;
  const render=()=>{
    const target=selected===device.platform?device:{platform:selected};
    const choice=recommendDownload(target);
    document.querySelectorAll<HTMLAnchorElement>('[data-device-download]').forEach(link=>{
      link.href=choice.url;
      const label=link.querySelector('[data-download-label]');
      if(label)label.textContent=choice.label;
      link.querySelectorAll<HTMLElement>('[data-download-mark]').forEach(mark=>mark.hidden=mark.dataset.downloadMark!==choice.icon);
    });
    document.querySelectorAll('[data-download-note]').forEach(note=>note.textContent=choice.note);
    const options=document.querySelector<HTMLElement>('[data-download-options]');
    if(options){
      options.dataset.preferred=selected==='linux'?'cli':selected;
      const manual=options.querySelector<HTMLDetailsElement>('.manual-downloads');
      if(manual&&selected==='linux')manual.open=true;
    }
    document.querySelectorAll<HTMLElement>('[data-download-platform]').forEach(row=>{
      const recommended=choice.file?row.dataset.downloadFile===choice.file:!target.arch&&row.dataset.downloadPlatform===selected;
      row.toggleAttribute('data-recommended',recommended);
      row.querySelector<HTMLElement>('[data-recommend-label]')!.hidden=!recommended;
    });
  };
  render();window.addEventListener('khor:localechange',render);
}
