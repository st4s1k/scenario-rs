import { TestBed } from '@angular/core/testing';
import { ExecutionStateService } from './execution-state.service';
import { TauriEventBridgeService } from '../warp-core/tauri-event-bridge.service';
import { setupTauriMock, TauriTestHarness } from '../testing/tauri-mocks';
import { Subject, EMPTY } from 'rxjs';
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
  let executionDiffSubject: Subject<StateDiff[]>;
  let bridgeServiceMock: any;

  beforeEach(() => {
    executionDiffSubject = new Subject<StateDiff[]>();
    bridgeServiceMock = {
      getStream: (id: string) => {
        if (id === 'execution-diff') return executionDiffSubject.asObservable();
        return EMPTY;
      },
    };

    TestBed.configureTestingModule({
      providers: [
        { provide: TauriEventBridgeService, useValue: bridgeServiceMock },
      ],
    });
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
        elapsed_ms: 500,
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
        elapsed_ms: 300,
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

  describe('init', () => {
    let tauri: TauriTestHarness;

    beforeEach(() => {
      tauri = setupTauriMock({
        'get_execution_state': null,
      });
    });

    it('should invoke get_execution_state to hydrate', async () => {
      // Given & When
      await service.init();

      // Then
      tauri.expectInvoked('get_execution_state');
    });

    it('should set executionState when hydrate returns state', async () => {
      // Given
      const state = makeState({ status: { kind: 'Running' } });
      tauri.setResponse('get_execution_state', state);

      // When
      await service.init();

      // Then
      expect(service.executionState()).toEqual(state);
    });

    it('should subscribe to execution-diff stream from bridge service', async () => {
      // Given & When
      await service.init();

      // Then
      expect((service as any).diffSub).toBeTruthy();
    });

    it('should apply diffs received via bridge service', async () => {
      // Given
      const state = makeState({ steps: [makeStep()] });
      tauri.setResponse('get_execution_state', state);
      await service.init();

      // When
      executionDiffSubject.next([{ kind: 'StepStatusChanged', step_index: 0, status: 'Running' }] as StateDiff[]);

      // Then
      expect(service.executionState()!.steps[0].status).toBe('Running');
    });

    it('should queue diffs and hydrate when state is null on diff arrival', async () => {
      // Given
      let getExecStateCallCount = 0;
      tauri.setResponse('get_execution_state', () => {
        getExecStateCallCount++;
        if (getExecStateCallCount === 1) return null;
        return makeState({ steps: [makeStep()] });
      });
      await service.init();

      // When
      executionDiffSubject.next([{ kind: 'StepStatusChanged', step_index: 0, status: 'Running' }] as StateDiff[]);

      await new Promise(resolve => setTimeout(resolve, 10));

      // Then
      expect(getExecStateCallCount).toBeGreaterThanOrEqual(2);
    });
  });

  describe('destroy', () => {
    it('should unsubscribe diff subscription', async () => {
      // Given
      setupTauriMock({ 'get_execution_state': null });
      await service.init();
      const unsubSpy = spyOn((service as any).diffSub, 'unsubscribe').and.callThrough();

      // When
      await service.destroy();

      // Then
      expect(unsubSpy).toHaveBeenCalled();
    });

    it('should not throw when called without init', async () => {
      // Given & When & Then
      await expectAsync(service.destroy()).toBeResolved();
    });
  });

  describe('hydrateAndFlush', () => {
    it('should skip if already hydrating', async () => {
      // Given
      const tauri = setupTauriMock({ 'get_execution_state': null });
      (service as any).hydrating = true;

      // When
      await (service as any).hydrateAndFlush();

      // Then
      tauri.expectNotInvoked('get_execution_state');
    });
  });

  describe('reset', () => {
    it('should set executionState to null', () => {
      // Given
      service.executionState.set(makeState({ status: { kind: 'Running' } }));

      // When
      service.reset();

      // Then
      expect(service.executionState()).toBeNull();
    });
  });

  describe('Running transition diff', () => {
    it('should reset state and rehydrate when diff contains Running transition', async () => {
      // Given
      const existingState = makeState({ steps: [makeStep()] });
      let hydrateCallCount = 0;
      const tauri = setupTauriMock({
        'get_execution_state': () => {
          hydrateCallCount++;
          return existingState;
        },
      });
      await service.init();

      // When
      executionDiffSubject.next([
        { kind: 'ExecutionStatusChanged', status: { kind: 'Running' } },
      ] as StateDiff[]);

      await new Promise(resolve => setTimeout(resolve, 10));

      // Then
      expect(hydrateCallCount).toBeGreaterThanOrEqual(2);
    });
  });
});
