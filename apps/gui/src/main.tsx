import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { missingWebKey } from "./api";
import { msg } from "./gen/catalog";
import "./styles/tokens.css";
import "./styles/app.css";

/**
 * A browser that reached the web face without a key gets a sentence
 * instead of the app.
 *
 * **Decided here rather than inside `App`**, for two reasons. It is
 * known before the first render — it is a fact about the address this
 * page was opened at, not state — so the app never mounts, never polls,
 * and never has to hold a "can I talk to anything" branch through its
 * whole tree. And an early return inside `App` would sit above its
 * hooks: harmless while this value is a module constant, and a trap the
 * day somebody makes it anything else.
 *
 * Why a screen at all: every poll in this app swallows its errors,
 * because one that threw on a blip would take the screen down for a
 * hiccup. Without this, arriving with no key looks exactly like khor
 * being broken — an empty page that never fills in, saying nothing.
 */
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    {missingWebKey ? (
      <div className="flex h-dvh items-center justify-center p-6">
        <p className="text-muted-foreground max-w-prose text-center text-sm whitespace-pre-line">
          {msg.web_no_key}
        </p>
      </div>
    ) : (
      <App />
    )}
  </React.StrictMode>,
);
