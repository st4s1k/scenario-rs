use crate::{
    event::{EventChannel, EventHandler},
    shared::SEPARATOR,
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
                app_handle.log_message("Scenario started...\n");
                let _ = app_handle.emit("execution-status", true);
            }
            ScenarioEvent::StepStarted {
                index,
                total_steps,
                description,
            } => {
                let task_number = index + 1;
                app_handle.log_message(format!(
                    "{SEPARATOR}\n[{task_number}/{total_steps}] {description}\n"
                ));
            }
            ScenarioEvent::RemoteSudoBefore(command) => {
                app_handle.log_message(format!("Executing:\n{command}\n"));
            }
            ScenarioEvent::RemoteSudoChannelOutput(output) => {
                let output = output.trim();
                let truncated_output = output
                    .chars()
                    .take(1000)
                    .collect::<String>()
                    .trim()
                    .to_string();
                app_handle.log_message(format!("{truncated_output}\n"));
                if output.len() > 1000 {
                    app_handle.log_message("...output truncated...\n");
                }
            }
            ScenarioEvent::SftpCopyBefore {
                source,
                destination,
            } => {
                app_handle.log_message(format!("Source:\n{source}\nDestination:\n{destination}\n"));
            }
            ScenarioEvent::SftpCopyProgress { current, total } => {
                let percentage = (*current as f64 / *total as f64) * 100.0;
                app_handle.log_message(format!("Progress: {:.1}%\n", percentage));
            }
            ScenarioEvent::OnFailStepsStarted => {
                app_handle.log_message(format!(
                    "{SEPARATOR}\n[on_fail] Starting failure recovery steps\n"
                ));
            }
            ScenarioEvent::OnFailStepStarted {
                index,
                total_steps,
                description,
            } => {
                let task_number = index + 1;
                app_handle.log_message(format!(
                    "{SEPARATOR}\n[on_fail] [{task_number}/{total_steps}] {description}\n"
                ));
            }
            ScenarioEvent::ScenarioCompleted => {
                app_handle.log_message(format!(
                    "{SEPARATOR}\nScenario completed successfully!\n{SEPARATOR}\n"
                ));
                let _ = app_handle.emit("execution-status", false);
            }
            ScenarioEvent::ScenarioError(error) => {
                app_handle.log_message(format!(
                    "{SEPARATOR}\nScenario execution failed: {error}\n{SEPARATOR}\n"
                ));
                let _ = app_handle.emit("execution-status", false);
            }
            ScenarioEvent::StepCompleted => {
                app_handle.log_message("Step completed\n");
            }
            ScenarioEvent::RemoteSudoAfter => {
                app_handle.log_message("Remote sudo command completed\n");
            }
            ScenarioEvent::SftpCopyAfter => {
                app_handle.log_message("SFTP copy finished\n");
            }
            ScenarioEvent::OnFailStepCompleted => {
                app_handle.log_message("On-fail step completed\n");
            }
            ScenarioEvent::OnFailStepsCompleted => {
                app_handle.log_message(format!("{SEPARATOR}\nOn-fail steps completed\n"));
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
