#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod autostart;
mod config;
mod filter;
mod launch;
mod security;
mod window_geometry;

slint::include_modules!();

use anyhow::{Context, Result};
use filter::{AppEntry, filter, flatten};
use notify::Watcher;
use slint::{Image, ModelRc, SharedString, VecModel};
use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
    sync::mpsc,
    time::{Duration, Instant},
};

struct Runtime {
    config: config::Config,
    config_path: PathBuf,
    entries: Vec<AppEntry>,
    icons: HashMap<String, Image>,
    selected_group: Option<String>,
    query: String,
    page: usize,
    #[cfg(windows)]
    activation_pending: bool,
    error: Option<String>,
}

const MAX_ICON_FILE_SIZE: u64 = 16 * 1024 * 1024;
const APPS_PER_PAGE: usize = 10;

fn page_bounds(item_count: usize, requested_page: usize) -> (usize, usize) {
    let page_count = item_count.div_ceil(APPS_PER_PAGE).max(1);
    (requested_page.min(page_count - 1), page_count)
}

fn main() -> Result<()> {
    #[cfg(windows)]
    if let Err(error) = security::require_standard_user() {
        use windows::{
            Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW},
            core::{HSTRING, w},
        };
        let message = HSTRING::from(format!("{error:#}"));
        unsafe {
            MessageBoxW(None, &message, w!("App Launcher"), MB_OK | MB_ICONERROR);
        }
        return Err(error);
    }

    #[cfg(windows)]
    let instance = single_instance()?;

    let config_path = config::config_path()?;
    config::ensure_config(&config_path)?;
    let (initial_config, initial_error) = match config::load(&config_path) {
        Ok(config) => (config, None),
        Err(error) => (
            config::Config {
                settings: config::Settings::default(),
                groups: Vec::new(),
            },
            Some(format!(
                "Config error: {error:#} (keeping the last valid configuration)"
            )),
        ),
    };
    let groups = group_labels(&initial_config);

    let ui = MainWindow::new().context("could not create launcher window")?;
    ui.set_active_group(0);
    let runtime = Rc::new(RefCell::new(Runtime {
        entries: flatten(&initial_config),
        icons: HashMap::new(),
        config: initial_config,
        config_path: config_path.clone(),
        selected_group: None,
        query: String::new(),
        page: 0,
        #[cfg(windows)]
        activation_pending: false,
        error: initial_error,
    }));
    let icons = load_icons(&runtime.borrow().entries);
    runtime.borrow_mut().icons = icons;

    let autostart_enabled = match autostart::enabled() {
        Ok(enabled) => enabled,
        Err(error) => {
            runtime.borrow_mut().error = Some(format!("Could not read startup setting: {error:#}"));
            false
        }
    };
    ui.set_groups(ModelRc::new(VecModel::from(
        groups
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    )));
    ui.set_autostart(autostart_enabled);
    ui.set_hotkey(runtime.borrow().config.settings.hotkey.clone().into());
    refresh_ui(&ui, &runtime);

    ui.on_query_changed({
        let runtime = runtime.clone();
        let weak = ui.as_weak();
        move |query| {
            {
                let mut state = runtime.borrow_mut();
                state.query = query.to_string();
                state.page = 0;
            }
            if let Some(ui) = weak.upgrade() {
                refresh_ui(&ui, &runtime);
            }
        }
    });
    ui.on_select_group({
        let runtime = runtime.clone();
        let weak = ui.as_weak();
        move |group| {
            let selected_group = if group > 0 {
                runtime
                    .borrow()
                    .config
                    .groups
                    .get((group - 1) as usize)
                    .map(|group| group.name.clone())
            } else {
                None
            };
            {
                let mut state = runtime.borrow_mut();
                state.selected_group = selected_group;
                state.page = 0;
            }
            if let Some(ui) = weak.upgrade() {
                ui.set_active_group(group);
                refresh_ui(&ui, &runtime);
            }
        }
    });
    ui.on_change_page({
        let runtime = runtime.clone();
        let weak = ui.as_weak();
        move |page| {
            runtime.borrow_mut().page = page.max(0) as usize;
            if let Some(ui) = weak.upgrade() {
                refresh_ui(&ui, &runtime);
            }
        }
    });
    ui.on_launch({
        let runtime = runtime.clone();
        let weak = ui.as_weak();
        move |index| {
            let app = {
                let state = runtime.borrow();
                filter(
                    &state.entries,
                    state.selected_group.as_deref(),
                    &state.query,
                )
                .get(state.page * APPS_PER_PAGE + index as usize)
                .map(|entry| entry.app.clone())
            };
            if let Some(app) = app {
                match launch::launch(&app) {
                    Ok(()) => {
                        if let Some(ui) = weak.upgrade() {
                            let _ = ui.hide();
                        }
                    }
                    Err(error) => {
                        runtime.borrow_mut().error =
                            Some(format!("Could not launch {}: {error:#}", app.name));
                        if let Some(ui) = weak.upgrade() {
                            refresh_ui(&ui, &runtime);
                        }
                    }
                }
            }
        }
    });
    ui.on_autostart_changed({
        let weak = ui.as_weak();
        move |enabled| {
            if let Err(error) = autostart::set_enabled(enabled) {
                eprintln!("Could not update startup setting: {error:#}");
                if let Some(ui) = weak.upgrade() {
                    ui.set_autostart(autostart::enabled().unwrap_or(false));
                    ui.set_error_text(
                        format!("Could not update startup setting: {error:#}").into(),
                    );
                }
            }
        }
    });
    ui.on_edit_config({
        let path = config_path.clone();
        move || {
            open_config(&path);
        }
    });
    ui.on_dismiss({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                let _ = ui.hide();
            }
        }
    });

    let (reload_tx, reload_rx) = mpsc::channel::<()>();
    let watcher = {
        let tx = reload_tx.clone();
        let mut watcher =
            notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
                if result.is_ok() {
                    let _ = tx.send(());
                }
            })
            .context("could not watch the configuration file")?;
        watcher
            .watch(
                config_path
                    .parent()
                    .context("configuration path has no parent")?,
                notify::RecursiveMode::NonRecursive,
            )
            .context("could not watch the configuration directory")?;
        watcher
    };
    let _watcher = watcher;

    #[cfg(windows)]
    let (mut hotkey_registration, hotkey_rx) =
        match hotkey::register(&runtime.borrow().config.settings.hotkey) {
            Ok((registration, receiver)) => (Some(registration), receiver),
            Err(error) => {
                runtime.borrow_mut().error = Some(format!("Global hotkey unavailable: {error:#}"));
                let (_, receiver) = mpsc::channel();
                (None, receiver)
            }
        };
    #[cfg(windows)]
    let (_tray_icon, tray_rx) = match tray::create() {
        Ok((icon, receiver)) => (Some(icon), receiver),
        Err(error) => {
            runtime.borrow_mut().error = Some(format!("System tray unavailable: {error:#}"));
            let (_, receiver) = mpsc::channel();
            (None, receiver)
        }
    };

    let weak = ui.as_weak();
    let runtime_for_timer = runtime.clone();
    let mut reload_deadline = None;
    #[cfg(windows)]
    let mut geometry_deadline = Instant::now();
    let reload_timer = slint::Timer::default();
    reload_timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(25),
        move || {
            #[cfg(windows)]
            {
                if runtime_for_timer.borrow().activation_pending
                    && let Some(ui) = weak.upgrade()
                    && ui.window().is_visible()
                    && set_topmost(&ui)
                {
                    runtime_for_timer.borrow_mut().activation_pending = false;
                }
                if Instant::now() >= geometry_deadline {
                    geometry_deadline = Instant::now() + Duration::from_millis(250);
                    if let Some(ui) = weak.upgrade()
                        && ui.window().is_visible()
                        && !runtime_for_timer.borrow().activation_pending
                    {
                        update_window_geometry(&ui);
                    }
                }
                if hotkey_rx.try_recv().is_ok()
                    && let Some(ui) = weak.upgrade()
                {
                    if ui.window().is_visible() && is_foreground_window(&ui) {
                        let _ = ui.hide();
                    } else {
                        show_launcher(&ui, &runtime_for_timer);
                    }
                }
                if let Some(action) = tray_rx.try_iter().last()
                    && let Some(ui) = weak.upgrade()
                {
                    match action {
                        tray::Action::Open => {
                            show_launcher(&ui, &runtime_for_timer);
                        }
                        tray::Action::Settings => {
                            show_launcher(&ui, &runtime_for_timer);
                            ui.set_settings_visible(true);
                        }
                        tray::Action::EditConfig => {
                            open_config(&runtime_for_timer.borrow().config_path)
                        }
                        tray::Action::Exit => {
                            if let Err(error) = slint::quit_event_loop() {
                                runtime_for_timer.borrow_mut().error =
                                    Some(format!("Could not exit launcher: {error}"));
                                show_launcher(&ui, &runtime_for_timer);
                            }
                        }
                    }
                }
                if instance.show_requested()
                    && let Some(ui) = weak.upgrade()
                {
                    show_launcher(&ui, &runtime_for_timer);
                }
                if let Some(ui) = weak.upgrade()
                    && ui.window().is_visible()
                    && !runtime_for_timer.borrow().activation_pending
                    && !is_foreground_window(&ui)
                {
                    let _ = ui.hide();
                }
            }
            for _ in reload_rx.try_iter() {
                reload_deadline = Some(Instant::now() + Duration::from_millis(300));
            }
            if reload_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                reload_deadline = None;
                let path = runtime_for_timer.borrow().config_path.clone();
                match config::load(&path) {
                    Ok(config) => {
                        let hotkey = config.settings.hotkey.clone();
                        {
                            let mut state = runtime_for_timer.borrow_mut();
                            state.entries = flatten(&config);
                            state.icons = load_icons(&state.entries);
                            state.config = config;
                            state.error = None;
                            state.selected_group = None;
                            state.query.clear();
                            state.page = 0;
                        }
                        #[cfg(windows)]
                        if let Some(registration) = hotkey_registration.as_mut()
                            && let Err(error) = registration.update(&hotkey)
                        {
                            runtime_for_timer.borrow_mut().error =
                                Some(format!("Global hotkey unavailable: {error:#}"));
                        }
                        if let Some(ui) = weak.upgrade() {
                            let groups = {
                                let state = runtime_for_timer.borrow();
                                group_labels(&state.config)
                                    .into_iter()
                                    .map(SharedString::from)
                                    .collect::<Vec<_>>()
                            };
                            ui.set_groups(ModelRc::new(VecModel::from(groups)));
                            ui.set_query(SharedString::default());
                            ui.set_active_group(0);
                            ui.set_hotkey(hotkey.into());
                            refresh_ui(&ui, &runtime_for_timer);
                        }
                    }
                    Err(error) => {
                        runtime_for_timer.borrow_mut().error = Some(format!(
                            "Config error: {error:#} (keeping the last valid configuration)"
                        ));
                        if let Some(ui) = weak.upgrade() {
                            refresh_ui(&ui, &runtime_for_timer);
                        }
                    }
                }
            }
        },
    );

    #[cfg(not(windows))]
    ui.show().context("could not show launcher")?;
    #[cfg(windows)]
    if !std::env::args().any(|arg| arg == "--background") {
        let weak = ui.as_weak();
        let runtime = runtime.clone();
        // Winit can only create the native window once its event loop is running.
        slint::Timer::single_shot(Duration::ZERO, move || {
            if let Some(ui) = weak.upgrade() {
                show_launcher(&ui, &runtime);
            }
        });
    }

    // The native tray icon is external to Slint, so hiding the popup must not end its loop.
    #[cfg(windows)]
    slint::run_event_loop_until_quit().context("launcher event loop failed")?;
    #[cfg(not(windows))]
    slint::run_event_loop().context("launcher event loop failed")?;
    let _ = ui.hide();
    Ok(())
}

