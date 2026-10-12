// Lab only (lane cm-d): one commit's page, alone, for timing it and for
// shots. Nothing here talks to a node — the page's one ask
// (`/api/git_commit`) is answered by whoever drives the browser
// (`scripts/cm-measure.mjs` routes it to a fixture built from this
// checkout's git). `?dark` / `?light` the theme, `?id=` the commit.
//
// `window.__t0` is the moment before the page is first rendered; the
// driver reads it against the moment every text file's diff is drawn.
import ReactDOM from "react-dom/client";

import "@/styles/tokens.css";
import "@/styles/app.css";

import { TooltipProvider } from "@/components/ui/tooltip";
import { CommitPage } from "@/views/CommitPage";

const q = new URLSearchParams(location.search);
const id = q.get("id") ?? "0".repeat(40);

(window as unknown as { __t0: number }).__t0 = performance.now();
ReactDOM.createRoot(document.getElementById("root")!).render(
  <TooltipProvider>
    <div className="flex h-dvh flex-col bg-card text-foreground">
      <CommitPage
        machine="lab"
        root="/lab"
        commit={{ id, author: "lab", when_ms: Date.now() - 3_600_000, subject: q.get("subject") ?? "lab", body: "" }}
        onBack={() => {}}
      />
    </div>
  </TooltipProvider>,
);
