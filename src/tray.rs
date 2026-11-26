use anyhow::{Context, Result};
use std::sync::mpsc::{self, Receiver};
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem},
};

#[derive(Clone, Copy, Debug)]
pub enum Action {
    Open,
    Settings,
    EditConfig,
    Exit,
}

pub fn create() -> Result<(TrayIcon, Receiver<Action>)> {
    let menu = Menu::new();
    let open = MenuItem::new("Open", true, None);
    let settings = MenuItem::new("Settings", true, None);
    let edit = MenuItem::new("Edit config", true, None);
    let exit = MenuItem::new("Exit", true, None);
    menu.append(&open)
        .context("could not add Open to tray menu")?;
    menu.append(&settings)
        .context("could not add Settings to tray menu")?;
    menu.append(&edit)
        .context("could not add Edit config to tray menu")?;
    menu.append(&exit)
        .context("could not add Exit to tray menu")?;

    let pixels = (0..16 * 16)
        .flat_map(|index| {
            let x = index % 16;
            let y = index / 16;
            let rounded_corner = !(3..=12).contains(&x) && !(3..=12).contains(&y);
            if rounded_corner {
                [0, 0, 0, 0]
            } else if ((5..=6).contains(&x) || (9..=10).contains(&x))
                && ((5..=6).contains(&y) || (9..=10).contains(&y))
            {
                [255, 255, 255, 255]
            } else {
                [55, 153, 255, 255]
            }
        })
        .collect();
    let icon = Icon::from_rgba(pixels, 16, 16).context("could not create tray icon")?;
    let tray = TrayIconBuilder::new()
        .with_tooltip("App Launcher")
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .build()
        .context("could not create tray icon")?;

    let (tx, rx) = mpsc::channel();
    let tray_id = tray.id().clone();
    let open_id = open.id().clone();
    let settings_id = settings.id().clone();
    let edit_id = edit.id().clone();
    let exit_id = exit.id().clone();
    let menu_tx = tx.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let action = if event.id == open_id {
            Some(Action::Open)
        } else if event.id == settings_id {
            Some(Action::Settings)
        } else if event.id == edit_id {
            Some(Action::EditConfig)
        } else if event.id == exit_id {
            Some(Action::Exit)
        } else {
            None
        };
        if let Some(action) = action {
            let _ = menu_tx.send(action);
        }
    }));
    let tray_id_for_click = tray_id.clone();
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        let is_open_click = match event {
            TrayIconEvent::Click {
                id,
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            }
            | TrayIconEvent::DoubleClick {
                id,
                button: MouseButton::Left,
                ..
            } => id == tray_id_for_click,
            _ => false,
        };
        if is_open_click {
            let _ = tx.send(Action::Open);
        }
    }));
    Ok((tray, rx))
}
