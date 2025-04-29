export type StepEventType = 'SftpCopyProgress' | 'RemoteSudoOutput' | 'StepCompleted' | 'StepFailed';

/**
 * Base interface for step events
 */
export interface BaseStepEvent {
  type: StepEventType;
}

/**
 * Event for SFTP copy progress
 */
export interface SftpCopyProgress extends BaseStepEvent {
  type: 'SftpCopyProgress';
  current: number;
  total: number;
  source: string;
  destination: string;
}

/**
 * Event for remote sudo command output
 */
export interface RemoteSudoOutput extends BaseStepEvent {
  type: 'RemoteSudoOutput';
  command: string;
  output: string;
}

/**
 * Event for a completed step
 */
export interface StepCompleted extends BaseStepEvent {
  type: 'StepCompleted';
  index: number;
}

/**
 * Event for a failed step
 */
export interface StepFailed extends BaseStepEvent {
  type: 'StepFailed';
  message: string;
}

/**
 * Union type for all possible step state events
 */
export type StepEvent =
  SftpCopyProgress
  | RemoteSudoOutput
  | StepCompleted
  | StepFailed;
