// This entry is always served as <site base>/experience/app/index.html.
export const siteRoot=new URL('../../',location.href).href.replace(/\/$/,'');
export const downloadUrl=siteRoot+'/download/';
