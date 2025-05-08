/**
 * Interface for step state events
 */
export interface StepStateEvent {
  step_index: number;
  steps_total: number;
  state: StepState;
}

/**
 * Interface for on-fail step state events
 */
export interface OnFailStepStateEvent {
  step_index: number;
  steps_total: number;
  on_fail_step_index: number;
  on_fail_steps_total: number;
  state: StepState;
}

/**
 * Union type for all possible step states
 */
export type StepState =
  SftpCopyProgress
  | RemoteSudoOutput
  | StepCompleted
  | StepFailed;

/**
 * SFTP copy progress state
 */
export interface SftpCopyProgress extends BaseStepState {
  type: 'SftpCopyProgress';
  current: number;
  total: number;
  source: string;
  destination: string;
}

/**
 * Remote sudo output state
 */
export interface RemoteSudoOutput extends BaseStepState {
  type: 'RemoteSudoOutput';
  command: string;
  output: string;
}

/**
 * Step completed state
 */
export interface StepCompleted extends BaseStepState {
  type: 'StepCompleted';
}

/**
 * Step failed state
 */
export interface StepFailed extends BaseStepState {
  type: 'StepFailed';
  message: string;
}

/**
 * Base interface for step states
 */
export interface BaseStepState {
  type: StepStateType;
}

/**
 * Type for step state
 */
export type StepStateType =
  'SftpCopyProgress'
  | 'RemoteSudoOutput'
  | 'StepCompleted'
  | 'StepFailed';