fn refresh_ui(ui: &MainWindow, runtime: &Rc<RefCell<Runtime>>) {
    let mut state = runtime.borrow_mut();
    let shown = filter(
        &state.entries,
        state.selected_group.as_deref(),
        &state.query,
    );
    let (page, page_count) = page_bounds(shown.len(), state.page);
    let items = shown
        .iter()
        .skip(page * APPS_PER_PAGE)
        .take(APPS_PER_PAGE)
        .map(|entry| {
            let icon = state
                .icons
                .get(entry.app.icon.as_deref().unwrap_or(&entry.app.path))
                .cloned()
                .unwrap_or_default();
            AppItem {
                name: entry.app.name.clone().into(),
                group: entry.group.clone().into(),
                path: entry.app.path.clone().into(),
                icon,
                missing: !Path::new(&entry.app.path).exists(),
                initial: entry
                    .app
                    .name
                    .chars()
                    .next()
                    .unwrap_or('?')
                    .to_string()
                    .into(),
            }
        })
        .collect::<Vec<_>>();
    state.page = page;
    ui.set_active_page(page as i32);
    ui.set_page_count(page_count as i32);
    ui.set_apps(ModelRc::new(VecModel::from(items)));
    ui.set_error_text(state.error.clone().unwrap_or_default().into());
}

