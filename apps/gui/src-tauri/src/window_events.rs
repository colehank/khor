//! Delivery of native events to the Windows main WebView over its command bridge.

use std::collections::HashSet;
use std::sync::Mutex;
use tauri::{AppHandle, Event, EventId, EventTarget, Listener, Manager, WebviewWindow};

pub struct MainWindowEvents {
    window: WebviewWindow,
    listeners: Mutex<HashSet<EventId>>,
}

impl MainWindowEvents {
    pub fn new(window: WebviewWindow) -> Self {
        Self {
            window,
            listeners: Mutex::new(HashSet::new()),
        }
    }

    pub fn listen(
        &self,
        app: &AppHandle,
        origin: &str,
        event: String,
        target: EventTarget,
        callback: u32,
    ) -> Result<EventId, String> {
        if event.is_empty()
            || event.len() > 128
            || !event.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b':' | b'/' | b'_')
            })
        {
            return Err("invalid window event name".into());
        }
        let mut listeners = self.listeners.lock().map_err(|error| error.to_string())?;
        let window = self.window.clone();
        let name = event.clone();
        let origin = serde_json::to_string(origin).map_err(|error| error.to_string())?;
        let forward = move |event: Event| {
            let Ok(payload) = serde_json::from_str::<serde_json::Value>(event.payload()) else {
                return;
            };
            let message =
                serde_json::json!({ "event": name, "id": event.id(), "payload": payload });
            let js = format!(
                "if (location.origin === {origin}) window.__TAURI_INTERNALS__?.runCallback({callback}, {message});"
            );
            let _ = window.eval(js);
        };
        let id = match target {
            EventTarget::Any => app.listen_any(event, forward),
            EventTarget::Window { label } if label == self.window.label() => app
                .get_window(&label)
                .ok_or("main window closed")?
                .listen(event, forward),
            EventTarget::WebviewWindow { label } if label == self.window.label() => {
                self.window.listen(event, forward)
            }
            _ => return Err("window event target is outside the main window".into()),
        };
        listeners.insert(id);
        Ok(id)
    }

    pub fn unlisten(&self, id: EventId) -> Result<(), String> {
        let mut listeners = self.listeners.lock().map_err(|error| error.to_string())?;
        if listeners.remove(&id) {
            self.window.unlisten(id);
        }
        Ok(())
    }
}
