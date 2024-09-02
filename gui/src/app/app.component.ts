import { CommonModule } from '@angular/common';
import { Component, signal } from '@angular/core';
import { FormControl, FormGroup, ReactiveFormsModule } from "@angular/forms";
import { RouterOutlet } from '@angular/router';
import { dialog } from "@tauri-apps/api";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";
import { NoRightClickDirective } from './no-right-click.directive';
import { TitlebarComponent } from "./titlebar/titlebar.component";
import { ClipboardModule } from 'ngx-clipboard';

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
    ClipboardModule,
    RouterOutlet,
    TitlebarComponent,
    NoRightClickDirective
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
      .subscribe((requiredFieldsPartial) => {
        for (const name in requiredFieldsPartial) {
          if (name) {
            this.requiredFields[name].value = requiredFieldsPartial[name]!;
          }
        }
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
    if (configPath.trim() === '') {
      return;
    }
    return invoke<{ [key: string]: string }>('load_config', { configPath })
      .then((requiredFields) => invoke<{ [key: string]: string }>('get_required_variables')
        .then((savedRequiredVariables) => {
          for (const name in requiredFields) {
            this.requiredFields[name] = {
              label: requiredFields[name],
              type: name.startsWith('path:') ? 'path' : 'text',
              value: savedRequiredVariables[name] || ''
            };
            console.log('Adding required field', name, this.requiredFields[name]);
            const formControl = new FormControl(this.requiredFields[name].value);
            this.requiredFieldsFormGroup.addControl(name, formControl);
          }
        })
      )
  }

  async updateRequiredVariables(): Promise<void> {
    const requiredVariables: { [key: string]: string } = {};
    for (const name in this.requiredFields) {
      requiredVariables[name] = this.requiredFields[name].value;
    }
    console.log('Updating required variables', JSON.stringify(requiredVariables, null, 2));
    return invoke('update_required_variables', { requiredVariables })
  }

  executeScenario(): void {
    this.isExecuting.set(true);
    invoke('execute_scenario')
      .then(() => this.isExecuting.set(false));
  }
}
