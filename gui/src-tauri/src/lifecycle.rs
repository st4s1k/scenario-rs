use crate::{app::ScenarioAppState, shared::SEPARATOR};
use scenario_rs::scenario::{
    lifecycle::{
        ExecutionLifecycle, RemoteSudoLifecycle, OnFailLifecycle, OnFailStepLifecycle,
        SftpCopyLifecycle, StepsLifecycle,
    },
    remote_sudo::RemoteSudo,
    on_fail::OnFailSteps,
    sftp_copy::SftpCopy,
    task::Task,
};
use std::{
    io::Read,
    sync::{Mutex, OnceLock},
};
use tauri::{AppHandle, Emitter, Manager};

static LIFECYCLE_HANDLER: OnceLock<LifecycleHandler> = OnceLock::new();

#[derive(Debug)]
pub struct LifecycleHandler {
    pub app_handle: AppHandle,
}

impl LifecycleHandler {
    pub fn try_initialize(window: AppHandle) -> ExecutionLifecycle {
        LIFECYCLE_HANDLER.get_or_init(|| LifecycleHandler::new(window));
        let mut lifecycle = ExecutionLifecycle::default();
        lifecycle.steps = steps_lifecycle();
        lifecycle
    }

    pub fn new(window: AppHandle) -> Self {
        Self { app_handle: window }
    }

    pub fn log_remote_sudo_before(&self, remote_sudo: &RemoteSudo) {
        let command = remote_sudo.command();
        self.log_message(format!("Executing:\n{command}\n"));
    }

    pub fn log_remote_sudo_channel_established(&self, channel: &mut dyn Read) {
        let mut output = String::new();
        if channel.read_to_string(&mut output).is_err() {
            self.log_message(format!(
                "{SEPARATOR}\nChannel output is not a valid UTF-8\n{SEPARATOR}\n"
            ));
            return;
        }
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

    pub fn log_sftp_copy_before(&self, sftp_copy: &SftpCopy) {
        let source_path = sftp_copy.source_path();
        let destination_path = sftp_copy.destination_path();
        self.log_message(format!(
            "Source:\n{source_path}\nDestination:\n{destination_path}\n"
        ));
    }

    pub fn log_on_fail_before(&self, on_fail_steps: &OnFailSteps) {
        if on_fail_steps.is_empty() {
            self.log_message(format!(
                "{SEPARATOR}\n[on_fail] No on_fail actions found\n"
            ));
        }
    }

    pub fn log_on_fail_step_before(
        &self,
        index: usize,
        on_fail_task: &Task,
        total_on_fail_steps: usize,
    ) {
        let task_number = index + 1;
        let description = on_fail_task.description();
        self.log_message(format!(
            "{SEPARATOR}\n[on_fail] [{task_number}/{total_on_fail_steps}] {description}\n"
        ));
    }

    pub fn log_message(&self, message: String) {
        let state = self.app_handle.state::<Mutex<ScenarioAppState>>();
        let mut state = state.lock().unwrap();
        state.output_log.push_str(&message);
        let _ = self.app_handle.emit("log-update", ());
    }
}

fn steps_lifecycle() -> StepsLifecycle {
    let mut lifecycle = StepsLifecycle::default();
    lifecycle.before = log_step_before;
    lifecycle.remote_sudo = remote_sudo_lifecycle();
    lifecycle.sftp_copy = sftp_copy_lifecycle();
    lifecycle.on_fail = on_fail_lifecycle();
    lifecycle
}

fn remote_sudo_lifecycle() -> RemoteSudoLifecycle {
    let mut lifecycle = RemoteSudoLifecycle::default();
    lifecycle.before = log_remote_sudo_before;
    lifecycle.channel_established = log_remote_sudo_channel_established;
    lifecycle
}

fn sftp_copy_lifecycle() -> SftpCopyLifecycle {
    let mut lifecycle = SftpCopyLifecycle::default();
    lifecycle.before = log_sftp_copy_before;
    lifecycle
}

fn on_fail_lifecycle() -> OnFailLifecycle {
    let mut lifecycle = OnFailLifecycle::default();
    lifecycle.before = log_on_fail_before;
    lifecycle.step = on_fail_step_lifecycle();
    lifecycle
}

fn on_fail_step_lifecycle() -> OnFailStepLifecycle {
    let mut lifecycle = OnFailStepLifecycle::default();
    lifecycle.before = log_on_fail_step_before;
    lifecycle
}

pub fn log_step_before(index: usize, task: &Task, total_steps: usize) {
    if let Some(logger) = LIFECYCLE_HANDLER.get() {
        let task_number: usize = index + 1;
        let description = task.description();
        logger.log_message(format!(
            "{SEPARATOR}\n[{task_number}/{total_steps}] {description}\n"
        ));
    }
}

pub fn log_remote_sudo_before(remote_sudo: &RemoteSudo) {
    if let Some(logger) = LIFECYCLE_HANDLER.get() {
        logger.log_remote_sudo_before(remote_sudo);
    }
}

pub fn log_remote_sudo_channel_established(channel: &mut dyn Read) {
    if let Some(logger) = LIFECYCLE_HANDLER.get() {
        logger.log_remote_sudo_channel_established(channel);
    }
}

pub fn log_sftp_copy_before(sftp_copy: &SftpCopy) {
    if let Some(logger) = LIFECYCLE_HANDLER.get() {
        logger.log_sftp_copy_before(sftp_copy);
    }
}

pub fn log_on_fail_before(on_fail_steps: &OnFailSteps) {
    if let Some(logger) = LIFECYCLE_HANDLER.get() {
        logger.log_on_fail_before(on_fail_steps);
    }
}

pub fn log_on_fail_step_before(index: usize, on_fail_task: &Task, total_on_fail_steps: usize) {
    if let Some(logger) = LIFECYCLE_HANDLER.get() {
        logger.log_on_fail_step_before(index, on_fail_task, total_on_fail_steps);
    }
}