fn group_labels(config: &config::Config) -> Vec<String> {
    let mut labels = vec!["All".to_owned()];
    for group in &config.groups {
        let base = if group.name.eq_ignore_ascii_case("All") {
            "All (group)".to_owned()
        } else {
            group.name.clone()
        };
        let mut label = base.clone();
        let mut suffix = 2;
        while labels
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(&label))
        {
            label = format!("{base} ({suffix})");
            suffix += 1;
        }
        labels.push(label);
    }
    labels
}

#[cfg(windows)]
fn show_launcher(ui: &MainWindow, runtime: &Rc<RefCell<Runtime>>) {
    {
        let mut state = runtime.borrow_mut();
        state.selected_group = None;
        state.query.clear();
        state.page = 0;
    }
    ui.set_query(SharedString::default());
    ui.set_active_group(0);
    ui.set_settings_visible(false);
    refresh_ui(ui, runtime);
    match ui.show() {
        Ok(()) => runtime.borrow_mut().activation_pending = !set_topmost(ui),
        Err(error) => {
            runtime.borrow_mut().error = Some(format!("Could not show launcher: {error}"));
            refresh_ui(ui, runtime);
        }
    }
}

#[cfg(windows)]
fn update_window_geometry(ui: &MainWindow) {
    if let Err(error) = window_geometry::update(
        ui.window(),
        ui.get_launcher_width(),
        ui.get_launcher_height(),
    ) {
        let message = format!("Could not position launcher: {error:#}");
        if ui.get_error_text().as_str() != message {
            eprintln!("{message}");
            ui.set_error_text(message.into());
        }
    }
}

