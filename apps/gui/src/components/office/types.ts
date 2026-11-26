export type OfficeComment = {
  text: string;
  author?: string;
  date?: string;
};
export type OfficeCommentAnchor = {
  surface: HTMLElement;
  page: number;
  x: number;
  y: number;
  width: number;
  height: number;
};
export type OfficeThread = OfficeComment & {
  key: string;
  location?: string;
  replies?: readonly OfficeComment[];
  jump?: () => Promise<OfficeCommentAnchor | boolean | null>;
};
export type WordReviewPoint = { x: number; y: number };
export type WordReviewEntry = OfficeThread & {
  points: WordReviewPoint[];
  revision?: string;
};
export type WordReviewSource = {
  page: (index: number, comments: boolean, revisions: boolean) => Promise<WordReviewEntry[]>;
  unanchored: (comments: boolean, revisions: boolean) => WordReviewEntry[];
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
