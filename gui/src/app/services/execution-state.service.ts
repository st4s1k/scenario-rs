import { Injectable, signal, computed } from '@angular/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import {
  ExecutionState,
  StateDiff,
} from '../models/step-state.model';

@Injectable({ providedIn: 'root' })
export class ExecutionStateService {
  readonly executionState = signal<ExecutionState | null>(null);

  readonly isExecuting = computed(() => {
    const state = this.executionState();
    return state?.status.kind === 'Running';
  });

  private unlistenDiff?: UnlistenFn;

  async init(): Promise<void> {
    const state = await invoke<ExecutionState | null>('get_execution_state');
    if (state) {
      this.executionState.set(state);
    }

    this.unlistenDiff = await listen<StateDiff[]>('execution-diff', (event) => {
      this.applyDiffs(event.payload);
    });
  }

  async destroy(): Promise<void> {
    this.unlistenDiff?.();
  }

  private applyDiffs(diffs: StateDiff[]): void {
    this.executionState.update(state => {
      if (!state) return state;
      // Shallow-clone top level and steps for immutable signal update
      const newState: ExecutionState = {
        status: { ...state.status },
        steps: state.steps.map(s => ({
          ...s,
          errors: [...s.errors],
          on_fail_steps: s.on_fail_steps.map(ofs => ({
            ...ofs,
            errors: [...ofs.errors],
          })),
        })),
      };
      for (const diff of diffs) {
        this.applyDiff(newState, diff);
      }
      return newState;
    });
  }

  private applyDiff(state: ExecutionState, diff: StateDiff): void {
    switch (diff.kind) {
      case 'ExecutionStatusChanged':
        state.status = diff.status;
        break;
      case 'StepStatusChanged':
        state.steps[diff.step_index].status = diff.status;
        break;
      case 'StepProgressUpdated':
        state.steps[diff.step_index].progress = diff.progress;
        break;
      case 'StepOutputAppended':
        state.steps[diff.step_index].output += diff.text;
        break;
      case 'StepErrorAdded':
        state.steps[diff.step_index].errors.push(diff.error);
        break;
      case 'OnFailStepStatusChanged':
        state.steps[diff.step_index].on_fail_steps[diff.on_fail_step_index].status = diff.status;
        break;
      case 'OnFailStepProgressUpdated':
        state.steps[diff.step_index].on_fail_steps[diff.on_fail_step_index].progress = diff.progress;
        break;
      case 'OnFailStepOutputAppended':
        state.steps[diff.step_index].on_fail_steps[diff.on_fail_step_index].output += diff.text;
        break;
      case 'OnFailStepErrorAdded':
        state.steps[diff.step_index].on_fail_steps[diff.on_fail_step_index].errors.push(diff.error);
        break;
    }
  }
}
