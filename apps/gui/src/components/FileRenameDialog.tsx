import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import { Dialog, TaskOverlay, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";

/** 重命名: the name only, in its directory (`FileTree`'s rule: a typed
    `/` would move the file somewhere nothing is showing). */
export function RenameDialog({
  open,
  name,
  onClose,
  onRename,
}: {
  open: boolean;
  name: string;
  onClose: () => void;
  onRename: (name: string) => Promise<void>;
}) {
  const [value, setValue] = useState(name);
  const [refused, setRefused] = useState<string | null>(null);
  useEffect(() => {
    if (open) {
      setValue(name);
      setRefused(null);
    }
  }, [open, name]);
  const go = () => {
    const v = value.trim();
    if (!v || v.includes("/")) return;
    onRename(v).then(onClose, (e: unknown) => setRefused(String((e as Error)?.message ?? e)));
  };
  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <TaskOverlay purpose="edit" data-file-rename>
        <DialogHeader>
          <DialogTitle>{gui.rename}</DialogTitle>
        </DialogHeader>
        <Input
          autoFocus
          enterKeyHint="done"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && go()}
        />
        {refused && <span className="text-callout text-state-failed">{refused}</span>}
        <DialogFooter>
          <Button data-dialog-cancel onClick={onClose}>
            {gui.delete_keep}
          </Button>
          <Button variant="primary" data-file-rename-go onClick={go}>
            {gui.rename}
          </Button>
        </DialogFooter>
      </TaskOverlay>
    </Dialog>
  );
}

