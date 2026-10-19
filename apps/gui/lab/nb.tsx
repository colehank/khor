// Lab only (lane nb-view): the notebook preview with nothing of the app
// around it, for the gate (`scripts/check-nb.mjs`) and for eyes. Nothing
// here talks to a node. `?dark` / `?light` the theme, `?ground=light|dark`
// the preview ground, `#sample` / `#big` a notebook drawn for a screenshot.
//
//   window.nbSample()            the sample notebook's text; nbExpected()
//                                what each cell's outputs should be
//   window.nbBig(n)              a notebook of `n` cells, every third a plot
//   window.nbShow(text)          draw it, resolve when settled
//   window.nbOpen(text)          draw it from nothing: ms to the first
//                                commit, and to the first screen painted
//   window.nbSource(text)        the same text in the source face's editor:
//                                ms until CodeMirror has drawn it
//   window.nbV3                  an nbformat 3 document
import { useState } from "react";
import { flushSync } from "react-dom";
import ReactDOM from "react-dom/client";

import "@/styles/tokens.css";
import "@/styles/app.css";

import { CodeEditor } from "@/components/CodeEditor.lazy";
import Notebook from "@/components/Notebook";

import { big, EXPECTED, plotPng, sample, V3 } from "./nb-sample";

type Shown = { text: string; face: "preview" | "source" };

const q = new URLSearchParams(location.search);
const ground = q.get("ground");

const settle = () =>
  new Promise<void>((r) => requestAnimationFrame(() => requestAnimationFrame(() => setTimeout(r, 30))));

let show: (s: Shown) => void = () => {};
let png = "";
const pic = () => (png ||= plotPng());

function Lab() {
  const [shown, setShown] = useState<Shown>(() => ({
    text: location.hash === "#sample" ? sample(pic()) : location.hash === "#big" ? big(300, pic()) : "",
    face: "preview",
  }));
  show = (s) => flushSync(() => setShown(s));
  return (
    <div
      data-file-preview="ipynb"
      {...(ground === "light" || ground === "dark" ? { "data-ground": ground } : {})}
      className="h-dvh overflow-auto bg-background"
    >
      {shown.text === "" ? null : shown.face === "preview" ? (
        <Notebook text={shown.text} machine="m" path="/work/demo/analysis.ipynb" />
      ) : (
        <div className="flex h-full flex-col">
          <CodeEditor path="/work/demo/analysis.ipynb" text={shown.text} editable onText={() => {}} />
        </div>
      )}
    </div>
  );
}

const w = window as unknown as Record<string, unknown>;
w.nbSample = () => sample(pic());
w.nbExpected = () => EXPECTED;
w.nbBig = (n: number) => big(n, pic());
w.nbV3 = V3;
w.nbShow = async (text: string) => {
  show({ text: "", face: "preview" });
  show({ text, face: "preview" });
  await settle();
};
w.nbOpen = async (text: string) => {
  show({ text: "", face: "preview" });
  await settle();
  const t0 = performance.now();
  show({ text, face: "preview" });
  const commit = performance.now() - t0;
  await new Promise((r) => requestAnimationFrame(() => r(null)));
  const frame = performance.now() - t0;
  return { commit, frame, drawn: document.querySelectorAll("[data-nb-drawn]").length, cells: document.querySelectorAll("[data-nb-cell]").length };
};
w.nbSource = async (text: string) => {
  show({ text: "", face: "preview" });
  await settle();
  const t0 = performance.now();
  show({ text, face: "source" });
  for (;;) {
    if (document.querySelector(".cm-content .cm-line")) break;
    await new Promise((r) => setTimeout(r, 5));
  }
  await new Promise((r) => requestAnimationFrame(() => r(null)));
  return { ms: performance.now() - t0 };
};

ReactDOM.createRoot(document.getElementById("root")!).render(<Lab />);
