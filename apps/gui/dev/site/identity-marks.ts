import {createElement} from 'react';
import {renderToStaticMarkup} from 'react-dom/server';
import {machineMark,sessionMark} from '../../src/components/BrandIcons';

// Build-time only: the story wears the same marks as the actual app.
export const deviceMarks=(['linux','macos','windows','ios'] as const).map(name=>({
  name,svg:renderToStaticMarkup(createElement(machineMark(name)),{identifierPrefix:`story-device-${name}-`}),
}));
export const harnessMarks=['claude','codex','cursor','shell'].map(name=>({
  name,svg:renderToStaticMarkup(createElement(sessionMark(name==='shell'?'shell':'gui',name)),{identifierPrefix:`story-harness-${name}-`}),
}));
