import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/tokens.css";
import "./styles/app.css";

// The keyless screen this file used to paint is the server's job now
// (记住一次批): the page itself sits behind the key, so a browser with
// no credential gets the refusal page before one byte of this script —
// and this script running at all means the door already opened (a `?k=`
// link, or the remembered cookie that same-origin fetches carry on
// their own).
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
