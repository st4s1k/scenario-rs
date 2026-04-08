import { CommonModule } from '@angular/common';
import { Component, Input, OnChanges, signal, SimpleChanges, WritableSignal } from '@angular/core';
import { Step, OnFailStep, Task } from '../models/scenario.model';
import { StepExecState, OnFailStepExecState, StepStatus } from '../models/step-state.model';
import { ExpandableComponent } from '../shared/expandable/expandable.component';
import { InfoBlockComponent } from '../shared/info-block/info-block.component';
import { ExpandableTitleComponent } from '../shared/expandable/expandable-title/expandable-title.component';
import { ComponentColorVariant } from '../models/enums';

interface UiState {
  expanded: boolean;
  errorsExpanded: boolean;
  onFailExpanded: boolean;
  statusColor: WritableSignal<ComponentColorVariant | undefined>;
  lastStatus?: StepStatus;
}

@Component({
  selector: 'execution-progress',
  imports: [
    CommonModule,
    InfoBlockComponent,
    ExpandableComponent,
    ExpandableTitleComponent
  ],
  templateUrl: './execution-progress.component.html',
  styleUrl: './execution-progress.component.scss'
})
export class ExecutionProgressComponent implements OnChanges {

  Math = Math;

  @Input() steps?: Step[];
  @Input() onFailSteps?: OnFailStep[];
  @Input({ required: true }) isExecuting!: boolean;

  @Input() stepExecStates: StepExecState[] = [];
  @Input() onFailExecStates?: OnFailStepExecState[];

  get displayStates(): readonly (StepExecState | OnFailStepExecState)[] {
    const states = this.onFailExecStates ?? this.stepExecStates;
    const anyStarted = states.some(s => s.status !== 'Pending');
    return anyStarted ? states : [];
  }

  uiStates: UiState[] = [];
  onFailStatusColor = signal<ComponentColorVariant | undefined>(undefined);

  private previousIsExecuting?: boolean;

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['stepExecStates'] || changes['onFailExecStates']
      || (changes['isExecuting']
        && this.previousIsExecuting === false
        && this.isExecuting === true)) {
      this.uiStates = [];
    }
    this.previousIsExecuting = this.isExecuting;
    this.syncAllUi();
  }

  getTask(index: number): Task | undefined {
    return this.steps?.[index]?.task ?? this.onFailSteps?.[index]?.task;
  }

  hasOnFailSteps(state: StepExecState | OnFailStepExecState): state is StepExecState {
    return 'on_fail_steps' in state && (state as StepExecState).on_fail_steps.length > 0;
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

  private syncAllUi(): void {
    const states = this.displayStates;
    for (let i = 0; i < states.length; i++) {
      this.syncUi(i, states[i].status);

      if ('on_fail_steps' in states[i]) {
        const onFailSteps = (states[i] as StepExecState).on_fail_steps;
        if (onFailSteps.length > 0) {
          const hasFailure = onFailSteps.some(s => s.status === 'Failed');
          const hasRunning = onFailSteps.some(s => s.status === 'Running');
          const allDone = onFailSteps.every(s => s.status === 'Completed' || s.status === 'Failed');

          if (hasFailure) {
            this.onFailStatusColor.set('red');
          } else if (hasRunning) {
            this.onFailStatusColor.set('blue');
          } else if (allDone) {
            this.onFailStatusColor.set('green');
          }
        }
      }
    }
  }

  private syncUi(index: number, status: StepStatus): void {
    while (this.uiStates.length <= index) {
      this.uiStates.push({
        expanded: true,
        errorsExpanded: true,
        onFailExpanded: true,
        statusColor: signal(undefined),
      });
    }
    const ui = this.uiStates[index];
    if (ui.lastStatus !== status) {
      switch (status) {
        case 'Running':
          ui.statusColor.set('blue');
          ui.expanded = true;
          break;
        case 'Completed':
          ui.statusColor.set('green');
          ui.expanded = false;
          break;
        case 'Failed':
          ui.statusColor.set('red');
          break;
      }
      ui.lastStatus = status;
    }
  }
}
