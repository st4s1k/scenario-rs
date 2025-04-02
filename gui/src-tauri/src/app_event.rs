use crate::{
    app::ScenarioAppState,
    event::{EventChannel, EventHandler},
    shared::SEPARATOR,
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
                app_handle.log_message(format!(
                    "{SEPARATOR}\nApplication state saved\n{SEPARATOR}\n"
                ));
            }
            AppEvent::FailedToSaveState(error) => {
                app_handle.log_message(format!(
                    "{SEPARATOR}\nFailed to save state: {error}\n{SEPARATOR}\n"
                ));
            }
            AppEvent::FailedToSerializeState(error) => {
                app_handle.log_message(format!(
                    "{SEPARATOR}\nFailed to serialize state: {error}\n{SEPARATOR}\n"
                ));
            }
            AppEvent::StateLoaded => {
                app_handle.log_message(format!(
                    "{SEPARATOR}\nApplication state loaded\n{SEPARATOR}\n"
                ));
            }
            AppEvent::FailedToLoadState(error) => {
                app_handle.log_message(format!(
                    "{SEPARATOR}\nFailed to load state: {error}\n{SEPARATOR}\n"
                ));
            }
            AppEvent::FailedToDeserializeState(error) => {
                app_handle.log_message(format!(
                    "{SEPARATOR}\nFailed to deserialize state: {error}\n{SEPARATOR}\n"
                ));
            }
            AppEvent::StateCleared => {
                app_handle.log_message(format!(
                    "{SEPARATOR}\nApplication state cleared\n{SEPARATOR}\n"
                ));
            }
            AppEvent::FailedToClearState(error) => {
                app_handle.log_message(format!(
                    "{SEPARATOR}\nFailed to clear state: {error}\n{SEPARATOR}\n"
                ));
            }
            AppEvent::ConfigLoaded(path) => {
                app_handle.log_message(format!(
                    "{SEPARATOR}\nConfiguration loaded from {path}\n{SEPARATOR}\n"
                ));
            }
            AppEvent::FailedToLoadConfig(error) => {
                app_handle.log_message(format!(
                    "{SEPARATOR}\nFailed to load configuration: {error}\n{SEPARATOR}\n"
                ));
            }
            AppEvent::RequiredVariablesUpdated => {
                app_handle.log_message(format!(
                    "{SEPARATOR}\nRequired variables updated\n{SEPARATOR}\n"
                ));
            }
            AppEvent::NoScenarioLoaded => {
                app_handle.log_message(format!("{SEPARATOR}\nNo scenario loaded\n{SEPARATOR}\n"));
            }
            AppEvent::FailedToGetResolvedVariables(error) => {
                app_handle.log_message(format!(
                    "{SEPARATOR}\nFailed to get resolved variables: {error}\n{SEPARATOR}\n"
                ));
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
