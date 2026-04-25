import { CommonModule } from '@angular/common';
import { Component, computed, inject, OnDestroy, OnInit, signal } from '@angular/core';
import { FormControl, FormGroup, ReactiveFormsModule, AbstractControl, ValidationErrors, AsyncValidatorFn } from "@angular/forms";
import { invoke } from "@tauri-apps/api/core";
import { TitlebarComponent } from "./warp-core/titlebar/titlebar.component";
import { ClipboardModule } from 'ngx-clipboard';
import * as dialog from "@tauri-apps/plugin-dialog"
import { Subscription, Observable, of, from } from 'rxjs';
import { debounceTime, switchMap, map } from 'rxjs/operators';
import { SidebarComponent } from './warp-core/sidebar/sidebar.component';
import { AutoScrollDirective } from './auto-scroll.directive';
import { ProgressStepperComponent } from './warp-core/progress-stepper/progress-stepper.component';
import { TextFieldModule } from '@angular/cdk/text-field';
import { ExpandableComponent } from './warp-core/expandable/expandable.component';
import { TooltipComponent } from './warp-core/tooltip/tooltip.component';
import { ExpandableTitleComponent } from './warp-core/expandable/expandable-title/expandable-title.component';
import { ConfirmDialogComponent } from './warp-core/confirm-dialog/confirm-dialog.component';
import { InfoBlockComponent } from './warp-core/info-block/info-block.component';
import { ExecutionStateService } from './services/execution-state.service';
import { ConfigDirtyService } from './services/config-dirty.service';
import { TauriEventBridgeService } from './warp-core/tauri-event-bridge.service';
import {
  isScenarioProgressStepData,
  mapScenarioStepsToStepperSteps,
  ScenarioProgressStepData,
} from './scenario-progress-stepper.adapter';
import { RequiredField, Task, Tasks, Step } from './models/scenario.model';
import { TaskProgress } from './models/step-state.model';
import { StepperStep } from './warp-core/progress-stepper/progress-stepper.component';

@Component({
  selector: 'app-root',
  imports: [
    CommonModule,
    ReactiveFormsModule,
    ClipboardModule,
    TitlebarComponent,
    SidebarComponent,
    AutoScrollDirective,
    ProgressStepperComponent,
    TextFieldModule,
    ExpandableComponent,
    TooltipComponent,
    ExpandableTitleComponent,
    ConfirmDialogComponent,
    InfoBlockComponent,
  ],
  templateUrl: './app.component.html',
  styleUrl: './app.component.scss'
})
export class AppComponent implements OnInit, OnDestroy {
  Object = Object;

  private executionStateService = inject(ExecutionStateService);
  private configDirtyService = inject(ConfigDirtyService);
  private bridgeService = inject(TauriEventBridgeService);

  // --- titlebar controls ---
  dryRun = signal(false);
  showClearConfirm = signal(false);

  // --- sidebar config ---
  readonly sidebarTabs = [
    { id: 'steps', title: 'Steps' },
    { id: 'tasks', title: 'Tasks' },
    { id: 'variables', title: 'Variables' },
  ];

  // --- sidebar content state ---
  taskExpandedMap: { [key: string]: boolean } = {};
  stepExpandedMap: { [index: number]: boolean } = {};
  onFailExpandedMap: { [index: number]: boolean } = {};
  onFailStepExpandedMap: { [key: string]: boolean } = {};

  configDirty = this.configDirtyService.isDirty;
  isTaskModified = (name: string) => this.configDirtyService.isTaskModified(name);
  isVariableModified = (name: string) => this.configDirtyService.isVariableModified(name);

  getOnFailStepKey(parentIndex: number, onFailIndex: number): string {
    return `${parentIndex}-${onFailIndex}`;
  }

  onFailStepExpanded(parentIndex: number, onFailIndex: number): boolean {
    return this.onFailStepExpandedMap[this.getOnFailStepKey(parentIndex, onFailIndex)] || false;
  }

  // --- scenario config form ---
  scenarioConfigPath = new FormControl<string>('', {
    asyncValidators: this.configPathValidator(),
  });

  private _lastInvalidScenarioConfigPathValue = false;

