// --- Execution State Machine Types ---

export type ExecutionStatus =
  { kind: 'Idle' }
  | { kind: 'Running' }
  | { kind: 'Completed' }
  | { kind: 'Failed'; error: string };

export type StepStatus = 'Pending' | 'Running' | 'Completed' | 'Failed' | 'Skipped';

export type TaskProgress =
  | { type: 'SftpCopy'; source: string; destination: string; bytes_transferred: number; bytes_total: number }
  | { type: 'RemoteSudo'; command: string; output: string };

export interface OnFailStepExecState {
  index: number;
  task_description: string;
  status: StepStatus;
  progress: TaskProgress | null;
  output: string;
  errors: string[];
}

export interface StepExecState {
  index: number;
  task_description: string;
  status: StepStatus;
  progress: TaskProgress | null;
  output: string;
  errors: string[];
  on_fail_steps: OnFailStepExecState[];
}

export interface ExecutionState {
  status: ExecutionStatus;
  steps: StepExecState[];
}

// --- State Diffs (streamed from backend) ---

export type StateDiff =
  | { kind: 'ExecutionStatusChanged'; status: ExecutionStatus }
  | { kind: 'StepStatusChanged'; step_index: number; status: StepStatus }
  | { kind: 'StepProgressUpdated'; step_index: number; progress: TaskProgress }
  | { kind: 'StepOutputAppended'; step_index: number; text: string }
  | { kind: 'StepErrorAdded'; step_index: number; error: string }
  | { kind: 'OnFailStepStatusChanged'; step_index: number; on_fail_step_index: number; status: StepStatus }
  | { kind: 'OnFailStepProgressUpdated'; step_index: number; on_fail_step_index: number; progress: TaskProgress }
  | { kind: 'OnFailStepOutputAppended'; step_index: number; on_fail_step_index: number; text: string }
  | { kind: 'OnFailStepErrorAdded'; step_index: number; on_fail_step_index: number; error: string };
