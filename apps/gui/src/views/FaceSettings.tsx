// What this machine looks like to everybody else — the 设置 page's first
// section (docs/MORE.md 设置: 「**这台机器的脸**、**中继**：现有
// `FaceSettings` 两节原样搬来」).
//
// **This was the whole settings screen, and it was a dialog.** It is a
// section of a page now (`views/more/SettingsPage`), and what that
// changed is the frame and nothing else: the axes, the wells, the
// no-state-of-its-own rule and the network section under it are
// untouched. The head of the dialog it used to be said the rail has
// exactly one 设置 and a second entry point would have to be invented —
// that is still true, and the door is now a menu item rather than a
// rail seat.
//
// **Mounting is opening.** The dialog stayed mounted with `open` false
// and re-read on every opening; the page is drawn only while it is on
// screen, so the mount *is* the opening and the prop that said so is
// gone (`NetSettings` did the same, one file over).
//
// **Nothing here derives a face.** Every swatch on this screen is an
// `Avatar` the node painted (`khor_gui_core::face_choices`, one call for
// the whole surface), handed to the same `AvatarFace` brush the list
// rows use. A settings screen that drew its own previews would be the
// second painter `khor_core::avatar` exists to prevent, and it would be
// the worst place for one: a preview is the only evidence anybody has
// before pressing, so a preview that lies is a choice made on a picture
// that never appears.
//
// The screen holds no style state either. Pressing an option writes
// through `restyle` and re-reads — so what is on screen is what the node
// says, and "did it take" is never a question this layer answers from
// memory. It is also why every face in the app moves in the same frame:
// the caller refetches rows and devices together, and all of them are
// painted from that one answer.
import { useCallback, useEffect, useRef, useState } from "react";

