import { CommonModule } from '@angular/common';
import { Component, signal, OnDestroy, HostListener } from '@angular/core';
import { FormControl, FormGroup, ReactiveFormsModule } from "@angular/forms";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { NoRightClickDirective } from './no-right-click.directive';
import { TitlebarComponent } from "./titlebar/titlebar.component";
import { ClipboardModule } from 'ngx-clipboard';
import * as dialog from "@tauri-apps/plugin-dialog"
import { Subscription } from 'rxjs';
import { SidebarComponent } from './sidebar/sidebar.component';

interface RequiredFieldsForm {
  [key: string]: FormControl<string | null>;
}

interface RequiredField {
  label: string;
  type: string;
  value: string;
}

interface DefinedVariables {
  [key: string]: string;
}

@Component({
  selector: 'app-root',
  imports: [
    CommonModule,
    ReactiveFormsModule,
    ClipboardModule,
    TitlebarComponent,
    NoRightClickDirective,
    SidebarComponent
  ],
  templateUrl: './app.component.html',
  styleUrl: './app.component.scss'
})
export class AppComponent implements OnDestroy {

  Object = Object;
  
  executionLog = new FormControl<string>('');
  scenarioConfigPath = new FormControl<string>('');
  requiredFields: { [key: string]: RequiredField } = {};
  requiredFieldsFormGroup = new FormGroup<RequiredFieldsForm>({});
  isExecuting = signal(false);
  private formValueChangesSubscription?: Subscription;
  definedVariables: DefinedVariables = {};

  unlistenLogUpdates?: UnlistenFn;
  unlistenExecutionStatus?: UnlistenFn;

  ngOnInit(): void {
    invoke<string>('get_log')
      .then((log) => this.executionLog.setValue(log));
    this.fetchConfigPath()
      .then(() => Promise.all([
        this.getRequiredVariables(),
        this.getDefinedVariables()
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
  }

  private async setupFormValueChangeListener(): Promise<void> {
    this.cleanupSubscriptions();

    this.formValueChangesSubscription = this.requiredFieldsFormGroup.valueChanges
      .subscribe((requiredFieldsPartial) => {
        for (const name in requiredFieldsPartial) {
          if (name) {
            this.requiredFields[name].value = requiredFieldsPartial[name]!;
          }
        }
        this.updateRequiredVariables().then(() => {
          this.getDefinedVariables();
        });
      });
  }

  private cleanupSubscriptions(): void {
    if (this.formValueChangesSubscription) {
      this.formValueChangesSubscription.unsubscribe();
      this.formValueChangesSubscription = undefined;
    }
  }

  async fetchConfigPath(): Promise<void> {
    return invoke<string>('get_config_path')
      .then((configPath) => {
        this.scenarioConfigPath.setValue(configPath);
      })
  }

  clearLog(): void {
    invoke('clear_log')
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
      await this.loadConfigFile();
      await this.getRequiredVariables();
      await this.getDefinedVariables();
    }
  }

  async loadConfigFile(): Promise<void> {
    const configPath = this.scenarioConfigPath.value || '';
    if (configPath.trim() === '') {
      return;
    }
    await invoke('load_config', { configPath });
  }

  private async getRequiredVariables(): Promise<void> {
    this.requiredFields = {};
    this.requiredFieldsFormGroup = new FormGroup<RequiredFieldsForm>({});
    return invoke<{ [key: string]: RequiredField }>('get_required_variables')
      .then((requiredVariables) => {
        for (const name in requiredVariables) {
          this.requiredFields[name] = {
            label: requiredVariables[name].label,
            type: name.startsWith('path:') ? 'path' : 'text',
            value: requiredVariables[name].value || ''
          };
          const formControl = new FormControl(this.requiredFields[name].value);
          this.requiredFieldsFormGroup.addControl(name, formControl);
        }
        this.setupFormValueChangeListener();
      });
  }

  private async getDefinedVariables(): Promise<void> {
    return invoke<{ [key: string]: string }>('get_defined_variables')
      .then((definedVariables) => {
        this.definedVariables = definedVariables || {};
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
    this.unlistenLogUpdates = await listen('log-update', () => {
      invoke<string>('get_log')
        .then((log) => {
          this.executionLog.setValue(log);
        });
    });
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
