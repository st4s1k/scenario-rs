import { TestBed } from '@angular/core/testing';
import { ExecutionStateService } from './execution-state.service';
import {
  ExecutionState,
  StepExecState,
  OnFailStepExecState,
  StateDiff,
} from '../models/step-state.model';

function makeOnFailStep(
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

function makeStep(overrides: Partial<StepExecState> = {}): StepExecState {
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

function makeState(overrides: Partial<ExecutionState> = {}): ExecutionState {
  return {
    status: { kind: 'Idle' },
    steps: [],
    ...overrides,
  };
}

describe('ExecutionStateService', () => {
  let service: ExecutionStateService;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    service = TestBed.inject(ExecutionStateService);
  });

  describe('isExecuting', () => {
    it('should return false when state is null', () => {
      // Given & When & Then
      expect(service.isExecuting()).toBe(false);
    });

    it('should return true when status is Running', () => {
      // Given
      service.executionState.set(makeState({ status: { kind: 'Running' } }));

      // When & Then
      expect(service.isExecuting()).toBe(true);
    });

    it('should return false when status is Idle', () => {
      // Given
      service.executionState.set(makeState({ status: { kind: 'Idle' } }));

      // When & Then
      expect(service.isExecuting()).toBe(false);
    });

    it('should return false when status is Completed', () => {
      // Given
      service.executionState.set(makeState({ status: { kind: 'Completed' } }));

      // When & Then
      expect(service.isExecuting()).toBe(false);
    });

    it('should return false when status is Failed', () => {
      // Given
      service.executionState.set(
        makeState({ status: { kind: 'Failed', error: 'oops' } })
      );

      // When & Then
      expect(service.isExecuting()).toBe(false);
    });
  });

  describe('applyDiffs', () => {
    it('should be a no-op when state is null', () => {
      // Given & When
      (service as any).applyDiffs([
        { kind: 'ExecutionStatusChanged', status: { kind: 'Running' } },
      ]);

      // Then
      expect(service.executionState()).toBeNull();
    });

    it('should apply ExecutionStatusChanged', () => {
      // Given
      service.executionState.set(makeState());

      // When
      (service as any).applyDiffs([
        { kind: 'ExecutionStatusChanged', status: { kind: 'Running' } },
      ]);

      // Then
      expect(service.executionState()!.status.kind).toBe('Running');
    });

    it('should apply StepStatusChanged', () => {
      // Given
      service.executionState.set(makeState({ steps: [makeStep()] }));

      // When
      (service as any).applyDiffs([
        { kind: 'StepStatusChanged', step_index: 0, status: 'Running' },
      ]);

      // Then
      expect(service.executionState()!.steps[0].status).toBe('Running');
    });

    it('should apply StepProgressUpdated with RemoteSudo', () => {
      // Given
      service.executionState.set(makeState({ steps: [makeStep()] }));
      const progress = {
        type: 'RemoteSudo' as const,
        command: 'ls -la',
        output: 'total 0',
      };

      // When
      (service as any).applyDiffs([
        { kind: 'StepProgressUpdated', step_index: 0, progress },
      ]);

      // Then
      expect(service.executionState()!.steps[0].progress).toEqual(progress);
    });

    it('should apply StepProgressUpdated with SftpCopy', () => {
      // Given
      service.executionState.set(makeState({ steps: [makeStep()] }));
      const progress = {
        type: 'SftpCopy' as const,
        source: '/a',
        destination: '/b',
        bytes_transferred: 512,
        bytes_total: 1024,
      };

      // When
      (service as any).applyDiffs([
        { kind: 'StepProgressUpdated', step_index: 0, progress },
      ]);

      // Then
      expect(service.executionState()!.steps[0].progress).toEqual(progress);
    });

    it('should apply StepOutputAppended', () => {
      // Given
      service.executionState.set(
        makeState({ steps: [makeStep({ output: 'hello ' })] })
      );

      // When
      (service as any).applyDiffs([
        { kind: 'StepOutputAppended', step_index: 0, text: 'world' },
      ]);

      // Then
      expect(service.executionState()!.steps[0].output).toBe('hello world');
    });

    it('should apply StepErrorAdded', () => {
      // Given
      service.executionState.set(makeState({ steps: [makeStep()] }));

      // When
      (service as any).applyDiffs([
        { kind: 'StepErrorAdded', step_index: 0, error: 'connection refused' },
      ]);

      // Then
      expect(service.executionState()!.steps[0].errors).toEqual([
        'connection refused',
      ]);
    });

    it('should accumulate multiple errors', () => {
      // Given
      service.executionState.set(
        makeState({ steps: [makeStep({ errors: ['first'] })] })
      );

      // When
      (service as any).applyDiffs([
        { kind: 'StepErrorAdded', step_index: 0, error: 'second' },
      ]);

      // Then
      expect(service.executionState()!.steps[0].errors).toEqual([
        'first',
        'second',
      ]);
    });

    it('should apply OnFailStepStatusChanged', () => {
      // Given
      service.executionState.set(
        makeState({
          steps: [makeStep({ on_fail_steps: [makeOnFailStep()] })],
        })
      );

      // When
      (service as any).applyDiffs([
        {
          kind: 'OnFailStepStatusChanged',
          step_index: 0,
          on_fail_step_index: 0,
          status: 'Running',
        },
      ]);

      // Then
      expect(
        service.executionState()!.steps[0].on_fail_steps[0].status
      ).toBe('Running');
    });

    it('should apply OnFailStepProgressUpdated', () => {
      // Given
      service.executionState.set(
        makeState({
          steps: [
            makeStep({
              on_fail_steps: [makeOnFailStep({ status: 'Running' })],
            }),
          ],
        })
      );
      const progress = {
        type: 'SftpCopy' as const,
        source: '/src',
        destination: '/dst',
        bytes_transferred: 100,
        bytes_total: 200,
      };

      // When
      (service as any).applyDiffs([
        {
          kind: 'OnFailStepProgressUpdated',
          step_index: 0,
          on_fail_step_index: 0,
          progress,
        },
      ]);

      // Then
      expect(
        service.executionState()!.steps[0].on_fail_steps[0].progress
      ).toEqual(progress);
    });

    it('should apply OnFailStepOutputAppended', () => {
      // Given
      service.executionState.set(
        makeState({
          steps: [
            makeStep({
              on_fail_steps: [makeOnFailStep({ output: 'start ' })],
            }),
          ],
        })
      );

      // When
      (service as any).applyDiffs([
        {
          kind: 'OnFailStepOutputAppended',
          step_index: 0,
          on_fail_step_index: 0,
          text: 'end',
        },
      ]);

      // Then
      expect(
        service.executionState()!.steps[0].on_fail_steps[0].output
      ).toBe('start end');
    });

    it('should apply OnFailStepErrorAdded', () => {
      // Given
      service.executionState.set(
        makeState({
          steps: [makeStep({ on_fail_steps: [makeOnFailStep()] })],
        })
      );

      // When
      (service as any).applyDiffs([
        {
          kind: 'OnFailStepErrorAdded',
          step_index: 0,
          on_fail_step_index: 0,
          error: 'fatal',
        },
      ]);

      // Then
      expect(
        service.executionState()!.steps[0].on_fail_steps[0].errors
      ).toEqual(['fatal']);
    });
  });

  describe('batch diffs', () => {
    it('should apply multiple diffs in a single batch', () => {
      // Given
      service.executionState.set(
        makeState({
          steps: [makeStep(), makeStep({ index: 1, task_description: 'Step 2' })],
        })
      );

      // When
      (service as any).applyDiffs([
        { kind: 'ExecutionStatusChanged', status: { kind: 'Running' } },
        { kind: 'StepStatusChanged', step_index: 0, status: 'Running' },
        { kind: 'StepOutputAppended', step_index: 0, text: 'output' },
        { kind: 'StepStatusChanged', step_index: 1, status: 'Running' },
      ] as StateDiff[]);

      // Then
      expect(service.executionState()!.status.kind).toBe('Running');
      expect(service.executionState()!.steps[0].status).toBe('Running');
      expect(service.executionState()!.steps[0].output).toBe('output');
      expect(service.executionState()!.steps[1].status).toBe('Running');
    });

    it('should produce a new state reference after applying diffs', () => {
      // Given
      const original = makeState({ steps: [makeStep()] });
      service.executionState.set(original);

      // When
      (service as any).applyDiffs([
        { kind: 'StepStatusChanged', step_index: 0, status: 'Running' },
      ]);

      // Then
      expect(service.executionState()).not.toBe(original);
    });
  });
});
