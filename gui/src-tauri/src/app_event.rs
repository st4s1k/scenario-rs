use crate::{
    app::ScenarioAppState,
    event::{EventChannel, EventHandler},
    utils::{LogMessage, SafeLock},
};
use scenario_rs::scenario::errors::{PlaceholderResolutionError, ScenarioError};
use std::sync::{mpsc::Sender, Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Debug)]
pub enum AppEvent {
    // State management events
    StateSuccessfullySaved,
    FailedToSaveState(Arc<std::io::Error>),
    FailedToSerializeState(Arc<serde_json::Error>),
    StateLoaded,
    FailedToLoadState(Arc<std::io::Error>),
    FailedToDeserializeState(Arc<serde_json::Error>),
    StateCleared,
    FailedToClearState(Arc<std::io::Error>),

    // Configuration events
    ConfigLoaded(String),
    FailedToLoadConfig(Arc<ScenarioError>),
    RequiredVariablesUpdated,

    // Execution events
    NoScenarioLoaded,

    // Resolved variables
    FailedToGetResolvedVariables(Arc<PlaceholderResolutionError>),

    // Log management
    ClearLog,
}

struct AppEventHandler;

impl EventHandler<AppEvent> for AppEventHandler {
    fn is_terminal(&self, _: &AppEvent) -> bool {
        false
    }

    fn handle(&self, event: &AppEvent, app_handle: &AppHandle) {
        match event {
            AppEvent::StateSuccessfullySaved => {
                app_handle.log_message(format!("Application state saved"));
            }
            AppEvent::FailedToSaveState(error) => {
                app_handle.log_message(format!("Failed to save state: {error}"));
            }
            AppEvent::FailedToSerializeState(error) => {
                app_handle.log_message(format!("Failed to serialize state: {error}"));
            }
            AppEvent::StateLoaded => {
                app_handle.log_message(format!("Application state loaded"));
            }
            AppEvent::FailedToLoadState(error) => {
                app_handle.log_message(format!("Failed to load state: {error}"));
            }
            AppEvent::FailedToDeserializeState(error) => {
                app_handle.log_message(format!("Failed to deserialize state: {error}"));
            }
            AppEvent::StateCleared => {
                app_handle.log_message(format!("Application state cleared"));
            }
            AppEvent::FailedToClearState(error) => {
                app_handle.log_message(format!("Failed to clear state: {error}"));
            }
            AppEvent::ConfigLoaded(path) => {
                app_handle.log_message(format!("Configuration loaded from {path}"));
            }
            AppEvent::FailedToLoadConfig(error) => {
                app_handle.log_message(format!("Failed to load configuration: {error}"));
            }
            AppEvent::RequiredVariablesUpdated => {
                app_handle.log_message(format!("Required variables updated"));
            }
            AppEvent::NoScenarioLoaded => {
                app_handle.log_message(format!("No scenario loaded"));
            }
            AppEvent::FailedToGetResolvedVariables(error) => {
                app_handle.log_message(format!("Failed to get resolved variables: {error}"));
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
