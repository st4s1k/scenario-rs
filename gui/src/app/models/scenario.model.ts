export interface RequiredFields {
  [key: string]: RequiredField;
}

export interface RequiredField {
  label: string;
  var_type: string;
  value: string;
  read_only: boolean;
}

export interface ResolvedVariables {
  [key: string]: string;
}

export interface Tasks {
  [key: string]: Task;
}

export type TaskType = 'SftpCopy' | 'RemoteSudo';

export interface Task {
  description: string;
  error_message: string;
  task_type: TaskType;
  command?: string;
  source_path?: string;
  destination_path?: string;
}

export interface OnFailStep {
  index: number;
  task: Task;
}

export interface Step {
  index: number;
  task: Task;
  on_fail_steps: OnFailStep[];
}
