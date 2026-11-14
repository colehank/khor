import { getCollection } from "astro:content";
import {installUrl} from '../deployment.mjs';

export const REPO = "https://github.com/colehank/khor";
export const INSTALL = `curl -fsSL ${installUrl} | sh`;
export const TAGLINE = "Work across your devices and agents";

/** The docs in reading order — the one list the side column, the
    overview and the previous/next pair all read. */
export async function docs() {
  return (await getCollection("docs")).sort((a, b) => a.data.order - b.data.order);
}
