use crate::{
    trace::{frontend_event_handler::StepState, layers::EventLayer, AppEvent},
    utils::SendEvent,
};
use scenario_rs::trace::ScenarioEventVisitor;
use std::sync::mpsc::Sender;
use tracing::{error, Event};

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
        let mut visitor = ScenarioEventVisitor::default();

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

                        self.sender.send_event(AppEvent::StepState {
                            state: StepState::StepFailed {
                                message: error.to_string(),
                            },
                        });
                    } else {
                        self.sender.send_event(AppEvent::LogMessage(format!(
                            "{}Scenario execution failed",
                            SCENARIO_PREFIX
                        )));

                        self.sender.send_event(AppEvent::StepState {
                            state: StepState::StepFailed {
                                message: "Scenario execution failed".to_string(),
                            },
                        });
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
                        self.sender.send_event(AppEvent::StepIndex { index });
                    }
                }
                "step_completed" => {
                    if let Some(index) = visitor.index {
                        self.sender.send_event(AppEvent::StepState {
                            state: StepState::StepCompleted { index },
                        });
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
                "remote_sudo_output" => {
                    if let (Some(command), Some(output)) = (&visitor.command, &visitor.output) {
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

                        self.sender.send_event(AppEvent::StepState {
                            state: StepState::RemoteSudoOutput {
                                command: command.to_owned(),
                                output: output.to_owned(),
                            },
                        });
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
                    if let (Some(current), Some(total), Some(source), Some(destination)) = (
                        visitor.current,
                        visitor.total,
                        visitor.source.as_ref(),
                        visitor.destination.as_ref(),
                    ) {
                        let percentage = (current as f64 / total as f64) * 100.0;
                        self.sender.send_event(AppEvent::LogMessage(format!(
                            "{}Progress: {:.1}%",
                            SCENARIO_PREFIX, percentage
                        )));

                        self.sender.send_event(AppEvent::StepState {
                            state: StepState::SftpCopyProgress {
                                source: source.to_owned(),
                                destination: destination.to_owned(),
                                current,
                                total,
                            },
                        });
                    }
                }
                "on_fail_steps_started" => {
                    self.sender.send_event(AppEvent::LogMessage(format!(
                        "{}[on_fail] Starting failure recovery steps",
                        SCENARIO_PREFIX
                    )));
                }
                "on_fail_steps_completed" => {
                    self.sender.send_event(AppEvent::LogMessage(format!(
                        "{}[on_fail] Failure recovery steps completed",
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
                "create_session_started" => {}
                "created_mock_session" => {}
                "session_created" => {}
                "steps_started" => {}
                "remote_sudo_completed" => {}
                "steps_completed" => {}
                "on_fail_step_completed" => {}
                _ => {
                    error!("Unrecognized event type: {}", event_type);
                }
            }
        }
    }
}
