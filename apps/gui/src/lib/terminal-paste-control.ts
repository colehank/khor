import { useEffect, useRef, useState } from "react";

import { inApp } from "@/api";
import { listenNativeEvent } from "@/lib/native-events";
import { pastedImage, type Pasted } from "@/lib/term-paste";

type Rect = [number, number, number, number];
type Seat = { frame: Rect; clip: Rect; radius: number; dark: boolean };
type PasteEvent = { owner: string; value: { text: string } | { images: { type: string; data: string }[] } | { scroll: number } };

const ios = inApp && (/iPhone|iPad|iPod/i.test(navigator.userAgent)
  || (/Mac/i.test(navigator.platform) && navigator.maxTouchPoints > 1));
let nextOwner = 0;
let tail = Promise.resolve();

function seatOf(cap: HTMLButtonElement, row: HTMLElement): Seat | null {
  if (document.hidden || !cap.isConnected) return null;
  const box = cap.getBoundingClientRect();
  const strip = row.getBoundingClientRect();
  const visual = window.visualViewport;
  const left = Math.max(box.left, strip.left, 0);
  const top = Math.max(box.top, strip.top, 0);
  const right = Math.min(box.right, strip.right, visual?.width ?? window.innerWidth);
  const bottom = Math.min(box.bottom, strip.bottom, visual?.height ?? window.innerHeight);
  if (right <= left || bottom <= top) return null;
  // A modal covering the row must also cover the corresponding native key.
  const hit = document.elementFromPoint((left + right) / 2, (top + bottom) / 2);
  if (!hit || !cap.contains(hit)) return null;
  const radius = parseFloat(getComputedStyle(cap.firstElementChild ?? cap).borderTopLeftRadius) || 0;
  return {
    frame: [box.x, box.y, box.width, box.height],
    clip: [left, top, right - left, bottom - top],
    radius,
    dark: document.documentElement.dataset.theme === "dark",
  };
}

/** In the iOS app the actual button is UIPasteControl. One press gives the
 * system permission to hand us that paste, including pictures. */
export function useTerminalPasteControl(onPaste?: (paste: Pasted) => void) {
  const button = useRef<HTMLButtonElement>(null);
  const latest = useRef(onPaste);
  latest.current = onPaste;
  const [native, setNative] = useState(false);
  const enabled = ios && !!onPaste;

  useEffect(() => {
    const cap = button.current;
    const row = cap?.closest<HTMLElement>("[data-term-keys-scroll]");
    if (!enabled || !cap || !row) return;
    // An item provider can complete after a WebView reload. Scope it to this document.
    const owner = `${performance.timeOrigin}:${++nextOwner}`;
    let alive = true;
    let frame = 0;
    let last = "";
    let generation = 0;
    let supported = true;
    let listening = false;
    const send = (geometry: Seat | null) => {
      const version = ++generation;
      // Match hide and show in call order, even if a previous reply arrives
      // after the tab that owned it has already unmounted.
      tail = tail.then(async () => {
        if ((!alive && geometry) || version !== generation) return;
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          if ((!alive && geometry) || version !== generation) return;
          const attached = await invoke<boolean>("terminal_paste_control", { owner, geometry });
          if (alive && version === generation) {
            supported = !geometry || attached;
            setNative(attached);
          }
        } catch {
          if (alive && version === generation) {
            supported = false;
            setNative(false);
          }
        }
      });
    };
    const measure = () => {
      frame = 0;
      if (!alive || !listening || !supported) return;
      const geometry = seatOf(cap, row);
      const stamp = JSON.stringify(geometry);
      if (stamp === last) return;
      last = stamp;
      send(geometry);
    };
    const schedule = () => {
      if (!frame) frame = requestAnimationFrame(measure);
    };
    const stop = listenNativeEvent<PasteEvent>("terminal-paste", ({ owner: from, value }) => {
      if (!alive || from !== owner || !cap.isConnected) return;
      if ("scroll" in value) row.scrollLeft += value.scroll;
      else if ("text" in value && value.text) latest.current?.({ text: value.text });
      else if ("images" in value) latest.current?.({ images: value.images.map(pastedImage) });
    }, { target: { kind: "WebviewWindow", label: "main" } });
    void stop.ready.then(ready => {
      listening = ready;
      if (alive && ready) schedule();
    });
    const size = new ResizeObserver(schedule);
    size.observe(cap);
    size.observe(row);
    const changes = new MutationObserver(schedule);
    changes.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    changes.observe(document.body, { childList: true, subtree: true });
    row.addEventListener("scroll", schedule);
    window.addEventListener("resize", schedule);
    window.visualViewport?.addEventListener("resize", schedule);
    document.addEventListener("visibilitychange", schedule);
    return () => {
      alive = false;
      cancelAnimationFrame(frame);
      size.disconnect();
      changes.disconnect();
      row.removeEventListener("scroll", schedule);
      window.removeEventListener("resize", schedule);
      window.visualViewport?.removeEventListener("resize", schedule);
      document.removeEventListener("visibilitychange", schedule);
      stop();
      send(null);
    };
  }, [enabled]);

  return { button, native };
}
