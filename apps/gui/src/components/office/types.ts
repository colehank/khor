import { useCallback, useState } from "react";

export type OfficeAction = { key: string; label: string; run: () => void; afterClose?: boolean };
export type OfficeTools = { find: () => void; items: OfficeAction[] };

export function useOfficeTools(machine: string, path: string | null) {
  const key = JSON.stringify([machine, path]);
  const [published, setPublished] = useState<{ key: string; tools: OfficeTools } | null>(null);
  const publish = useCallback((tools: OfficeTools | null) => {
    setPublished(previous => tools ? { key, tools } : previous?.key === key ? null : previous);
  }, [key]);
  return { tools: published?.key === key ? published.tools : null, publish };
}

export type OfficeComment = {
  text: string;
  author?: string;
  date?: string;
};
export type OfficeThread = OfficeComment & {
  key: string;
  location?: string;
  replies?: readonly OfficeComment[];
  jump?: () => Promise<unknown> | void;
};
export type OfficePages = {
  index: number;
  count: number;
};
export type OfficeNavigation = {
  page: () => OfficePages;
  go: (index: number) => Promise<void> | void;
  thumbnail: (index: number) => Promise<ImageBitmap>;
};
