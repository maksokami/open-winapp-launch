use anyhow::{Context, Result, bail};
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager,
    hotkey::{Code, HotKey, Modifiers},
};
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
    mpsc::{self, Receiver},
};

pub struct Registration {
    manager: GlobalHotKeyManager,
    hotkey: HotKey,
    event_id: Arc<AtomicU32>,
}

impl Registration {
    pub fn update(&mut self, spec: &str) -> Result<()> {
        let next = parse(spec)?;
        if next.id() == self.hotkey.id() {
            return Ok(());
        }
        self.manager
            .register(next)
            .with_context(|| format!("could not register hotkey {spec}"))?;
        if let Err(error) = self.manager.unregister(self.hotkey) {
            let _ = self.manager.unregister(next);
            return Err(error).context("could not replace the previous global hotkey");
        }
        self.hotkey = next;
        self.event_id.store(next.id(), Ordering::Release);
        Ok(())
    }
}

pub fn register(spec: &str) -> Result<(Registration, Receiver<()>)> {
    let hotkey = parse(spec)?;
    let manager = GlobalHotKeyManager::new().context("could not initialize global hotkeys")?;
    manager
        .register(hotkey)
        .with_context(|| format!("could not register hotkey {spec}"))?;

    let (tx, rx) = mpsc::channel();
    let event_id = Arc::new(AtomicU32::new(hotkey.id()));
    let event_id_for_handler = event_id.clone();
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.id == event_id_for_handler.load(Ordering::Acquire)
            && event.state == global_hotkey::HotKeyState::Pressed
        {
            let _ = tx.send(());
        }
    }));
    Ok((
        Registration {
            manager,
            hotkey,
            event_id,
        },
        rx,
    ))
}

fn parse(spec: &str) -> Result<HotKey> {
    let mut modifiers = Modifiers::empty();
    let mut key = None;
    for token in spec
        .split('+')
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        match token.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "alt" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            "win" | "super" => modifiers |= Modifiers::META,
            token => {
                if key.is_some() {
                    bail!("hotkey must contain exactly one non-modifier key");
                }
                key = Some(key_code(token).context("unsupported key in hotkey")?);
            }
        }
    }
    let key = key.context("hotkey is missing its key")?;
    if modifiers.is_empty() {
        bail!("hotkey must include at least one modifier");
    }
    Ok(HotKey::new(Some(modifiers), key))
}

fn key_code(token: &str) -> Option<Code> {
    Some(match token.to_ascii_uppercase().as_str() {
        "A" => Code::KeyA,
        "B" => Code::KeyB,
        "C" => Code::KeyC,
        "D" => Code::KeyD,
        "E" => Code::KeyE,
        "F" => Code::KeyF,
        "G" => Code::KeyG,
        "H" => Code::KeyH,
        "I" => Code::KeyI,
        "J" => Code::KeyJ,
        "K" => Code::KeyK,
        "L" => Code::KeyL,
        "M" => Code::KeyM,
        "N" => Code::KeyN,
        "O" => Code::KeyO,
        "P" => Code::KeyP,
        "Q" => Code::KeyQ,
        "R" => Code::KeyR,
        "S" => Code::KeyS,
        "T" => Code::KeyT,
        "U" => Code::KeyU,
        "V" => Code::KeyV,
        "W" => Code::KeyW,
        "X" => Code::KeyX,
        "Y" => Code::KeyY,
        "Z" => Code::KeyZ,
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "SPACE" => Code::Space,
        "F1" => Code::F1,
        "F2" => Code::F2,
        "F3" => Code::F3,
        "F4" => Code::F4,
        "F5" => Code::F5,
        "F6" => Code::F6,
        "F7" => Code::F7,
        "F8" => Code::F8,
        "F9" => Code::F9,
        "F10" => Code::F10,
        "F11" => Code::F11,
        "F12" => Code::F12,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_hotkey() {
        assert!(parse("Ctrl+Alt+E").is_ok());
        assert!(parse("Shift+F12").is_ok());
    }

    #[test]
    fn rejects_ambiguous_or_modifierless_hotkeys() {
        assert!(parse("Ctrl+E+F").is_err());
        assert!(parse("E").is_err());
        assert!(parse("Ctrl+Unknown").is_err());
    }
}
