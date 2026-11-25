import { useEffect, useRef, useState } from "react";
import { OverwriteModal } from "./OverwriteModal";

export type OverwriteAsk = {
  sourcePath: string;
  snapshotLabel: string;
  targetPath: string;
};

export function useOverwriteAsk() {
  const [ask, setAsk] = useState<OverwriteAsk | null>(null);
  const resolveRef = useRef<((ok: boolean) => void) | null>(null);

  function finish(ok: boolean) {
    const resolve = resolveRef.current;
    resolveRef.current = null;
    setAsk(null);
    resolve?.(ok);
  }

  useEffect(() => {
    if (!ask) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        finish(false);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [ask]);

  useEffect(() => {
    return () => {
      const resolve = resolveRef.current;
      resolveRef.current = null;
      resolve?.(false);
    };
  }, []);

  function askOverwrite(next: OverwriteAsk): Promise<boolean> {
    return new Promise((resolve) => {
      const prev = resolveRef.current;
      resolveRef.current = null;
      prev?.(false);
      resolveRef.current = resolve;
      setAsk(next);
    });
  }

  const modal = ask ? (
    <OverwriteModal
      {...ask}
      onConfirm={() => finish(true)}
      onCancel={() => finish(false)}
    />
  ) : null;

  return { askOverwrite, modal };
}
