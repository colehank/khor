import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { isMenuFace } from "./api";
import { MenuFace } from "./components/OmniMenu";
import "./styles/tokens.css";
import "./styles/app.css";

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
