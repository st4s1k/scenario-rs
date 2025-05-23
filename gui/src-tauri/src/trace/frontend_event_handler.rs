use crate::trace::event_handler::EventHandler;
use chrono::Local;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum StepState {
    SftpCopyProgress {
        source: String,
        destination: String,
        current: u64,
        total: u64,
    },
    RemoteSudoOutput {
        command: String,
        output: String,
    },
    StepStarted,
    StepCompleted,
    StepFailed {
        message: String,
    },
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    Execution(bool),
    LogMessage(String),
    LogPlainMessage(String),
    StepState {
        step_index: usize,
        steps_total: usize,
        state: StepState,
    },
    OnFailStepState {
        step_index: usize,
        steps_total: usize,
        on_fail_step_index: usize,
        on_fail_steps_total: usize,
        state: StepState,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct StepStateEvent {
    pub step_index: usize,
    pub steps_total: usize,
    pub state: StepState,
}

#[derive(Debug, Clone, Serialize)]
pub struct OnFailStepStateEvent {
    pub step_index: usize,
    pub steps_total: usize,
    pub on_fail_step_index: usize,
    pub on_fail_steps_total: usize,
    pub state: StepState,
}

pub struct FrontendEventHandler;

impl EventHandler<AppEvent> for FrontendEventHandler {
    fn is_terminal(&self, _event: &AppEvent) -> bool {
        false
    }

    fn handle(&self, event: &AppEvent, app_handle: &AppHandle) {
        match event {
            AppEvent::Execution(is_starting) => {
                let _ = app_handle.emit("execution-status", *is_starting);
            }
            AppEvent::LogMessage(message) => {
                let timestamp = Local::now().format("%H:%M:%S.%3f").to_string();
                let message = format!("[{timestamp}] {message}");
                let _ = app_handle.emit("log-message", message);
            }
            AppEvent::LogPlainMessage(text) => {
                let _ = app_handle.emit("log-message", text);
            }
            AppEvent::StepState {
                step_index,
                steps_total,
                state,
            } => {
                let event = StepStateEvent {
                    step_index: *step_index,
                    steps_total: *steps_total,
                    state: state.clone(),
                };
                let _ = app_handle.emit("step-state", event);
            }
            AppEvent::OnFailStepState {
                step_index,
                steps_total,
                on_fail_step_index,
                on_fail_steps_total,
                state,
            } => {
                let event = OnFailStepStateEvent {
                    step_index: *step_index,
                    steps_total: *steps_total,
                    on_fail_step_index: *on_fail_step_index,
                    on_fail_steps_total: *on_fail_steps_total,
                    state: state.clone(),
                };
                let _ = app_handle.emit("on-fail-step-state", event);
            }
        }
    }
}
