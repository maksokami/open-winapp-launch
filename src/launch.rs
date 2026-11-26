use crate::config::App;
use anyhow::{Context, Result, bail};
use std::{path::Path, process::Command};

pub fn launch(app: &App) -> Result<()> {
    crate::security::validate_executable(&app.path)?;
    let executable = Path::new(&app.path);
    if !executable.is_file() {
        bail!("application not found: {}", app.path);
    }

    let executable = executable
        .canonicalize()
        .context("could not resolve application path")?;
    #[cfg(windows)]
    ensure_local_resolved_path(&executable)?;
    if !executable
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
    {
        bail!("application must resolve to an .exe file, not a script or file association");
    }
    let mut command = Command::new(&executable);
    command.args(&app.args);
    if let Some(workdir) = &app.workdir {
        crate::security::validate_local_path(workdir)?;
        let workdir = Path::new(workdir)
            .canonicalize()
            .context("could not resolve working directory")?;
        #[cfg(windows)]
        ensure_local_resolved_path(&workdir)?;
        command.current_dir(workdir);
    } else if let Some(parent) = executable.parent() {
        command.current_dir(parent);
    }
    command
        .spawn()
        .with_context(|| format!("could not launch {}", app.name))?;
    Ok(())
}

#[cfg(windows)]
fn ensure_local_resolved_path(path: &Path) -> Result<()> {
    use std::path::{Component, Prefix};
    use windows::{Win32::Storage::FileSystem::GetDriveTypeW, core::HSTRING};
    if !matches!(
        path.components().next(),
        Some(Component::Prefix(prefix))
            if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
    ) {
        bail!("application and working directory must resolve to local drive paths");
    }
    let root: std::path::PathBuf = path.components().take(2).collect();
    const DRIVE_REMOTE: u32 = 4;
    let drive_type = unsafe { GetDriveTypeW(&HSTRING::from(root.as_os_str())) };
    if drive_type == DRIVE_REMOTE || drive_type == 0 || drive_type == 1 {
        bail!(
            "application and working directory must be on an accessible local drive, not a mapped network drive"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_unsafe_launches_before_spawning() {
        for path in ["cmd.exe", r"C:\payload.cmd", r"\\server\share\payload.exe"] {
            let app = App {
                name: "Unsafe".into(),
                path: path.into(),
                args: vec!["/c".into(), "echo unexpected".into()],
                workdir: None,
                icon: None,
                group: None,
            };
            assert!(launch(&app).is_err());
        }
    }

    #[cfg(windows)]
    #[test]
    fn rejects_resolved_network_paths() {
        assert!(ensure_local_resolved_path(Path::new(r"\\server\share\app.exe")).is_err());
        assert!(ensure_local_resolved_path(Path::new(r"\\?\UNC\server\share\app.exe")).is_err());
        assert!(ensure_local_resolved_path(Path::new(r"C:\Windows\System32\notepad.exe")).is_ok());
    }
}
