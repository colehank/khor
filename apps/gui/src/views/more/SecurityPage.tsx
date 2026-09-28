// 安全 (docs/MORE.md 安全).
//
// **Five sections, and every one of them is a thing khor already does.**
// The lock is `khor lock`, the two 二次确认 switches are `khor config`,
// the web key is `khor web --new`, showing a machine the door is
// `khor forget`, and the reset is `khor id --reset`.
// Nothing here decides anything: the floor on a PIN, the ladder of idle
// timeouts, the refusal to switch the lock on with no key to it and the
// machine-name confirmation are all the node's, so this page and the
// terminal cannot come to mean different things by the same act.
//
// **危险动作二次确认 is two switches, not the three docs/MORE.md lists.**
// 删除文件 already has a 「不再提醒」 tick of its own, kept per face in
// `localStorage` because it is 「一个习惯,不是一个关于网络的决定」
// (`RemoveDialog`), and the verification side ruled on 2026-09-20 that it
// stays there: a config key would turn one browser's habit into one
// answer a phone and a desktop share. The other two are `khor config`
// keys and are drawn from the node's own table (`ConfigSections`), which
// is also why their section heading is not spelled here.
import { useCallback, useEffect, useState } from "react";

import {
  fetchWebKey,
  resetIdentity,
  rotateWebKey,
  setLock,
  setLockPin,
  setLockTimeout,
  type DeviceRow,
  type LockFacts,
} from "@/api";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";
import { ConfigSections } from "@/views/more/ConfigRows";
import { ActionRow } from "@/components/settings/ActionRow";
import { PopupRow } from "@/components/settings/PopupRow";
import { SwitchRow } from "@/components/settings/SwitchRow";
import { MoreFrame, MoreSection } from "@/views/more/MorePage";

