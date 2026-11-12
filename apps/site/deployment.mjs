const segment=(process.env.KHOR_SITE_BASE??'/').replace(/^\/+|\/+$/g,'');
export const deployment={
  site:process.env.KHOR_SITE_URL??'https://khor.cloud',
  base:segment?`/${segment}/`:'/',
};
export const installUrl=new URL(`${deployment.base}install.sh`,deployment.site).href;
export const deploymentPath=path=>path.startsWith('/')&&!path.startsWith('//')?deployment.base.replace(/\/$/,'')+path:path;
