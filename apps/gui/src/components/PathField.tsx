// A directory box that completes as it is typed in (2026-09-27).
//
// With no caller context, start at the selected machine's actual home (2026-10-18).
// Explicit paths stay intact; erasing a field does not fight the user's input.
// Focus offers the typed parent's children, with a bounded scrolling list.
//
// **Completion, not validation.** A machine that is slow, unreachable or
// refuses the listing means no suggestions and nothing else: the box
// keeps taking anything, and whether the path is real is still the
// verb's question, answered in the verb's own sentence (`DirPicker`'s
// head says why a check here would be a third opinion).
//
// **One listing per parent, per opening.** The cache lives as long as
// this component, and every dialog that draws it unmounts it on close, so
// a directory made since the last opening is listed afresh while typing
// back and forth over one parent asks once.
import { useEffect, useId, useRef, useState, type ComponentProps } from "react";

import { fetchLs, type DirListing } from "@/api";
import { OmniMenu } from "@/components/OmniMenu";
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";
import { imeKey } from "@/lib/ime";
import {directoryPrefix,homeRelative,settledPath,splitPath} from "@/lib/path-input";
export {settledPath,splitPath} from "@/lib/path-input";

/** How long typing has to pause before a machine is asked. */
const SETTLE_MS = 150;

type Offer = {
  /** Only the exact input this listing was filtered for can offer its rows. */
  typed: string;
  /** The typed parent these names complete, exactly as typed. */
  parent: string;
  /** The same parent as the machine spells it — for `~`, the real home. */
  real: string;
  names: string[];
  /** Which of `names` are directories (a file completes without `/`). */
  dirs: Set<string>;
  truncated: boolean;
};

