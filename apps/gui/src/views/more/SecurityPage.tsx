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
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";
import { ConfigSections } from "@/views/more/ConfigRows";
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

  return (
    <MoreFrame page="security" onClose={onClose}>
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
              onToggle={() => after(setLock(!lock.on))}
            />

            <div className="flex flex-wrap items-center gap-2">
              <Input
                data-security-pin
                type="password"
                inputMode="numeric"
                autoComplete="off"
                aria-label={lock.has_pin ? gui.lock_change_pin : gui.lock_set_pin}
                placeholder={lock.has_pin ? gui.lock_change_pin : gui.lock_set_pin}
                value={pin}
                onChange={(e) => setPin(e.target.value)}
                className="min-h-touch min-w-0 flex-1"
              />
              <Button
                data-security-pin-save
                // The floor is the node's number, carried on the answer
                // so this box refuses with the same one the node would
                // (`LockFacts::pin_min`) rather than a copy of it.
                disabled={pin.length < lock.pin_min}
                onClick={() => {
                  after(setLockPin(pin));
                  setPin("");
                }}
                className="min-h-touch"
              >
                {gui.lock_pin_save}
              </Button>
              {lock.has_pin && (
                <Button
                  variant="ghost"
                  data-security-pin-clear
                  onClick={() => after(setLockPin(""))}
                  className="min-h-touch"
                >
                  {gui.lock_pin_clear}
                </Button>
              )}
            </div>
            <p className="text-caption text-muted-foreground">{gui.lock_pin_floor(lock.pin_min)}</p>

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
              onPick={(v) => after(setLockTimeout(Number(v)))}
            />

            {lock.on && (
              <Button
                data-security-lock-now
                onClick={onLockNow}
                className="mt-2 min-h-touch self-start"
              >
                {gui.lock_now}
              </Button>
            )}
          </>
        )}
      </MoreSection>

      {/* Which section this is, and which keys are in it, are the
          node's answer — 安全 asks for the rows that say they belong to
          it (`khor_node::config::SECTIONS`). */}
      <ConfigSections page="security" onChanged={onSettingChanged} />

      <MoreSection name="web-key" label={gui.web_key_section}>
        {key === null ? (
          <p className="text-callout text-muted-foreground">{gui.web_key_none}</p>
        ) : (
          <div className="flex flex-wrap items-center gap-2">
            {/* **Masked until somebody asks.** This page can be on a
                screen somebody else is looking at, or in a recording;
                a key that is simply printed is a key that leaks by
                being opened. */}
            <span data-web-key className="min-w-0 flex-1 truncate font-mono text-callout">
              {showKey ? key : "•".repeat(key.length)}
            </span>
            <Button
              variant="ghost"
              size="sm"
              data-web-key-show
              onClick={() => setShowKey((was) => !was)}
              className="min-h-touch min-w-touch"
            >
              {showKey ? gui.web_key_hide : gui.web_key_show}
            </Button>
          </div>
        )}
        <Button
          data-web-key-rotate
          onClick={() => {
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
          className="min-h-touch self-start"
        >
          {gui.web_key_rotate}
        </Button>
        {rotated && (
          <p data-web-key-rotated className="text-caption text-muted-foreground">
            {gui.web_key_rotated}
          </p>
        )}
      </MoreSection>

      {/* 配对的机器 left this page on 2026-09-20 (docs/MORE.md 第二轮:
          已配对机器从头像卡和安全页拿掉). It was a list here *and* a
          list on the identity card, and the network page's map is the
          one place a machine lives now — 移除设备 with it, in the card
          of the machine it is about (`MarkPlace`). What is left on 安全
          is what this page is for: the keys and the acts that cost
          something. */}

      <MoreSection name="reset" label={gui.reset_section}>
        {/* Red, because this is the one thing on this page that cannot
            be undone. The machine's own name is the confirmation and
            **the node is what checks it** — a rule here as well would be
            two rules about one irreversible act. */}
        <p data-reset-warn className="text-callout text-destructive">
          {gui.reset_warn}
        </p>
        <div className="flex flex-wrap items-center gap-2">
          <Input
            data-reset-name
            autoComplete="off"
            aria-label={gui.reset_type_name}
            placeholder={gui.reset_type_name}
            value={typedName}
            onChange={(e) => setTypedName(e.target.value)}
            className="min-h-touch min-w-0 flex-1"
          />
          <Button
            variant="ghost"
            data-reset-do
            // The box is checked here only so the button is not live
            // over an empty field; the answer that counts is the node's
            // (`lock::reset_identity` refuses by name).
            disabled={typedName.trim() !== (me?.shown ?? "") || !me}
            onClick={() => {
              setError(null);
              resetIdentity(typedName).then(
                () => onReset(),
                (e: unknown) => setError(String(e instanceof Error ? e.message : e)),
              );
            }}
            className="min-h-touch text-destructive"
          >
            {gui.reset_do}
          </Button>
        </div>
      </MoreSection>
    </MoreFrame>
  );
}
