import { useEffect } from "react";

export function useSearchHotkey(opts: {
  enabled: boolean;
  blocked: boolean;
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
        if (!opts.blocked) opts.onOpen();
        return;
      }
      if (e.key === "Escape" && opts.open && !opts.blocked && !e.defaultPrevented) {
        e.preventDefault();
        opts.onCloseIdle();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [
    opts.enabled,
    opts.blocked,
    opts.open,
    opts.onOpen,
    opts.onCloseIdle,
  ]);
}
