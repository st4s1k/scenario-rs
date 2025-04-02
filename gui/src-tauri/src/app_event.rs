use crate::{
    app::ScenarioAppState,
    event::{EventChannel, EventHandler},
    utils::{LogMessage, SafeLock},
};
use std::sync::{mpsc::Sender, Mutex};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone)]
pub enum AppEvent {
    LogMessage(String),
    ClearLog,
}

struct AppEventHandler;

impl EventHandler<AppEvent> for AppEventHandler {
    fn is_terminal(&self, _: &AppEvent) -> bool {
        false
    }

    fn handle(&self, event: &AppEvent, app_handle: &AppHandle) {
        match event {
            AppEvent::LogMessage(message) => {
                app_handle.log_message(message.clone());
            }
            AppEvent::ClearLog => {
                let state = app_handle.state::<Mutex<ScenarioAppState>>();
                let mut state = state.safe_lock();
                state.output_log.clear();
                let _ = app_handle.emit("log-update", ());
            }
        }
    }
}

pub struct AppEventChannel(EventChannel<AppEvent>);

impl AppEventChannel {
    pub fn new(app_handle: &AppHandle) -> Self {
        Self(EventChannel::new(app_handle, AppEventHandler))
    }

    pub fn sender(&self) -> &Sender<AppEvent> {
        self.0.sender()
    }
}
