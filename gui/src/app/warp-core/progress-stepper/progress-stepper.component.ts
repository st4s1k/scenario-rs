import { CommonModule } from '@angular/common';
import {
  Component,
  ContentChild,
  Input,
  OnChanges,
  signal,
  SimpleChanges,
  TemplateRef,
  WritableSignal,
} from '@angular/core';
import { ComponentColorVariant } from '../component-types';
import { ExpandableComponent } from '../expandable/expandable.component';
import { InfoBlockComponent } from '../info-block/info-block.component';
import { ExpandableTitleComponent } from '../expandable/expandable-title/expandable-title.component';

export type StepRunStatus = 'Pending' | 'Running' | 'Completed' | 'Failed' | 'Skipped';

export interface StepperStep {
  index: number;
  title: string;
  status: StepRunStatus;
  errors: string[];
  subSteps?: StepperStep[];
  data?: unknown;
}

export interface StepBodyContext {
  $implicit: StepperStep;
  index: number;
}

interface UiState {
  expanded: boolean;
  errorsExpanded: boolean;
  subStepsExpanded: boolean;
  statusColor: WritableSignal<ComponentColorVariant | undefined>;
  subStepsStatusColor: WritableSignal<ComponentColorVariant | undefined>;
  lastStatus?: StepRunStatus;
}

@Component({
  selector: 'progress-stepper',
  imports: [
    CommonModule,
    InfoBlockComponent,
    ExpandableComponent,
    ExpandableTitleComponent,
  ],
  templateUrl: './progress-stepper.component.html',
  styleUrl: './progress-stepper.component.scss',
})
export class ProgressStepperComponent implements OnChanges {

  @Input() steps: StepperStep[] = [];
  @Input({ required: true }) isExecuting!: boolean;
  @Input() emptyMessage: string = 'No steps to display';
  @Input() subStepsLabel: string = 'Sub-steps';
  @Input() bodyTemplate?: TemplateRef<StepBodyContext>;

  @ContentChild('stepBody', { static: false })
  set projectedBody(template: TemplateRef<StepBodyContext> | undefined) {
    if (template) this.bodyTemplate = template;
  }

  uiStates: UiState[] = [];

  private previousIsExecuting?: boolean;

  get displayStates(): readonly StepperStep[] {
    const anyStarted = this.steps.some(s => s.status !== 'Pending');
    return anyStarted ? this.steps : [];
  }

  ngOnChanges(changes: SimpleChanges): void {
    const executionRestarted = changes['isExecuting']
      && this.previousIsExecuting === false
      && this.isExecuting === true;

    if (executionRestarted) {
      this.uiStates = [];
    }

    this.previousIsExecuting = this.isExecuting;
    this.syncAllUi();
  }

  hasSubSteps(step: StepperStep): boolean {
    return !!step.subSteps && step.subSteps.length > 0;
  }

  private syncAllUi(): void {
    const states = this.displayStates;
    for (let i = 0; i < states.length; i++) {
      this.syncUi(i, states[i].status);
      const ui = this.uiStates[i];

      const subSteps = states[i].subSteps;
      if (subSteps && subSteps.length > 0) {
        const hasFailure = subSteps.some(s => s.status === 'Failed');
        const hasRunning = subSteps.some(s => s.status === 'Running');
        const allDone = subSteps.every(s => s.status === 'Completed' || s.status === 'Failed');

        if (hasFailure) {
          ui.subStepsStatusColor.set('red');
        } else if (hasRunning) {
          ui.subStepsStatusColor.set('blue');
        } else if (allDone) {
          ui.subStepsStatusColor.set('green');
        } else {
          ui.subStepsStatusColor.set(undefined);
        }
      } else {
        ui.subStepsStatusColor.set(undefined);
      }
    }
  }

  private syncUi(index: number, status: StepRunStatus): void {
    while (this.uiStates.length <= index) {
      this.uiStates.push({
        expanded: true,
        errorsExpanded: true,
        subStepsExpanded: true,
        statusColor: signal(undefined),
        subStepsStatusColor: signal(undefined),
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
