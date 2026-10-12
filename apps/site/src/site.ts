import { getCollection } from "astro:content";

export const REPO = "https://github.com/colehank/khor";
export const INSTALL = "curl -fsSL https://khor.cloud/install.sh | sh";
export const TAGLINE = "一张网连起你所有设备上的智能体会话";

/** The docs in reading order — the one list the side column, the
    overview and the previous/next pair all read. */
export async function docs() {
  return (await getCollection("docs")).sort((a, b) => a.data.order - b.data.order);
}