export function PathField({
  machine,
  value,
  onValueChange,
  onSubmit,
  keepHome = true,
  files = false,
  ...rest
}: Omit<ComponentProps<"input">, "value" | "onChange" | "onKeyDown"> & {
  /** Where the directory is; `""` is this machine, the API's spelling. */
  machine: string;
  value: string;
  onValueChange: (value: string) => void;
  /** Enter with no suggestion lit — the form's own Enter. */
  onSubmit?: () => void;
  /** Whether an accepted completion keeps a typed `~`. The wizard's
      verbs expand `~` on the machine that runs them; `ws_add` takes
      absolute paths only, so its box swaps in the home the listing
      answered with rather than handing it a `~` to refuse. */
  keepHome?: boolean;
  /** Offer files too, not only directories — the conversation's 「a file
      on this machine」 (round CHAT). A file completes without a `/`. */
  files?: boolean;
}) {
  const cache = useRef(new Map<string, Promise<DirListing | null>>());
  const [offer, setOffer] = useState<Offer | null>(null);
  const [at, setAt] = useState(-1);
  const [shut, setShut] = useState(false);
  const [focused, setFocused] = useState(false);
  const [home,setHome]=useState<{machine:string;path:string}|null>(null);
  const edits=useRef({machine,typed:false});
  if(edits.current.machine!==machine)edits.current={machine,typed:false};
  useEffect(()=>{
    let live=true;
    fetchLs(machine,"").then(got=>{if(live)setHome({machine,path:directoryPrefix(got.path)})},()=>{if(live&&keepHome)setHome({machine,path:"~/"})});
    return()=>{live=false};
  },[machine,keepHome]);
  useEffect(()=>{
    if(value===""&&!edits.current.typed&&home?.machine===machine)onValueChange(home.path);
  },[value,machine,home,onValueChange]);
  const list = useId();

  // A different machine is a different disk: nothing listed for the
  // last one says anything about this one.
  useEffect(() => {
    cache.current.clear();
  }, [machine]);

  useEffect(() => {
    setAt(-1);
    setShut(false);
    const cut = splitPath(value);
    if (cut === null) {
      setOffer(null);
      return;
    }
    let live = true;
    const ls = (path: string) => {
      const key = `${machine}\0${path}`;
      let got = cache.current.get(key);
      if (!got) {
        // Silent on failure, by the ruling above: no suggestions is the
        // whole of what a machine that cannot answer costs.
        got = fetchLs(machine, path).catch(() => null);
        cache.current.set(key, got);
      }
      return got;
    };
    const timer = setTimeout(async () => {
      const { parent, leaf } = cut;
      let real: string;
      if (/^~[\\/]/.test(parent)) {
        // `""` is how `ls` asks for that machine's home; its answer
        // carries the real path, the only way to learn what that home
        // is called.
        const home = await ls("");
        if (!home) return live && setOffer(null);
        const under = settledPath(parent.slice(2));
        real = under === "" ? home.path : homeRelative(home.path,under);
      } else {
        real = settledPath(parent);
      }
      const got = await ls(/^~[\\/]$/.test(parent) ? "" : real);
      if (!live) return;
      if (!got) return setOffer(null);
      const low = leaf.toLowerCase();
      const names = got.entries
        .filter(
          (e) =>
            (e.dir || files) &&
            e.name !== leaf &&
            e.name.toLowerCase().startsWith(low) &&
            // Dot-named directories only when a dot was typed — a
            // shell's rule, and the one that keeps a home's forty
            // `.cache`s out of the first screen.
            (leaf.startsWith(".") || !e.name.startsWith(".")),
        )
        .map((e) => e.name);
      setOffer({ typed:value, parent, real: got.path, names, truncated:got.truncated, dirs: new Set(got.entries.filter((e) => e.dir).map((e) => e.name)) });
    }, SETTLE_MS);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  }, [value, machine, files]);

  const fresh = offer?.typed === value ? offer : null;
  const names = fresh?.names ?? [];
  const open = focused && !shut && (names.length > 0 || fresh?.truncated === true);

  const take = (name: string) => {
    if (!fresh) return;
    const base =
      !keepHome && /^~[\\/]/.test(fresh.parent)
        ? directoryPrefix(fresh.real)
        : fresh.parent;
    onValueChange(fresh.dirs.has(name) ? directoryPrefix(`${base}${name}`) : `${base}${name}`);
  };

  return (
    <div className="relative min-w-0 flex-1">
      <Input
        {...rest}
        role="combobox"
        aria-expanded={open}
        aria-controls={list}
        aria-autocomplete="list"
        autoComplete="off"
        spellCheck={false}
        placeholder={gui.dir_path_hint}
        value={value}
        onChange={(e) => {edits.current.typed=true;onValueChange(e.target.value)}}
        onFocus={() => setFocused(true)}
        onBlur={() => setFocused(false)}
        onKeyDown={(e) => {
          if (imeKey(e)) return;
          if (open && names.length > 0 && e.key === "ArrowDown") {
            e.preventDefault();
            setAt((n) => (n + 1) % names.length);
          } else if (open && names.length > 0 && e.key === "ArrowUp") {
            e.preventDefault();
            setAt((n) => (n <= 0 ? names.length - 1 : n - 1));
          } else if (open && names.length > 0 && e.key === "Tab" && !e.shiftKey) {
            e.preventDefault();
            take(names[at < 0 ? 0 : at]);
          } else if (e.key === "Enter") {
            if (open && at >= 0) {
              e.preventDefault();
              take(names[at]);
            } else {
              onSubmit?.();
            }
          } else if (open && e.key === "Escape") {
            // The dialog's own Escape stands down while this is open
            // (`ui/dialog`, `aria-expanded`), so the first Escape
            // closes the list and the second the dialog.
            setShut(true);
          }
        }}
      />
      {/* Hangs under the box, no overlay (20-states 浮层: 搜索结果垂在框
          下面). The app's one menu, so it reads as the omnibox's does.
          **In the flow on a narrow face** (UX-4): a dialog there is a
          sheet at the foot of the screen that scrolls its own content
          (`ui/dialog`), so a list hung out of it ran off the window and
          over the sheet's buttons, and one turned upward was cut at the
          sheet's top. In the flow it still hangs under the box, and the
          sheet, held by its foot, grows upward to fit it. */}
      {open && (
        <div id={list} role="listbox" data-path-suggestions>
          <OmniMenu key={machine+"\0"+value}
            rows={{
              groups: [
                ...(fresh?.truncated ? [{key:"cut",label:gui.path_results_cut,role:"text" as const,candidates:[]}] : []),
                {
                  key: "dirs",
                  label: "",
                  role: "text",
                  candidates: names.map((n) => ({ key: n, label: n })),
                },
              ],
              at,
            }}
            onPick={take}
            className="absolute top-full z-20 mt-1 w-full rounded-lg shadow-(--shadow-float) [[data-narrow]_&]:static"
          />
        </div>
      )}
    </div>
  );
}