import { fetchFaceChoices, restyle, type FaceChoices } from "@/api";
import { AvatarFace } from "@/components/Avatar";
import { Button } from "@/components/ui/button";
import type { Avatar } from "@/gen/bindings/Avatar";
import { avatar, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { faceWord } from "@/words";
import { MoreFold } from "@/views/more/MorePage";


/*
 * **`Axis` retired 2026-09-21.** It drew a heading over each row of
 * options. The editor the user drew has none — a big face, a row of
 * styles, a row of colours, one button (docs/MORE.md 第二轮) — and a
 * word over pictures that say what they are is the decorative
 * explanation docs/UX.md zeroes out. The names did not vanish with it:
 * each row carries its own `aria-label` from the same catalog keys, so
 * a screen reader still hears 画法 and 色板.
 */

/**
 * One option: this machine's face under it, and its word.
 *
 * **The swatch is the size a face is worn at** — `size-avatar`, the same
 * as a list row's — and there is deliberately no larger preview
 * anywhere on this screen. Judging a face at a size it is never shown at
 * is choosing on evidence you will not get: blow marble up and its two
 * blurred veins separate into shapes you can weigh, and not one of those
 * shapes is on the screen where the face actually lands.
 *
 * A `Button` from `ui/`, laid out the way the rail's items are. There is
 * one control family in this app and this is not the place to start a
 * second one.
 */
function FaceOption({
  axis,
  option,
  label,
  face,
  on,
  onPick,
}: {
  /** Which axis this belongs to — a verification anchor, and the reason
      three rows of look-alike buttons can be told apart. */
  axis: string;
  option: string;
  label: string;
  face: Avatar;
  on: boolean;
  onPick: () => void;
}) {
  return (
    <Button
      variant="ghost"
      // Sized by its content: three fixed control heights and a mark
      // stacked over a word do not fit each other. See the `auto` note
      // on `ui/button` for why this cannot be an `h-auto` from here.
      size="auto"
      data-face-option={option}
      data-axis={axis}
      data-on={on}
      // Pressed rather than selected: these are toggles in a set, and a
      // screen reader should say which one this machine is wearing.
      aria-pressed={on}
      onClick={onPick}
      className={cn(
        "flex-col gap-1 rounded-md px-2 py-2 text-caption text-muted-foreground",
        // **The picked one is the selected row's grey block** — the
        // `fill-strong` a selected list row wears (`radius-md`, the
        // selection radius), with its word in full `ink`: one language
        // for "this is the one" across the app, and one that reads on
        // any face. A block rather than a ring because the thing picked
        // is a picture: a ring hugs its square button, not the round
        // face inside it, and a block holds the face and its word as one
        // thing, the way Finder shows a picked icon.
        //
        // ⚠️ 墓碑 (2026-09-28): it was `text-primary ring-2 ring-primary`
        // — an olive ring and an olive word. `brand-strong` has exactly
        // three posts (the open switch, the focus ring, a dialog's one
        // primary button) and a picked option is not one of them.
        on && "bg-fill-strong text-foreground hover:bg-fill-strong",
      )}
    >
      <AvatarFace face={face} className="size-avatar" />
      <span>{label}</span>
    </Button>
  );
}

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
 * app; the seam gets cut when a second screen needs one, which is the
 * same rule the node's adaptors follow.
 *
 * **The only control in the app whose target is `size-touch` rather than
 * a `min-h-touch` floor over a smaller graphic**, and it is worth saying
 * why, because the rule in `tokens.css` reads the other way (`--touch`
 * sizes hit areas, never glyphs). That rule exists because a button's
 * glyph and its target are two different things, so growing the target
 * must not grow the picture. **Here they are one thing.** There is no
 * glyph: the well *is* the color, the browser paints it across the whole
 * element, and it is what a finger aims at. A 30 swatch inside a 44
 * hit area would be the failure that rule is written against from the
 * other side — a target the eye was shown at one size and the finger
 * finds at another — and there is no press state to paint on the grown
 * box either, because pressing hands off to the OS picker.
 *
 * Measured before it was chosen: at 390 the dialog is 358 wide with 24
 * of padding, so the axis has 308, and five 44s with the row's `gap-2`
 * come to 252. One row, 56 to spare — the `flex-wrap` is still there and
 * still unused, which is the answer to「does it fit on a phone」rather
 * than a hope.
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
  // The node is the truth: pressing a factory palette replaces all five,
  // and each well has to follow rather than keep showing what it was.
  useEffect(() => setDraft(value), [value]);

  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const commit = () => onCommit(el.value);
    el.addEventListener("change", commit);
    return () => el.removeEventListener("change", commit);
  }, [onCommit]);

  // **A dot of the colour, and nothing around it** — the row the user
  // drew is 「一排五个色点」. The browser's own well is a square inset in
  // a bordered box; `app.css` takes the inset and the inner border off,
  // so the colour fills the whole disc, and `avatar-edge` is the one
  // hairline — the same edge a face wears, so a pale slot still stands
  // off a pale card.
  //
  // **The well the picker is open for wears an ink ring** — what an
  // active `NSColorWell` shows, so the eye can go from the colour panel
  // back to the slot it is changing. Ink, not `brand-strong`: it marks
  // a choice, and olive has three posts, none of them this; the keyboard
  // focus ring stays `ring`.
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
        "size-[var(--touch)] cursor-pointer appearance-none rounded-full border border-avatar-edge bg-transparent p-0",
        on && "outline-2 outline-offset-2 outline-foreground",
        "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring",
      )}
    />
  );
}

