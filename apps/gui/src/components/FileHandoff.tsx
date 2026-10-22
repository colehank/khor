import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { sizeWord } from "@/views/FilesPane";
import type { FileHandoffControl } from "@/hooks/use-file-handoff";

type Props = { handoff: FileHandoffControl };

export function FileHandoffAction({ handoff, preview = false }: Props & {preview?:boolean}) {
  if (!handoff.supported && !(preview && handoff.previewSupported)) return null;
  const busy = handoff.progress.phase === "downloading" || handoff.progress.phase === "opening";
  return (
    <div className="flex flex-wrap items-center justify-center gap-in">
    {preview && handoff.previewSupported && <Button data-file-native-preview className="min-h-touch min-w-touch" disabled={busy} onClick={handoff.preview}>{gui.file_face_preview}</Button>}
    {handoff.supported && <Button data-file-open-other className="min-h-touch min-w-touch" disabled={busy} onClick={handoff.open}>
      {gui.file_open_other}
    </Button>}
    </div>
  );
}

/** Floats over the file; showing progress never resizes its sheet. */
export function FileHandoffStatus({ handoff }: Props) {
  const p = handoff.progress;
  if (p.phase === "idle") return null;
  return (
    <div data-file-handoff-status className="pointer-events-none absolute inset-x-0 bottom-0 z-10 flex justify-center p-2">
      <div className="pointer-events-auto flex max-w-full flex-col gap-in rounded-lg border bg-popover px-row py-in text-callout shadow-(--shadow-float)">
        <div className="flex items-center gap-in">
          <span role="status" className="min-w-0 break-words">
            {p.phase === "error" ? p.error : p.phase === "opening" ? (p.intent === "preview" ? gui.file_face_preview : gui.file_open_presenting) : gui.file_open_preparing}
          </span>
          {p.phase === "downloading" && (
            <>
              <span className="whitespace-nowrap tabular-nums text-muted-foreground">
                {sizeWord(p.moved)}{p.total > 0 ? ` / ${sizeWord(p.total)}` : ""}
              </span>
              <Button size="sm" className="min-h-touch" onClick={handoff.cancel}>{gui.delete_keep}</Button>
            </>
          )}
          {p.phase === "error" && (
            <>
              <Button size="sm" className="min-h-touch" onClick={handoff.retry}>{gui.region_retry}</Button>
              <Button size="sm" className="min-h-touch" onClick={handoff.cancel}>{gui.delete_keep}</Button>
            </>
          )}
        </div>
        {p.phase === "downloading" && p.total > 0 && (
          <div role="progressbar" aria-label={gui.file_open_preparing} aria-valuenow={p.moved} aria-valuemin={0} aria-valuemax={p.total} className="h-progress-h overflow-hidden rounded-full bg-fill">
            <div className="h-full bg-foreground" style={{ width: `${Math.min(100, p.moved / p.total * 100)}%` }} />
          </div>
        )}
      </div>
    </div>
  );
}
