// events.rs
#[derive(Clone, Debug)]
pub enum ScenarioEvent {
    // Scenario-level events
    ScenarioStarted,
    ScenarioCompleted,
    ScenarioError(String),

    // Step-level events
    StepStarted {
        index: usize,
        total_steps: usize,
        description: String,
    },
    StepCompleted,

    // RemoteSudo events
    RemoteSudoBefore(String),
    RemoteSudoChannelOutput(String),
    RemoteSudoAfter,

    // SftpCopy events
    SftpCopyBefore {
        source: String,
        destination: String,
    },
    SftpCopyProgress {
        current: u64,
        total: u64,
    },
    SftpCopyAfter,

    // OnFail events
    OnFailStepsStarted,
    OnFailStepStarted {
        index: usize,
        total_steps: usize,
        description: String,
    },
    OnFailStepCompleted,
    OnFailStepsCompleted,
}
