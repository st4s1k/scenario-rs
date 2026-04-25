import { SimpleChange } from '@angular/core';
import {
  ProgressStepperComponent,
  StepperStep,
} from './progress-stepper.component';

function makeStep(overrides: Partial<StepperStep> = {}): StepperStep {
  return {
    index: 0,
    title: 'Test step',
    status: 'Pending',
    errors: [],
    ...overrides,
  };
}

function triggerChanges(
  component: ProgressStepperComponent,
  changes: Record<string, { prev: any; curr: any; first: boolean }>
): void {
  const simpleChanges: Record<string, SimpleChange> = {};
  for (const [key, val] of Object.entries(changes)) {
    simpleChanges[key] = new SimpleChange(val.prev, val.curr, val.first);
  }
  component.ngOnChanges(simpleChanges);
}

describe('ProgressStepperComponent', () => {
  let component: ProgressStepperComponent;

  beforeEach(() => {
    component = new ProgressStepperComponent();
    component.isExecuting = true;
  });

  describe('hasSubSteps', () => {
    it('should return true for step with subSteps', () => {
      // Given
      const step = makeStep({ subSteps: [makeStep()] });

      // When & Then
      expect(component.hasSubSteps(step)).toBe(true);
    });

    it('should return false for step with empty subSteps', () => {
      // Given
      const step = makeStep({ subSteps: [] });

      // When & Then
      expect(component.hasSubSteps(step)).toBe(false);
    });

    it('should return false for step without subSteps', () => {
      // Given
      const step = makeStep();

      // When & Then
      expect(component.hasSubSteps(step)).toBe(false);
    });
  });

  describe('displayStates', () => {
    it('should return empty array when all steps are Pending', () => {
      // Given
      component.steps = [makeStep()];

      // When & Then
      expect(component.displayStates).toEqual([]);
    });

    it('should return steps when any step has started', () => {
      // Given
      component.steps = [makeStep({ status: 'Running' })];

      // When & Then
      expect(component.displayStates).toBe(component.steps);
    });
  });

  describe('projectedBody setter', () => {
    it('should set bodyTemplate when a template is projected', () => {
      // Given
      const fakeTemplate: any = { name: 'fake-template' };

      // When
      component.projectedBody = fakeTemplate;

      // Then
      expect(component.bodyTemplate).toBe(fakeTemplate);
    });

    it('should preserve existing bodyTemplate when projected is undefined', () => {
      // Given
      const existing: any = { name: 'existing' };
      component.bodyTemplate = existing;

      // When
      component.projectedBody = undefined;

      // Then
      expect(component.bodyTemplate).toBe(existing);
    });
  });

  describe('syncUi status colors', () => {
    it('should set blue for Running status', () => {
      // Given
      component.steps = [makeStep({ status: 'Running' })];

      // When
      triggerChanges(component, {
        steps: { prev: null, curr: component.steps, first: true },
      });

      // Then
      expect(component.uiStates[0].statusColor()).toBe('blue');
    });

    it('should set green for Completed status', () => {
      // Given
      component.steps = [makeStep({ status: 'Completed' })];

      // When
      triggerChanges(component, {
        steps: { prev: null, curr: component.steps, first: true },
      });

      // Then
      expect(component.uiStates[0].statusColor()).toBe('green');
    });

    it('should set red for Failed status', () => {
      // Given
      component.steps = [makeStep({ status: 'Failed' })];

      // When
      triggerChanges(component, {
        steps: { prev: null, curr: component.steps, first: true },
      });

      // Then
      expect(component.uiStates[0].statusColor()).toBe('red');
    });

    it('should leave statusColor unset for Pending', () => {
      // Given
      component.steps = [makeStep({ status: 'Pending' })];

      // When
      triggerChanges(component, {
        steps: { prev: null, curr: component.steps, first: true },
      });

      // Then
      expect(component.uiStates.length).toBe(0);
    });

    it('should leave statusColor undefined for Skipped status', () => {
      // Given
      component.steps = [makeStep({ status: 'Skipped' })];

      // When
      triggerChanges(component, {
        steps: { prev: null, curr: component.steps, first: true },
      });

      // Then
      expect(component.uiStates[0].statusColor()).toBeUndefined();
    });

    it('should collapse step on Completed', () => {
      // Given
      component.steps = [makeStep({ status: 'Completed' })];

      // When
      triggerChanges(component, {
        steps: { prev: null, curr: component.steps, first: true },
      });

      // Then
      expect(component.uiStates[0].expanded).toBe(false);
    });

    it('should expand step on Running', () => {
      // Given
      component.steps = [makeStep({ status: 'Running' })];

      // When
      triggerChanges(component, {
        steps: { prev: null, curr: component.steps, first: true },
      });

      // Then
      expect(component.uiStates[0].expanded).toBe(true);
    });

    it('should not update color when status did not change', () => {
      // Given
      component.steps = [makeStep({ status: 'Running' })];
      triggerChanges(component, {
        steps: { prev: null, curr: component.steps, first: true },
      });
      expect(component.uiStates[0].statusColor()).toBe('blue');
      component.uiStates[0].statusColor.set(undefined);

      // When
      triggerChanges(component, {
        isExecuting: { prev: true, curr: true, first: false },
      });

      // Then
      expect(component.uiStates[0].statusColor()).toBeUndefined();
    });
  });

  describe('syncUi for multiple steps', () => {
    it('should create uiStates for each step', () => {
      // Given
      component.steps = [
        makeStep({ index: 0, status: 'Completed' }),
        makeStep({ index: 1, status: 'Running' }),
        makeStep({ index: 2, status: 'Pending' }),
      ];

      // When
      triggerChanges(component, {
        steps: { prev: null, curr: component.steps, first: true },
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
      component.steps = [makeStep({ status: 'Completed' })];
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

  describe('subStepsStatusColor', () => {
    it('should set red when any sub-step failed', () => {
      // Given
      component.steps = [
        makeStep({
          status: 'Failed',
          subSteps: [
            makeStep({ status: 'Completed' }),
            makeStep({ index: 1, status: 'Failed' }),
          ],
        }),
      ];

      // When
      triggerChanges(component, {
        steps: { prev: null, curr: component.steps, first: true },
      });

      // Then
      expect(component.uiStates[0].subStepsStatusColor()).toBe('red');
    });

    it('should set blue when a sub-step is running', () => {
      // Given
      component.steps = [
        makeStep({
          status: 'Failed',
          subSteps: [makeStep({ status: 'Running' })],
        }),
      ];

      // When
      triggerChanges(component, {
        steps: { prev: null, curr: component.steps, first: true },
      });

      // Then
      expect(component.uiStates[0].subStepsStatusColor()).toBe('blue');
    });

    it('should set green when all sub-steps completed', () => {
      // Given
      component.steps = [
        makeStep({
          status: 'Failed',
          subSteps: [
            makeStep({ status: 'Completed' }),
            makeStep({ index: 1, status: 'Completed' }),
          ],
        }),
      ];

      // When
      triggerChanges(component, {
        steps: { prev: null, curr: component.steps, first: true },
      });

      // Then
      expect(component.uiStates[0].subStepsStatusColor()).toBe('green');
    });

    it('should leave subStepsStatusColor undefined when no sub-steps exist', () => {
      // Given
      component.steps = [makeStep({ status: 'Running', subSteps: [] })];

      // When
      triggerChanges(component, {
        steps: { prev: null, curr: component.steps, first: true },
      });

      // Then
      expect(component.uiStates[0].subStepsStatusColor()).toBeUndefined();
    });

    it('should leave subStepsStatusColor undefined when sub-steps are pending', () => {
      // Given
      component.steps = [
        makeStep({
          status: 'Running',
          subSteps: [makeStep({ status: 'Pending' })],
        }),
      ];

      // When
      triggerChanges(component, {
        steps: { prev: null, curr: component.steps, first: true },
      });

      // Then
      expect(component.uiStates[0].subStepsStatusColor()).toBeUndefined();
    });

    it('should leave subStepsStatusColor undefined when subSteps is undefined', () => {
      // Given
      component.steps = [makeStep({ status: 'Running' })];

      // When
      triggerChanges(component, {
        steps: { prev: null, curr: component.steps, first: true },
      });

      // Then
      expect(component.uiStates[0].subStepsStatusColor()).toBeUndefined();
    });

    it('should track sub-step status color independently for each step', () => {
      // Given
      component.steps = [
        makeStep({
          status: 'Failed',
          subSteps: [makeStep({ status: 'Running' })],
        }),
        makeStep({
          index: 1,
          status: 'Failed',
          subSteps: [makeStep({ status: 'Failed' })],
        }),
      ];

      // When
      triggerChanges(component, {
        steps: { prev: null, curr: component.steps, first: true },
      });

      // Then
      expect(component.uiStates[0].subStepsStatusColor()).toBe('blue');
      expect(component.uiStates[1].subStepsStatusColor()).toBe('red');
    });
  });
});
