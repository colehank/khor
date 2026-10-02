// The whole window, when this face is locked (docs/MORE.md 安全).
//
// # It is not a layer over the app — the app is not there
//
// 「锁住时：整窗一张锁屏（脸 + 名字 + 解锁），**页面里不留任何会话内容**
// （不是盖一层——卸掉，解锁后再挂）」. The shell returns this *instead of*
// the shell (`App.tsx`), so there is no list behind it, no conversation
// in the document, and nothing to reach with a devtools inspector or a
// screen reader. A cover would have left every one of those readable.
//
// It is also why this file asks the node for nothing but the unlock:
// a lock screen that fetched anything would be putting the answer it
// fetched into a locked page.
//
// # What it says is who this machine is, and nothing else
//
// A face and a name — enough to know which machine you are looking at,
// which is the question somebody has when they walk up to a screen. Not
// how many sessions are waiting, not who last said something: those are
// the contents, and the contents are what is locked.
import { useEffect, useRef, useState } from "react";

import type { DeviceRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";

export function LockScreen({
  me,
  onOpen,
  systemUnlock,
}: {
  /** This machine's row, or `null` before the table has answered — the
      face and the name are painted from it, and nothing else on this
      screen depends on it. */
  me: DeviceRow | null;
  /** Tries a PIN. `false` is a wrong PIN, never a throw. */
  onOpen: (pin: string) => Promise<boolean>;
  /** The app's other key on macOS (Touch ID / the login password).
      Absent everywhere else — a browser has no such thing, and a button
      that refuses is worse than no button. */
  systemUnlock?: () => void;
}) {
  const [pin, setPin] = useState("");
  const [wrong, setWrong] = useState(false);
  const [asking, setAsking] = useState(false);
  const box = useRef<HTMLInputElement>(null);

  // The box has the caret the moment the screen arrives: this screen has
  // exactly one thing to do, and making somebody click into it first is
  // a step that exists for no reason.
  useEffect(() => {
    box.current?.focus();
  }, []);

  const tryIt = async () => {
    if (!pin || asking) return;
    setAsking(true);
    try {
      const took = await onOpen(pin);
      if (!took) {
        setWrong(true);
        setPin("");
        box.current?.focus();
      }
    } catch {
      // khor could not read its own record — a different thing from a
      // wrong PIN, and said as a different thing (`khor_node::lock`).
      setWrong(true);
    } finally {
      setAsking(false);
    }
  };

  return (
    <div
      data-lock-screen
      // The whole window, and its own drag region: with the title bar
      // gone (第八轮 第 58 条) this is the only surface there is, so
      // without one a locked window could not be moved at all.
      //
      // **The one component that marks a drag region itself, by name**
      // (docs/SHELL.md 第十五轮 第 3 条). Everywhere else the shell marks
      // L0, L1 and R0 once (`components/shell.ts`) — but a locked window
      // draws no shell: `App` returns this instead of L and R, so there
      // is nothing of the shell's to carry the handle. `check-face`
      // (第十五轮 e) allows this file and the shell's, and nothing else.
      data-tauri-drag-region="deep"
      className="flex h-dvh flex-col items-center justify-center gap-group bg-background p-pane"
    >
      <MachineAvatar face={me?.face ?? null} className="size-avatar-xl" />
      <div data-lock-name className="text-title-md font-title">
        {me?.shown ?? ""}
      </div>
      <form
        className="flex w-full max-w-measure flex-col items-center gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          void tryIt();
        }}
      >
        <Input
          ref={box}
          data-lock-pin
          type="password"
          // A phone shows the number pad for this, and a PIN is mostly
          // digits — `inputMode` asks for the pad without refusing the
          // letters a longer passphrase has (`khor_node::lock::PIN_MIN`
          // counts characters, not digits).
          inputMode="numeric"
          autoComplete="off"
          aria-label={gui.lock_pin}
          placeholder={gui.lock_pin}
          value={pin}
          onChange={(e) => {
            setPin(e.target.value);
            setWrong(false);
          }}
          className="min-h-touch w-full text-center"
        />
        {wrong && (
          <div data-lock-wrong className="text-callout text-destructive">
            {gui.lock_wrong}
          </div>
        )}
        {/* Drawn once a PIN is typed (F1.9: not greyed). */}
        {pin && (
          <Button type="submit" data-lock-open disabled={asking} aria-busy={asking || undefined} className="min-h-touch w-full">
            {gui.lock_open}
          </Button>
        )}
        {systemUnlock && (
          <Button
            type="button"
            variant="ghost"
            data-lock-system
            onClick={systemUnlock}
            className="min-h-touch w-full"
          >
            {gui.lock_system}
          </Button>
        )}
      </form>
    </div>
  );
}