export function FaceSettings({
  machine = "",
  onChanged,
}: {
  /** Which machine's avatar this edits — a device id, and **empty is
      this one**. 「头像设置这, 要支持对每台机器的修改」 (the user,
      2026-09-20): a machine row's 更改头像… opens this same editor on
      that machine (the map's card did, until 2026-09-28), and a face is
      全网同一份 — anyone may set anyone's. */
  machine?: string;
  /** Called after a change took, so the faces elsewhere in the app come
      from the node's new answer rather than from this screen. */
  onChanged: () => void;
}) {
  const [choices, setChoices] = useState<FaceChoices | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Which well the picker was opened from (`ColorWell`'s ring). It lets
  // go the way an active colour well does: when another one is opened,
  // or a press lands anywhere that is not a well, or Escape. **Not on
  // commit**: the system panel stays up after a pick, and the ring is
  // how the eye finds the slot it is still changing.
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

  // Read on every opening — which is every mount now that this is a
  // page: this machine's style can have been changed from a terminal
  // since the last look.
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
      // way a palette reaches the library (`Node::restyle`) — pressing a
      // factory set and dragging one well travel one path, so there is
      // no second one that could behave differently.
      const colors = choices.colors.map((c, i) => (i === index ? hex : c));
      apply({ colors });
    },
    [choices, apply],
  );

  // 「换一组配色」: the next factory set, wrapping. **A button rather than
  // the row of seven swatches this used to be** — 「出厂色板做成一个「换一组
  // 配色」的按钮, 按一下换一组」 (docs/MORE.md 第二轮). Nothing has to
  // say which set you landed on, because the five wells directly above
  // it are that answer.
  //
  // A palette nobody recognises (a slot edited by hand, so `preset` is
  // `null`) starts the walk at the first set rather than nowhere: the
  // press has to do something visible or it is the 「做了但没变化」
  // docs/UX.md refuses.
  const shuffle = useCallback(() => {
    if (!choices || choices.palettes.length === 0) return;
    const at = choices.palettes.findIndex((p) => p.key === choices.preset);
    const next = choices.palettes[(at + 1) % choices.palettes.length];
    apply({ colors: next.colors });
  }, [choices, apply]);

  return (
    <div data-face-settings className="flex flex-col gap-group">
      {error && (
        <div data-dialog-error className="text-callout text-destructive">
          {error}
        </div>
      )}
      {choices && (
        <div className="flex flex-col items-center gap-4">
          {/* The big face. **The one place in this app a face is drawn
              larger than it is worn**, and it is the editor's subject
              rather than a preview: what the other rows change, this
              shows. The swatches stay at `--avatar` for the reason the
              old head gave — judging a face at a size it is never shown
              at is choosing on evidence you will not get. */}
          <AvatarFace face={choices.face} className="size-avatar-xl" />

          {/* 画法, as thumbnails. No heading: the pictures say what the
              choice is, and a word over them would be the decorative
              explanation docs/UX.md zeroes out. The name is still said
              — to a screen reader, through the row's label. */}
          <div
            data-face-axis="variant"
            aria-label={avatar.axis_variant}
            role="group"
            className="flex flex-wrap justify-center gap-1"
          >
            {choices.variants.map((v) => (
              <FaceOption
                key={v.key}
                axis="variant"
                option={v.key}
                label={faceWord(v.key)}
                face={v.face}
                on={choices.variant === v.key}
                onPick={() => apply({ variant: v.key })}
              />
            ))}
          </div>

          {/* The five slots, and pressing one opens the system picker
              (`ColorWell` — `<input type="color">`, which on WebKit is
              macOS's own panel; `npm run wk` measures that it really
              opens and really changes the value). */}
          <div
            data-face-wells
            aria-label={avatar.axis_palette}
            role="group"
            className="flex flex-wrap justify-center gap-2"
          >
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

          <Button
            data-face-shuffle
            onClick={shuffle}
            className="min-h-touch"
          >
            {gui.face_shuffle}
          </Button>

          {/* 形状 is folded. 「不是所有的设置都要去铺在界面上」 — three
              rounded corners are the rarest of the three axes and the
              one nobody arrives looking for, so the page's default shape
              is the face, the styles, the colours and one button. */}
          <MoreFold name="face-advanced" label={gui.advanced}>
            <div
              data-face-axis="shape"
              aria-label={avatar.axis_shape}
              role="group"
              className="flex flex-wrap gap-1"
            >
              {choices.shapes.map((sh) => (
                <FaceOption
                  key={sh.key}
                  axis="shape"
                  option={sh.key}
                  label={faceWord(sh.key)}
                  face={sh.face}
                  on={choices.shape === sh.key}
                  onPick={() => apply({ shape: sh.key })}
                />
              ))}
            </div>
          </MoreFold>
        </div>
      )}
      {/* 中继 stood here until 2026-09-20 and is on 网络 now, behind
          高级 (docs/MORE.md 第二轮: 中继从通用搬到网络页的「高级」里).
          It was never a face axis; it was here because 设置 was the one
          door, and it is on the network page because that is what it is
          about. */}
    </div>
  );
}
