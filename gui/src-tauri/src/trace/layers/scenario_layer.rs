use crate::{
    trace::{frontend_event_handler::StepState, layers::EventLayer, AppEvent},
    utils::SendEvent,
};
use scenario_rs::trace::ScenarioEventVisitor;
use std::sync::mpsc::Sender;
use tracing::span::Record;
use tracing::{error, span::Attributes, Event, Id, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan};

pub struct ScenarioEventLayer {
    pub sender: Sender<AppEvent>,
}

impl ScenarioEventLayer {
    pub fn new(sender: Sender<AppEvent>) -> Self {
        Self { sender }
    }
}

impl EventLayer for ScenarioEventLayer {
    fn on_new_span<S>(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>)
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        let mut visitor = ScenarioEventVisitor::default();
        attrs.record(&mut visitor);
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(visitor);
        }
    }

    fn on_record<S>(&self, id: &Id, record: &Record<'_>, ctx: Context<'_, S>)
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        if let Some(span) = ctx.span(id) {
            if let Some(v) = span.extensions_mut().get_mut::<ScenarioEventVisitor>() {
                record.record(v);
            }
        }
    }

    fn process_event<S>(&self, event: &Event<'_>, ctx: Context<'_, S>)
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        let mut visitor = ScenarioEventVisitor::default();

        event.record(&mut visitor);

        if let Some(scope) = ctx.event_scope(event) {
            for span in scope.from_root() {
                if let Some(extra) = span.extensions().get::<ScenarioEventVisitor>() {
                    visitor.merge(extra);
                }
            }
        }

        const SCENARIO_PREFIX: &str = "[SCN]";

        if let Some(scenario_event) = visitor.scenario_event {
            match scenario_event.as_str() {
                "error" => {
                    self.sender.send_event(AppEvent::Execution(false));
                    if let Some(scenario_error) = visitor.scenario_error {
                        if let (Some(step_index), Some(steps_total)) =
                            (visitor.step_index, visitor.steps_total)
                        {
                            let state = StepState::StepFailed {
                                message: scenario_error.to_string(),
                            };
                            if let (Some(on_fail_step_index), Some(on_fail_steps_total)) =
                                (visitor.on_fail_step_index, visitor.on_fail_steps_total)
                            {
                                self.sender.send_event(AppEvent::LogMessage(format!(
                                    "{} [{}/{}] [on-fail] [{}/{}] {}",
                                    SCENARIO_PREFIX,
                                    step_index + 1,
                                    steps_total,
                                    on_fail_step_index + 1,
                                    on_fail_steps_total,
                                    scenario_error
                                )));
                                self.sender.send_event(AppEvent::OnFailStepState {
                                    step_index,
                                    steps_total,
                                    on_fail_step_index,
                                    on_fail_steps_total,
                                    state,
                                });
                            } else {
                                self.sender.send_event(AppEvent::LogMessage(format!(
                                    "{} [{}/{}] {}",
                                    SCENARIO_PREFIX,
                                    step_index + 1,
                                    steps_total,
                                    scenario_error
                                )));
                                self.sender.send_event(AppEvent::StepState {
                                    step_index,
                                    steps_total,
                                    state,
                                });
                            }
                        } else {
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} {}",
                                SCENARIO_PREFIX, scenario_error
                            )));
                        }
                    } else {
                        self.sender.send_event(AppEvent::LogMessage(format!(
                            "{} Scenario execution failed",
                            SCENARIO_PREFIX
                        )));
                    }
                }
                "scenario_started" => {
                    self.sender.send_event(AppEvent::LogMessage(format!(
                        "{} Scenario started...",
                        SCENARIO_PREFIX
                    )));
                    self.sender.send_event(AppEvent::Execution(true));
                }
                "scenario_completed" => {
                    self.sender.send_event(AppEvent::LogMessage(format!(
                        "{} Scenario completed successfully!",
                        SCENARIO_PREFIX
                    )));
                    self.sender.send_event(AppEvent::Execution(false));
                }
                "step_started" => {
                    if let (Some(step_index), Some(steps_total), Some(task_description)) = (
                        visitor.step_index,
                        visitor.steps_total,
                        visitor.task_description,
                    ) {
                        self.sender.send_event(AppEvent::LogMessage(format!(
                            "{} [{}/{}] {}",
                            SCENARIO_PREFIX,
                            step_index + 1,
                            steps_total,
                            task_description
                        )));
                        self.sender.send_event(AppEvent::StepState {
                            step_index,
                            steps_total,
                            state: StepState::StepStarted,
                        });
                    }
                }
                "step_completed" => {
                    if let (Some(step_index), Some(steps_total)) =
                        (visitor.step_index, visitor.steps_total)
                    {
                        self.sender.send_event(AppEvent::StepState {
                            step_index,
                            steps_total,
                            state: StepState::StepCompleted,
                        });
                    }
                }
                "remote_sudo_started" => {
                    if let (Some(step_index), Some(steps_total), Some(remote_sudo_command)) = (
                        visitor.step_index,
                        visitor.steps_total,
                        visitor.remote_sudo_command,
                    ) {
                        if let (Some(on_fail_step_index), Some(on_fail_steps_total)) =
                            (visitor.on_fail_step_index, visitor.on_fail_steps_total)
                        {
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] [on-fail] [{}/{}] Command: {}",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total,
                                on_fail_step_index + 1,
                                on_fail_steps_total,
                                remote_sudo_command
                            )));
                        } else {
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] Command: {}",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total,
                                remote_sudo_command
                            )));
                        }
                    }
                }
                "remote_sudo_output" => {
                    if let (
                        Some(step_index),
                        Some(steps_total),
                        Some(remote_sudo_command),
                        Some(remote_sudo_output),
                    ) = (
                        visitor.step_index,
                        visitor.steps_total,
                        visitor.remote_sudo_command,
                        visitor.remote_sudo_output,
                    ) {
                        let state = StepState::RemoteSudoOutput {
                            command: remote_sudo_command.to_owned(),
                            output: remote_sudo_output.to_owned(),
                        };
                        if let (Some(on_fail_step_index), Some(on_fail_steps_total)) =
                            (visitor.on_fail_step_index, visitor.on_fail_steps_total)
                        {
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] [on-fail] [{}/{}] Output:",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total,
                                on_fail_step_index + 1,
                                on_fail_steps_total
                            )));
                            self.sender.send_event(AppEvent::OnFailStepState {
                                step_index,
                                steps_total,
                                on_fail_step_index,
                                on_fail_steps_total,
                                state,
                            });
                        } else {
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] Output:",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total
                            )));
                            self.sender.send_event(AppEvent::StepState {
                                step_index,
                                steps_total,
                                state,
                            });
                        }
                        self.sender.send_event(AppEvent::LogPlainMessage(remote_sudo_output));
                    }
                }
                "sftp_copy_started" => {
                    if let (
                        Some(step_index),
                        Some(steps_total),
                        Some(sftp_copy_source),
                        Some(sftp_copy_destination),
                    ) = (
                        visitor.step_index,
                        visitor.steps_total,
                        visitor.sftp_copy_source,
                        visitor.sftp_copy_destination,
                    ) {
                        if let (Some(on_fail_step_index), Some(on_fail_steps_total)) =
                            (visitor.on_fail_step_index, visitor.on_fail_steps_total)
                        {
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] [on-fail] [{}/{}] Source: {}",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total,
                                on_fail_step_index + 1,
                                on_fail_steps_total,
                                sftp_copy_source
                            )));
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] [on-fail] [{}/{}] Destination: {}",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total,
                                on_fail_step_index + 1,
                                on_fail_steps_total,
                                sftp_copy_destination
                            )));
                        } else {
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] Source: {}",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total,
                                sftp_copy_source
                            )));
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] Destination: {}",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total,
                                sftp_copy_destination
                            )));
                        }
                    }
                }
                "sftp_copy_completed" => {
                    if let (Some(step_index), Some(steps_total)) =
                        (visitor.step_index, visitor.steps_total)
                    {
                        if let (Some(on_fail_step_index), Some(on_fail_steps_total)) =
                            (visitor.on_fail_step_index, visitor.on_fail_steps_total)
                        {
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] [on-fail] [{}/{}] SFTP copy finished",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total,
                                on_fail_step_index + 1,
                                on_fail_steps_total
                            )));
                        } else {
                            self.sender.send_event(AppEvent::LogMessage(format!(
                                "{} [{}/{}] SFTP copy finished",
                                SCENARIO_PREFIX,
                                step_index + 1,
                                steps_total
                            )));
                        }
                    }
                }
                "sftp_copy_progress" => {
                    if let (
                        Some(sftp_copy_progress_current),
                        Some(sftp_copy_progress_total),
                        Some(sftp_copy_source),
                        Some(sftp_copy_destination),
                    ) = (
                        visitor.sftp_copy_progress_current,
                        visitor.sftp_copy_progress_total,
                        visitor.sftp_copy_source,
                        visitor.sftp_copy_destination,
                    ) {
                        let percentage = (sftp_copy_progress_current as f64
                            / sftp_copy_progress_total as f64)
                            * 100.0;

                        if let (Some(step_index), Some(steps_total)) =
                            (visitor.step_index, visitor.steps_total)
                        {
                            let state = StepState::SftpCopyProgress {
                                source: sftp_copy_source,
                                destination: sftp_copy_destination,
                                current: sftp_copy_progress_current,
                                total: sftp_copy_progress_total,
                            };
                            if let (Some(on_fail_step_index), Some(on_fail_steps_total)) =
                                (visitor.on_fail_step_index, visitor.on_fail_steps_total)
                            {
                                self.sender.send_event(AppEvent::LogMessage(format!(
                                    "{} [{}/{}] [on-fail] [{}/{}] Progress: {:.1}%",
                                    SCENARIO_PREFIX,
                                    step_index + 1,
                                    steps_total,
                                    on_fail_step_index + 1,
                                    on_fail_steps_total,
                                    percentage
                                )));
                                self.sender.send_event(AppEvent::OnFailStepState {
                                    step_index,
                                    steps_total,
                                    on_fail_step_index,
                                    on_fail_steps_total,
                                    state,
                                });
                            } else {
                                self.sender.send_event(AppEvent::LogMessage(format!(
                                    "{} [{}/{}] Progress: {:.1}%",
                                    SCENARIO_PREFIX,
                                    step_index + 1,
                                    steps_total,
                                    percentage
                                )));
                                self.sender.send_event(AppEvent::StepState {
                                    step_index,
                                    steps_total,
                                    state,
                                });
                            }
                        }
                    }
                }
                "on_fail_steps_started" => {
                    if let (Some(step_index), Some(steps_total), Some(on_fail_steps_total)) = (
                        visitor.step_index,
                        visitor.steps_total,
                        visitor.on_fail_steps_total,
                    ) {
                        self.sender.send_event(AppEvent::LogMessage(format!(
                            "{} [{}/{}] [on-fail] ({}) Starting failure recovery steps",
                            SCENARIO_PREFIX,
                            step_index + 1,
                            steps_total,
                            on_fail_steps_total
                        )));
                    }
                }
                "on_fail_steps_completed" => {
                    if let (Some(step_index), Some(steps_total), Some(on_fail_steps_total)) = (
                        visitor.step_index,
                        visitor.steps_total,
                        visitor.on_fail_steps_total,
                    ) {
                        self.sender.send_event(AppEvent::LogMessage(format!(
                            "{} [{}/{}] [on-fail] ({}) Failure recovery steps completed",
                            SCENARIO_PREFIX,
                            step_index + 1,
                            steps_total,
                            on_fail_steps_total
                        )));
                    }
                }
                "on_fail_step_started" => {
                    if let (
                        Some(step_index),
                        Some(steps_total),
                        Some(on_fail_step_index),
                        Some(on_fail_steps_total),
                        Some(task_description),
                    ) = (
                        visitor.step_index,
                        visitor.steps_total,
                        visitor.on_fail_step_index,
                        visitor.on_fail_steps_total,
                        visitor.task_description,
                    ) {
                        self.sender.send_event(AppEvent::LogMessage(format!(
                            "{} [{}/{}] [on-fail] [{}/{}] {}",
                            SCENARIO_PREFIX,
                            step_index + 1,
                            steps_total,
                            on_fail_step_index + 1,
                            on_fail_steps_total,
                            task_description
                        )));
                        self.sender.send_event(AppEvent::OnFailStepState {
                            step_index,
                            steps_total,
                            on_fail_step_index,
                            on_fail_steps_total,
                            state: StepState::StepStarted,
                        });
                    }
                }
                "on_fail_step_completed" => {
                    if let (
                        Some(step_index),
                        Some(steps_total),
                        Some(on_fail_step_index),
                        Some(on_fail_steps_total),
                    ) = (
                        visitor.step_index,
                        visitor.steps_total,
                        visitor.on_fail_step_index,
                        visitor.on_fail_steps_total,
                    ) {
                        self.sender.send_event(AppEvent::OnFailStepState {
                            step_index,
                            steps_total,
                            on_fail_step_index,
                            on_fail_steps_total,
                            state: StepState::StepCompleted,
                        });
                    }
                }
                "create_session_started" => {}
                "created_mock_session" => {}
                "session_created" => {}
                "steps_started" => {}
                "remote_sudo_completed" => {}
                "steps_completed" => {}
                _ => {
                    error!("Unrecognized event type: {}", scenario_event);
                }
            }
        }
    }
}
