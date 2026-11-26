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
