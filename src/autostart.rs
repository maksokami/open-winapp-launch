#[cfg(windows)]
use anyhow::Context;
use anyhow::Result;

#[cfg(windows)]
pub fn enabled() -> Result<bool> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};
    let key = match RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
    {
        Ok(key) => key,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error).context("could not open the Windows startup registry key"),
    };
    match key.get_value::<String, _>("AppLauncher") {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).context("could not read the Windows startup setting"),
    }
}

#[cfg(windows)]
pub fn set_enabled(enabled: bool) -> Result<()> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .context("could not open the Windows startup registry key")?
        .0;
    if enabled {
        let exe = std::env::current_exe().context("could not locate the application executable")?;
        key.set_value(
            "AppLauncher",
            &format!("\"{}\" --background", exe.display()),
        )
        .context("could not enable startup")?;
    } else {
        match key.delete_value("AppLauncher") {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("could not disable startup"),
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn enabled() -> Result<bool> {
    Ok(false)
}

#[cfg(not(windows))]
pub fn set_enabled(_enabled: bool) -> Result<()> {
    anyhow::bail!("Start with Windows is only available in the Windows build")
}
