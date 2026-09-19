// 安全 (docs/MORE.md 安全).
//
// **Four sections, and every one of them is a thing khor already does.**
// The lock is `khor lock`, the web key is `khor web --new`, showing a
// machine the door is `khor forget`, and the reset is `khor id --reset`.
// Nothing here decides anything: the floor on a PIN, the ladder of idle
// timeouts, the refusal to switch the lock on with no key to it and the
// machine-name confirmation are all the node's, so this page and the
// terminal cannot come to mean different things by the same act.
//
// **What is not here yet**: 危险动作二次确认 (删除文件 / 接管会话 /
// 移出工作区). Those three are config keys — a stored preference the
// three call sites read — and they land with `khor config` in batch D,
// where the rest of the stored settings are. Drawing three switches over
// a store that has no such keys would be the 灰着的开关 docs/MORE.md
// refuses.
import { useCallback, useEffect, useState } from "react";

import {
  fetchWebKey,
  forgetDevice,
  resetIdentity,
  rotateWebKey,
  setLock,
  setLockPin,
  setLockTimeout,
  type DeviceRow,
  type LockFacts,
} from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { MoreFrame, MoreSection } from "@/views/more/MorePage";

/** One toggle in this app's one control family: a `Button` wearing its
    own state, the way the relay switch and the face options do. There is
    no checkbox in `ui/`, and this is not the place to start a second
    family. */
function Switch({
  name,
  label,
  on,
  onPress,
}: {
  name: string;
  label: string;
  on: boolean;
  onPress: () => void;
}) {
  return (
    <Button
      variant="ghost"
      size="sm"
      data-security-switch={name}
      data-on={on}
      aria-pressed={on}
      onClick={onPress}
      className={cn(
        // A ring in both states: off is the state this ships in and the
        // state somebody has to discover it in (`NetSettings`'s own
        // measurement, 2026-09-18).
        // Both axes: 「永不」 is two characters, and `px-2` around them
        // is 38 wide — under the floor on the axis nobody thinks to
        // check (the 修 button on 自检 learned the same thing).
        "min-h-touch min-w-touch self-start rounded-md px-2 text-xs",
        on ? "text-primary ring-2 ring-primary" : "text-muted-foreground ring-1 ring-border",
      )}
    >
      {label}
    </Button>
  );
}

export function SecurityPage({
  onClose,
  lock,
  onLockChanged,
  onLockNow,
  devices,
  onForgot,
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
  devices: DeviceRow[];
  onForgot: () => void;
}) {
  const [error, setError] = useState<string | null>(null);
  const [pin, setPin] = useState("");
  const [key, setKey] = useState<string | null>(null);
  const [showKey, setShowKey] = useState(false);
  const [rotated, setRotated] = useState(false);
  const [typedName, setTypedName] = useState("");

  const me = devices.find((d) => d.me) ?? null;
  const others = devices.filter((d) => !d.me);

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
        <div data-security-error className="text-sm text-destructive">
          {error}
        </div>
      )}

      <MoreSection name="lock" label={gui.lock_section}>
        {/* **Nothing is drawn where the store could not be read.** The
            ladder and the floor are the node's, and inventing either
            here would be this page offering a rung `khor lock` refuses. */}
        {lock === null ? (
          <p className="text-sm text-muted-foreground">{gui.backend_unreachable}</p>
        ) : (
          <>
            <Switch
              name="lock"
              label={gui.lock_switch}
              on={lock.on}
              onPress={() => after(setLock(!lock.on))}
            />
            <p className="text-xs text-muted-foreground">{gui.lock_switch_note}</p>

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
            <p className="text-xs text-muted-foreground">{gui.lock_pin_floor(lock.pin_min)}</p>

            {/* The ladder the node keeps, painted in its order — a page
                with rungs of its own would offer a setting `khor lock
                timeout` refuses by name. */}
            <h3 className="mt-2 text-sm font-label text-muted-foreground">{gui.lock_idle}</h3>
            <div className="flex flex-wrap gap-1">
              {lock.timeouts.map((m) => (
                <Switch
                  key={m}
                  name={`timeout-${m}`}
                  label={m === 0 ? gui.lock_idle_never_pick : gui.lock_idle_pick(m)}
                  on={lock.timeout_min === m}
                  onPress={() => after(setLockTimeout(m))}
                />
              ))}
            </div>

            {lock.on && (
              <Button
                variant="outline"
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

      <MoreSection name="web-key" label={gui.web_key_section}>
        {key === null ? (
          <p className="text-sm text-muted-foreground">{gui.web_key_none}</p>
        ) : (
          <div className="flex flex-wrap items-center gap-2">
            {/* **Masked until somebody asks.** This page can be on a
                screen somebody else is looking at, or in a recording;
                a key that is simply printed is a key that leaks by
                being opened. */}
            <span data-web-key className="min-w-0 flex-1 truncate font-mono text-sm">
              {showKey ? key : "•".repeat(key.length)}
            </span>
            <Button
              variant="ghost"
              size="sm"
              data-web-key-show
              onClick={() => setShowKey((was) => !was)}
              className="min-h-touch"
            >
              {showKey ? gui.web_key_hide : gui.web_key_show}
            </Button>
          </div>
        )}
        <Button
          variant="outline"
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
          <p data-web-key-rotated className="text-xs text-muted-foreground">
            {gui.web_key_rotated}
          </p>
        )}
      </MoreSection>

      <MoreSection name="paired" label={gui.forget_section}>
        {others.length === 0 ? (
          <p className="text-sm text-muted-foreground">{gui.identity_alone}</p>
        ) : (
          <ul className="flex flex-col">
            {others.map((d) => (
              <li
                key={d.id}
                data-security-machine={d.id}
                className="flex min-w-0 items-center gap-row border-b py-1 text-sm last:border-b-0"
              >
                <MachineAvatar face={d.face} className="size-avatar flex-none" />
                <span className="min-w-0 flex-1 truncate">{d.shown}</span>
                <Button
                  variant="ghost"
                  size="sm"
                  data-security-forget={d.id}
                  onClick={() => {
                    setError(null);
                    forgetDevice(d.name).then(onForgot, (e: unknown) =>
                      setError(String(e instanceof Error ? e.message : e)),
                    );
                  }}
                  className="min-h-touch flex-none"
                >
                  {gui.forget_put_out}
                </Button>
              </li>
            ))}
          </ul>
        )}
        {/* The whole cost in one clause, and **the same clause the row
            menu's confirm uses** — it is one act, so it is one sentence
            (`forget-machine-confirm`). */}
        <p className="text-xs text-muted-foreground">{gui.forget_machine_confirm}</p>
      </MoreSection>

      <MoreSection name="reset" label={gui.reset_section}>
        {/* Red, because this is the one thing on this page that cannot
            be undone. The machine's own name is the confirmation and
            **the node is what checks it** — a rule here as well would be
            two rules about one irreversible act. */}
        <p data-reset-warn className="text-sm text-destructive">
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
                () => onForgot(),
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