#[cfg(windows)]
fn is_foreground_window(ui: &MainWindow) -> bool {
    use raw_window_handle::HasWindowHandle;
    use slint::ComponentHandle;
    use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::GetForegroundWindow};

    let raw_window = ui.window().window_handle();
    let Ok(handle) = raw_window.window_handle() else {
        return true;
    };
    let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return true;
    };
    let window = HWND(win32.hwnd.get() as *mut _);
    unsafe { GetForegroundWindow() == window }
}

fn load_icons(entries: &[AppEntry]) -> HashMap<String, Image> {
    let mut icons = HashMap::new();
    for entry in entries {
        let source = entry.app.icon.as_deref().unwrap_or(&entry.app.path);
        if !icons.contains_key(source) {
            icons.insert(source.to_owned(), icon_for(source));
        }
    }
    icons
}

fn icon_for(source: &str) -> Image {
    let path = Path::new(source);
    let is_raster_icon = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "ico" | "png" | "jpg" | "jpeg"
            )
        });
    if is_raster_icon
        && std::fs::metadata(path).is_ok_and(|metadata| metadata.len() <= MAX_ICON_FILE_SIZE)
        && let Ok(mut reader) = image::ImageReader::open(path)
            .and_then(|reader| reader.with_guessed_format().map_err(std::io::Error::other))
    {
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(2048);
        limits.max_image_height = Some(2048);
        limits.max_alloc = Some(64 * 1024 * 1024);
        reader.limits(limits);
        if let Ok(decoded) = reader.decode() {
            let rgba = decoded.to_rgba8();
            return image_from_rgba(rgba.as_raw(), rgba.width(), rgba.height());
        }
    }
    #[cfg(windows)]
    if let Some(image) = extract_shell_icon(source) {
        return image;
    }
    Image::default()
}

