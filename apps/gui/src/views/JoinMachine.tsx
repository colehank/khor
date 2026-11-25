import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";

import { openCameraSettings, pair, said, scanTicket, type Ticket } from "@/api";
import { GlassCircle } from "@/components/Glass";
import { IconBack, IconChevronDown, IconFileTray, IconKeyboard } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { TaskActions } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";
import type { JoinNetworkSample } from "@/lib/join-network-lab";
import { revealPairingCamera } from "@/lib/pairing-camera-surface";
import { pairingCodeFromPhoto, pairingIssuer } from "@/lib/pairing-ticket-image";
import "@/styles/pairing.css";

type Step = "scan" | "photos" | "confirm" | "joined";
type CameraProblem = "denied" | "failed" | "wrong-code";
type PhotoState = "idle" | "selecting" | "reading";

function ExampleCode({ code, className }: { code: Ticket["qr"]; className: string }) {
  if (!code) return null;
  return (
    <svg aria-hidden className={className} viewBox={`0 0 ${code.span} ${code.span}`}>
      <path d={code.path} fill="currentColor" />
    </svg>
  );
}

export function JoinMachine({ open, sample, credential, onCredentialChange, onClose, onJoined, inviteHeight, onJoinHeightChange }: {
  open: boolean;
  sample: JoinNetworkSample | null;
  credential: string;
  onCredentialChange: (value: string) => void;
  onClose: () => void;
  onJoined?: () => void;
  inviteHeight: number;
  onJoinHeightChange: (height: number | null) => void;
}) {
  const [step, setStep] = useState<Step>("scan");
  const [expanded, setExpanded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState("");
  const [live, setLive] = useState(false);
  const [problem, setProblem] = useState<CameraProblem | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [visible, setVisible] = useState(!document.hidden);
  const [photoState, setPhotoState] = useState<PhotoState>("idle");
  const [photoError, setPhotoError] = useState<string | null>(null);
  const confirming = useRef(false);
  const surface = useRef<HTMLElement>(null);
  const picker = useRef<HTMLInputElement>(null);
  const attachPicker = useCallback((input: HTMLInputElement | null) => {
    picker.current = input;
    if (!input) return;
    const cancelled = () => setPhotoState("idle");
    input.addEventListener("cancel", cancelled);
    return () => {
      input.removeEventListener("cancel", cancelled);
      picker.current = null;
    };
  }, []);
  const cameraRequest = useRef<AbortController | null>(null);
  const photoRequest = useRef<AbortController | null>(null);
  const picked = sample?.credential === credential;
  const device = picked ? sample?.device : pairingIssuer(credential.trim());
  const scanning = step === "scan" && open && visible && photoState === "idle" && !sample;

  useLayoutEffect(() => {
    onJoinHeightChange(inviteHeight);
    return () => onJoinHeightChange(null);
  }, [inviteHeight, onJoinHeightChange]);
  useEffect(() => {
    const sync = () => setVisible(!document.hidden);
    document.addEventListener("visibilitychange", sync);
    return () => document.removeEventListener("visibilitychange", sync);
  }, []);
  useLayoutEffect(() => {
    if (live && scanning && surface.current) return revealPairingCamera(surface.current);
  }, [live, scanning]);
  useEffect(() => {
    if (!open) photoRequest.current?.abort();
    return () => photoRequest.current?.abort();
  }, [open]);

  const fill = useCallback((ticket: string) => {
    onCredentialChange(ticket);
    setExpanded(false);
    setError(null);
    setStep("confirm");
  }, [onCredentialChange]);
  useEffect(() => {
    if (!scanning) return;
    const request = new AbortController();
    cameraRequest.current = request;
    setProblem(null);
    void scanTicket(() => setLive(true), request.signal).then(read => {
      if (request.signal.aborted) return;
      setLive(false);
      if ("ticket" in read) {
        if (pairingIssuer(read.ticket)) fill(read.ticket);
        else setProblem("wrong-code");
      } else if ("refused" in read) {
        setProblem(read.refused);
      }
    });
    return () => {
      request.abort();
      cameraRequest.current = null;
      setLive(false);
    };
  }, [scanning, attempt, fill]);

  const choosePhoto = () => {
    if (sample) {
      setStep("photos");
      return;
    }
    const input = picker.current;
    if (!input) return;
    cameraRequest.current?.abort();
    setLive(false);
    setPhotoError(null);
    setPhotoState("selecting");
    input.click();
  };

  const readPhoto = async (file: File | undefined) => {
    if (!file) {
      setPhotoState("idle");
      return;
    }
    cameraRequest.current?.abort();
    const request = new AbortController();
    photoRequest.current = request;
    setLive(false);
    setPhotoState("reading");
    try {
      const ticket = await pairingCodeFromPhoto(file, request.signal);
      if (request.signal.aborted) return;
      if (ticket && pairingIssuer(ticket)) fill(ticket);
      else setPhotoError(gui.scan_photo_no_ticket);
    } catch {
      if (!request.signal.aborted) setPhotoError(gui.scan_photo_failed);
    } finally {
      if (!request.signal.aborted) setPhotoState("idle");
    }
  };

  const join = async () => {
    if (!credential.trim() || confirming.current) return;
    confirming.current = true;
    setBusy(true);
    setError(null);
    try {
      setResult(await pair(credential.trim()));
      setStep("joined");
      onJoined?.();
    } catch (failure) {
      setError(said(failure));
    } finally {
      confirming.current = false;
      setBusy(false);
    }
  };

  if (step === "scan" || step === "photos") {
    const photos = step === "photos" && sample;
    const notice = problem === "denied" ? gui.scan_camera_denied
      : problem === "failed" ? gui.scan_no_camera
        : problem === "wrong-code" ? gui.scan_wrong_code : null;
    return (
      <>
        <TaskActions close={<Button onClick={onClose}>{gui.close}</Button>} />
        <section
          ref={surface}
          data-join-camera={step}
          data-camera-live={(live && scanning) || undefined}
          className="pairing-camera flex min-h-0 flex-1 flex-col overflow-hidden rounded-2xl"
        >
          {photos ? (
            <>
              <header className="flex items-center gap-row px-row pt-row">
                <GlassCircle aria-label={gui.back} tip={false} className="pairing-camera-control size-touch"
                  onClick={() => setStep("scan")}>
                  <IconBack className="size-icon-mark" />
                </GlassCircle>
                <span className="text-headline font-label">{gui.scan_photo_library}</span>
              </header>
              <div data-sheet-hold className="flex min-h-0 flex-1 flex-col items-start overflow-y-auto overscroll-contain px-pane py-group">
                <button type="button" data-join-pick-sample onClick={() => fill(photos.credential)}
                  className="glass flex shrink-0 flex-col items-center gap-in rounded-xl p-in text-callout">
                  <ExampleCode code={photos.qr} className="pairing-paper size-28 rounded-md p-in" />
                  {photos.device}
                </button>
              </div>
            </>
          ) : (
            <>
              <div className="pairing-camera-scene grid min-h-0 flex-1 place-content-center">
                <div className="flex min-w-0 flex-col items-center gap-in px-row">
                  {notice || photoState !== "idle" ? (
                    <div className="flex flex-col items-center gap-in text-center text-callout" role="status">
                      <span>
                        {photoState === "reading" ? gui.scan_photo_reading
                          : photoState === "selecting" ? gui.scan_photo_library : notice}
                      </span>
                      {photoState === "idle" && (problem === "denied" ? (
                        <Button variant="ghost" className="glass pairing-camera-control min-h-touch" data-scan-settings
                          onClick={() => void openCameraSettings()}>{gui.scan_open_settings}</Button>
                      ) : (
                        <Button variant="ghost" className="glass pairing-camera-control min-h-touch"
                          onClick={() => setAttempt(value => value + 1)}>{gui.region_retry}</Button>
                      ))}
                    </div>
                  ) : (
                    <>
                      <div className="pairing-camera-finder grid aspect-square place-items-center rounded-2xl">
                        {sample && <button type="button" aria-label={gui.scan_sample} onClick={() => fill(sample.credential)}
                          className="pairing-paper pairing-sample flex flex-col items-center gap-xxs rounded-lg px-base py-in">
                          <span className="text-caption font-label">{sample.device}</span>
                          <ExampleCode code={sample.qr} className="pairing-sample-code" />
                        </button>}
                      </div>
                      <span className="pairing-camera-hint rounded-full px-in py-xxs text-center text-callout">
                        {sample ? gui.scan_sample : gui.scan_aim}
                      </span>
                    </>
                  )}
                </div>
              </div>
              <div data-pairing-camera-actions className="flex flex-col gap-in p-row">
                {photoError && <span className="pairing-camera-hint self-center rounded-full px-in py-xxs text-center text-callout"
                  role="alert">{photoError}</span>}
                <div className="flex items-center justify-between gap-in">
                  <Button variant="ghost" data-join-photos className="pairing-camera-control glass min-h-touch gap-in"
                    disabled={photoState !== "idle"} onClick={choosePhoto}>
                    <IconFileTray className="size-icon-mark" />
                    {gui.scan_photo_library}
                  </Button>
                  <Button variant="ghost" data-join-manual className="pairing-camera-control glass min-h-touch gap-in"
                    onClick={() => {
                      cameraRequest.current?.abort();
                      photoRequest.current?.abort();
                      setPhotoState("idle");
                      setExpanded(true);
                      setError(null);
                      setStep("confirm");
                    }}>
                    <IconKeyboard className="size-icon-mark" />
                    {gui.scan_enter_credential}
                  </Button>
                </div>
              </div>
              {!sample && <input ref={attachPicker} data-join-photo-file type="file" accept="image/*" hidden
                onChange={event => {
                  const file = event.currentTarget.files?.[0];
                  event.currentTarget.value = "";
                  void readPhoto(file);
                }} />}
            </>
          )}
        </section>
      </>
    );
  }

  if (step === "joined") return (
    <>
      <p data-join-result role="status" className="text-body">{result}</p>
      <TaskActions close={<Button onClick={onClose}>{gui.close}</Button>} />
    </>
  );

  return (
    <>
      <div data-join-confirm data-sheet-hold className="flex min-h-0 flex-col gap-row overflow-y-auto overscroll-contain [&>*]:shrink-0">
        {device && <p className="text-title-md font-title break-words">{gui.pair_confirm_host(device)}</p>}
        {!expanded && (
          <Button variant="ghost" aria-expanded={false} onClick={() => setExpanded(true)}
            className="min-h-touch justify-between bg-fill text-foreground">
            {gui.pair_credential_filled}
            <IconChevronDown className="size-icon-mark" />
          </Button>
        )}
        {expanded && (
          <label className="flex min-w-0 flex-col gap-in text-callout text-muted-foreground">
            {gui.pair_remote_ticket}
            <Input data-ticket-input value={credential} placeholder={gui.pair_remote_placeholder}
              autoComplete="off" spellCheck={false}
              onChange={event => { onCredentialChange(event.target.value); setError(null); }} />
          </label>
        )}
        {error && <p role="alert" className="text-callout text-destructive">{error}</p>}
      </div>
      <TaskActions busy={busy} dismissClosesTask
        back={<Button onClick={() => setStep("scan")}>{gui.back}</Button>}
        primary={<Button data-join disabled={!credential.trim()} onClick={() => void join()}>
          {gui.join_with_a_ticket}
        </Button>} />
    </>
  );
}
