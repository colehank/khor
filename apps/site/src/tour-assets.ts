import connectLight from './assets/tour/connect-light.webp';
import connectDark from './assets/tour/connect-dark.webp';
import connectMobileLight from './assets/tour/connect-mobile-light.webp';
import connectMobileDark from './assets/tour/connect-mobile-dark.webp';
import sessionsLight from './assets/tour/sessions-light.webp';
import sessionsDark from './assets/tour/sessions-dark.webp';
import sessionsMobileLight from './assets/tour/sessions-mobile-light.webp';
import sessionsMobileDark from './assets/tour/sessions-mobile-dark.webp';
import workspaceLight from './assets/tour/workspace-light.webp';
import workspaceDark from './assets/tour/workspace-dark.webp';
import workspaceMobileLight from './assets/tour/workspace-mobile-light.webp';
import workspaceMobileDark from './assets/tour/workspace-mobile-dark.webp';
import changesLight from './assets/tour/changes-light.webp';
import changesDark from './assets/tour/changes-dark.webp';
import changesMobileLight from './assets/tour/changes-mobile-light.webp';
import changesMobileDark from './assets/tour/changes-mobile-dark.webp';
import mac from './assets/tour/face-mac.webp';
import linux from './assets/tour/face-linux.webp';
import windows from './assets/tour/face-windows.webp';

// These are art-directed crops of the same real App, not separate demos.
// A picture downloads only the crop and theme needed by this viewport.
export const tourImages={
  connect:{light:connectLight,dark:connectDark,mobileLight:connectMobileLight,mobileDark:connectMobileDark},
  sessions:{light:sessionsLight,dark:sessionsDark,mobileLight:sessionsMobileLight,mobileDark:sessionsMobileDark},
  workspace:{light:workspaceLight,dark:workspaceDark,mobileLight:workspaceMobileLight,mobileDark:workspaceMobileDark},
  changes:{light:changesLight,dark:changesDark,mobileLight:changesMobileLight,mobileDark:changesMobileDark},
};
export const peerFaces={mac,linux,windows};