fn image_from_rgba(pixels: &[u8], width: u32, height: u32) -> Image {
    let buffer =
        slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(pixels, width, height);
    Image::from_rgba8(buffer)
}

#[cfg(windows)]
fn extract_shell_icon(source: &str) -> Option<Image> {
    use std::{mem::size_of, os::windows::ffi::OsStrExt, ptr};
    use windows::{
        Win32::{
            Foundation::HANDLE,
            Graphics::Gdi::{
                BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection,
                DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, HGDIOBJ, ReleaseDC, SelectObject,
            },
            Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
            UI::{
                Shell::{SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGetFileInfoW},
                WindowsAndMessaging::{DI_NORMAL, DestroyIcon, DrawIconEx},
            },
        },
        core::PCWSTR,
    };

    const ICON_SIZE: i32 = 64;
    let path: Vec<u16> = std::ffi::OsStr::new(source)
        .encode_wide()
        .chain(Some(0))
        .collect();
    unsafe {
        let mut file_info = SHFILEINFOW::default();
        let result = SHGetFileInfoW(
            PCWSTR(path.as_ptr()),
            FILE_ATTRIBUTE_NORMAL,
            Some(&mut file_info),
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        );
        if result == 0 || file_info.hIcon.0.is_null() {
            return None;
        }

        let screen_dc = GetDC(None);
        if screen_dc.0.is_null() {
            let _ = DestroyIcon(file_info.hIcon);
            return None;
        }
        let memory_dc = CreateCompatibleDC(Some(screen_dc));
        if memory_dc.0.is_null() {
            let _ = ReleaseDC(None, screen_dc);
            let _ = DestroyIcon(file_info.hIcon);
            return None;
        }

        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: ICON_SIZE,
                biHeight: -ICON_SIZE,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = ptr::null_mut();
        let bitmap = match CreateDIBSection(
            Some(memory_dc),
            &info,
            DIB_RGB_COLORS,
            &mut pixels,
            None::<HANDLE>,
            0,
        ) {
            Ok(bitmap) => bitmap,
            Err(_) => {
                let _ = DeleteDC(memory_dc);
                let _ = ReleaseDC(None, screen_dc);
                let _ = DestroyIcon(file_info.hIcon);
                return None;
            }
        };
        if pixels.is_null() {
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
            let _ = DeleteDC(memory_dc);
            let _ = ReleaseDC(None, screen_dc);
            let _ = DestroyIcon(file_info.hIcon);
            return None;
        }
        let old_bitmap = SelectObject(memory_dc, HGDIOBJ(bitmap.0));
        let drawn = DrawIconEx(
            memory_dc,
            0,
            0,
            file_info.hIcon,
            ICON_SIZE,
            ICON_SIZE,
            0,
            None,
            DI_NORMAL,
        );
        let image = if drawn.is_ok() {
            let bgra = std::slice::from_raw_parts(
                pixels.cast::<u8>(),
                (ICON_SIZE * ICON_SIZE * 4) as usize,
            );
            let mut rgba = Vec::with_capacity(bgra.len());
            for pixel in bgra.as_chunks::<4>().0 {
                rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
            }
            Some(image_from_rgba(&rgba, ICON_SIZE as u32, ICON_SIZE as u32))
        } else {
            None
        };
        let _ = SelectObject(memory_dc, old_bitmap);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(memory_dc);
        let _ = ReleaseDC(None, screen_dc);
        let _ = DestroyIcon(file_info.hIcon);
        image
    }
}

