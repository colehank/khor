// What this machine looks like to everybody else.
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
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";

import { fetchFaceChoices, restyle, type FaceChoices } from "@/api";
import { AvatarFace } from "@/components/Avatar";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { Avatar } from "@/gen/bindings/Avatar";
import { avatar, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { faceWord } from "@/words";

/**
 * One axis and its heading. The heading names the choice; it does not
 * explain it (docs/UX.md 文案: 装饰性说明归零).
 *
 * `name` is the stable key and `label` the word — separate because the
 * options underneath carry the key too, and verification that had to
 * match a heading by its translated text would be reading the catalog
 * to check the catalog.
 */
function Axis({ name, label, children }: { name: string; label: string; children: ReactNode }) {
  return (
    <div data-face-axis={name} className="flex flex-col gap-2">
      <h2 className="text-sm text-muted-foreground">{label}</h2>
      {children}
    </div>
  );
}

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
        "flex-col gap-1 rounded-md px-2 py-2 text-xs text-muted-foreground",
        // The brand color marks the current one, the same way the rail
        // marks the landing you are on — one mark for "this is where
        // you are", not a second vocabulary. The ring carries it as
        // well as the text, because on this screen the thing being
        // looked at is a picture and the word underneath is small.
        on && "text-primary ring-2 ring-primary",
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
 */
function ColorWell({
  index,
  value,
  onCommit,
}: {
  index: number;
  value: string;
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

  return (
    <input
      ref={ref}
      type="color"
      data-color-slot={index}
      aria-label={avatar.axis_palette_slot(index + 1)}
      value={draft}
      onChange={(e) => setDraft(e.target.value)}
      className="size-ctl-md cursor-pointer rounded-md border bg-transparent p-0"
    />
  );
}

export function FaceSettings({
  open,
  onOpenChange,
  onChanged,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Called after a change took, so the faces elsewhere in the app come
      from the node's new answer rather than from this screen. */
  onChanged: () => void;
}) {
  const [choices, setChoices] = useState<FaceChoices | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(
    () =>
      fetchFaceChoices().then(setChoices, (e: unknown) =>
        setError(String(e instanceof Error ? e.message : e)),
      ),
    [],
  );

  // Read on opening, not on mounting: this machine's style can have been
  // changed from a terminal since the last look, and the screen that
  // shows what you are wearing must not show a remembered answer.
  useEffect(() => {
    if (!open) return;
    setError(null);
    void load();
  }, [open, load]);

  // Two-argument `then`, not `.then().catch()`: the rejection handler
  // must see the restyle failing and nothing else. Chained, a re-read
  // that threw after a change that took would report the change as
  // failed while the machine is already wearing it.
  const apply = useCallback(
    (change: { colors?: string[]; variant?: string; shape?: string }) => {
      setError(null);
      restyle(change).then(
        () => {
          void load();
          onChanged();
        },
        (e: unknown) => setError(String(e instanceof Error ? e.message : e)),
      );
    },
    [load, onChanged],
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

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent data-face-settings>
        <DialogHeader>
          <DialogTitle>{gui.settings}</DialogTitle>
        </DialogHeader>
        {error && (
          <div data-dialog-error className="text-sm text-destructive">
            {error}
          </div>
        )}
        {choices && (
          <div className="flex flex-col gap-5">
            <Axis name="palette" label={avatar.axis_palette}>
              <div className="flex flex-wrap gap-1">
                {choices.palettes.map((p) => (
                  <FaceOption
                    key={p.key}
                    axis="palette"
                    option={p.key}
                    label={faceWord(p.key)}
                    face={p.face}
                    // Nothing is marked once a slot has been changed by
                    // hand, and that is the honest answer: a nearest
                    // match would light a button nobody pressed.
                    on={choices.preset === p.key}
                    onPick={() => apply({ colors: p.colors })}
                  />
                ))}
              </div>
              <div className="flex flex-wrap gap-2">
                {choices.colors.map((c, i) => (
                  <ColorWell key={i} index={i} value={c} onCommit={(hex) => pickColor(i, hex)} />
                ))}
              </div>
            </Axis>

            <Axis name="variant" label={avatar.axis_variant}>
              <div className="flex flex-wrap gap-1">
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
            </Axis>

            <Axis name="shape" label={avatar.axis_shape}>
              <div className="flex flex-wrap gap-1">
                {choices.shapes.map((s) => (
                  <FaceOption
                    key={s.key}
                    axis="shape"
                    option={s.key}
                    label={faceWord(s.key)}
                    face={s.face}
                    on={choices.shape === s.key}
                    onPick={() => apply({ shape: s.key })}
                  />
                ))}
              </div>
            </Axis>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
