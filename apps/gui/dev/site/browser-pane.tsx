import type {ComponentProps} from 'react';
import type {BrowserPane as NativeBrowserPane} from '../../src/views/BrowserPane';
import {siteRoot} from './urls';
export {normalize} from '../../src/views/BrowserPane';

// The native App overlays a separate webview in this seat. Its website
// fixture supplies one local sample page instead; surrounding controls
// remain the production App's. No proxy or real remote device is opened.
export function BrowserPane({url,onOpened}:ComponentProps<typeof NativeBrowserPane>){
  return <iframe data-site-resource-page title="Atlas Web · 订单" src={siteRoot+'/lab/atlas/'}
    className="h-full w-full border-0" sandbox="allow-same-origin"
    onLoad={()=>onOpened(url)}/>;
}
