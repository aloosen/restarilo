use crate::smb_unc::{status_from_raw_os_error, ShareAccessStatus};
use std::sync::Mutex;

static SIGNIN_LOCK: Mutex<()> = Mutex::new(());

/// Interactive SMB sign-in for `share_root` (`\\server\share`).
/// `hwnd` is the Restarilo main window handle (Windows only).
pub fn sign_in_share(
    share_root: &str,
    hwnd: isize,
    prompt_alternate: bool,
) -> ShareAccessStatus {
    let _guard = match SIGNIN_LOCK.try_lock() {
        Ok(g) => g,
        Err(_) => {
            return ShareAccessStatus::Unknown {
                raw_os_error: None,
                message: "sign-in already in progress".into(),
            };
        }
    };
    sign_in_share_inner(share_root, hwnd, prompt_alternate)
}

#[cfg(windows)]
fn sign_in_share_inner(share_root: &str, hwnd: isize, prompt_alternate: bool) -> ShareAccessStatus {
    use std::ptr;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{ERROR_CANCELLED, ERROR_SUCCESS, HWND};
    use windows::Win32::NetworkManagement::WNet::{
        WNetAddConnection3W, CONNECT_INTERACTIVE, CONNECT_PROMPT, NETRESOURCEW, RESOURCETYPE_DISK,
        RESOURCE_GLOBALNET,
    };

    let mut remote: Vec<u16> = share_root.encode_utf16().chain(std::iter::once(0)).collect();
    let mut nr = NETRESOURCEW {
        dwScope: RESOURCE_GLOBALNET,
        dwType: RESOURCETYPE_DISK,
        dwDisplayType: 0,
        dwUsage: 0,
        lpLocalName: windows::core::PWSTR(ptr::null_mut()),
        lpRemoteName: windows::core::PWSTR(remote.as_mut_ptr()),
        lpComment: windows::core::PWSTR(ptr::null_mut()),
        lpProvider: windows::core::PWSTR(ptr::null_mut()),
    };

    let mut flags = CONNECT_INTERACTIVE;
    if prompt_alternate {
        flags |= CONNECT_PROMPT;
    }

    let window = HWND(hwnd as *mut std::ffi::c_void);
    // SAFETY: interactive connect; Restarilo passes no SMB credentials.
    let err = unsafe {
        WNetAddConnection3W(window, &mut nr, PCWSTR::null(), PCWSTR::null(), flags)
    };

    if err == ERROR_SUCCESS {
        return ShareAccessStatus::Ok;
    }
    if err == ERROR_CANCELLED {
        return ShareAccessStatus::Cancelled;
    }
    status_from_raw_os_error(err.0 as i32)
}

#[cfg(not(windows))]
fn sign_in_share_inner(
    _share_root: &str,
    _hwnd: isize,
    _prompt_alternate: bool,
) -> ShareAccessStatus {
    ShareAccessStatus::Unknown {
        raw_os_error: None,
        message: "SMB sign-in is only available on Windows".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_windows_never_ok() {
        let s = sign_in_share(r"\\nas\backups", 0, false);
        match s {
            ShareAccessStatus::Unknown { message, .. } => {
                assert!(message.contains("only available on Windows"));
            }
            other => panic!("expected Unknown, got {other:?}"),
        }
    }
}
