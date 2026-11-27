import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export function useActionCancel(actionId: string | null) {
  const requested = useRef<string | null>(null);
  const [status, setStatus] = useState<{ id: string; error?: string }>();
  const cancel = useCallback(() => {
    if (!actionId || requested.current === actionId) return;
    requested.current = actionId;
    setStatus({ id: actionId });
    void invoke("cancel_action", { actionId }).catch(() => {
      if (requested.current !== actionId) return;
      requested.current = null;
      setStatus((current) =>
        current?.id === actionId ? { id: actionId, error: "Could not cancel. Try again." } : current,
      );
    });
  }, [actionId]);

  useEffect(() => {
    if (!actionId) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      cancel();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [actionId, cancel]);

  const current = status?.id === actionId ? status : undefined;
  return { cancel, cancelling: !!current && !current.error, cancelError: current?.error };
}
