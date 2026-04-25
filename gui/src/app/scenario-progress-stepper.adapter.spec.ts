import {
  isScenarioProgressStepData,
  mapScenarioStepsToStepperSteps,
} from './scenario-progress-stepper.adapter';
import { OnFailStep, Step, Task } from './models/scenario.model';
import { OnFailStepExecState, StepExecState, TaskProgress } from './models/step-state.model';

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    description: 'Deploy',
    error_message: 'Deploy failed',
    task_type: 'RemoteSudo',
    command: 'echo hi',
    ...overrides,
  };
}

function makeStep(overrides: Partial<Step> = {}): Step {
  return {
    index: 0,
    task: makeTask(),
    on_fail_steps: [],
    ...overrides,
  };
}

function makeOnFailStep(overrides: Partial<OnFailStep> = {}): OnFailStep {
  return {
    index: 0,
    task: makeTask({ description: 'Rollback', command: 'rollback.sh' }),
    ...overrides,
  };
}

function makeSftpProgress(
  overrides: Partial<Extract<TaskProgress, { type: 'SftpCopy' }>> = {}
): Extract<TaskProgress, { type: 'SftpCopy' }> {
  return {
    type: 'SftpCopy',
    source: 'source.zip',
    destination: '/tmp/source.zip',
    bytes_transferred: 512,
    bytes_total: 1024,
    elapsed_ms: 2000,
    ...overrides,
  };
}

function makeOnFailExecState(overrides: Partial<OnFailStepExecState> = {}): OnFailStepExecState {
  return {
    index: 0,
    task_description: 'Rollback',
    status: 'Completed',
    progress: null,
    output: 'rollback done',
    errors: [],
    ...overrides,
  };
}

function makeStepExecState(overrides: Partial<StepExecState> = {}): StepExecState {
  return {
    index: 0,
    task_description: 'Deploy from backend',
    status: 'Running',
    progress: makeSftpProgress(),
    output: 'uploading',
    errors: ['slow network'],
    on_fail_steps: [],
    ...overrides,
  };
}

describe('scenario-progress-stepper.adapter', () => {
  describe('mapScenarioStepsToStepperSteps', () => {
    it('should map backend execution state into stepper data', () => {
      // Given
      const steps = [
        makeStep({
          on_fail_steps: [
            makeOnFailStep(),
          ],
        }),
      ];
      const execStates = [
        makeStepExecState({
          on_fail_steps: [
            makeOnFailExecState({
              status: 'Failed',
              errors: ['rollback failed'],
            }),
          ],
        }),
      ];

      // When
      const result = mapScenarioStepsToStepperSteps(steps, execStates);

      // Then
      expect(result).toEqual([
        jasmine.objectContaining({
          index: 0,
          title: 'Deploy from backend',
          status: 'Running',
          errors: ['slow network'],
          data: jasmine.objectContaining({
            task: jasmine.objectContaining({ description: 'Deploy' }),
            progress: jasmine.objectContaining({ type: 'SftpCopy' }),
            output: 'uploading',
          }),
          subSteps: [
            jasmine.objectContaining({
              index: 0,
              title: 'Rollback',
              status: 'Failed',
              errors: ['rollback failed'],
              data: jasmine.objectContaining({
                output: 'rollback done',
              }),
            }),
          ],
        }),
      ]);
    });

    it('should fall back to scenario definitions when execution state is missing', () => {
      // Given
      const steps = [
        makeStep({
          index: 4,
          task: makeTask({ description: 'Copy files', task_type: 'SftpCopy' }),
          on_fail_steps: [
            makeOnFailStep({
              index: 1,
              task: makeTask({ description: 'Cleanup temp files' }),
            }),
          ],
        }),
      ];

      // When
      const result = mapScenarioStepsToStepperSteps(steps, []);

      // Then
      expect(result).toEqual([
        {
          index: 4,
          title: 'Copy files',
          status: 'Pending',
          errors: [],
          data: {
            task: jasmine.objectContaining({ description: 'Copy files' }),
            progress: null,
            output: '',
          },
          subSteps: [
            {
              index: 1,
              title: 'Cleanup temp files',
              status: 'Pending',
              errors: [],
              data: {
                task: jasmine.objectContaining({ description: 'Cleanup temp files' }),
                progress: null,
                output: '',
              },
            },
          ],
        },
      ]);
    });

    it('should omit subSteps when a step has no on-fail steps', () => {
      // Given
      const steps = [makeStep({ on_fail_steps: [] })];

      // When
      const result = mapScenarioStepsToStepperSteps(steps, [makeStepExecState()]);

      // Then
      expect(result[0].subSteps).toBeUndefined();
    });

    it('should fall back to on-fail step defaults when on-fail execution state is missing', () => {
      // Given
      const steps = [
        makeStep({
          on_fail_steps: [
            makeOnFailStep({
              index: 2,
              task: makeTask({ description: 'Rollback package' }),
            }),
          ],
        }),
      ];
      const execStates = [
        makeStepExecState({
          on_fail_steps: [],
        }),
      ];

      // When
      const result = mapScenarioStepsToStepperSteps(steps, execStates);

      // Then
      expect(result[0].subSteps).toEqual([
        jasmine.objectContaining({
          index: 2,
          title: 'Rollback package',
          status: 'Pending',
          errors: [],
          data: jasmine.objectContaining({
            progress: null,
            output: '',
          }),
        }),
      ]);
    });
  });

  describe('isScenarioProgressStepData', () => {
    it('should return false for nullish and non-object values', () => {
      // Given / When / Then
      expect(isScenarioProgressStepData(null)).toBe(false);
      expect(isScenarioProgressStepData(undefined)).toBe(false);
      expect(isScenarioProgressStepData('not-an-object')).toBe(false);
    });

    it('should return false when task is missing or invalid', () => {
      // Given / When / Then
      expect(isScenarioProgressStepData({})).toBe(false);
      expect(isScenarioProgressStepData({ task: 'deploy' })).toBe(false);
      expect(isScenarioProgressStepData({ task: { description: 42 } })).toBe(false);
      expect(isScenarioProgressStepData({ task: { description: 'Deploy', error_message: 42 } })).toBe(false);
      expect(
        isScenarioProgressStepData({
          task: { description: 'Deploy', error_message: 'bad', task_type: 42 },
        })
      ).toBe(false);
    });

    it('should return true for valid scenario progress step data', () => {
      // Given
      const value = {
        task: makeTask(),
        progress: makeSftpProgress(),
        output: 'done',
      };

      // When / Then
      expect(isScenarioProgressStepData(value)).toBe(true);
    });
  });
});
