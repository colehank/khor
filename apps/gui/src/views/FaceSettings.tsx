// A machine's face, as the edit window shows it — the half of 编辑… that
// is the picture (`MachineEditDialog` holds the other half, the name).
//
// # Where this has lived, and why it is here now
//
// It was the whole settings screen (a dialog), then a section of the
// 设置 page, then of 这台机器 with a copy in a window off every machine
// row's 更改头像… (2026-09-28). **2026-09-30, the owner**: 「『我』的里面
// 不要有更改头像的设置，这在机器页也能设置，不要冗余。更改头像的弹窗 UI
// 有点丑，能不能右键→编辑，然后这里可以改名称、头像等诸多信息」. So one
// door (the row's 编辑…), one window, and this is drawn in it and nowhere
// else — 这台机器 states the face, it does not edit it.
//
// # The shape: a form, not a column of loose buttons
//
// What made the old one ugly was that it was a centred stack — a face, a
// row of ghost buttons each holding a small face and a word, five 44
// discs, a lone button, a fold — with no labels and no edge to read down.
// This is the window's form instead (`SheetForm`, components/Dialog:
// questions right-aligned in the left column, answers in the right):
//
//   - **the face, large, above the form** — the one place a face is
//     drawn bigger than it is worn, because it is the editor's subject:
//     every row below changes it and it shows the result;
//   - **画法 and 形状 as segmented controls** — three fixed choices each,
//     which is exactly what `SegmentedControl` is for (≤ 4, never grows),
//     the way macOS lays out a picture's options. No thumbnails: the
//     big face above is the preview, and it is the node's painting;
//   - **色板 as five wells and 随机配色** on one line.
//
// 形状 is no longer folded behind 高级. The fold was 2026-09-20's answer to
// 「不是所有的设置都要去铺在界面上，显得非常杂乱」 on a page where the editor
// was a heap; in a window whose every row is one system control, a
// disclosure for one row is the clutter.
//
// # Nothing here derives a face
//
// The preview is an `Avatar` the node painted (`khor_gui_core::face_choices`,
// one call for the whole window), handed to the same `AvatarFace` brush
// the list rows use. A window that drew its own would be the second
// painter `khor_core::avatar` exists to prevent.
//
// The window holds no style state either. Picking writes through
// `restyle` and re-reads — so what is on screen is what the node says,
// and every face in the app moves in the same frame (the caller refetches
// the table). That is also why the name waits for 完成 and the face does
// not: a face is a pick, seen at once; a name is typed, and half a name
// is not one.
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";

import { fetchFaceChoices, restyle, type FaceChoices } from "@/api";
import { AvatarFace } from "@/components/Avatar";
import { Button } from "@/components/ui/button";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { FormLabel, SheetForm } from "@/components/ui/sheet-form";
import { avatar, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { faceWord } from "@/words";

/**
 * One color slot.
 *
 * **The native `change` event, not React's `onChange`.** React maps
 * `onChange` onto `input`, which on a color well fires for every pixel
 * of the drag — and every one of those would be a file written and a
 * CRDT document flushed. `change` fires once, when the picker is let go,
 * which is also the moment the person means it.
 *
 * Not in `components/ui/`: that directory is the vendored shadcn family,
 * and shadcn has no color control. There is exactly one of these in the
 * app.
 *
 * **The well is its own target**, so it is sized by `--hit` (`size-touch`)
 * — the control height under a mouse, a finger's 44 under a finger. There
 * is no glyph inside a larger hit box here: the browser paints the colour
 * across the whole element, and that is what the finger aims at.
 * ⚠️ 墓碑 (2026-09-30): it was `var(--touch)`, 44 under a mouse too — five
 * discs half again as tall as every control beside them, which was a good
 * part of what read as ugly.
 */
function ColorWell({
  index,
  value,
  on,
  onOpen,
  onCommit,
}: {
  index: number;
  value: string;
  /** This is the well the picker was opened from. */
  on: boolean;
  onOpen: () => void;
  onCommit: (hex: string) => void;
}) {
  const [draft, setDraft] = useState(value);
  // The node is the truth: pressing 随机配色 replaces all five, and each
  // well has to follow rather than keep showing what it was.
  useEffect(() => setDraft(value), [value]);

  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const commit = () => onCommit(el.value);
    el.addEventListener("change", commit);
    return () => el.removeEventListener("change", commit);
  }, [onCommit]);

  // **A dot of the colour, and nothing around it.** `app.css` takes the
  // browser's inset and inner border off, so the colour fills the whole
  // disc, and `avatar-edge` is the one hairline — the edge a face wears,
  // so a pale slot still stands off a pale window.
  //
  // **The well the picker is open for wears an ink ring** — what an
  // active `NSColorWell` shows. Ink, not `brand-strong`: it marks a
  // choice, and olive has three posts, none of them this.
  return (
    <input
      ref={ref}
      type="color"
      data-color-slot={index}
      data-on={on}
      aria-label={avatar.axis_palette_slot(index + 1)}
      value={draft}
      onClick={onOpen}
      onChange={(e) => setDraft(e.target.value)}
      className={cn(
        "size-touch flex-none appearance-none rounded-full border border-avatar-edge bg-transparent p-0",
        on && "outline-2 outline-offset-2 outline-foreground",
        "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring",
      )}
    />
  );
}

