import {createElement} from 'react';
import {renderToStaticMarkup} from 'react-dom/server';
import {machineMark,sessionMark} from '../../src/components/BrandIcons';
import {IconCopy,IconCheck,IconNetwork,IconWifi,IconCellular} from '../../src/components/icons';

export const networkTypeMarks=Object.fromEntries([
  ['a',IconNetwork],['b',IconWifi],['c',IconCellular],
].map(([name,Icon])=>[name,renderToStaticMarkup(createElement(Icon as typeof IconNetwork),{identifierPrefix:`site-network-${name}-`})]));

export const copyControlMarks=(prefix:string)=>({
  copy:renderToStaticMarkup(createElement(IconCopy),{identifierPrefix:`site-${prefix}-copy-`}),
  check:renderToStaticMarkup(createElement(IconCheck),{identifierPrefix:`site-${prefix}-check-`}),
});

// Build-time only: the story wears the same marks as the actual app.
export const deviceMarks=(['linux','macos','windows','ios'] as const).map(name=>({
  name,svg:renderToStaticMarkup(createElement(machineMark(name)),{identifierPrefix:`story-device-${name}-`}),
}));
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
