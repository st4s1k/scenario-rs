use crate::scenario::{
    remote_sudo::RemoteSudo,
    on_fail::OnFailSteps,
    sftp_copy::SftpCopy,
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
    pub steps: StepsLifecycle,
}

impl Default for ExecutionLifecycle {
    fn default() -> Self {
        ExecutionLifecycle {
            before: |_| {},
            steps: Default::default(),
        }
    }
}

pub struct StepsLifecycle {
    pub before: fn(index: usize, task: &Task, total_steps: usize),
    pub remote_sudo: RemoteSudoLifecycle,
    pub sftp_copy: SftpCopyLifecycle,
    pub on_fail: OnFailLifecycle,
}

impl Default for StepsLifecycle {
    fn default() -> Self {
        StepsLifecycle {
            before: |_, _, _| {},
            remote_sudo: Default::default(),
            sftp_copy: Default::default(),
            on_fail: Default::default(),
        }
    }
}

pub struct OnFailLifecycle {
    pub before: fn(on_fail_steps: &OnFailSteps),
    pub step: OnFailStepLifecycle,
}

impl Default for OnFailLifecycle {
    fn default() -> Self {
        OnFailLifecycle {
            before: |_| {},
            step: Default::default(),
        }
    }
}

pub struct OnFailStepLifecycle {
    pub before: fn(index: usize, on_fail_task: &Task, total_on_fail_steps: usize),
    pub remote_sudo: RemoteSudoLifecycle,
    pub sftp_copy: SftpCopyLifecycle,
}

impl Default for OnFailStepLifecycle {
    fn default() -> Self {
        OnFailStepLifecycle {
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
