
## Requirements

- Windows 11 x64 (build 22000 or newer). Windows 10 is not officially supported.
- Rust via rustup; the exact compiler is pinned in `rust-toolchain.toml`, with the `x86_64-pc-windows-msvc` target.
- For a native Windows build, Visual Studio 2022 Build Tools with the **Desktop development with C++** workload and Windows SDK.
- Inno Setup 6, only if you want to build the installer.

Install Rust from [rustup.rs](https://rustup.rs/), then in PowerShell install the target:

```powershell
rustup target add x86_64-pc-windows-msvc
```

## Develop and Build on Windows directly

These commands require **Windows-native Rust**, not Cargo running inside WSL. In PowerShell, `rustc -vV` should report `host: x86_64-pc-windows-msvc`. Open PowerShell in the repository directory. Build and launch the app in one step:

```powershell
cargo run --locked
```

Or build the optimized executable and run it directly:

```powershell
rustup target add x86_64-pc-windows-msvc

cargo build --locked --release --target x86_64-pc-windows-msvc
.\target\x86_64-pc-windows-msvc\release\app-launcher.exe
```

On first launch, the app creates `%APPDATA%\AppLauncher\config.yaml` with example groups. It opens the launcher window; use **Ctrl+Alt+E** to hide or show it. Launching an app hides only the popup: the launcher stays running in the tray and can be reopened with the hotkey. Escape, the popup close button, and focus loss also hide rather than exit. Use **Exit** in the tray menu to stop the launcher. The tray icon menu also has **Open**, **Settings**, and **Edit config**. The Settings panel can enable startup when you sign in. To start directly in the tray without opening the window, pass `--background`:

```powershell
.\target\x86_64-pc-windows-msvc\release\app-launcher.exe --background
```

The launcher has no minimize control. If Windows Show Desktop (Windows+D) hides or minimizes it, reopen it with **Ctrl+Alt+E** or tray **Open**; reopening restores the popup rather than leaving it minimized.

The popup uses a fixed **five-column, two-row** layout with no scrollbars (704 x 457 logical pixels, plus 38 pixels when an error is displayed). It centers in the **primary Windows display's work area**, excluding the taskbar, every time it opens and while visible when monitors or display scaling change. Docking, undocking, and switching to a TV therefore do not retain the previous screen's size or position. Groups and search results with more than ten apps use **Previous** and **Next** pages; changing the search or group returns to the first page.

## Development and Build with WSL Ubuntu

Run the app on Windows. You can develop, test, and cross-compile it from WSL Ubuntu. First install the Linux host compiler and fontconfig development dependencies; Cargo needs these to compile build scripts even when targeting Windows:

```sh
sudo apt-get update
sudo apt-get install build-essential pkg-config libfontconfig1-dev
cc --version
```

From the project directory in WSL, run unit tests and a Windows-target type-check:

```sh
cargo test --locked
rustup target add x86_64-pc-windows-gnu
cargo check --locked --target x86_64-pc-windows-gnu
```

`cargo check` verifies Windows-target code but does not produce a runnable Windows executable.

To produce an executable from WSL, install the MinGW-w64 toolchain and use the **GNU** Windows target:

```sh
sudo apt-get install gcc-mingw-w64-x86-64
rustup target add x86_64-pc-windows-gnu
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
cargo build --locked --release --target x86_64-pc-windows-gnu
```

Then run the executable from Windows PowerShell in the project directory:

```powershell
.\target\x86_64-pc-windows-gnu\release\app-launcher.exe
```

The installer script expects an MSVC executable, so use the native Windows build or the Windows CI workflow when packaging.

The stricter Windows launch policy does not prevent WSL tests or cross-builds.
Tests use Windows-style fixture paths without launching those programs on Linux.
Actual Windows token checks, startup behavior, and installer smoke tests still
need Windows. On Windows, double-clicking or ordinary sign-in startup works
normally; **Run as administrator** or starting from an elevated terminal is
refused before configuration is read.

### Troubleshooting: `linker cc not found`

This is a missing **Linux host compiler**, not an application code error. Check `cargo -vV`: if it reports `host: x86_64-unknown-linux-gnu`, Cargo is running in WSL. Install the host dependencies above, then retry. Installing a Rust Windows target alone does not install either the host C compiler or the Windows linker.

Do not use `cargo build --target x86_64-pc-windows-msvc` in a standard WSL setup: fixing `cc` only resolves the host build scripts. Producing an MSVC executable also requires an explicitly configured cross-toolchain and Windows SDK libraries. Use the WSL GNU build above, or Windows-native Rust with Visual Studio Build Tools for MSVC.

## Build the installer

Build the MSVC release executable, then run the packaging script from a clean,
committed checkout in Windows PowerShell 7. It generates dependency notices,
portable/source ZIPs, the installer, and checksums:

```powershell
cargo build --locked --release --target x86_64-pc-windows-msvc
cargo install cargo-bundle-licenses --version 5.0.0 --locked
.\scripts\package.ps1
```

For version 0.1.0, the installer is `dist\AppLauncher-Setup-0.1.0.exe`.
It installs per-user without elevation; optional tasks add a desktop shortcut,
enable startup, or launch the app after installation. Move any existing `dist`
directory before packaging again. The release workflow runs these steps for
version tags and creates a draft GitHub Release. See the
[release guide](docs/RELEASING.md) for the complete process and review checklist.
