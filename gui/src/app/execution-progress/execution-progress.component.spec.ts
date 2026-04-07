import { SimpleChange } from '@angular/core';
import { ExecutionProgressComponent } from './execution-progress.component';
import {
  StepExecState,
  OnFailStepExecState,
} from '../models/step-state.model';
import { Task } from '../models/scenario.model';

function makeOnFailExecState(
  overrides: Partial<OnFailStepExecState> = {}
): OnFailStepExecState {
  return {
    index: 0,
    task_description: 'Recovery task',
    status: 'Pending',
    progress: null,
    output: '',
    errors: [],
    ...overrides,
  };
}

function makeExecState(
  overrides: Partial<StepExecState> = {}
): StepExecState {
  return {
    index: 0,
    task_description: 'Test task',
    status: 'Pending',
    progress: null,
    output: '',
    errors: [],
    on_fail_steps: [],
    ...overrides,
  };
}

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    description: 'Deploy',
    error_message: 'Deploy failed',
    task_type: 'RemoteSudo',
    command: 'echo hi',
    ...overrides,
  };
}

function triggerChanges(
  component: ExecutionProgressComponent,
  changes: Record<string, { prev: any; curr: any; first: boolean }>
): void {
  const simpleChanges: Record<string, SimpleChange> = {};
  for (const [key, val] of Object.entries(changes)) {
    simpleChanges[key] = new SimpleChange(val.prev, val.curr, val.first);
  }
  component.ngOnChanges(simpleChanges);
}

