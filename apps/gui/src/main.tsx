import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { isMenuFace } from "./api";
import { MenuFace } from "./components/OmniMenu";
import "./styles/tokens.css";
import "./styles/app.css";
import { followSystemTheme, mirroredPref, wearTheme } from "./theme";
import { mirroredLang, speakLang } from "./lang";

// **The theme is stamped before React is asked for a frame** (批 D2).
//
// Every stylesheet keys its dark half off `:root[data-theme]` now
// (tokens.css says why: the alternative was a second copy of the
// palette), so an unstamped root paints light — and a dark-desktop
// machine would show one white frame before the first answer arrived.
//
// It is stamped from the **mirror** here, not from the store: the store
// is behind an async call and this has to be synchronous to be worth
// anything. `index.html` has already done exactly this before the
// bundle loaded; doing it again costs one attribute write and is what
// keeps this file honest when that frame is not there (the web face
// served from a different document, a test harness mounting the app).
// The real answer replaces it the moment `khor config` answers
// (`App.tsx`), and the two agree on every launch but the first after a
// change.
wearTheme(mirroredPref());
followSystemTheme();

// **…and the language, for the same reason and from the same kind of
// mirror** (批 D3). Every word on screen is a property read off the
// generated catalog, and a read is not reactive — so this has to be
// settled before the first render rather than corrected after it.
//
// From the mirror, not the store: the store is behind an async call.
// The real answer replaces it the moment `khor config` answers
// (`App.tsx`), and the two agree on every launch but the first after
// somebody changes it.
speakLang(mirroredLang());

// The keyless screen this file used to paint is the server's job now
// (记住一次批): the page itself sits behind the key, so a browser with
// no credential gets the refusal page before one byte of this script —
// and this script running at all means the door already opened (a `?k=`
// link, or the remembered cookie that same-origin fetches carry on
// their own).
//
// **Two roots, one page.** The app's completions are drawn in a small
// window of its own when a browsing tab's page would otherwise cover them
// (`show_menu`), and that window loads this same page with `?face=menu` on
// it. A second HTML entry point was the other road and it is the worse
// one: the menu's look would then be defined in two places, and the two
// would drift the first time either was restyled.
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>{isMenuFace ? <MenuFace /> : <App />}</React.StrictMode>,
);