fn open_config(path: &Path) {
    #[cfg(windows)]
    let result = security::system_notepad().and_then(|notepad| {
        std::process::Command::new(notepad)
            .arg(path)
            .spawn()
            .context("could not start the Windows system editor")
    });
    #[cfg(not(windows))]
    let result = std::process::Command::new("xdg-open").arg(path).spawn();
    if let Err(error) = result {
        eprintln!("Could not open config file: {error}");
    }
}

#[cfg(windows)]
fn set_topmost(ui: &MainWindow) -> bool {
    use raw_window_handle::HasWindowHandle;
    use slint::ComponentHandle;
    use windows::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{
            GWL_EXSTYLE, GWL_STYLE, GetWindowLongPtrW, HWND_TOPMOST, SW_RESTORE, SWP_FRAMECHANGED,
            SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SetForegroundWindow, SetWindowLongPtrW,
            SetWindowPos, ShowWindow, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW, WS_MINIMIZEBOX,
        },
    };
    let raw_window = ui.window().window_handle();
    if let Ok(handle) = raw_window.window_handle()
        && let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw()
    {
        let hwnd = HWND(win32.hwnd.get() as *mut _);
        unsafe {
            let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let _ = SetWindowLongPtrW(
                hwnd,
                GWL_EXSTYLE,
                (style | WS_EX_TOOLWINDOW.0 as isize) & !(WS_EX_APPWINDOW.0 as isize),
            );
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
            let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, style & !(WS_MINIMIZEBOX.0 as isize));
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_FRAMECHANGED,
            );
            let _ = SetForegroundWindow(hwnd);
        }
        update_window_geometry(ui);
        return true;
    }
    false
}

#[cfg(windows)]
mod hotkey;
#[cfg(windows)]
mod tray;

