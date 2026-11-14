import {createElement} from 'react';
import {renderToStaticMarkup} from 'react-dom/server';
import {machineMark,sessionMark} from '../../src/components/BrandIcons';
import {IconCopy,IconCheck,IconNetwork,IconWifi,IconCellular,IconBrowser,IconFile,IconBranch,IconChat,IconServer,IconWorkstation,IconDesktop,IconMobile,IconLink,IconEye,IconArrow} from '../../src/components/icons';

export const navigationMarks={
  explore:renderToStaticMarkup(createElement(IconEye),{identifierPrefix:'site-nav-explore-'}),
  docs:renderToStaticMarkup(createElement(IconFile),{identifierPrefix:'site-nav-docs-'}),
  download:renderToStaticMarkup(createElement(IconArrow,{to:'down'}),{identifierPrefix:'site-nav-download-'}),
};

export const capabilityMarks=Object.fromEntries(([
  ['web',IconBrowser],['files',IconFile],['git',IconBranch],['check',IconCheck],
  ['terminal',sessionMark('shell',null)],['message',IconChat],['link',IconLink],
] as const).map(([name,Icon])=>[name,renderToStaticMarkup(createElement(Icon),{identifierPrefix:`site-capability-${name}-`})]));

export const networkTypeMarks=Object.fromEntries([
  ['a',IconNetwork],['b',IconWifi],['c',IconCellular],
].map(([name,Icon])=>[name,renderToStaticMarkup(createElement(Icon as typeof IconNetwork),{identifierPrefix:`site-network-${name}-`})]));

export const copyControlMarks=(prefix:string)=>({
  copy:renderToStaticMarkup(createElement(IconCopy),{identifierPrefix:`site-${prefix}-copy-`}),
  check:renderToStaticMarkup(createElement(IconCheck),{identifierPrefix:`site-${prefix}-check-`}),
});

// Build-time only: the story wears the same marks as the actual app.
export const networkDeviceMarks=Object.fromEntries(([
  ['linux',IconServer],['mac',IconWorkstation],['windows',IconDesktop],['phone',IconMobile],
] as const).map(([id,Icon])=>[id,renderToStaticMarkup(createElement(Icon),{identifierPrefix:`site-device-${id}-`})]));
export const deviceMarks=[
  {name:'linux',svg:networkDeviceMarks.linux},{name:'macos',svg:networkDeviceMarks.mac},
  {name:'windows',svg:networkDeviceMarks.windows},{name:'ios',svg:networkDeviceMarks.phone},
];
export const harnessMarks=['claude','codex','cursor','shell'].map(name=>({
  name,svg:renderToStaticMarkup(createElement(sessionMark(name==='shell'?'shell':'gui',name)),{identifierPrefix:`story-harness-${name}-`}),
}));

export const supportedDevices=(['macos','windows','linux','ios','android'] as const).map((name,index)=>({
  name:['macOS','Windows','Linux','iOS','Android'][index],
  svg:renderToStaticMarkup(createElement(machineMark(name)),{identifierPrefix:`supported-device-${name}-`}),
}));
export const supportedHarnesses=['claude','codex','cursor','shell','khor'].map((name,index)=>({
  name:['Claude','Codex','Cursor','Shell','Khor'][index],
  svg:renderToStaticMarkup(createElement(sessionMark(name==='shell'?'shell':'gui',name)),{identifierPrefix:`supported-harness-${name}-`}),
}));
