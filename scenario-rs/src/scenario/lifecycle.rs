use crate::scenario::steps::Steps;
use crate::scenario::{
    remote_sudo::RemoteSudo,
    sftp_copy::SftpCopy,
    step::Step,
    task::Task,
    Scenario,
};
use indicatif::ProgressBar;
use std::{
    fs::File,
    io::{Read, Write},
};

pub struct ExecutionLifecycle {
    pub before: fn(scenario: &Scenario),
    pub task: TaskLifecycle,
}

impl Default for ExecutionLifecycle {
    fn default() -> Self {
        ExecutionLifecycle {
            before: |_| {},
            task: Default::default(),
        }
    }
}

pub struct TaskLifecycle {
    pub before: fn(index: usize, task: &Task, tasks: &Steps),
    pub remote_sudo: RemoteSudoLifecycle,
    pub sftp_copy: SftpCopyLifecycle,
    pub rollback: RollbackLifecycle,
}

impl Default for TaskLifecycle {
    fn default() -> Self {
        TaskLifecycle {
            before: |_, _, _| {},
            remote_sudo: Default::default(),
            sftp_copy: Default::default(),
            rollback: Default::default(),
        }
    }
}

pub struct RollbackLifecycle {
    pub before: fn(step: &Step),
    pub step: RollbackStepLifecycle,
}

impl Default for RollbackLifecycle {
    fn default() -> Self {
        RollbackLifecycle {
            before: |_| {},
            step: Default::default(),
        }
    }
}

pub struct RollbackStepLifecycle {
    pub before: fn(index: usize, rollback_task: &Task, rollback_tasks: &Vec<String>),
    pub remote_sudo: RemoteSudoLifecycle,
    pub sftp_copy: SftpCopyLifecycle,
}

impl Default for RollbackStepLifecycle {
    fn default() -> Self {
        RollbackStepLifecycle {
            before: |_, _, _| {},
            remote_sudo: Default::default(),
            sftp_copy: Default::default(),
        }
    }
}

pub struct RemoteSudoLifecycle {
    pub before: fn(remote_sudo: &RemoteSudo),
    pub channel_established: fn(channel_reader: &mut dyn Read),
}

impl Default for RemoteSudoLifecycle {
    fn default() -> Self {
        RemoteSudoLifecycle {
            before: |_| {},
            channel_established: |_| {},
        }
    }
}

pub struct SftpCopyLifecycle {
    pub before: fn(sftp_copy: &SftpCopy),
    pub files_ready: fn(source_file: &File, destination_writer: &mut dyn Write, pb: &ProgressBar),
    pub after: fn(),
}

impl Default for SftpCopyLifecycle {
    fn default() -> Self {
        SftpCopyLifecycle {
            before: |_| {},
            files_ready: |_, _, _| {},
            after: || {},
        }
    }
}
