import { CommonModule } from '@angular/common';
import { Component, signal, OnDestroy } from '@angular/core';
import { FormControl, FormGroup, ReactiveFormsModule, AbstractControl, ValidationErrors, AsyncValidatorFn } from "@angular/forms";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { NoRightClickDirective } from './no-right-click.directive';
import { TitlebarComponent } from "./titlebar/titlebar.component";
import { ClipboardModule } from 'ngx-clipboard';
import * as dialog from "@tauri-apps/plugin-dialog"
import { Subscription, Observable, of, from } from 'rxjs';
import { debounceTime, switchMap, map } from 'rxjs/operators';
import { SidebarComponent } from './sidebar/sidebar.component';
import { AutoScrollDirective } from './auto-scroll.directive';
import { ExecutionProgressComponent } from './execution-progress/execution-progress.component';
import { TextFieldModule } from '@angular/cdk/text-field';
import { ExpandableComponent } from './shared/expandable/expandable.component';
import { TooltipComponent } from './shared/tooltip/tooltip.component';

interface RequiredFieldsForm {
  [key: string]: FormControl<string | null>;
}

interface RequiredField {
  label: string;
  var_type: string;
  value: string;
  read_only: boolean;
}

interface ResolvedVariables {
  [key: string]: string;
}

export type TaskType = 'SftpCopy' | 'RemoteSudo';

export interface Task {
  description: string;
  error_message: string;
  task_type: TaskType;
  command?: string;
  source_path?: string;
  destination_path?: string;
}

export interface OnFailStep {
  index: number;
  task: Task;
}

export interface Step {
  index: number;
  task: Task;
  on_fail_steps: OnFailStep[];
}

@Component({
  selector: 'app-root',
  imports: [
    CommonModule,
    ReactiveFormsModule,
    ClipboardModule,
    TitlebarComponent,
    NoRightClickDirective,
    SidebarComponent,
    AutoScrollDirective,
    ExecutionProgressComponent,
    TextFieldModule,
    ExpandableComponent,
    TooltipComponent
  ],
  templateUrl: './app.component.html',
  styleUrl: './app.component.scss'
})
export class AppComponent implements OnDestroy {
  Object = Object;

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
  requiredFieldsFormGroup = new FormGroup<RequiredFieldsForm>({});
  private requiredFieldsChangesSubscription?: Subscription;

  isExecuting = signal(false);

  executionLog = signal('');
  private pendingLogBuffer: string[] = [];
  private flushTimeout: ReturnType<typeof setTimeout> | undefined;

  resolvedVariables: ResolvedVariables = {};
  tasks: { [key: string]: Task } = {};
  steps: Step[] = [];

  unlistenLogUpdates?: UnlistenFn;
  unlistenExecutionStatus?: UnlistenFn;

  ngOnInit(): void {
    this.fetchConfigPath()
      .then(() => Promise.all([
        this.getRequiredVariables(),
        this.getResolvedVariables(),
        this.getTasks(),
        this.getSteps()
      ]));

    this.setupFormValueChangeListener();
    this.setupLogUpdatesListener();
    this.setupExecutionStatusListener();
  }

  ngOnDestroy(): void {
    this.cleanupSubscriptions();
    if (this.unlistenLogUpdates) {
      this.unlistenLogUpdates();
    }
    if (this.unlistenExecutionStatus) {
      this.unlistenExecutionStatus();
    }
    this.flushBufferedLog();
  }

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
      })
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
    this.requiredFieldsFormGroup = new FormGroup<RequiredFieldsForm>({});
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
    return invoke<{ [key: string]: Task }>('get_tasks')
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
    return invoke('update_required_variables', { requiredVariables })
  }

  private async setupLogUpdatesListener(): Promise<void> {
    this.unlistenLogUpdates = await listen<string>('log-message', (event) => {
      this.pendingLogBuffer.push(event.payload);

      if (!this.flushTimeout) {
        this.flushTimeout = setTimeout(() => this.flushBufferedLog(), 100);
      }
    });
  }

  private flushBufferedLog(): void {
    const chunk = this.pendingLogBuffer.join('\n');
    this.executionLog.update(prev => {
      const combined = prev === '' ? chunk : prev + '\n' + chunk;
      const maxSize = 1_000_000; // ~1MB
      return combined.length > maxSize
        ? combined.slice(combined.length - maxSize)
        : combined;
    });
    this.pendingLogBuffer.length = 0;
    this.flushTimeout = undefined;
  }

  private async setupExecutionStatusListener(): Promise<void> {
    this.unlistenExecutionStatus = await listen<boolean>('execution-status', (event) => {
      this.isExecuting.set(event.payload);
    });
  }

  executeScenario(): void {
    invoke('execute_scenario');
  }
}
