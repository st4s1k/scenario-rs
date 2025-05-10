import { CommonModule } from '@angular/common';
import { Component, Input, OnChanges, OnDestroy, OnInit, signal, SimpleChanges, WritableSignal } from '@angular/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { Step, Task } from '../app.component';
import { AutoScrollDirective } from '../auto-scroll.directive';
import { OnFailStepStateEvent, RemoteSudoOutput, SftpCopyProgress, StepState, StepStateEvent } from '../models/step-state.model';
import { ExpandableComponent } from '../shared/expandable/expandable.component';
import { InfoBlockComponent } from '../shared/info-block/info-block.component';
import { ExpandableTitleComponent } from '../shared/expandable/expandable-title/expandable-title.component';

type StepStatus = 'executing' | 'completed' | 'failed' | 'pending';

interface DisplayStep {
  task: Task;
  state: WritableSignal<StepState | undefined>;
  status: StepStatus;
  errors?: string[];
  expanded: boolean;
  errorsExpanded: boolean;
  onFailExpanded: boolean;
}

@Component({
  selector: 'execution-progress',
  imports: [
    CommonModule,
    AutoScrollDirective,
    InfoBlockComponent,
    ExpandableComponent,
    ExpandableTitleComponent
  ],
  templateUrl: './execution-progress.component.html',
  styleUrl: './execution-progress.component.scss'
})
export class ExecutionProgressComponent implements OnInit, OnChanges, OnDestroy {

  Math = Math;

  @Input() variant: 'primary' | 'error' = 'primary';
  @Input() steps?: Step[];
  @Input({ required: true }) isExecuting!: boolean;
  @Input() displaySteps: DisplayStep[] = [];

  displayOnFailSteps: DisplayStep[] = [];
  onFailStatus: StepStatus = 'pending';

  unlistenStepState?: UnlistenFn;
  unlistenOnFailStepState?: UnlistenFn;

  private previousIsExecuting?: boolean;

  ngOnInit(): void {
    this.previousIsExecuting = this.isExecuting;
    if (this.steps !== undefined) {
      this.displaySteps = [];
      this.setupStepStateListener();
      this.setupOnFailStepStateListener();
    }
  }

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['isExecuting']
      && this.previousIsExecuting === false
      && this.isExecuting === true) {
      this.displaySteps = [];
      this.displayOnFailSteps = [];
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
      const stepEvent = event.payload;
      const index = stepEvent.step_index;
      const task = this.steps![index].task;
      const state = stepEvent.state;

      this.displaySteps = this.updateDisplaySteps(this.displaySteps, index, task, state);
    });
  }

  private async setupOnFailStepStateListener(): Promise<void> {
    this.unlistenOnFailStepState = await listen<OnFailStepStateEvent>("on-fail-step-state", (event) => {
      console.log('on-fail-step-state', event.payload);

      const stepEvent = event.payload;
      const index = stepEvent.on_fail_step_index;
      const task = this.steps![stepEvent.step_index].on_fail_steps![index].task;
      const state = stepEvent.state;

      if (stepEvent.state.type === 'StepFailed') {
        this.onFailStatus = 'failed';
      } else if (this.isExecuting) {
        this.onFailStatus = 'executing';
      } else if (this.onFailStatus !== 'failed') {
        this.onFailStatus = 'completed';
      }

      this.displayOnFailSteps = this.updateDisplaySteps(this.displayOnFailSteps, index, task, state);
    });
  }

  private updateDisplaySteps(
    displaySteps: DisplayStep[],
    index: number,
    task: Task,
    state: StepState,
  ): DisplayStep[] {
    switch (state.type) {
      case 'StepStarted':
        displaySteps[index] = {
          task,
          state: signal<StepState | undefined>(state),
          status: 'pending',
          errors: [],
          expanded: true,
          errorsExpanded: true,
          onFailExpanded: true,
        };
        break;
      case 'SftpCopyProgress':
        displaySteps[index].status = 'executing';
        displaySteps[index].state.set({
          type: 'SftpCopyProgress',
          current: state.current,
          total: state.total,
          source: state.source,
          destination: state.destination
        } as SftpCopyProgress);
        break;
      case 'RemoteSudoOutput':
        displaySteps[index].status = 'executing';
        displaySteps[index].state.set({
          type: 'RemoteSudoOutput',
          command: state.command,
          output: state.output
        } as RemoteSudoOutput);
        break;
      case 'StepCompleted':
        displaySteps[index].status = 'completed';
        displaySteps[index].expanded = false;
        break;
      case 'StepFailed':
        displaySteps[index].status = 'failed';
        displaySteps[index].errors?.unshift(state.message);
        break;
    }
    return displaySteps;
  }
}