#[cfg(windows)]
struct Instance {
    _mutex: windows::Win32::Foundation::HANDLE,
    event: windows::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
impl Instance {
    fn show_requested(&self) -> bool {
        use windows::Win32::System::Threading::{ResetEvent, WaitForSingleObject};
        unsafe {
            if WaitForSingleObject(self.event, 0) == windows::Win32::Foundation::WAIT_OBJECT_0 {
                let _ = ResetEvent(self.event);
                true
            } else {
                false
            }
        }
    }
}

#[cfg(windows)]
fn single_instance() -> Result<Instance> {
    use windows::Win32::{
        Foundation::{CloseHandle, ERROR_ALREADY_EXISTS},
        System::Threading::{CreateEventW, CreateMutexW, SetEvent},
    };
    use windows::core::PCWSTR;
    let mutex_name: Vec<u16> = "Local\\AppLauncher.Singleton\0".encode_utf16().collect();
    let event_name: Vec<u16> = "Local\\AppLauncher.Show\0".encode_utf16().collect();
    unsafe {
        let event = CreateEventW(None, true, false, PCWSTR(event_name.as_ptr()))
            .context("could not create single-instance event")?;
        let mutex = CreateMutexW(None, true, PCWSTR(mutex_name.as_ptr()))
            .context("could not create single-instance mutex")?;
        let already_exists = windows::Win32::Foundation::GetLastError() == ERROR_ALREADY_EXISTS;
        if already_exists {
            let _ = SetEvent(event);
            let _ = CloseHandle(event);
            let _ = CloseHandle(mutex);
            std::process::exit(0);
        }
        Ok(Instance {
            _mutex: mutex,
            event,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_pages_results_without_growing_the_fixed_window() {
        use slint::{
            Model,
            platform::{
                Platform, WindowAdapter,
                software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
            },
        };

        struct TestPlatform(Rc<MinimalSoftwareWindow>);
        impl Platform for TestPlatform {
            fn create_window_adapter(
                &self,
            ) -> std::result::Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
                Ok(self.0.clone())
            }
        }
        slint::platform::set_platform(Box::new(TestPlatform(MinimalSoftwareWindow::new(
            RepaintBufferType::ReusedBuffer,
        ))))
        .unwrap();
        let config = config::parse(&format!(
            "groups:\n  - name: Test\n    apps:\n{}",
            (1..=21)
                .map(|index| format!(
                    "      - name: App {index}\n        path: C:\\Apps\\missing.exe\n"
                ))
                .collect::<String>()
        ))
        .unwrap();
        let runtime = Rc::new(RefCell::new(Runtime {
            entries: flatten(&config),
            config,
            config_path: PathBuf::new(),
            icons: HashMap::new(),
            selected_group: None,
            query: String::new(),
            page: 0,
            #[cfg(windows)]
            activation_pending: false,
            error: None,
        }));
        let ui = MainWindow::new().unwrap();
        refresh_ui(&ui, &runtime);
        assert_eq!(ui.get_launcher_width(), 704.0);
        assert_eq!(ui.get_launcher_height(), 457.0);
        assert_eq!(ui.get_page_count(), 3);
        assert_eq!(ui.get_apps().row_count(), 10);
        assert_eq!(ui.get_apps().row_data(9).unwrap().name, "App 10");

        runtime.borrow_mut().page = 1;
        refresh_ui(&ui, &runtime);
        assert_eq!(ui.get_active_page(), 1);
        assert_eq!(ui.get_apps().row_count(), 10);
        assert_eq!(ui.get_apps().row_data(0).unwrap().name, "App 11");
        assert_eq!(ui.get_apps().row_data(9).unwrap().name, "App 20");

        runtime.borrow_mut().page = 2;
        refresh_ui(&ui, &runtime);
        assert_eq!(ui.get_apps().row_count(), 1);
        assert_eq!(ui.get_apps().row_data(0).unwrap().name, "App 21");
        assert_eq!(ui.get_launcher_height(), 457.0);

        runtime.borrow_mut().query = "App 21".into();
        refresh_ui(&ui, &runtime);
        assert_eq!(ui.get_active_page(), 0);
        assert_eq!(runtime.borrow().page, 0);
        assert_eq!(ui.get_page_count(), 1);

        runtime.borrow_mut().error = Some("Test error".into());
        refresh_ui(&ui, &runtime);
        assert_eq!(ui.get_launcher_height(), 495.0);
        assert_eq!(ui.get_launcher_width(), 704.0);
    }

    #[test]
    fn pagination_keeps_every_app_accessible_in_ten_item_pages() {
        for count in [0, 1, 5, 10, 11, 20, 21, 100] {
            let (_, pages) = page_bounds(count, 0);
            let indices = (0..pages)
                .flat_map(|page| (page * APPS_PER_PAGE..count).take(APPS_PER_PAGE))
                .collect::<Vec<_>>();
            assert_eq!(indices, (0..count).collect::<Vec<_>>());
            assert_eq!(page_bounds(count, usize::MAX), (pages - 1, pages));
        }
        assert_eq!(page_bounds(0, 2), (0, 1));
        assert_eq!(page_bounds(10, 1), (0, 1));
        assert_eq!(page_bounds(11, 1), (1, 2));
    }

    #[test]
    fn group_labels_disambiguate_reserved_and_duplicate_labels() {
        let config = config::parse(
            "groups:\n  - name: All\n    apps: []\n  - name: All (group)\n    apps: []\n  - name: All (group) (2)\n    apps: []\n",
        )
        .unwrap();
        assert_eq!(
            group_labels(&config),
            vec![
                "All".to_owned(),
                "All (group)".to_owned(),
                "All (group) (2)".to_owned(),
                "All (group) (2) (2)".to_owned()
            ]
        );
    }
}
