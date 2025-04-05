use crate::{
    event::{EventChannel, EventHandler},
    utils::LogMessage,
};
use scenario_rs::scenario::events::ScenarioEvent;
use std::sync::mpsc::Sender;
use tauri::{AppHandle, Emitter};

struct ScenarioEventHandler;

impl EventHandler<ScenarioEvent> for ScenarioEventHandler {
    fn is_terminal(&self, event: &ScenarioEvent) -> bool {
        matches!(
            event,
            ScenarioEvent::ScenarioCompleted | ScenarioEvent::ScenarioError(_)
        )
    }

    fn handle(&self, event: &ScenarioEvent, app_handle: &AppHandle) {
        match event {
            ScenarioEvent::ScenarioStarted => {
                app_handle.log_message("Scenario started...");
                let _ = app_handle.emit("execution-status", true);
            }
            ScenarioEvent::StepStarted {
                index,
                total_steps,
                description,
            } => {
                let task_number = index + 1;
                app_handle.log_message(format!("[{task_number}/{total_steps}] {description}"));
            }
            ScenarioEvent::RemoteSudoBefore(command) => {
                app_handle.log_message(format!("Executing: {command}"));
            }
            ScenarioEvent::RemoteSudoChannelOutput(output) => {
                let output = output.trim();
                let truncated_output = output
                    .chars()
                    .take(1000)
                    .collect::<String>()
                    .trim()
                    .to_string();
                app_handle.log_message(format!("{truncated_output}"));
                if output.len() > 1000 {
                    app_handle.log_message("...output truncated...");
                }
            }
            ScenarioEvent::SftpCopyBefore {
                source,
                destination,
            } => {
                app_handle.log_message(format!("Source: {source}"));
                app_handle.log_message(format!("Destination: {destination}"));
            }
            ScenarioEvent::SftpCopyProgress { current, total } => {
                let percentage = (*current as f64 / *total as f64) * 100.0;
                app_handle.log_message(format!("Progress: {:.1}%", percentage));
            }
            ScenarioEvent::OnFailStepsStarted => {
                app_handle.log_message(format!("[on_fail] Starting failure recovery steps"));
            }
            ScenarioEvent::OnFailStepStarted {
                index,
                total_steps,
                description,
            } => {
                let task_number = index + 1;
                app_handle.log_message(format!(
                    "[on_fail] [{task_number}/{total_steps}] {description}"
                ));
            }
            ScenarioEvent::ScenarioCompleted => {
                app_handle.log_message(format!("Scenario completed successfully!"));
                let _ = app_handle.emit("execution-status", false);
            }
            ScenarioEvent::ScenarioError(error) => {
                app_handle.log_message(format!("Scenario execution failed: {error}"));
                let _ = app_handle.emit("execution-status", false);
            }
            ScenarioEvent::StepCompleted => {
                app_handle.log_message("Step completed");
            }
            ScenarioEvent::RemoteSudoAfter => {
                app_handle.log_message("Remote sudo command completed");
            }
            ScenarioEvent::SftpCopyAfter => {
                app_handle.log_message("SFTP copy finished");
            }
            ScenarioEvent::OnFailStepCompleted => {
                app_handle.log_message("On-fail step completed");
            }
            ScenarioEvent::OnFailStepsCompleted => {
                app_handle.log_message(format!("On-fail steps completed"));
            }
        }
    }
}

pub struct ScenarioEventChannel(EventChannel<ScenarioEvent>);

impl ScenarioEventChannel {
    pub fn new(app_handle: &AppHandle) -> Self {
        Self(EventChannel::new(app_handle, ScenarioEventHandler))
    }

    pub fn sender(&self) -> &Sender<ScenarioEvent> {
        self.0.sender()
    }
}