describe('ExecutionProgressComponent', () => {
  let component: ExecutionProgressComponent;

  beforeEach(() => {
    component = new ExecutionProgressComponent();
    component.isExecuting = true;
  });

  describe('formatBytes', () => {
    it('should return "0 B" for zero', () => {
      // Given & When & Then
      expect(component.formatBytes(0)).toBe('0 B');
    });

    it('should format bytes', () => {
      // Given & When & Then
      expect(component.formatBytes(500)).toBe('500.00 B');
    });

    it('should format kilobytes', () => {
      // Given & When & Then
      expect(component.formatBytes(1024)).toBe('1.00 KB');
    });

    it('should format megabytes', () => {
      // Given & When & Then
      expect(component.formatBytes(1048576)).toBe('1.00 MB');
    });

    it('should format gigabytes', () => {
      // Given & When & Then
      expect(component.formatBytes(1073741824)).toBe('1.00 GB');
    });

    it('should format fractional values', () => {
      // Given & When & Then
      expect(component.formatBytes(1536)).toBe('1.50 KB');
    });
  });

  describe('hasOnFailSteps', () => {
    it('should return true for StepExecState with on_fail_steps', () => {
      // Given
      const state = makeExecState({
        on_fail_steps: [makeOnFailExecState()],
      });

      // When & Then
      expect(component.hasOnFailSteps(state)).toBe(true);
    });

    it('should return false for StepExecState with empty on_fail_steps', () => {
      // Given
      const state = makeExecState({ on_fail_steps: [] });

      // When & Then
      expect(component.hasOnFailSteps(state)).toBe(false);
    });

    it('should return false for OnFailStepExecState', () => {
      // Given
      const state = makeOnFailExecState();

      // When & Then
      expect(component.hasOnFailSteps(state)).toBe(false);
    });
  });

  describe('displayStates', () => {
    it('should return stepExecStates when onFailExecStates is undefined', () => {
      // Given
      component.stepExecStates = [makeExecState()];
      component.onFailExecStates = undefined;

      // When & Then
      expect(component.displayStates).toBe(component.stepExecStates);
    });

    it('should return onFailExecStates when defined', () => {
      // Given
      const onFail = [makeOnFailExecState()];
      component.onFailExecStates = onFail;

      // When & Then
      expect(component.displayStates).toBe(onFail);
    });
  });

  describe('getTask', () => {
    it('should return task from steps by index', () => {
      // Given
      component.steps = [
        { index: 0, task: makeTask(), on_fail_steps: [] },
      ];

      // When & Then
      expect(component.getTask(0)?.description).toBe('Deploy');
    });

    it('should return task from onFailSteps when steps is undefined', () => {
      // Given
      component.onFailSteps = [
        { index: 0, task: makeTask({ description: 'Rollback' }) },
      ];

      // When & Then
      expect(component.getTask(0)?.description).toBe('Rollback');
    });

    it('should return undefined for out-of-bounds index', () => {
      // Given
      component.steps = [];

      // When & Then
      expect(component.getTask(5)).toBeUndefined();
    });
  });

  describe('syncUi status colors', () => {
    it('should set blue for Running status', () => {
      // Given
      component.stepExecStates = [makeExecState({ status: 'Running' })];

      // When
      triggerChanges(component, {
        stepExecStates: { prev: null, curr: component.stepExecStates, first: true },
      });

      // Then
      expect(component.uiStates[0].statusColor()).toBe('blue');
    });

    it('should set green for Completed status', () => {
      // Given
      component.stepExecStates = [makeExecState({ status: 'Completed' })];

      // When
      triggerChanges(component, {
        stepExecStates: { prev: null, curr: component.stepExecStates, first: true },
      });

      // Then
      expect(component.uiStates[0].statusColor()).toBe('green');
    });

    it('should set red for Failed status', () => {
      // Given
      component.stepExecStates = [makeExecState({ status: 'Failed' })];

      // When
      triggerChanges(component, {
        stepExecStates: { prev: null, curr: component.stepExecStates, first: true },
      });

      // Then
      expect(component.uiStates[0].statusColor()).toBe('red');
    });

    it('should leave statusColor undefined for Pending', () => {
      // Given
      component.stepExecStates = [makeExecState({ status: 'Pending' })];

      // When
      triggerChanges(component, {
        stepExecStates: { prev: null, curr: component.stepExecStates, first: true },
      });

      // Then
      expect(component.uiStates[0].statusColor()).toBeUndefined();
    });

    it('should collapse step on Completed', () => {
      // Given
      component.stepExecStates = [makeExecState({ status: 'Completed' })];

      // When
      triggerChanges(component, {
        stepExecStates: { prev: null, curr: component.stepExecStates, first: true },
      });

      // Then
      expect(component.uiStates[0].expanded).toBe(false);
    });

    it('should expand step on Running', () => {
      // Given
      component.stepExecStates = [makeExecState({ status: 'Running' })];

      // When
      triggerChanges(component, {
        stepExecStates: { prev: null, curr: component.stepExecStates, first: true },
      });

      // Then
      expect(component.uiStates[0].expanded).toBe(true);
    });

    it('should not update color if status did not change', () => {
      // Given
      component.stepExecStates = [makeExecState({ status: 'Running' })];
      triggerChanges(component, {
        stepExecStates: { prev: null, curr: component.stepExecStates, first: true },
      });
      component.uiStates[0].statusColor.set(undefined);

      // When
      triggerChanges(component, {
        stepExecStates: { prev: component.stepExecStates, curr: component.stepExecStates, first: false },
      });

      // Then
      expect(component.uiStates[0].statusColor()).toBeUndefined();
    });
  });

  describe('syncUi for multiple steps', () => {
    it('should create uiStates for each step', () => {
      // Given
      component.stepExecStates = [
        makeExecState({ index: 0, status: 'Completed' }),
        makeExecState({ index: 1, status: 'Running' }),
        makeExecState({ index: 2, status: 'Pending' }),
      ];

      // When
      triggerChanges(component, {
        stepExecStates: { prev: null, curr: component.stepExecStates, first: true },
      });

      // Then
      expect(component.uiStates.length).toBe(3);
      expect(component.uiStates[0].statusColor()).toBe('green');
      expect(component.uiStates[1].statusColor()).toBe('blue');
      expect(component.uiStates[2].statusColor()).toBeUndefined();
    });
  });

  describe('execution restart', () => {
    it('should reset uiStates when isExecuting transitions false to true', () => {
      // Given
      component.stepExecStates = [makeExecState({ status: 'Completed' })];
      component.isExecuting = false;
      triggerChanges(component, {
        isExecuting: { prev: undefined, curr: false, first: true },
      });

      // When
      component.isExecuting = true;
      triggerChanges(component, {
        isExecuting: { prev: false, curr: true, first: false },
      });

      // Then
      expect(component.uiStates.length).toBe(1);
      expect(component.uiStates[0].statusColor()).toBe('green');
    });
  });

  describe('onFailStatusColor', () => {
    it('should set red when any on-fail step failed', () => {
      // Given
      component.stepExecStates = [
        makeExecState({
          status: 'Failed',
          on_fail_steps: [
            makeOnFailExecState({ status: 'Completed' }),
            makeOnFailExecState({ index: 1, status: 'Failed' }),
          ],
        }),
      ];

      // When
      triggerChanges(component, {
        stepExecStates: { prev: null, curr: component.stepExecStates, first: true },
      });

      // Then
      expect(component.onFailStatusColor()).toBe('red');
    });

    it('should set blue when an on-fail step is running', () => {
      // Given
      component.stepExecStates = [
        makeExecState({
          status: 'Failed',
          on_fail_steps: [makeOnFailExecState({ status: 'Running' })],
        }),
      ];

      // When
      triggerChanges(component, {
        stepExecStates: { prev: null, curr: component.stepExecStates, first: true },
      });

      // Then
      expect(component.onFailStatusColor()).toBe('blue');
    });

    it('should set green when all on-fail steps completed', () => {
      // Given
      component.stepExecStates = [
        makeExecState({
          status: 'Failed',
          on_fail_steps: [
            makeOnFailExecState({ status: 'Completed' }),
            makeOnFailExecState({ index: 1, status: 'Completed' }),
          ],
        }),
      ];

      // When
      triggerChanges(component, {
        stepExecStates: { prev: null, curr: component.stepExecStates, first: true },
      });

      // Then
      expect(component.onFailStatusColor()).toBe('green');
    });

    it('should leave onFailStatusColor undefined when no on-fail steps exist', () => {
      // Given
      component.stepExecStates = [makeExecState({ status: 'Running', on_fail_steps: [] })];

      // When
      triggerChanges(component, {
        stepExecStates: { prev: null, curr: component.stepExecStates, first: true },
      });

      // Then
      expect(component.onFailStatusColor()).toBeUndefined();
    });
  });

  describe('syncUi for Skipped status', () => {
    it('should leave statusColor undefined for Skipped status', () => {
      // Given
      component.stepExecStates = [makeExecState({ status: 'Skipped' })];

      // When
      triggerChanges(component, {
        stepExecStates: { prev: null, curr: component.stepExecStates, first: true },
      });

      // Then
      expect(component.uiStates[0].statusColor()).toBeUndefined();
    });
  });

  describe('syncUi with onFailExecStates', () => {
    it('should use onFailExecStates for display when provided', () => {
      // Given
      component.onFailExecStates = [
        makeOnFailExecState({ status: 'Running' }),
      ];

      // When
      triggerChanges(component, {
        stepExecStates: { prev: null, curr: [], first: true },
      });

      // Then
      expect(component.uiStates[0].statusColor()).toBe('blue');
    });
  });
});