export function SecurityPage({
  onClose,
  lock,
  onLockChanged,
  onLockNow,
  onSettingChanged,
  devices,
  onReset,
}: {
  onClose: () => void;
  /** What the shell already asked for — **not fetched again here**. Two
      readers of one answer is how two surfaces come to show two
      different locks. */
  lock: LockFacts | null;
  /** The shell re-reads after a change, so what is on screen is what the
      node says rather than what this page just sent. */
  onLockChanged: () => void;
  onLockNow: () => void;
  /** A 二次确认 switch changed: the shell re-reads, and the rows that
      read those keys are drawn from that answer. */
  onSettingChanged: () => void;
  /** Only this machine is read off it: 重置身份 wants this machine's
      own name typed back. The list of the others left this page on
      2026-09-20 (see the tombstone below). */
  devices: DeviceRow[];
  /** This machine's identity was reset — every surface repaints, and
      the lock is re-read with it. */
  onReset: () => void;
}) {
  const [error, setError] = useState<string | null>(null);
  const [pin, setPin] = useState("");
  const [key, setKey] = useState<string | null>(null);
  const [showKey, setShowKey] = useState(false);
  const [rotated, setRotated] = useState(false);
  const [typedName, setTypedName] = useState("");
  const [pinning, setPinning] = useState(false);
  // The PIN window was opened by the lock switch rather than the PIN
  // row: saving sets the PIN **and then turns the lock on**, which is
  // what the person pressed the switch for.
  const [arming, setArming] = useState(false);
  const [resetting, setResetting] = useState(false);

  const me = devices.find((d) => d.me) ?? null;

  useEffect(() => {
    void fetchWebKey().then(setKey, (e: unknown) =>
      setError(String(e instanceof Error ? e.message : e)),
    );
  }, []);

  // Two-argument `then` everywhere below, and the re-read is never in
  // the chain: a read that threw after a change that took would report
  // the change as failed while the machine is already wearing it
  // (`FaceSettings`'s rule).
  const after = useCallback(
    (p: Promise<unknown>) => {
      setError(null);
      p.then(
        () => onLockChanged(),
        (e: unknown) => setError(String(e instanceof Error ? e.message : e)),
      );
    },
    [onLockChanged],
  );

  // Saving from the PIN window. From the switch it is two acts in the
  // node's order — the PIN, then the lock, which `khor_node::lock`
  // refuses without one — and the facts are re-read either way, so a
  // lock that refused still shows the PIN that took.
  const savePin = (typed: string) => {
    setPin("");
    setPinning(false);
    setArming(false);
    if (!arming || lock === null || lock.on) {
      after(setLockPin(typed));
      return;
    }
    setError(null);
    setLockPin(typed)
      .then(() => setLock(true))
      .then(
        () => onLockChanged(),
        (e: unknown) => {
          setError(String(e instanceof Error ? e.message : e));
          onLockChanged();
        },
      );
  };

  return (
    <MoreFrame
      page="security"
      onClose={onClose}
      // **「立即锁定」 is this page's button, at R0's right end** (D9;
      // 2026-09-28). It is not a setting — nothing about it is kept —
      // and it is not any row's act: the switch above says whether the
      // lock exists, this uses it. ⚠️ 墓碑: a lone push button under the
      // idle row, left-aligned in the card — the shape 「重新生成」 had
      // just been moved out of. Only while the lock is on: before that
      // there is nothing to lock with (`khor_node::lock` refuses).
      actions={
        lock?.on ? (
          <Button
            size="sm"
            data-security-lock-now
            onClick={onLockNow}
            className="min-h-touch min-w-touch flex-none"
          >
            {gui.lock_now}
          </Button>
        ) : undefined
      }
    >
      {error && (
        <div data-security-error className="text-callout text-destructive">
          {error}
        </div>
      )}

      <MoreSection name="lock" label={gui.lock_section}>
        {/* **Nothing is drawn where the store could not be read.** The
            ladder and the floor are the node's, and inventing either
            here would be this page offering a rung `khor lock` refuses. */}
        {lock === null ? (
          <p className="text-callout text-muted-foreground">{gui.backend_unreachable}</p>
        ) : (
          <>
            <SwitchRow
              name="lock"
              mark={{ "data-security-switch": "lock" }}
              label={gui.lock_switch}
              hint={gui.lock_switch_note}
              on={lock.on}
              onToggle={() => {
                // **With no PIN the switch asks for one first.** The node
                // refuses a lock with no key (`khor_node::lock::set_on`),
                // and the PIN row below is not live until the lock is
                // on — so this is the one road to the lock, and saving
                // the PIN takes it the rest of the way (`savePin`).
                if (!lock.on && !lock.has_pin) {
                  setPin("");
                  setArming(true);
                  setPinning(true);
                  return;
                }
                after(setLock(!lock.on));
              }}
            />

            {/* **The PIN is an action row** (components/SettingsGroup):
                its act asks for the digits, so it opens a window and
                wears the ellipsis. ⚠️ 墓碑 (2026-09-27): a password box
                and two buttons stood in the card here, a form drawn in
                a row's place.

                **Its state is the row's value** — 已设置 / 未设置 left
                of the button — not a note under 「PIN」 saying 「PIN 设过
                了」 (⚠️ 墓碑 2026-09-28, the row said PIN twice).

                **It and the idle row depend on the lock**, the way macOS
                greys what sits under a switch that is off: there, not
                live, until the lock is on (`check-face` 4I). */}
            <ActionRow
              name="lock-pin"
              label={gui.lock_pin}
              value={lock.has_pin ? gui.lock_pin_is_set : gui.lock_pin_not_set}
              actionLabel={lock.has_pin ? gui.lock_change_pin_ask : gui.lock_set_pin_ask}
              disabled={!lock.on}
              onPress={() => {
                setPin("");
                setArming(false);
                setPinning(true);
              }}
              mark={{ "data-security-pin-open": "" }}
            />

            {/* The ladder the node keeps, painted in its order — a page
                with rungs of its own would offer a setting `khor lock
                timeout` refuses by name. */}
            <PopupRow
              name="lock-timeout"
              label={gui.lock_idle}
              value={String(lock.timeout_min)}
              choices={lock.timeouts.map((m) => ({
                value: String(m),
                label: m === 0 ? gui.lock_idle_never_pick : gui.lock_idle_pick(m),
              }))}
              disabled={!lock.on}
              onPick={(v) => after(setLockTimeout(Number(v)))}
            />

          </>
        )}
      </MoreSection>

      {/* Which section this is, and which keys are in it, are the
          node's answer — 安全 asks for the rows that say they belong to
          it (`khor_node::config::SECTIONS`). */}
      <ConfigSections page="security" onChanged={onSettingChanged} />

      <MoreSection name="web-key" label={gui.web_key_section}>
        {/* **One row: the key, and 换一把 as its trailing act**, like
            every other settings row. ⚠️ 墓碑 (2026-09-28): 换一把 was a
            bare button under the row, on a line of its own at the left.

            **Masked until somebody asks.** This page can be on a screen
            somebody else is looking at, or in a recording; a key that is
            simply printed is a key that leaks by being opened. 显示 /
            遮住 belongs to the key, so it sits beside it in the title —
            the way a secure field carries its own reveal — and the
            row's one act stays at the trailing edge. What 换一把 did is
            said under the key, as the row's note. */}
        <ActionRow
          name="web-key"
          label={
            key === null ? (
              <span className="text-muted-foreground">{gui.web_key_none}</span>
            ) : (
              <span className="flex min-w-0 items-center gap-in">
                <span data-web-key className="min-w-0 truncate font-mono">
                  {showKey ? key : "•".repeat(key.length)}
                </span>
                <Button
                  variant="ghost"
                  size="sm"
                  data-web-key-show
                  aria-pressed={showKey}
                  onClick={() => setShowKey((was) => !was)}
                  className="min-h-touch min-w-touch flex-none"
                >
                  {showKey ? gui.web_key_hide : gui.web_key_show}
                </Button>
              </span>
            )
          }
          hint={rotated ? <span data-web-key-rotated>{gui.web_key_rotated}</span> : undefined}
          actionLabel={gui.web_key_rotate}
          onPress={() => {
            setError(null);
            rotateWebKey().then(
              (fresh) => {
                setKey(fresh);
                setShowKey(false);
                setRotated(true);
              },
              (e: unknown) => setError(String(e instanceof Error ? e.message : e)),
            );
          }}
          mark={{ "data-web-key-rotate": "" }}
        />
      </MoreSection>

      {/* 配对的机器 left this page on 2026-09-20 (docs/MORE.md 第二轮:
          已配对机器从头像卡和安全页拿掉). It was a list here *and* a
          list on the identity card, and the network page's map is the
          one place a machine lives now — 移除设备 with it, in the card
          of the machine it is about (`MarkPlace`). What is left on 安全
          is what this page is for: the keys and the acts that cost
          something. */}

      {/* **A dangerous act is an action row with an ellipsis, and the
          real confirmation is in a window** (components/SettingsGroup:
          「危险的动作（清除、重置）用 action 行 + 一个带省略号的按钮」).
          ⚠️ 墓碑 (2026-09-27): the name box and the red button stood in
          the card, a form in a row's place. */}
      {/* No group name: the row's own title says it, and a heading
          over a one-row card saying the same words is the same fact
          twice. */}
      <MoreSection name="reset">
        <ActionRow
          name="reset"
          label={gui.reset_section}
          hint={gui.reset_warn}
          actionLabel={gui.reset_ask}
          disabled={!me}
          onPress={() => {
            setError(null);
            setTypedName("");
            setResetting(true);
          }}
          mark={{ "data-reset-open": "" }}
        />
      </MoreSection>

      {lock !== null && (
        <Dialog
          open={pinning}
          onOpenChange={(open) => {
            setPinning(open);
            if (!open) setArming(false);
          }}
        >
          <DialogContent data-security-pin-dialog>
            <DialogHeader>
              <DialogTitle>{lock.has_pin ? gui.lock_change_pin : gui.lock_set_pin}</DialogTitle>
            </DialogHeader>
            <p className="text-callout text-muted-foreground">{gui.lock_pin_floor(lock.pin_min)}</p>
            <form
              onSubmit={(e) => {
                e.preventDefault();
                if (pin.length < lock.pin_min) return;
                savePin(pin);
              }}
            >
              <Input
                data-security-pin
                type="password"
                inputMode="numeric"
                autoComplete="off"
                aria-label={lock.has_pin ? gui.lock_change_pin : gui.lock_set_pin}
                value={pin}
                onChange={(e) => setPin(e.target.value)}
                className="min-h-touch w-full"
              />
            </form>
            <DialogFooter>
              {/* Taking the PIN away is the other thing this window can
                  do, on the left and plain — the one primary act is 存下. */}
              {lock.has_pin && (
                <Button
                  variant="ghost"
                  data-security-pin-clear
                  onClick={() => {
                    after(setLockPin(""));
                    setPinning(false);
                  }}
                  className="mr-auto"
                >
                  {gui.lock_pin_clear}
                </Button>
              )}
              <Button
                onClick={() => {
                  setPinning(false);
                  setArming(false);
                }}
              >
                {gui.delete_keep}
              </Button>
              <Button
                variant="primary"
                data-security-pin-save
                // The floor is the node's number, carried on the answer
                // so this box refuses with the same one the node would
                // (`LockFacts::pin_min`) rather than a copy of it.
                disabled={pin.length < lock.pin_min}
                onClick={() => savePin(pin)}
              >
                {gui.lock_pin_save}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      )}

      <Dialog open={resetting} onOpenChange={setResetting}>
        <DialogContent data-reset-dialog>
          <DialogHeader>
            <DialogTitle>{gui.reset_title(me?.shown ?? "")}</DialogTitle>
          </DialogHeader>
          {/* Red, because this is the one thing on this page that cannot
              be undone. The machine's own name is the confirmation and
              **the node is what checks it** — a rule here as well would
              be two rules about one irreversible act. */}
          <p data-reset-warn className="text-callout text-destructive">
            {gui.reset_warn}
          </p>
          <Input
            data-reset-name
            autoComplete="off"
            aria-label={gui.reset_type_name}
            placeholder={gui.reset_type_name}
            value={typedName}
            onChange={(e) => setTypedName(e.target.value)}
            className="min-h-touch w-full"
          />
          {error && (
            <p data-reset-refused className="text-callout text-destructive">
              {error}
            </p>
          )}
          <DialogFooter>
            <Button onClick={() => setResetting(false)}>{gui.delete_keep}</Button>
            <Button
              variant="destructive"
              data-reset-do
              // The box is checked here only so the button is not live
              // over an empty field; the answer that counts is the node's
              // (`lock::reset_identity` refuses by name).
              disabled={typedName.trim() !== (me?.shown ?? "") || !me}
              onClick={() => {
                setError(null);
                resetIdentity(typedName).then(
                  () => {
                    setResetting(false);
                    onReset();
                  },
                  (e: unknown) => setError(String(e instanceof Error ? e.message : e)),
                );
              }}
            >
              {gui.reset_do}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </MoreFrame>
  );
}