export function FaceSettings({
  machine,
  onChanged,
  children,
}: {
  /** Which machine's face this edits — a device id. A face is 全网同一份:
      anyone may set anyone's. */
  machine: string;
  /** Called after a change took, so the faces elsewhere in the app come
      from the node's new answer rather than from this window. */
  onChanged: () => void;
  /** Rows of the same form that come before the face's own (the name) —
      one form, one left column, one edge to read down. */
  children?: ReactNode;
}) {
  const [choices, setChoices] = useState<FaceChoices | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Which well the picker was opened from (`ColorWell`'s ring). It lets
  // go the way an active colour well does: when another one is opened,
  // or a press lands anywhere that is not a well, or Escape. **Not on
  // commit**: the system panel stays up after a pick.
  const [editing, setEditing] = useState<number | null>(null);
  useEffect(() => {
    if (editing === null) return;
    const away = (e: PointerEvent) => {
      if (!(e.target instanceof Element) || !e.target.closest("[data-color-slot]")) setEditing(null);
    };
    const esc = (e: KeyboardEvent) => {
      if (e.key === "Escape") setEditing(null);
    };
    document.addEventListener("pointerdown", away, true);
    document.addEventListener("keydown", esc);
    return () => {
      document.removeEventListener("pointerdown", away, true);
      document.removeEventListener("keydown", esc);
    };
  }, [editing]);

  const load = useCallback(
    () =>
      fetchFaceChoices(machine).then(setChoices, (e: unknown) =>
        setError(String(e instanceof Error ? e.message : e)),
      ),
    [machine],
  );

  // Read on every opening — the window is mounted only while it is up,
  // and a face can have been changed from a terminal since the last look.
  useEffect(() => {
    setError(null);
    void load();
  }, [load]);

  // Two-argument `then`, not `.then().catch()`: the rejection handler
  // must see the restyle failing and nothing else. Chained, a re-read
  // that threw after a change that took would report the change as
  // failed while the machine is already wearing it.
  const apply = useCallback(
    (change: { colors?: string[]; variant?: string; shape?: string }) => {
      setError(null);
      restyle(machine, change).then(
        () => {
          void load();
          onChanged();
        },
        (e: unknown) => setError(String(e instanceof Error ? e.message : e)),
      );
    },
    [machine, load, onChanged],
  );

  const pickColor = useCallback(
    (index: number, hex: string) => {
      if (!choices) return;
      // The whole palette every time, because five colors is the only
      // way a palette reaches the library (`Node::restyle`) — pressing
      // 随机配色 and dragging one well travel one path.
      const colors = choices.colors.map((c, i) => (i === index ? hex : c));
      apply({ colors });
    },
    [choices, apply],
  );

  // 随机配色: the next factory set, wrapping (docs/MORE.md 第二轮). The five
  // wells beside it are the answer to which set you landed on. A palette
  // nobody recognises (a slot edited by hand) starts the walk at the
  // first set — the press has to do something visible.
  const shuffle = useCallback(() => {
    if (!choices || choices.palettes.length === 0) return;
    const at = choices.palettes.findIndex((p) => p.key === choices.preset);
    const next = choices.palettes[(at + 1) % choices.palettes.length];
    apply({ colors: next.colors });
  }, [choices, apply]);

  return (
    <div data-face-settings className="flex min-w-0 flex-col gap-group">
      {/* The subject, large: what every row below changes. A blank disc
          of the same size until the node has answered, so the form does
          not jump down under the cursor when it does. */}
      <div data-face-preview className="flex justify-center">
        {choices ? (
          <AvatarFace face={choices.face} className="size-avatar-xl" />
        ) : (
          <span aria-hidden className="size-avatar-xl rounded-full bg-fill" />
        )}
      </div>
      <SheetForm>
        {children}
        {choices && (
          <>
            <FormLabel>{avatar.axis_variant}</FormLabel>
            <SegmentedControl
              label={avatar.axis_variant}
              value={choices.variant}
              onChange={(v) => apply({ variant: v })}
              mark={{ "data-face-axis": "variant" }}
              className="justify-self-start"
              options={choices.variants.map((v) => ({
                key: v.key,
                label: faceWord(v.key),
                mark: { "data-face-option": v.key, "data-axis": "variant" },
              }))}
            />
            <FormLabel>{avatar.axis_palette}</FormLabel>
            <div className="flex min-w-0 flex-wrap items-center gap-in">
              {/* Pressing a well opens the system picker (`<input
                  type="color">`, macOS's own panel on WebKit; `npm run
                  wk` measures that it opens and changes the value). */}
              <div data-face-wells aria-label={avatar.axis_palette} role="group" className="flex items-center gap-in">
                {choices.colors.map((c, i) => (
                  <ColorWell
                    key={i}
                    index={i}
                    value={c}
                    on={editing === i}
                    onOpen={() => setEditing(i)}
                    onCommit={(hex) => pickColor(i, hex)}
                  />
                ))}
              </div>
              <Button data-face-shuffle onClick={shuffle}>
                {gui.face_shuffle}
              </Button>
            </div>
            <FormLabel>{avatar.axis_shape}</FormLabel>
            <SegmentedControl
              label={avatar.axis_shape}
              value={choices.shape}
              onChange={(v) => apply({ shape: v })}
              mark={{ "data-face-axis": "shape" }}
              className="justify-self-start"
              options={choices.shapes.map((sh) => ({
                key: sh.key,
                label: faceWord(sh.key),
                mark: { "data-face-option": sh.key, "data-axis": "shape" },
              }))}
            />
          </>
        )}
      </SheetForm>
      {error && (
        <p data-dialog-error className="text-callout text-destructive">
          {error}
        </p>
      )}
    </div>
  );
}
