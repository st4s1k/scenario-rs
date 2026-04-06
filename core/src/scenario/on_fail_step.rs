use crate::{
    scenario::{errors::OnFailError, task::Task, variables::Variables},
    session::Session,
    state::{ExecutionStateManager, TaskTracker},
    state::types::StepStatus,
    trace::ScenarioEvent,
};
use tracing::{debug, instrument};

#[derive(Clone, Debug)]
pub struct OnFailStep {
    pub(crate) index: usize,
    pub(crate) task: Task,
}

impl From<(usize, Task)> for OnFailStep {
    fn from((index, task): (usize, Task)) -> Self {
        Self { index, task }
    }
}

impl OnFailStep {
    /// Returns the index of the on-fail step
    pub fn index(&self) -> usize {
        self.index
    }

    /// Returns the task associated with this on-fail step
    pub fn task(&self) -> &Task {
        &self.task
    }

    /// Execute the on-fail step
    #[instrument(
        name = "on_fail_step",
        skip_all,
        fields(on_fail_step.index = self.index)
    )]
    pub(crate) fn execute(
        &self,
        session: &Session,
        variables: &Variables,
        state_manager: Option<&ExecutionStateManager>,
        parent_step_index: usize,
    ) -> Result<(), OnFailError> {
        debug!(
            scenario.event = ScenarioEvent::OnFailStepStarted.as_str(),
            task.description = self.task.description()
        );

        if let Some(sm) = state_manager {
            sm.update_on_fail_step_status(parent_step_index, self.index, StepStatus::Running);
        }

        let tracker =
            state_manager.map(|sm| TaskTracker::for_on_fail_step(sm, parent_step_index, self.index));

        let result = match &self.task {
            Task::RemoteSudo { remote_sudo, .. } => remote_sudo
                .execute(session, variables, tracker.as_ref())
                .map_err(OnFailError::CannotOnFailRemoteSudo)
                .map_err(|error| {
                    debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %error);
                    error
                }),
            Task::SftpCopy { sftp_copy, .. } => sftp_copy
                .execute(session, variables, tracker.as_ref())
                .map_err(OnFailError::CannotOnFailSftpCopy)
                .map_err(|error| {
                    debug!(scenario.event = ScenarioEvent::Error.as_str(), scenario.error = %error);
                    error
                }),
        };

        if result.is_ok() {
            debug!(scenario.event = ScenarioEvent::OnFailStepCompleted.as_str());
            if let Some(sm) = state_manager {
                sm.update_on_fail_step_status(
                    parent_step_index,
                    self.index,
                    StepStatus::Completed,
                );
            }
        } else if let Some(sm) = state_manager {
            sm.update_on_fail_step_status(parent_step_index, self.index, StepStatus::Failed);
            if let Err(ref e) = result {
                sm.add_on_fail_step_error(parent_step_index, self.index, e.to_string());
            }
        }

        result
    }
}
