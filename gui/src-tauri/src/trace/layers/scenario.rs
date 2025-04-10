use crate::trace::{layers::EventLayer, AppEvent};
use scenario_rs::{trace::ScenarioEventVisitor, utils::SendEvent};
use std::sync::mpsc::Sender;
use tracing::Event;

pub struct ScenarioEventLayer {
    pub sender: Sender<AppEvent>,
}

impl ScenarioEventLayer {
    pub fn new(sender: Sender<AppEvent>) -> Self {
        Self { sender }
    }
}

impl EventLayer for ScenarioEventLayer {
    fn process_event(&self, event: &Event<'_>) {
        let mut visitor = ScenarioEventVisitor {
            event_type: None,
            description: None,
            index: None,
            total_steps: None,
            command: None,
            output: None,
            error: None,
            source: None,
            destination: None,
            current: None,
            total: None,
        };

        event.record(&mut visitor);

        const SCENARIO_PREFIX: &str = "[SCN] ";

        if let Some(event_type) = &visitor.event_type {
            match event_type.as_str() {
                "error" => {
                    if let Some(error) = &visitor.error {
                        self.sender.send_event(AppEvent::LogMessage(format!(
                            "{}{error}",
                            SCENARIO_PREFIX
                        )));
                    } else {
                        self.sender.send_event(AppEvent::LogMessage(format!(
                            "{}Scenario execution failed",
                            SCENARIO_PREFIX
                        )));
                    }
                    self.sender.send_event(AppEvent::Execution(false));
                }
                "scenario_started" => {
                    self.sender.send_event(AppEvent::LogMessage(format!(
                        "{}Scenario started...",
                        SCENARIO_PREFIX
                    )));
                    self.sender.send_event(AppEvent::Execution(true));
                }
                "scenario_completed" => {
                    self.sender.send_event(AppEvent::LogMessage(format!(
                        "{}Scenario completed successfully!",
                        SCENARIO_PREFIX
                    )));
                    self.sender.send_event(AppEvent::Execution(false));
                }
                "step_started" => {
                    if let (Some(index), Some(total_steps), Some(description)) = (
                        visitor.index,
                        visitor.total_steps,
                        visitor.description.as_ref(),
                    ) {
                        let task_number = index + 1;
                        self.sender.send_event(AppEvent::LogMessage(format!(
                            "{}[{task_number}/{total_steps}] {description}",
                            SCENARIO_PREFIX
                        )));
                    }
                }
                "remote_sudo_started" => {
                    if let Some(command) = &visitor.command {
                        self.sender.send_event(AppEvent::LogMessage(format!(
                            "{}Executing: {command}",
                            SCENARIO_PREFIX
                        )));
                    }
                }
                "remote_sudo_channel_output" => {
                    if let Some(output) = &visitor.output {
                        let output = output.trim();
                        let truncated_output = output
                            .chars()
                            .take(1000)
                            .collect::<String>()
                            .trim()
                            .to_string();
                        self.sender
                            .send_event(AppEvent::LogMessage(truncated_output));
                        if output.len() > 1000 {
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{}...output truncated...",
                                SCENARIO_PREFIX
                            )));
                        }
                    }
                }
                "sftp_copy_started" => {
                    if let (Some(source), Some(destination)) =
                        (visitor.source.as_ref(), visitor.destination.as_ref())
                    {
                        self.sender.send_event(AppEvent::LogMessage(format!(
                            "{}Source: {source}",
                            SCENARIO_PREFIX
                        )));
                        self.sender.send_event(AppEvent::LogMessage(format!(
                            "{}Destination: {destination}",
                            SCENARIO_PREFIX
                        )));
                    }
                }
                "sftp_copy_completed" => {
                    self.sender.send_event(AppEvent::LogMessage(format!(
                        "{}SFTP copy finished",
                        SCENARIO_PREFIX
                    )));
                }
                "sftp_copy_progress" => {
                    if let (Some(current), Some(total)) = (visitor.current, visitor.total) {
                        let percentage = (current as f64 / total as f64) * 100.0;
                        self.sender.send_event(AppEvent::LogMessage(format!(
                            "{}Progress: {:.1}%",
                            SCENARIO_PREFIX, percentage
                        )));
                    }
                }
                "on_fail_steps_started" => {
                    self.sender.send_event(AppEvent::LogMessage(format!(
                        "{}[on_fail] Starting failure recovery steps",
                        SCENARIO_PREFIX
                    )));
                }
                "on_fail_step_started" => {
                    if let (Some(index), Some(total_steps), Some(description)) = (
                        visitor.index,
                        visitor.total_steps,
                        visitor.description.as_ref(),
                    ) {
                        let task_number = index + 1;
                        self.sender.send_event(AppEvent::LogMessage(format!(
                            "{}[on_fail] [{task_number}/{total_steps}] {description}",
                            SCENARIO_PREFIX
                        )));
                    }
                }
                _ => {}
            }
        }
    }
}
