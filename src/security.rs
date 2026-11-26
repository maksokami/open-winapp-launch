use anyhow::{Result, bail};

pub fn validate_local_path(value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    if bytes.len() < 3
        || !bytes[0].is_ascii_alphabetic()
        || bytes[1] != b':'
        || bytes[2] != b'\\'
        || value[3..].contains(['/', ':', '"', '<', '>', '|', '?', '*'])
        || value.chars().any(char::is_control)
        || (!value[3..].is_empty()
            && value[3..]
                .split('\\')
                .any(|part| part.is_empty() || part.ends_with(['.', ' '])))
    {
        bail!(
            "use a fully qualified local Windows path, such as C:\\Apps\\app.exe; relative, network, device, and ambiguous paths are not allowed"
        );
    }
    Ok(())
}

pub fn validate_executable(value: &str) -> Result<()> {
    validate_local_path(value)?;
    if !value.to_ascii_lowercase().ends_with(".exe") {
        bail!(
            "only local .exe applications are supported; scripts and file associations are not allowed"
        );
    }
    Ok(())
}

#[cfg(windows)]
pub fn require_standard_user() -> Result<()> {
    use anyhow::Context;
    use std::mem::size_of;
    use windows::Win32::{
        Foundation::{CloseHandle, HANDLE},
        Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation},
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };

    let mut token = HANDLE::default();
    let mut elevation = TOKEN_ELEVATION::default();
    let mut length = 0;
    unsafe {
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)
            .context("could not inspect the process token")?;
        let result = GetTokenInformation(
            token,
            TokenElevation,
            Some((&mut elevation as *mut TOKEN_ELEVATION).cast()),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut length,
        );
        let close_result = CloseHandle(token);
        result.context("could not inspect process elevation")?;
        close_result.context("could not close the process token")?;
    }
    if elevation.TokenIsElevated != 0 {
        bail!(
            "App Launcher must not run as administrator. Close it and open it normally; do not use an elevated terminal or an elevated startup task."
        );
    }
    Ok(())
}

#[cfg(windows)]
pub fn system_notepad() -> Result<std::path::PathBuf> {
    use anyhow::ensure;
    use windows::Win32::System::SystemInformation::GetSystemDirectoryW;

    let mut buffer = vec![0u16; 32768];
    let length = unsafe { GetSystemDirectoryW(Some(&mut buffer)) } as usize;
    ensure!(
        length > 0 && length < buffer.len(),
        "could not locate the Windows system directory"
    );
    Ok(std::path::PathBuf::from(String::from_utf16(&buffer[..length])?).join("notepad.exe"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_local_executables_without_requiring_them_to_exist() {
        for path in [r"C:\Windows\System32\calc.exe", r"D:\My Apps\APP.EXE"] {
            validate_executable(path).unwrap();
        }
    }

    #[test]
    fn rejects_search_paths_scripts_devices_and_ambiguous_names() {
        for path in [
            "calc.exe",
            r".\app.exe",
            r"C:app.exe",
            r"\app.exe",
            r"\\server\share\app.exe",
            r"\\?\C:\app.exe",
            r"\\.\C:\app.exe",
            r"C:\Apps\..\app.exe",
            r"C:\Apps.\app.exe",
            r"C:\Apps \app.exe",
            r"C:\app.exe:payload.exe",
            r"C:\app.cmd",
            r"C:\app.bat",
            r"C:\app.ps1",
            r"C:/Apps/app.exe",
            "C:\\app.exe\0",
            "\"C:\\app.exe\"",
            "%SystemRoot%\\app.exe",
        ] {
            assert!(validate_executable(path).is_err(), "accepted {path:?}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn inspects_the_real_process_token_without_silently_allowing_errors() {
        if let Err(error) = require_standard_user() {
            assert!(
                error
                    .to_string()
                    .starts_with("App Launcher must not run as administrator."),
                "unexpected token inspection failure: {error:#}"
            );
        }
    }
}
