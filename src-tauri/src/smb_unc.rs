use serde::Serialize;

pub const ERROR_FILE_NOT_FOUND: i32 = 2;
pub const ERROR_PATH_NOT_FOUND: i32 = 3;
pub const ERROR_ACCESS_DENIED: i32 = 5;
pub const ERROR_INVALID_PASSWORD: i32 = 86;
pub const ERROR_BAD_NETPATH: i32 = 53;
pub const ERROR_LOGON_FAILURE: i32 = 1326;
pub const ERROR_SESSION_CREDENTIAL_CONFLICT: i32 = 1219;
pub const ERROR_NETWORK_UNREACHABLE: i32 = 1231;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ShareAccessStatus {
    Ok,
    NeedsSignIn,
    AccessDenied,
    Unreachable,
    NotFound,
    CredentialConflict,
    Timeout,
    CheckInProgress,
    Cancelled,
    Unknown {
        raw_os_error: Option<i32>,
        message: String,
    },
}

pub fn is_unc_path(path: &str) -> bool {
    let p = path.trim();
    p.starts_with(r"\\") || p.starts_with("//")
}

/// Forward-slash UNC → backslash form; other paths returned unchanged.
pub fn normalize_unc_path(path: &str) -> String {
    let p = path.trim();
    if p.starts_with("//") {
        return format!(r"\\{}", p[2..].replace('/', r"\"));
    }
    p.to_string()
}

/// Share root `\\server\share` from a UNC repository path.
pub fn share_root(path: &str) -> Result<String, String> {
    if !is_unc_path(path) {
        return Err("not a UNC path".into());
    }
    let n = normalize_unc_path(path);
    let rest = n.trim_start_matches('\\');
    let mut parts = rest.split('\\').filter(|s| !s.is_empty());
    let server = parts.next().ok_or_else(|| "UNC path missing server".to_string())?;
    let share = parts.next().ok_or_else(|| "UNC path missing share".to_string())?;
    Ok(format!(r"\\{server}\{share}"))
}

pub fn status_from_raw_os_error(code: i32) -> ShareAccessStatus {
    match code {
        ERROR_ACCESS_DENIED => ShareAccessStatus::AccessDenied,
        ERROR_LOGON_FAILURE | ERROR_INVALID_PASSWORD => ShareAccessStatus::NeedsSignIn,
        ERROR_SESSION_CREDENTIAL_CONFLICT => ShareAccessStatus::CredentialConflict,
        ERROR_BAD_NETPATH | ERROR_NETWORK_UNREACHABLE => ShareAccessStatus::Unreachable,
        ERROR_PATH_NOT_FOUND | ERROR_FILE_NOT_FOUND => ShareAccessStatus::NotFound,
        other => ShareAccessStatus::Unknown {
            raw_os_error: Some(other),
            message: format!("Windows error {other}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_normalizes_both_spellings() {
        assert!(is_unc_path(r"\\nas\backups\repo"));
        assert!(is_unc_path("//nas/backups/repo"));
        assert!(!is_unc_path("/var/backup"));
        assert!(!is_unc_path("sftp:user@host:/repo"));
        assert_eq!(
            normalize_unc_path("//nas/backups/repo"),
            r"\\nas\backups\repo"
        );
        assert_eq!(
            normalize_unc_path(r"\\nas\backups\repo"),
            r"\\nas\backups\repo"
        );
    }

    #[test]
    fn share_root_from_both_spellings() {
        assert_eq!(
            share_root(r"\\nas\backups\team-repo").unwrap(),
            r"\\nas\backups"
        );
        assert_eq!(
            share_root("//nas/backups/team-repo").unwrap(),
            r"\\nas\backups"
        );
        assert!(share_root("/local/repo").is_err());
    }

    #[test]
    fn maps_locked_win32_codes() {
        assert_eq!(
            status_from_raw_os_error(ERROR_ACCESS_DENIED),
            ShareAccessStatus::AccessDenied
        );
        assert_eq!(
            status_from_raw_os_error(ERROR_LOGON_FAILURE),
            ShareAccessStatus::NeedsSignIn
        );
        assert_eq!(
            status_from_raw_os_error(ERROR_INVALID_PASSWORD),
            ShareAccessStatus::NeedsSignIn
        );
        assert_eq!(
            status_from_raw_os_error(ERROR_SESSION_CREDENTIAL_CONFLICT),
            ShareAccessStatus::CredentialConflict
        );
        assert_eq!(
            status_from_raw_os_error(ERROR_BAD_NETPATH),
            ShareAccessStatus::Unreachable
        );
        assert_eq!(
            status_from_raw_os_error(ERROR_NETWORK_UNREACHABLE),
            ShareAccessStatus::Unreachable
        );
        assert_eq!(
            status_from_raw_os_error(ERROR_PATH_NOT_FOUND),
            ShareAccessStatus::NotFound
        );
        assert_eq!(
            status_from_raw_os_error(ERROR_FILE_NOT_FOUND),
            ShareAccessStatus::NotFound
        );
        match status_from_raw_os_error(9999) {
            ShareAccessStatus::Unknown {
                raw_os_error: Some(9999),
                ..
            } => {}
            other => panic!("expected Unknown, got {other:?}"),
        }
        assert_eq!(
            ShareAccessStatus::CheckInProgress,
            ShareAccessStatus::CheckInProgress
        );
    }
}
