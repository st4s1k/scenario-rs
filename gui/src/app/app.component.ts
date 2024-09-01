import { CommonModule } from '@angular/common';
import { Component, signal } from '@angular/core';
import { FormControl, FormGroup, ReactiveFormsModule } from "@angular/forms";
import { RouterOutlet } from '@angular/router';
import { dialog } from "@tauri-apps/api"; // Import Tauri's dialog API
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";
import { TitlebarComponent } from "./titlebar/titlebar.component";

interface RequiredFieldsForm {
  [key: string]: FormControl<string | null>;
}

interface RequiredField {
  label: string;
  type: string;
  value: string;
}

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [
    CommonModule,
    ReactiveFormsModule,
    RouterOutlet,
    TitlebarComponent
  ],
  templateUrl: './app.component.html',
  styleUrl: './app.component.scss'
})
export class AppComponent {
  executionLog = new FormControl<string>('');
  scenarioConfigPath = new FormControl<string>('');
  requiredFields: { [key: string]: RequiredField } = {};
  requiredFieldsFormGroup = new FormGroup<RequiredFieldsForm>({});
  isExecuting = signal(false);

  unlisten = listen('log-update', () => {
    invoke<string>('get_log')
      .then((log) => {
        this.executionLog.setValue(log);
      });
  });

  ngOnInit(): void {
    invoke<string>('get_log')
      .then((log) => this.executionLog.setValue(log));
    this.fetchConfigPath()
      .then(() => this.loadConfigFile());
    this.requiredFieldsFormGroup.valueChanges
      .subscribe((partialValue) => {
        Object.entries(partialValue).forEach(([key, value]) => {
          if (value) {
            this.requiredFields[key].value = value;
          }
        });
        this.updateRequiredVariables();
      });
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
        extensions: ['json']
      }]
    });

    if (configPath && typeof configPath === 'string') {
      this.scenarioConfigPath.setValue(configPath);
      await this.loadConfigFile();
    }
  }

  async loadConfigFile(): Promise<void> {
    const configPath = this.scenarioConfigPath.value || '';
    return invoke<{ [key: string]: string }>('load_config', { configPath })
      .then((requiredFieldsMap) => invoke<{ [key: string]: string }>('get_required_variables')
        .then((savedRequiredVariables) =>
          Object.entries(requiredFieldsMap).forEach(([key, value]) => {
            this.requiredFields[key] = {
              label: value,
              type: key.startsWith('path:') ? 'path' : 'text',
              value: savedRequiredVariables[key] || ''
            };
            console.log('Adding required field', key, this.requiredFields[key]);
            const formControl = new FormControl(this.requiredFields[key].value);
            this.requiredFieldsFormGroup.addControl(key, formControl);
          })
        )
      )
  }

  async updateRequiredVariables(): Promise<void> {
    const requiredVariables: { [key: string]: string } = {};
    Object.entries(this.requiredFields).forEach(([name, requiredField]) => {
      requiredVariables[name] = requiredField.value;
    });
    console.log('Updating required variables', JSON.stringify(requiredVariables, null, 2));
    return invoke('update_required_variables', { requiredVariables })
  }

  executeScenario(): void {
    this.isExecuting.set(true);
    invoke('execute_scenario')
      .then(() => this.isExecuting.set(false));
  }
}
