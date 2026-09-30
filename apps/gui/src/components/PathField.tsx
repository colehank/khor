// A directory box that completes as it is typed in (2026-09-27).
//
// **The owner's ruling, whole**: 「"+" 弹出的窗口，涉及到目录的，都不要推荐
// 任何路径，简单一个输入框，用户可以边写边自动推荐。」 So nothing is offered
// before a character is typed — no recent places, no home shortcut, no
// tree to walk — and what is offered after is the one thing a shell's
// Tab would offer: the directories under what has been typed so far, on
// the machine the dialog is pointed at.
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

/** How many completions hang under the box. A handful: this is Tab in a
    shell, not a directory listing, and the next character narrows it. */
const SHOWN = 8;
/** How long typing has to pause before a machine is asked. */
const SETTLE_MS = 150;

/** What has been typed, cut where the last segment starts. `null` is
    「nothing to complete」: no slash yet, or a relative path — the verbs
    behind these boxes take absolute paths and `~`, and a listing of a
    relative one would be relative to nothing. */
export function splitPath(typed: string): { parent: string; leaf: string } | null {
  const cut = typed.lastIndexOf("/");
  if (cut < 0) return null;
  const parent = typed.slice(0, cut + 1);
  if (!parent.startsWith("/") && !parent.startsWith("~/")) return null;
  return { parent, leaf: typed.slice(cut + 1) };
}

/** The path as the verb should receive it: trimmed, and without the
    trailing slash a completion leaves behind (`/` alone stays `/`). */
export function settledPath(typed: string): string {
  const t = typed.trim();
  return t.length > 1 && t.endsWith("/") ? t.replace(/\/+$/, "") || "/" : t;
}

type Offer = {
  /** The typed parent these names complete, exactly as typed. */
  parent: string;
  /** The same parent as the machine spells it — for `~`, the real home. */
  real: string;
  names: string[];
};

export function PathField({
  machine,
  value,
  onValueChange,
  onSubmit,
  keepHome = true,
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
}) {
  const cache = useRef(new Map<string, Promise<DirListing | null>>());
  const [offer, setOffer] = useState<Offer | null>(null);
  const [at, setAt] = useState(-1);
  const [shut, setShut] = useState(false);
  const [focused, setFocused] = useState(false);
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
      if (parent.startsWith("~/")) {
        // `""` is how `ls` asks for that machine's home; its answer
        // carries the real path, the only way to learn what that home
        // is called.
        const home = await ls("");
        if (!home) return live && setOffer(null);
        const under = parent.slice(2).replace(/\/+$/, "");
        real = under === "" ? home.path : `${home.path.replace(/\/+$/, "")}/${under}`;
      } else {
        real = parent === "/" ? "/" : parent.replace(/\/+$/, "") || "/";
      }
      const got = await ls(parent === "~/" ? "" : real);
      if (!live) return;
      if (!got) return setOffer(null);
      const low = leaf.toLowerCase();
      const names = got.entries
        .filter(
          (e) =>
            e.dir &&
            e.name !== leaf &&
            e.name.toLowerCase().startsWith(low) &&
            // Dot-named directories only when a dot was typed — a
            // shell's rule, and the one that keeps a home's forty
            // `.cache`s out of the first screen.
            (leaf.startsWith(".") || !e.name.startsWith(".")),
        )
        .slice(0, SHOWN)
        .map((e) => e.name);
      setOffer({ parent, real: got.path, names });
    }, SETTLE_MS);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  }, [value, machine]);

  const names = offer?.names ?? [];
  const open = focused && !shut && names.length > 0;

  const take = (name: string) => {
    if (!offer) return;
    const base =
      !keepHome && offer.parent.startsWith("~/")
        ? `${offer.real.replace(/\/+$/, "")}/`
        : offer.parent;
    onValueChange(`${base}${name}/`);
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
        onChange={(e) => onValueChange(e.target.value)}
        onFocus={() => setFocused(true)}
        onBlur={() => setFocused(false)}
        onKeyDown={(e) => {
          if (e.nativeEvent.isComposing) return;
          if (open && e.key === "ArrowDown") {
            e.preventDefault();
            setAt((n) => (n + 1) % names.length);
          } else if (open && e.key === "ArrowUp") {
            e.preventDefault();
            setAt((n) => (n <= 0 ? names.length - 1 : n - 1));
          } else if (open && e.key === "Tab" && !e.shiftKey) {
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
          下面). The app's one menu, so it reads as the omnibox's does. */}
      {open && (
        <div id={list} role="listbox" data-path-suggestions>
          <OmniMenu
            rows={{
              groups: [
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
            className="absolute top-full z-20 mt-1 w-full rounded-lg shadow-(--shadow-float)"
          />
        </div>
      )}
    </div>
  );
}
