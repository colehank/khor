import { getCollection } from "astro:content";
import {installUrl} from '../deployment.mjs';

export const REPO = "https://github.com/colehank/khor";
export const INSTALL = `curl -fsSL ${installUrl} | sh`;
export const TAGLINE = "Work across your devices and agents";

/** The docs in reading order — the one list the side column, the
    overview and the previous/next pair all read. */
export async function docs() {
  const [english,chinese]=await Promise.all([getCollection('docs'),getCollection('docsZh')]);
  const translated=new Map(chinese.map(page=>[page.id,page]));
  return english.sort((a,b)=>a.data.order-b.data.order).map(page=>({...page,translation:translated.get(page.id),titleZh:translated.get(page.id)?.data.title??page.data.title,summaryZh:translated.get(page.id)?.data.summary??page.data.summary}));
}
