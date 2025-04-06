use crate::event::{EventChannel, EventHandler};
use scenario_rs::scenario::events::ScenarioEvent;
use std::sync::mpsc::Sender;
use tauri::{AppHandle, Emitter};
use tracing::info;

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
                info!("Scenario started...");
                let _ = app_handle.emit("execution-status", true);
            }
            ScenarioEvent::StepStarted {
                index,
                total_steps,
                description,
            } => {
                let task_number = index + 1;
                info!("[{task_number}/{total_steps}] {description}");
            }
            ScenarioEvent::RemoteSudoBefore(command) => {
                info!("Executing: {command}");
            }
            ScenarioEvent::RemoteSudoChannelOutput(output) => {
                let output = output.trim();
                let truncated_output = output
                    .chars()
                    .take(1000)
                    .collect::<String>()
                    .trim()
                    .to_string();
                info!("{truncated_output}");
                if output.len() > 1000 {
                    info!("...output truncated...");
                }
            }
            ScenarioEvent::SftpCopyBefore {
                source,
                destination,
            } => {
                info!("Source: {source}");
                info!("Destination: {destination}");
            }
            ScenarioEvent::SftpCopyProgress { current, total } => {
                let percentage = (*current as f64 / *total as f64) * 100.0;
                info!("Progress: {:.1}%", percentage);
            }
            ScenarioEvent::OnFailStepsStarted => {
                info!("[on_fail] Starting failure recovery steps");
            }
            ScenarioEvent::OnFailStepStarted {
                index,
                total_steps,
                description,
            } => {
                let task_number = index + 1;
                info!("[on_fail] [{task_number}/{total_steps}] {description}");
            }
            ScenarioEvent::ScenarioCompleted => {
                info!("Scenario completed successfully!");
                let _ = app_handle.emit("execution-status", false);
            }
            ScenarioEvent::ScenarioError(error) => {
                info!("Scenario execution failed: {error}");
                let _ = app_handle.emit("execution-status", false);
            }
            ScenarioEvent::StepCompleted => {
                info!("Step completed");
            }
            ScenarioEvent::RemoteSudoAfter => {
                info!("Remote sudo command completed");
            }
            ScenarioEvent::SftpCopyAfter => {
                info!("SFTP copy finished");
            }
            ScenarioEvent::OnFailStepCompleted => {
                info!("On-fail step completed");
            }
            ScenarioEvent::OnFailStepsCompleted => {
                info!("On-fail steps completed");
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
