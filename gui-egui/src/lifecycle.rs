use crate::shared::SEPARATOR;
use scenario_rs::scenario::{
    lifecycle::{
        ExecutionLifecycle,
        RemoteSudoLifecycle,
        RollbackLifecycle,
        RollbackStepLifecycle,
        SftpCopyLifecycle,
        StepsLifecycle,
    },
    remote_sudo::RemoteSudo,
    rollback::RollbackSteps,
    sftp_copy::SftpCopy,
    task::Task
};
use std::{
    io::Read,
    sync::{mpsc, OnceLock},
};

static LIFECYCLE_HANDLER: OnceLock<LifecycleHandler> = OnceLock::new();

#[derive(Debug)]
pub struct LifecycleHandler {
    pub tx: mpsc::Sender<String>,
}

impl LifecycleHandler {
    pub fn try_initialize(tx: mpsc::Sender<String>) {
        LIFECYCLE_HANDLER.get_or_init(|| LifecycleHandler::new(tx));
    }

    pub fn new(tx: mpsc::Sender<String>) -> Self {
        Self { tx }
    }

    pub fn log_remote_sudo_before(&self, remote_sudo: &RemoteSudo) {
        let command = remote_sudo.command();
        let log_message = format!("Executing:\n{command}\n");
        let _ = self.tx.send(log_message);
    }

    pub fn log_remote_sudo_channel_established(&self, channel: &mut dyn Read) {
        let mut output = String::new();
        if channel.read_to_string(&mut output).is_err() {
            let log_message = format!("{}\nChannel output is not a valid UTF-8\n{}\n", SEPARATOR, SEPARATOR);
            let _ = self.tx.send(log_message);
            return;
        }
        let output = output.trim();
        let truncated_output = output.chars().take(1000).collect::<String>().trim().to_string();
        let log_message = format!("{}\n", truncated_output);
        let _ = self.tx.send(log_message);
        if output.len() > 1000 {
            let log_message = "...output truncated...\n".to_string();
            let _ = self.tx.send(log_message);
        }
    }

    pub fn log_sftp_copy_before(&self, sftp_copy: &SftpCopy) {
        let source_path = sftp_copy.source_path();
        let destination_path = sftp_copy.destination_path();
        let log_message = format!("Source:\n{source_path}\nDestination:\n{destination_path}\n");
        let _ = self.tx.send(log_message);
    }

    pub fn log_rollback_before(&self, rollback_steps: &RollbackSteps) {
        if rollback_steps.is_empty() {
            let log_message = format!("{SEPARATOR}\n[rollback] No rollback actions found\n");
            let _ = self.tx.send(log_message);
        }
    }

    pub fn log_rollback_step_before(&self, index: usize, rollback_task: &Task, total_rollback_steps: usize) {
        let task_number = index + 1;
        let description = rollback_task.description();
        let log_message = format!("{SEPARATOR}\n[rollback] [{task_number}/{total_rollback_steps}] {description}\n");
        let _ = self.tx.send(log_message);
    }
}

pub fn execution_lifecycle() -> ExecutionLifecycle {
    let mut lifecycle = ExecutionLifecycle::default();
    lifecycle.steps = steps_lifecycle();
    lifecycle
}

fn steps_lifecycle() -> StepsLifecycle {
    let mut lifecycle = StepsLifecycle::default();
    lifecycle.before = log_step_before;
    lifecycle.remote_sudo = remote_sudo_lifecycle();
    lifecycle.sftp_copy = sftp_copy_lifecycle();
    lifecycle.rollback = rollback_lifecycle();
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

fn rollback_lifecycle() -> RollbackLifecycle {
    let mut lifecycle = RollbackLifecycle::default();
    lifecycle.before = log_rollback_before;
    lifecycle.step = rollback_step_lifecycle();
    lifecycle
}

fn rollback_step_lifecycle() -> RollbackStepLifecycle {
    let mut lifecycle = RollbackStepLifecycle::default();
    lifecycle.before = log_rollback_step_before;
    lifecycle
}

pub fn log_step_before(index: usize, task: &Task, total_steps: usize) {
    if let Some(logger) = LIFECYCLE_HANDLER.get() {
        let task_number: usize = index + 1;
        let description = task.description();
        let log_message = format!("{SEPARATOR}\n[{task_number}/{total_steps}] {description}\n");
        let _ = logger.tx.send(log_message);
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

pub fn log_rollback_before(rollback_steps: &RollbackSteps) {
    if let Some(logger) = LIFECYCLE_HANDLER.get() {
        logger.log_rollback_before(rollback_steps);
    }
}

pub fn log_rollback_step_before(index: usize, rollback_task: &Task, total_rollback_steps: usize) {
    if let Some(logger) = LIFECYCLE_HANDLER.get() {
        logger.log_rollback_step_before(index, rollback_task, total_rollback_steps);
    }
}