import type { Terminal } from '@xterm/xterm';
type Sample = {
    started: number;
    route: string;
    frameGap: number;
    sendMs: number;
    queued: number;
    parses: number;
    renders: number;
};
/** Bounded aggregate timings, never terminal contents. Healthy gestures say
 * nothing; a slow one is reported at most once per 30 seconds per pane. */
export function scrollHealth() {
    let sample: Sample | null = null, lastFrame = 0, lastNote = -Infinity, pending: {
        at: number;
        queued: number;
    } | null = null;
    return {
        start(at: number, route: string) { sample = { started: at, route, frameGap: 0, sendMs: 0, queued: 0, parses: 0, renders: 0 }; lastFrame = at; },
        frame(at: number) { if (sample) {
            sample.frameGap = Math.max(sample.frameGap, at - lastFrame);
            lastFrame = at;
        } },
        sending(at: number, queued: number) { pending = { at, queued }; },
        sent(start: number, end: number, queued: number) { pending = null; if (sample && end >= sample.started) {
            sample.sendMs = Math.max(sample.sendMs, end - start);
            sample.queued = Math.max(sample.queued, queued);
        } },
        parsed() { if (sample)
            sample.parses++; },
        rendered() { if (sample)
            sample.renders++; },
        cancel() { sample = null; pending = null; },
        finish(at: number, distance: number): Sample | null {
            const out = sample;
            sample = null;
            if (!out || distance < 10 || at - lastNote < 30000)
                return null;
            if (pending) {
                out.sendMs = Math.max(out.sendMs, at - pending.at);
                out.queued = Math.max(out.queued, pending.queued);
            }
            out.frameGap = Math.max(out.frameGap, at - lastFrame);
            if (out.frameGap < 100 && out.sendMs < 250 && out.queued < 4096)
                return null;
            lastNote = at;
            return out;
        },
    };
}
/** Observe any later drag too, not only the first touch after attach. The
 * frame sampler exists only during a touch plus its short momentum tail,
 * for at most three seconds. It neither cancels nor changes an input. */
export function notesScrollHealth(o: {
    enabled: boolean;
    term: Terminal;
    scroller: HTMLElement;
    shown: () => boolean;
    route: () => string;
    note: (line: string) => void;
}): {
    sending: (at: number, queued: number) => void;
    sent: (start: number, end: number, queued: number) => void;
    stop: () => void;
} {
    const health = scrollHealth();
    if (!o.enabled)
        return { sending: () => { }, sent: () => { }, stop: () => { } };
    let active = false, frame = 0, tail = 0, limit = 0, origin = 0, distance = 0;
    const clear = () => { active = false; cancelAnimationFrame(frame); clearTimeout(tail); clearTimeout(limit); };
    const finish = () => {
        if (!active)
            return;
        const result = health.finish(performance.now(), distance);
        clear();
        if (result)
            o.note(`slow scroll: ${result.route}, frame gap ${Math.round(result.frameGap)} ms, send ${Math.round(result.sendMs)} ms, queue ${result.queued} chars, parsed ${result.parses}, rendered ${result.renders}`);
    };
    const cancel = () => { clear(); health.cancel(); };
    const tick = (at: number) => {
        if (!active)
            return;
        if (document.hidden || !o.shown()) {
            cancel();
            return;
        }
        health.frame(at);
        frame = requestAnimationFrame(tick);
    };
    const start = (e: TouchEvent) => {
        if (e.touches.length !== 1 || document.hidden || !o.shown()) {
            cancel();
            return;
        }
        finish();
        const t = e.touches[0];
        origin = t.clientY;
        distance = 0;
        active = true;
        health.start(performance.now(), o.route());
        frame = requestAnimationFrame(tick);
        limit = window.setTimeout(finish, 3000);
    };
    const move = (e: TouchEvent) => {
        if (!active)
            return;
        if (e.touches.length !== 1) {
            cancel();
            return;
        }
        const t = e.touches[0];
        distance = Math.max(distance, Math.abs(t.clientY - origin));
    };
    const end = () => { if (active) {
        clearTimeout(tail);
        tail = window.setTimeout(finish, 500);
    } };
    const visibility = () => { if (document.hidden)
        cancel(); };
    const parsed = o.term.onWriteParsed(() => health.parsed()), rendered = o.term.onRender(() => health.rendered());
    o.scroller.addEventListener('touchstart', start, { passive: true });
    o.scroller.addEventListener('touchmove', move, { passive: true });
    window.addEventListener('touchend', end, { capture: true, passive: true });
    window.addEventListener('touchcancel', end, { capture: true, passive: true });
    document.addEventListener('visibilitychange', visibility);
    return { sending: health.sending, sent: health.sent, stop() {
            cancel();
            parsed.dispose();
            rendered.dispose();
            o.scroller.removeEventListener('touchstart', start);
            o.scroller.removeEventListener('touchmove', move);
            window.removeEventListener('touchend', end, true);
            window.removeEventListener('touchcancel', end, true);
            document.removeEventListener('visibilitychange', visibility);
        } };
}
