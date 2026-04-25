import { StepperStep } from './warp-core/progress-stepper/progress-stepper.component';
import { OnFailStep, Step, Task } from './models/scenario.model';
import {
  OnFailStepExecState,
  StepExecState,
  StepStatus,
  TaskProgress,
} from './models/step-state.model';

export interface ScenarioProgressStepData {
  task: Task;
  progress: TaskProgress | null;
  output: string;
}

export function mapScenarioStepsToStepperSteps(
  steps: Step[],
  stepExecStates: StepExecState[]
): StepperStep[] {
  const stepExecStatesByIndex = new Map(
    stepExecStates.map((state) => [state.index, state] as const)
  );

  return steps.map((step) => {
    const execState = stepExecStatesByIndex.get(step.index);
    const onFailExecStatesByIndex = new Map(
      (execState?.on_fail_steps ?? []).map((state) => [state.index, state] as const)
    );

    const subSteps = step.on_fail_steps.map((onFailStep) =>
      mapScenarioSubStep(onFailStep, onFailExecStatesByIndex.get(onFailStep.index))
    );

    return createStepperStep(
      step.index,
      execState?.task_description ?? step.task.description,
      execState?.status ?? 'Pending',
      execState?.errors ?? [],
      step.task,
      execState?.progress ?? null,
      execState?.output ?? '',
      subSteps
    );
  });
}

export function isScenarioProgressStepData(
  value: unknown
): value is ScenarioProgressStepData {
  if (!value || typeof value !== 'object') {
    return false;
  }

  const candidate = value as Partial<ScenarioProgressStepData>;

  return !!candidate.task
    && typeof candidate.task === 'object'
    && typeof candidate.task.description === 'string'
    && typeof candidate.task.error_message === 'string'
    && typeof candidate.task.task_type === 'string';
}

function mapScenarioSubStep(
  step: OnFailStep,
  execState?: OnFailStepExecState
): StepperStep {
  return createStepperStep(
    step.index,
    execState?.task_description ?? step.task.description,
    execState?.status ?? 'Pending',
    execState?.errors ?? [],
    step.task,
    execState?.progress ?? null,
    execState?.output ?? ''
  );
}

function createStepperStep(
  index: number,
  title: string,
  status: StepStatus,
  errors: string[],
  task: Task,
  progress: TaskProgress | null,
  output: string,
  subSteps: StepperStep[] = []
): StepperStep {
  return {
    index,
    title,
    status,
    errors: [...errors],
    data: {
      task,
      progress,
      output,
    },
    ...(subSteps.length > 0 ? { subSteps } : {}),
  };
}