  get isInvalidScenarioConfigPath(): boolean {
    if (this.scenarioConfigPath.pending) {
      return this._lastInvalidScenarioConfigPathValue;
    }
    this._lastInvalidScenarioConfigPathValue =
      this.scenarioConfigPath.invalid && (this.scenarioConfigPath.dirty || this.scenarioConfigPath.touched);
    return this._lastInvalidScenarioConfigPathValue;
  }

  requiredFieldsExpanded = true;
  executionProgressExpanded = true;
  logExpanded = true;

  requiredFields: { [key: string]: RequiredField } = {};
  requiredFieldsFormGroup = new FormGroup<{ [key: string]: FormControl<string | null> }>({});
  private requiredFieldsChangesSubscription?: Subscription;

  isExecuting = this.executionStateService.isExecuting;
  stepExecStates = computed(() => this.executionStateService.executionState()?.steps ?? []);
  executionError = computed(() => {
    const status = this.executionStateService.executionState()?.status;
    return status?.kind === 'Failed' ? status.error : null;
  });

  executionLog = signal('');
  private pendingLogBuffer: string[] = [];
  private flushTimeout: ReturnType<typeof setTimeout> | undefined;
  private lastWasProgress = false;

  private logMessageSub?: Subscription;
  private logProgressSub?: Subscription;

  resolvedVariables: { [key: string]: string } = {};
  tasks: Tasks = {};
  steps: Step[] = [];

  progressSteps(): StepperStep[] {
    return mapScenarioStepsToStepperSteps(this.steps, this.stepExecStates());
  }

