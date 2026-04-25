import { Injectable, inject, signal, computed } from '@angular/core';
import { Subscription } from 'rxjs';
import { invoke } from '@tauri-apps/api/core';
import {
  ExecutionState,
  StateDiff,
} from '../models/step-state.model';
import { TauriEventBridgeService } from '../warp-core/tauri-event-bridge.service';

@Injectable({ providedIn: 'root' })
export class ExecutionStateService {
  readonly executionState = signal<ExecutionState | null>(null);

  readonly isExecuting = computed(() => {
    const state = this.executionState();
    return state?.status.kind === 'Running';
  });

  private bridgeService = inject(TauriEventBridgeService);
  private diffSub?: Subscription;
  private hydrating = false;
  private pendingDiffs: StateDiff[][] = [];

  async init(): Promise<void> {
    await this.hydrate();

    this.diffSub = this.bridgeService.getStream<StateDiff[]>('execution-diff').subscribe(payload => {
      const hasRunningTransition = payload.some(
        d => d.kind === 'ExecutionStatusChanged' && d.status.kind === 'Running'
      );

      if (hasRunningTransition) {
        this.executionState.set(null);
        this.pendingDiffs.push(payload);
        this.hydrateAndFlush();
        return;
      }

      if (this.hydrating || !this.executionState()) {
        this.pendingDiffs.push(payload);
        if (!this.hydrating) {
          this.hydrateAndFlush();
        }
        return;
      }

      this.applyDiffs(payload);
    });
  }

  async destroy(): Promise<void> {
    this.diffSub?.unsubscribe();
  }

  reset(): void {
    this.executionState.set(null);
  }

  private async hydrate(): Promise<void> {
    const state = await invoke<ExecutionState | null>('get_execution_state');
    if (state) {
      this.executionState.set(state);
    }
  }

  private async hydrateAndFlush(): Promise<void> {
    if (this.hydrating) return;
    this.hydrating = true;
    try {
      await this.hydrate();
      const queued = this.pendingDiffs.splice(0);
      for (const batch of queued) {
        this.applyDiffs(batch);
      }
    } finally {
      this.hydrating = false;
    }
  }

  private applyDiffs(diffs: StateDiff[]): void {
    this.executionState.update(state => {
      if (!state) return state;
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
