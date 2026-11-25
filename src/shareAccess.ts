export type ShareAccessStatus =
  | { kind: "ok" }
  | { kind: "needsSignIn" }
  | { kind: "accessDenied" }
  | { kind: "unreachable" }
  | { kind: "notFound" }
  | { kind: "credentialConflict" }
  | { kind: "timeout" }
  | { kind: "checkInProgress" }
  | { kind: "cancelled" }
  | { kind: "unknown"; raw_os_error?: number | null; message: string };

export function isOk(status: ShareAccessStatus | null): boolean {
  return status?.kind === "ok";
}

export function isShareTimeoutError(message: string): boolean {
  return message.includes("SHARE_TIMEOUT:");
}

export function shareTimeoutStatus(): ShareAccessStatus {
  return { kind: "timeout" };
}