  scenarioStepData(step: StepperStep): ScenarioProgressStepData | null {
    return isScenarioProgressStepData(step.data) ? step.data : null;
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

  getTransferStats(elapsedMs: number, bytesTransferred: number): string {
    if (bytesTransferred === 0 || elapsedMs < 100) return '';
    const elapsed = elapsedMs / 1000;
    const mbps = (bytesTransferred / (1024 * 1024)) / elapsed;
    return `${elapsed.toFixed(1)}s, ${mbps.toFixed(2)} MB/s`;
  }

  transferProgress(progress: TaskProgress | null): number {
    if (!progress || progress.type !== 'SftpCopy' || progress.bytes_total === 0) {
      return 0;
    }

    return Math.round((progress.bytes_transferred / progress.bytes_total) * 100);
  }

  transferLabel(progress: TaskProgress | null): string {
    if (!progress || progress.type !== 'SftpCopy') {
      return '0% (0 B / 0 B)';
    }

    const progressPercentage = this.transferProgress(progress);
    const current = this.formatBytes(progress.bytes_transferred);
    const total = this.formatBytes(progress.bytes_total);
    const stats = this.getTransferStats(progress.elapsed_ms, progress.bytes_transferred);

    return stats
      ? `${progressPercentage}% (${current} / ${total} - ${stats})`
      : `${progressPercentage}% (${current} / ${total})`;
  }

  ngOnInit(): void {
    invoke<boolean>('get_dry_run').then(v => this.dryRun.set(v));

    this.fetchConfigPath()
      .then(() => Promise.all([
        this.getRequiredVariables(),
        this.getResolvedVariables(),
        this.getTasks(),
        this.getSteps(),
        this.configDirtyService.syncFromBackend(),
      ]));

    this.setupFormValueChangeListener();
    this.setupLogUpdatesListener();
    this.setupLogProgressListener();
    this.executionStateService.init();
  }

  ngOnDestroy(): void {
    this.cleanupSubscriptions();
    this.logMessageSub?.unsubscribe();
    this.logProgressSub?.unsubscribe();
    this.executionStateService.destroy();
    this.flushBufferedLog();
  }

  // --- titlebar domain controls ---

  toggleDryRun(): void {
    const newValue = !this.dryRun();
    this.dryRun.set(newValue);
    invoke('set_dry_run', { dryRun: newValue });
  }

  saveState(): void {
    invoke('save_state');
  }

  clearState(): void {
    this.showClearConfirm.set(true);
  }

  onClearConfirmed(confirmed: boolean): void {
    this.showClearConfirm.set(false);
    if (confirmed) {
      invoke('clear_state');
    }
  }

  // --- sidebar action bar ---

  async saveConfig(): Promise<void> {
    await invoke('save_config');
    this.configDirtyService.markClean();
  }

  async discardChanges(): Promise<void> {
    await invoke('discard_config_changes');
    this.configDirtyService.markClean();
    await this.onConfigDiscarded();
  }

  async saveTask(name: string, event: Event): Promise<void> {
    event.stopPropagation();
    await invoke('save_task_config', { taskName: name });
    await this.configDirtyService.syncFromBackend();
  }

  async discardTask(name: string, event: Event): Promise<void> {
    event.stopPropagation();
    await invoke('discard_task_config', { taskName: name });
    await this.configDirtyService.syncFromBackend();
    await this.onConfigDiscarded();
  }

  async saveVariable(name: string, event: Event): Promise<void> {
    event.stopPropagation();
    await invoke('save_variable_config', { name });
    await this.configDirtyService.syncFromBackend();
  }

  async discardVariable(name: string, event: Event): Promise<void> {
    event.stopPropagation();
    await invoke('discard_variable_config', { name });
    await this.configDirtyService.syncFromBackend();
    await this.onConfigDiscarded();
  }

  onTaskFieldInput(taskName: string): void {
    this.configDirtyService.markTaskDirty(taskName);
  }

  onTaskFieldChanged(taskName: string, field: string, event: Event): void {
    const value = (event.target as HTMLInputElement).value;
    const task = { ...this.tasks[taskName], [field]: value };
    this.onTaskChanged({ name: taskName, task });
  }

  onVariableInput(name: string): void {
    this.configDirtyService.markVariableDirty(name);
  }

  // --- scenario form ---

  private async setupFormValueChangeListener(): Promise<void> {
    this.cleanupSubscriptions();

    this.requiredFieldsChangesSubscription = this.requiredFieldsFormGroup.valueChanges
      .pipe(debounceTime(300))
      .subscribe((requiredFieldsPartial) => {
        for (const name in requiredFieldsPartial) {
          if (name) {
            this.requiredFields[name].value = requiredFieldsPartial[name]!;
          }
        }
        this.updateRequiredVariables().then(() => {
          this.getResolvedVariables();
        });
      });
  }

  private cleanupSubscriptions(): void {
    if (this.requiredFieldsChangesSubscription) {
      this.requiredFieldsChangesSubscription.unsubscribe();
      this.requiredFieldsChangesSubscription = undefined;
    }
  }

  async fetchConfigPath(): Promise<void> {
    return invoke<string>('get_config_path')
      .then((configPath) => {
        this.scenarioConfigPath.setValue(configPath);
      });
  }

  clearLog(): void {
    this.executionLog.set('');
  }

  async selectRequiredFile(requiredFieldName: string): Promise<void> {
    const requiredFieldLabel = this.requiredFields[requiredFieldName].label;
    const selectedFilePath = await dialog.open({
      multiple: false,
      filters: [{
        name: requiredFieldLabel || '<unknown>',
        extensions: ['*']
      }]
    });

    if (selectedFilePath && typeof selectedFilePath === 'string') {
      this.requiredFields[requiredFieldName].value = selectedFilePath;
      this.requiredFieldsFormGroup.controls[requiredFieldName].setValue(selectedFilePath);
    }
  }

  async selectConfigFile(): Promise<void> {
    const configPath = await dialog.open({
      multiple: false,
      filters: [{
        name: 'Configuration File',
        extensions: ['toml']
      }]
    });

    if (configPath && typeof configPath === 'string') {
      this.scenarioConfigPath.setValue(configPath);
      await this.loadConfig();
    }
  }

  async validatePathAndLoadConfig(): Promise<void> {
    const path = this.scenarioConfigPath.value;
    if (!path || path.trim() === '') {
      this.scenarioConfigPath.setErrors(null);
    } else if (await invoke<boolean>('is_valid_config_path', { path })) {
      await this.loadConfig();
    } else {
      this.scenarioConfigPath.setErrors({ invalidPath: true });
    }
  }

  async loadConfig() {
    this.executionStateService.reset();
    await invoke('load_config', { configPath: this.scenarioConfigPath.value });
    await this.getTasks();
    await this.getSteps();
    await this.getRequiredVariables();
    await this.getResolvedVariables();
  }

  configPathValidator(): AsyncValidatorFn {
    return (control: AbstractControl): Observable<ValidationErrors | null> => {
      if (!control.value || control.value.trim() === '') {
        return of(null);
      }

      return of(control.value).pipe(
        debounceTime(500),
        switchMap(path =>
          from(invoke<boolean>('is_valid_config_path', { path })).pipe(
            map(isValid => isValid ? null : { invalidPath: true })
          )
        )
      );
    };
  }

  private async getRequiredVariables(): Promise<void> {
    this.requiredFields = {};
    this.requiredFieldsFormGroup = new FormGroup<{ [key: string]: FormControl<string | null> }>({});
    return invoke<{ [key: string]: RequiredField }>('get_required_variables')
      .then((requiredVariables) => {
        this.requiredFields = requiredVariables;
        for (const name in requiredVariables) {
          if (!requiredVariables[name].read_only) {
            const formControl = new FormControl(this.requiredFields[name].value);
            this.requiredFieldsFormGroup.addControl(name, formControl);
          }
        }
        this.setupFormValueChangeListener();
      });
  }

  private async getResolvedVariables(): Promise<void> {
    return invoke<{ [key: string]: string }>('get_resolved_variables')
      .then((resolvedVariables) => {
        this.resolvedVariables = resolvedVariables || {};
      });
  }

  private async getTasks(): Promise<void> {
    return invoke<Tasks>('get_tasks')
      .then((tasks) => {
        this.tasks = tasks || {};
      });
  }

  private async getSteps(): Promise<void> {
    return invoke<Step[]>('get_steps')
      .then((steps) => {
        this.steps = steps || [];
      });
  }

  async updateRequiredVariables(): Promise<void> {
    const requiredVariables: { [key: string]: string } = {};
    for (const name in this.requiredFields) {
      requiredVariables[name] = this.requiredFields[name].value;
    }
    return invoke('update_required_variables', { requiredVariables });
  }

  private setupLogUpdatesListener(): void {
    this.logMessageSub = this.bridgeService.getStream<string>('log-message').subscribe(message => {
      this.lastWasProgress = false;
      this.pendingLogBuffer.push(message);

      if (!this.flushTimeout) {
        this.flushTimeout = setTimeout(() => this.flushBufferedLog(), 100);
      }
    });
  }

  private setupLogProgressListener(): void {
    this.logProgressSub = this.bridgeService.getStream<string>('log-progress').subscribe(message => {
      this.flushBufferedLog();
      const prev = this.executionLog();
      if (this.lastWasProgress) {
        const lastNewline = prev.lastIndexOf('\n');
        this.executionLog.set(
          lastNewline === -1
            ? message
            : prev.slice(0, lastNewline) + '\n' + message
        );
      } else {
        this.lastWasProgress = true;
        this.executionLog.set(prev === '' ? message : prev + '\n' + message);
      }
    });
  }

  flushBufferedLog(): void {
    if (this.pendingLogBuffer.length === 0) {
      return;
    }
    const chunk = this.pendingLogBuffer.join('\n');
    this.executionLog.update(prev => {
      const combined = prev === '' ? chunk : prev + '\n' + chunk;
      const maxSize = 1_000_000;
      return combined.length > maxSize
        ? combined.slice(combined.length - maxSize)
        : combined;
    });
    this.pendingLogBuffer.length = 0;
    this.flushTimeout = undefined;
  }

  executeScenario(): void {
    this.executionStateService.reset();
    invoke('execute_scenario');
  }

  async onTaskChanged(event: { name: string; task: Task }): Promise<void> {
    await invoke('update_task', { taskName: event.name, task: event.task });
    this.configDirtyService.markTaskDirty(event.name);
    await this.configDirtyService.syncFromBackend();
    await this.getTasks();
    await this.getSteps();
  }

  async onVariableChanged(event: { name: string; value: string }): Promise<void> {
    await invoke('update_defined_variable', { name: event.name, value: event.value });
    this.configDirtyService.markVariableDirty(event.name);
    await this.configDirtyService.syncFromBackend();
    await this.getResolvedVariables();
  }

  async onConfigDiscarded(): Promise<void> {
    this.configDirtyService.markClean();
    await Promise.all([
      this.getTasks(),
      this.getSteps(),
      this.getRequiredVariables(),
      this.getResolvedVariables(),
    ]);
  }
}
