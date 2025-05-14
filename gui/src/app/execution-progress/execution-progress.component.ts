import { CommonModule } from '@angular/common';
import { Component, Input, OnChanges, OnDestroy, OnInit, signal, SimpleChanges, WritableSignal } from '@angular/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { Step, Task } from '../models/scenario.model';
import { AutoScrollDirective } from '../auto-scroll.directive';
import { OnFailStepStateEvent, RemoteSudoOutput, SftpCopyProgress, StepState, StepStateEvent } from '../models/step-state.model';
import { ExpandableComponent } from '../shared/expandable/expandable.component';
import { InfoBlockComponent } from '../shared/info-block/info-block.component';
import { ExpandableTitleComponent } from '../shared/expandable/expandable-title/expandable-title.component';
import { ComponentColorVariant } from '../models/enums';

type StepStatus = 'executing' | 'completed' | 'failed' | 'pending';

interface StepView {
  task: Task;
  state: WritableSignal<StepState | undefined>;
  status: StepStatus;
  errors?: string[];
  expanded: boolean;
  errorsExpanded: boolean;
  onFailExpanded: boolean;
  statusColor: WritableSignal<ComponentColorVariant | undefined>;
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

  @Input() steps?: Step[];
  @Input({ required: true }) isExecuting!: boolean;
  @Input() stepViews: StepView[] = [];

  onFailStepViews: StepView[] = [];
  onFailStatus: StepStatus = 'pending';
  onFailStatusColor: WritableSignal<ComponentColorVariant | undefined> = signal(undefined);

  unlistenStepState?: UnlistenFn;
  unlistenOnFailStepState?: UnlistenFn;

  private previousIsExecuting?: boolean;

  ngOnInit(): void {
    this.previousIsExecuting = this.isExecuting;
    if (this.steps !== undefined) {
      this.stepViews = [];
      this.setupStepStateListener();
      this.setupOnFailStepStateListener();
    }
  }

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['isExecuting']
      && this.previousIsExecuting === false
      && this.isExecuting === true) {
      this.stepViews = [];
      this.onFailStepViews = [];
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

      this.stepViews = this.updateStepViews(this.stepViews, index, task, state);
    });
  }

  private async setupOnFailStepStateListener(): Promise<void> {
    this.unlistenOnFailStepState = await listen<OnFailStepStateEvent>("on-fail-step-state", (event) => {
      const stepEvent = event.payload;
      const index = stepEvent.on_fail_step_index;
      const task = this.steps![stepEvent.step_index].on_fail_steps![index].task;
      const state = stepEvent.state;

      if (stepEvent.state.type === 'StepFailed') {
        this.onFailStatusColor.set('red');
      } else if (this.isExecuting) {
        this.onFailStatusColor.set('blue');
      } else if (this.onFailStatusColor() !== 'red') {
        this.onFailStatusColor.set('green');
      }

      this.onFailStepViews = this.updateStepViews(this.onFailStepViews, index, task, state);
    });
  }

  private updateStepViews(
    stepViews: StepView[],
    index: number,
    task: Task,
    state: StepState,
  ): StepView[] {
    switch (state.type) {
      case 'StepStarted':
        stepViews[index] = {
          task,
          state: signal<StepState | undefined>(state),
          status: 'pending',
          errors: [],
          expanded: true,
          errorsExpanded: true,
          onFailExpanded: true,
          statusColor: signal(undefined)
        };
        break;
      case 'SftpCopyProgress':
        stepViews[index].status = 'executing';
        stepViews[index].state.set({
          type: 'SftpCopyProgress',
          current: state.current,
          total: state.total,
          source: state.source,
          destination: state.destination
        } as SftpCopyProgress);
        stepViews[index].statusColor.set('blue');
        break;
      case 'RemoteSudoOutput':
        stepViews[index].status = 'executing';
        stepViews[index].state.set({
          type: 'RemoteSudoOutput',
          command: state.command,
          output: state.output
        } as RemoteSudoOutput);
        stepViews[index].statusColor.set('blue');
        break;
      case 'StepCompleted':
        stepViews[index].status = 'completed';
        stepViews[index].expanded = false;
        stepViews[index].statusColor.set('green');
        break;
      case 'StepFailed':
        stepViews[index].status = 'failed';
        stepViews[index].errors?.unshift(state.message);
        stepViews[index].statusColor.set('red');
        break;
    }
    return stepViews;
  }
}
