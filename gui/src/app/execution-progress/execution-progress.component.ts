import { CommonModule } from '@angular/common';
import { Component, Input, OnDestroy, OnInit, signal, WritableSignal } from '@angular/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { OnFailStep, Step, Task } from '../app.component';
import { AutoScrollDirective } from '../auto-scroll.directive';
import { OnFailStepStateEvent, RemoteSudoOutput, SftpCopyProgress, StepState, StepStateEvent } from '../models/step-state.model';
import { ExpandableComponent } from '../shared/expandable/expandable.component';
import { InfoBlockComponent } from '../shared/info-block/info-block.component';

type StepStatus = 'executing' | 'completed' | 'failed' | 'pending';

interface DisplayStep {
  task: Task;
  state: WritableSignal<StepState>;
  status: StepStatus;
  errorMessage?: string;
  expanded: boolean;
  onFailExpanded: boolean;
  onFailSteps?: OnFailStep[];
}

@Component({
  selector: 'execution-progress',
  imports: [
    CommonModule,
    AutoScrollDirective,
    InfoBlockComponent,
    ExpandableComponent,
  ],
  templateUrl: './execution-progress.component.html',
  styleUrl: './execution-progress.component.scss'
})
export class ExecutionProgressComponent implements OnInit, OnDestroy {

  Math = Math;

  @Input() variant: 'primary' | 'error' = 'primary';
  @Input() steps?: Step[];
  @Input() onFailSteps?: OnFailStep[];

  displaySteps: DisplayStep[] = [];

  unlistenStepState?: UnlistenFn;
  unlistenOnFailStepState?: UnlistenFn;

  ngOnInit(): void {
    this.displaySteps = [];
    if (this.steps !== undefined) {
      this.setupStepStateListener();
    }
    if (this.onFailSteps !== undefined) {
      this.setupOnFailStepStateListener();
    }
  }

  ngOnDestroy(): void {
    if (this.unlistenStepState) {
      this.unlistenStepState();
    }
    if (this.unlistenOnFailStepState) {
      this.unlistenOnFailStepState();
    }
  }

  formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const sizeFactor = 1024;
    const decimals = 2;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB'];
    const exponent = Math.floor(Math.log(bytes) / Math.log(sizeFactor));
    const baseSize = Math.pow(sizeFactor, exponent);
    const convertedSize = bytes / baseSize;
    return convertedSize.toFixed(decimals) + ' ' + sizes[exponent];
  }

  private async setupStepStateListener(): Promise<void> {
    this.unlistenStepState = await listen<StepStateEvent>("step-state", (event) => {
      this.updateDisplayStep(event.payload);
    });
  }

  private async setupOnFailStepStateListener(): Promise<void> {
    this.unlistenOnFailStepState = await listen<OnFailStepStateEvent>("on-fail-step-state", (event) => {
      this.updateDisplayOnFailStep(event.payload);
    });
  }

  private updateDisplayStep(
    stepEvent: StepStateEvent
  ): void {
    if (this.steps === undefined) {
      console.error("Steps are not defined");
      return;
    }

    const index = stepEvent.step_index;
    const task = this.steps[index].task;
    const state = stepEvent.state;
    const status = this.getDisplayStatus(state);
    const onFailSteps = this.steps[index].on_fail_steps;

    this.updateDisplaySteps(index, task, state, status, onFailSteps);
  }

  private updateDisplayOnFailStep(
    stepEvent: OnFailStepStateEvent
  ): void {
    if (this.onFailSteps === undefined) {
      console.error("OnFailSteps are not defined");
      return;
    }

    const index = stepEvent.on_fail_step_index;
    const task = this.onFailSteps[index].task;
    const state = stepEvent.state;
    const status = this.getDisplayStatus(state);
    const onFailSteps = undefined;

    this.updateDisplaySteps(index, task, state, status, onFailSteps);
  }

  private updateDisplaySteps(
    index: number,
    task: Task,
    state: StepState,
    status: StepStatus,
    onFailSteps?: OnFailStep[]
  ): void {
    if (this.displaySteps[index] === undefined) {
      this.displaySteps[index] = {
        task,
        state: signal<StepState>(state),
        status,
        errorMessage: undefined,
        expanded: true,
        onFailExpanded: true,
        onFailSteps,
      };
      this.displaySteps[index].state.set(state);
      this.displaySteps.slice(0, index).forEach((step) => (step.expanded = false));
    } else {
      const displayStep = this.displaySteps[index];
      displayStep.status = status;
      displayStep.errorMessage = state.type === 'StepFailed' ? state.message : undefined;
      displayStep.expanded = status !== 'completed';
      const oldDisplayState = displayStep.state();
      const newDisplayState = this.updateDisplayState(oldDisplayState, state);
      displayStep.state.set(newDisplayState);
    }
  }

  private updateDisplayState(
    oldDisplayState: StepState,
    state: StepState
  ): StepState {
    switch (state.type) {
      case 'SftpCopyProgress':
        return {
          type: 'SftpCopyProgress',
          current: state.current,
          total: state.total,
          source: state.source,
          destination: state.destination
        } as SftpCopyProgress;
      case 'RemoteSudoOutput':
        return {
          type: 'RemoteSudoOutput',
          command: state.command,
          output: state.output
        } as RemoteSudoOutput;
      default:
        return oldDisplayState;
    }
  }

  private getDisplayStatus(state: StepState): StepStatus {
    if (state) {
      switch (state.type) {
        case 'StepCompleted':
          return 'completed';
        case 'StepFailed':
          return 'failed';
        case 'SftpCopyProgress':
        case 'RemoteSudoOutput':
          return 'executing';
      }
    } else {
      return 'pending';
    }
  }
}
