import { useEffect } from "react";

export function useSearchHotkey(opts: {
  enabled: boolean;
  actionBusy: boolean;
  open: boolean;
  onOpen: () => void;
  onCloseIdle: () => void;
}) {
  useEffect(() => {
    if (!opts.enabled) return;
    const onKey = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key.toLowerCase() === "f") {
        e.preventDefault();
        opts.onOpen();
        return;
      }
      if (e.key === "Escape" && opts.open && !opts.actionBusy) {
        e.preventDefault();
        opts.onCloseIdle();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [
    opts.enabled,
    opts.actionBusy,
    opts.open,
    opts.onOpen,
    opts.onCloseIdle,
  ]);
}
