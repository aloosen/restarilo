import type { ShareAccessStatus } from "./shareAccess";

type Props = {
  status: ShareAccessStatus | null;
  onSignIn: () => void;
  onTryAnotherAccount: () => void;
  onRetry: () => void;
  disabled?: boolean;
};

function messageFor(status: ShareAccessStatus): string {
  switch (status.kind) {
    case "needsSignIn":
      return "Sign in to the network share is required before snapshots can load.";
    case "accessDenied":
      return "Access was denied. The current Windows account may lack permission — try another account.";
    case "unreachable":
      return "The network share could not be reached.";
    case "notFound":
      return "The repository path was not found on the share.";
    case "credentialConflict":
      return "Windows already has a connection to this server with different credentials. Existing connections were left unchanged.";
    case "timeout":
      return "The share did not respond in time.";
    case "checkInProgress":
      return "A previous access check is still finishing. Retry is safe and will not hang.";
    case "cancelled":
      return "Sign-in was cancelled. You can try again.";
    case "unknown":
      return status.message || "Could not access the network share.";
    case "ok":
      return "";
  }
}

export function ShareAccessPanel({
  status,
  onSignIn,
  onTryAnotherAccount,
  onRetry,
  disabled,
}: Props) {
  if (!status || status.kind === "ok") return null;

  const showSignIn =
    status.kind === "needsSignIn" ||
    status.kind === "accessDenied" ||
    status.kind === "cancelled";
  const showAlternate = status.kind === "accessDenied";
  const showRetry =
    status.kind === "unreachable" ||
    status.kind === "timeout" ||
    status.kind === "checkInProgress" ||
    status.kind === "notFound" ||
    status.kind === "credentialConflict" ||
    status.kind === "unknown";

  return (
    <div className="app-share-access" role="status">
      <pre className="app-error">{messageFor(status)}</pre>
      <div className="app-share-actions">
        {showSignIn && (
          <button type="button" disabled={disabled} onClick={onSignIn}>
            Sign in to network share
          </button>
        )}
        {showAlternate && (
          <button type="button" disabled={disabled} onClick={onTryAnotherAccount}>
            Try another account
          </button>
        )}
        {showRetry && (
          <button type="button" disabled={disabled} onClick={onRetry}>
            Retry
          </button>
        )}
      </div>
    </div>
  );
}
