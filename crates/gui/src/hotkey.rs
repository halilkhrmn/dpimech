//! Global hotkey: Ctrl+Alt+D toggles all profiles.
//! TODO(phase 5): make the shortcut configurable in Settings.

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use tokio::sync::mpsc;

use crate::bridge::Command;

/// Returns the manager, which must stay alive for the hotkey to remain registered.
/// Failing to register (e.g. the combination is taken) is not fatal.
pub fn register(commands: mpsc::UnboundedSender<Command>) -> Option<GlobalHotKeyManager> {
    let manager = GlobalHotKeyManager::new().ok()?;
    let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyD);
    manager.register(hotkey).ok()?;
    let id = hotkey.id();
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.id == id && event.state == HotKeyState::Pressed {
            let _ = commands.send(Command::ToggleAll);
        }
    }));
    Some(manager)
}
