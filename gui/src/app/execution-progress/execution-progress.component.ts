import { Component, Input, OnDestroy, signal, WritableSignal, OnChanges, SimpleChanges, AfterViewInit } from '@angular/core';
import { Task } from '../app.component';
import { CommonModule } from '@angular/common';
import { AutoScrollDirective } from '../auto-scroll.directive';
import { InfoBlockComponent } from '../shared/info-block/info-block.component';
import { ExpandableComponent } from '../shared/expandable/expandable.component';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { StepEvent } from '../models/step-state.model';

type StepStatus = 'executing' | 'completed' | 'failed' | 'pending';

type StepStateType = 'SftpCopyState' | 'RemoteSudoState';

interface BaseStepState {
  type: StepStateType;
}

interface SftpCopyState extends BaseStepState {
  type: 'SftpCopyState';
  current: number;
  total: number;
  source: string;
  destination: string;
}

interface RemoteSudoState extends BaseStepState {
  type: 'RemoteSudoState';
  command: string;
  output: string;
}

interface DisplayStep {
  index: number;
  task: Task;
  state: WritableSignal<StepState | undefined>;
  status: StepStatus;
  errorMessage?: string;
  expanded: boolean;
}

type StepState = SftpCopyState | RemoteSudoState;

@Component({
  selector: 'execution-progress',
  imports: [
    CommonModule,
    AutoScrollDirective,
    InfoBlockComponent,
    ExpandableComponent
  ],
  templateUrl: './execution-progress.component.html',
  styleUrl: './execution-progress.component.scss'
})
export class ExecutionProgressComponent implements OnChanges, OnDestroy, AfterViewInit {

  Object = Object;
  Math = Math;

  @Input() tasks: Task[] = [];

  currentStepIndex = signal(-1);
  displaySteps: DisplayStep[] = [];

  unlistenStepState?: UnlistenFn;
  unlistenCurrentStepIndex?: UnlistenFn;

  ngOnInit(): void {
    this.setupStepStateListener();
    this.setupCurrentStepIndexListener();
  }

  ngAfterViewInit(): void {
  }

  ngOnDestroy(): void {
    if (this.unlistenStepState) {
      this.unlistenStepState();
    }
    if (this.unlistenCurrentStepIndex) {
      this.unlistenCurrentStepIndex();
    }
  }

  ngOnChanges(changes: SimpleChanges): void {
    if (changes["tasks"]) {
      this.initializeDisplaySteps();
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

  private initializeDisplaySteps() {
    if (this.tasks.length === 0) {
      this.displaySteps = [];
    } else {
      this.displaySteps = this.tasks.map((task, index) => ({
        index,
        task,
        state: signal(undefined),
        status: 'pending',
        errorMessage: undefined,
        expanded: false
      } as DisplayStep));
    }
  }

  private async setupStepStateListener(): Promise<void> {
    this.unlistenStepState = await listen<StepEvent>('step-state', (event) => {
      this.updateDisplayStep(event.payload);
    });
  }

  private async setupCurrentStepIndexListener(): Promise<void> {
    this.unlistenCurrentStepIndex = await listen<number>('step-index', (event) => {
      this.currentStepIndex.set(event.payload);
    });
  }

  private updateDisplayStep(
    stepEvent: StepEvent
  ): void {
    const index = this.currentStepIndex();
    this.displaySteps[index] = {
      ...this.displaySteps[index],
      status: this.getDisplayStatus(stepEvent),
      errorMessage: stepEvent.type === 'StepFailed' ? stepEvent.message : undefined,
      expanded: true
    };
    const oldDisplayState = this.displaySteps[index].state();
    const newDisplayState = this.updateDisplayState(oldDisplayState, stepEvent);
    this.displaySteps[index].state.set(newDisplayState);
    this.displaySteps.slice(0, index).forEach((step) => (step.expanded = false));
  }

  private updateDisplayState(
    oldDisplayState: StepState | undefined,
    stepEvent: StepEvent
  ): StepState | undefined {
    if (stepEvent) {
      switch (stepEvent.type) {
        case 'SftpCopyProgress':
          return {
            type: 'SftpCopyState',
            current: stepEvent.current,
            total: stepEvent.total,
            source: stepEvent.source,
            destination: stepEvent.destination
          } as SftpCopyState;
        case 'RemoteSudoOutput':
          return {
            type: 'RemoteSudoState',
            command: stepEvent.command,
            output: stepEvent.output
          } as RemoteSudoState;
      }
    }
    return oldDisplayState;
  }

  private getDisplayStatus(stepEvent: StepEvent): StepStatus {
    if (stepEvent) {
      switch (stepEvent.type) {
        case 'StepCompleted':
          return 'completed';
        case 'StepFailed':
          return 'failed';
      }
      return 'executing';
    } else {
      return 'pending';
    }
  }
}
