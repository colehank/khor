// Inline stroke glyphs — no icon font, no CDN (the app must be whole
// offline). Sized by the caller through font-size (1em box).

function Glyph({ children }: { children: React.ReactNode }) {
  return (
    <svg
      width="1.4em"
      height="1.4em"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {children}
    </svg>
  );
}

export function IconSessions() {
  return (
    <Glyph>
      <path d="M21 12a8 8 0 0 1-8 8H4l2.5-2.7A8 8 0 1 1 21 12z" />
    </Glyph>
  );
}

export function IconDevices() {
  return (
    <Glyph>
      <rect x="3" y="5" width="18" height="12" rx="2" />
      <path d="M9 21h6" />
      <path d="M12 17v4" />
    </Glyph>
  );
}

export function IconMore() {
  return (
    <Glyph>
      <circle cx="6" cy="12" r="0.9" />
      <circle cx="12" cy="12" r="0.9" />
      <circle cx="18" cy="12" r="0.9" />
    </Glyph>
  );
}

export function IconBack() {
  return (
    <Glyph>
      <path d="M15 5l-7 7 7 7" />
    </Glyph>
  );
}
