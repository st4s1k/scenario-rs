use crate::{app::ScenarioAppState, shared::SEPARATOR};
use scenario_rs::scenario::events::Event;
use std::sync::{
    mpsc,
    mpsc::{Receiver, Sender},
    Mutex, OnceLock,
};
use tauri::{AppHandle, Emitter, Manager};

static LIFECYCLE_HANDLER: OnceLock<LifecycleHandler> = OnceLock::new();

#[derive(Debug)]
pub struct LifecycleHandler {
    pub app_handle: AppHandle,
}

impl LifecycleHandler {
    pub fn try_initialize(window: AppHandle) -> Sender<Event> {
        LIFECYCLE_HANDLER.get_or_init(|| LifecycleHandler::new(window));

        let (tx, rx) = mpsc::channel::<Event>();

        tauri::async_runtime::spawn(async move {
            for event in rx {
                if let Some(logger) = LIFECYCLE_HANDLER.get() {
                    logger.handle_event(&event);
                }

                if let Event::ScenarioCompleted | Event::ScenarioError(_) = event {
                    break;
                }
            }
        });

        tx
    }

    pub fn new(window: AppHandle) -> Self {
        Self { app_handle: window }
    }

    pub fn handle_event(&self, event: &Event) {
        match event {
            Event::ScenarioStarted => {
                self.log_message("Scenario started...\n".to_string());
                let _ = self.app_handle.emit("execution-status", true);
            }
            Event::StepStarted {
                index,
                total_steps,
                description,
            } => {
                let task_number = index + 1;
                self.log_message(format!(
                    "{SEPARATOR}\n[{task_number}/{total_steps}] {description}\n"
                ));
            }
            Event::RemoteSudoBefore(command) => {
                self.log_message(format!("Executing:\n{command}\n"));
            }
            Event::RemoteSudoChannelOutput(output) => {
                let output = output.trim();
                let truncated_output = output
                    .chars()
                    .take(1000)
                    .collect::<String>()
                    .trim()
                    .to_string();
                self.log_message(format!("{truncated_output}\n"));
                if output.len() > 1000 {
                    self.log_message("...output truncated...\n".to_string());
                }
            }
            Event::SftpCopyBefore {
                source,
                destination,
            } => {
                self.log_message(format!("Source:\n{source}\nDestination:\n{destination}\n"));
            }
            Event::SftpCopyProgress { current, total } => {
                // For GUI, we might want to update a progress indicator here
                // This is handled separately from logging text
                let percentage = (*current as f64 / *total as f64) * 100.0;
                self.log_message(format!("Progress: {:.1}%\n", percentage));
            }
            Event::OnFailStepsStarted => {
                self.log_message(format!(
                    "{SEPARATOR}\n[on_fail] Starting failure recovery steps\n"
                ));
            }
            Event::OnFailStepStarted {
                index,
                total_steps,
                description,
            } => {
                let task_number = index + 1;
                self.log_message(format!(
                    "{SEPARATOR}\n[on_fail] [{task_number}/{total_steps}] {description}\n"
                ));
            }
            Event::ScenarioCompleted => {
                self.log_message(format!(
                    "{SEPARATOR}\nScenario completed successfully!\n{SEPARATOR}\n"
                 ));
                let _ = self.app_handle.emit("execution-status", false);
            }
            Event::ScenarioError(error) => {
                self.log_message(format!(
                    "{SEPARATOR}\nScenario execution failed: {error}\n{SEPARATOR}\n"
                ));
                let _ = self.app_handle.emit("execution-status", false);
            }
            Event::StepCompleted => {
                self.log_message("Step completed\n".to_string());
            }
            Event::RemoteSudoAfter => {
                self.log_message("Remote sudo command completed\n".to_string());
            }
            Event::SftpCopyAfter => {
                self.log_message("SFTP copy finished\n".to_string());
            }
            Event::OnFailStepCompleted => {
                self.log_message("On-fail step completed\n".to_string());
            }
            Event::OnFailStepsCompleted => {
                self.log_message(format!("{SEPARATOR}\nOn-fail steps completed\n"));
            }
        }
    }

    pub fn log_message(&self, message: String) {
        let state = self.app_handle.state::<Mutex<ScenarioAppState>>();
        let mut state = state.lock().unwrap();
        state.output_log.push_str(&message);
        let _ = self.app_handle.emit("log-update", ());
    }
}
