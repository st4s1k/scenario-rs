import { CommonModule } from '@angular/common';
import { Component, signal } from '@angular/core';
import {
  FormControl,
  FormGroup,
  FormsModule,
  ReactiveFormsModule
} from "@angular/forms";
import { RouterOutlet } from '@angular/router';
import { dialog } from "@tauri-apps/api"; // Import Tauri's dialog API
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";
import { TitlebarComponent } from "./titlebar/titlebar.component";

interface RequiredField {
  name: string;
  label: string;
  type: string;
}

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [
    CommonModule,
    RouterOutlet,
    FormsModule,
    ReactiveFormsModule,
    TitlebarComponent
  ],
  templateUrl: './app.component.html',
  styleUrl: './app.component.scss'
})
export class AppComponent {
  executionLog = signal('');
  scenarioConfigPath = new FormControl('');
  requiredFields = signal<RequiredField[]>([]);
  requiredFieldsValuesForm = new FormGroup({});

  unlisten = listen<string>('log-update', () => {
    invoke<string>('get_log')
      .then((log) => {
        this.executionLog.set(log);
      });
  });

  ngOnInit(): void {
    invoke<string>('get_log')
      .then((log) => {
        this.executionLog.set(log);
      });

    this.fetchConfigPath()
      .then(() => {
        if (this.scenarioConfigPath.value && this.scenarioConfigPath.value.trim() !== '') {
          this.loadConfigFile();
        }
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
    const requiredFieldLabel = this.requiredFields().find((field) => field.name === requiredFieldName)?.label;
    const selectedFilePath = await dialog.open({
      multiple: false,
      filters: [{
        name: requiredFieldLabel || '<unknown> File',
        extensions: ['*']
      }]
    });

    if (selectedFilePath && typeof selectedFilePath === 'string') {
      this.requiredFieldsValuesForm.get(requiredFieldName)?.setValue(selectedFilePath);
    }

    this.executionLog.set(this.executionLog() + `${requiredFieldLabel || '<unknown>'}: ${selectedFilePath}\n`);
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
    invoke<{ [key: string]: string }>('load_config', { configPath: this.scenarioConfigPath.value })
      .then((requiredFieldsObject) => {
        let requiredFields = Object.entries(requiredFieldsObject).map(([key, value]) => ({
          name: key,
          label: value,
          value: '',
          type: key.startsWith('path:') ? 'path' : 'text'
        }));
        this.requiredFields.set(requiredFields);
        requiredFields.forEach((field) => {
          this.requiredFieldsValuesForm.addControl(field.name, new FormControl(''));
        });
      });
  }

  executeScenario(): void {
    let requiredVariables = this.requiredFieldsValuesForm.getRawValue();
    invoke('execute_scenario', { requiredVariables });
  }
}
