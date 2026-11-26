# App Launcher – Requirements

## 1. Goal
Minimal, fast, secure Windows 11 (x64) app launcher. A hotkey opens a topmost popup of app icons from a user-maintained YAML file; click launches the app.

## 2. Functional Requirements
| ID | Requirement |
|----|-------------|
| F1 | Global hotkey **Ctrl+Alt+E** shows launcher on top of all windows (press again / Esc / focus loss / close button hides it). |
| F2 | Config is a single YAML file with a list of apps (path to executable). |
| F3 | Icon auto-extracted from the executable; optional `icon` path override (.ico/.png/.exe/.dll). |
| F4 | Optional launch `args` per app (and optionally `workdir`). |
| F5 | Custom, dynamically defined groups (e.g. Utils, Tools, Dev, Photo Editors). Any group name allowed; no fixed list. |
| F6 | Order in YAML = order in UI (groups and apps within groups). |
| F7 | Group tabs in UI: `All` + one tab per YAML group (reference UI also shows Recent/System – optional). |
| F8 | Search bar at top: filters by app name **and** group name. |
| F9 | Settings: toggle "Start with Windows". Settings gear opens it; also option to open the YAML file. |
| F10 | Click icon launches app, then hides launcher. |
| F11 | Config hot-reload on file save; invalid YAML shows a clear error, keeps last good config. |
| F12 | Tray icon (Open, Settings, Edit config, Exit). |

## 3. Non-Functional Requirements
- **Minimal**: small binary (target < 10 MB), few dependencies.
- **Fast**: popup visible < 100 ms after hotkey; idle RAM low (target < 50 MB), ~0% CPU idle.
- **Secure**: no network access, no elevation required, launch via `CreateProcess`/`ShellExecute` with argv (no shell string concatenation), config read-only parsing with strict schema, per-user install and data.
- **Single instance**.
- **Windows 11 x64** only (Win10 nice-to-have).
- **Use RUST**. Only run rust in WSL for development here

## 4. UI (per reference image)
Light, clean, rounded window; top row: search box (left), settings gear and close (right); below it a tab bar of groups with active-tab underline; below it a grid of rounded icon cards with name labels (truncated with ellipsis). Light/dark theme follows system (nice-to-have).

## 5. Proposed YAML Schema
```yaml
settings:            # optional
  hotkey: "Ctrl+Alt+E"
groups:              # order = tab order
  - name: Dev
    apps:            # order = icon order
      - name: Visual Studio Code
        path: 'C:\Program Files\Microsoft VS Code\Code.exe'
        args: ['--new-window']        # optional
        icon: 'C:\icons\vscode.png'   # optional
  - name: Tools
    apps:
      - { name: Notepad, path: 'C:\Windows\notepad.exe' }
```
Open: allow also flat `apps:` with per-app `group:` field (see BRAINSTORM Q).

## 6. Packaging
Windows x64 installer (per-user, no admin), uninstall support, optional "Start with Windows" and "Launch after install". Autostart stored in `HKCU\...\Run`.

## 7. Out of Scope (v1)
Auto-discovery of installed apps, cloud sync, plugins, macOS/Linux.
