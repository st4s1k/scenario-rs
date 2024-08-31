import { CommonModule } from '@angular/common';
import { Component, signal, WritableSignal } from '@angular/core';
import { FormsModule, ReactiveFormsModule } from "@angular/forms";
import { RouterOutlet } from '@angular/router';
import { dialog } from "@tauri-apps/api"; // Import Tauri's dialog API
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";
import { TitlebarComponent } from "./titlebar/titlebar.component";

interface RequiredField {
  label: string;
  type: string;
  value: WritableSignal<string>;
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
  scenarioConfigPath = signal('');
  requiredFields: { [key: string]: RequiredField } = {};
  executionLoading = signal(false);

  unlisten = listen('log-update', () => {
    console.log('Received log update signal');
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
        if (this.scenarioConfigPath()) {
          this.loadConfigFile();
        }
      });
  }

  async fetchConfigPath(): Promise<void> {
    return invoke<string>('get_config_path')
      .then((configPath) => {
        this.scenarioConfigPath.set(configPath);
      })
  }

  clearLog(): void {
    invoke('clear_log')
  }

  async selectRequiredFile(requiredFieldName: string): Promise<void> {
    const requiredFieldLabel = this.requiredFields[requiredFieldName]?.label;
    const selectedFilePath = await dialog.open({
      multiple: false,
      filters: [{
        name: requiredFieldLabel || '<unknown>',
        extensions: ['*']
      }]
    });

    if (selectedFilePath && typeof selectedFilePath === 'string') {
      this.requiredFields[requiredFieldName].value.set(selectedFilePath);
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
      this.scenarioConfigPath.set(configPath);
      await this.loadConfigFile();
    }
  }

  async loadConfigFile(): Promise<void> {
    return invoke<{ [key: string]: string }>('load_config', { configPath: this.scenarioConfigPath() })
      .then(async (requiredFieldsMap) => {
        let savedRequiredVariables = await invoke<{ [key: string]: string }>('get_required_variables');
        Object.entries(requiredFieldsMap).forEach(([key, value]) =>
          this.requiredFields[key] = {
            label: value,
            type: key.startsWith('path:') ? 'path' : 'text',
            value: signal(savedRequiredVariables[key] || '')
          });
        console.log('Required fields', JSON.stringify(this.requiredFields, null, 2));
      })
  }

  async updateRequiredVariables(): Promise<void> {
    let requiredVariables: { [key: string]: string } = {};
    Object.entries(this.requiredFields).forEach(([key, field]) => {
      requiredVariables[key] = field.value();
    });
    console.log('Updating required variables', JSON.stringify(requiredVariables, null, 2));
    return invoke('update_required_variables', { requiredVariables })
  }

  executeScenario(): void {
    this.executionLoading.set(true);
    invoke('execute_scenario')
      .then(() => {
        this.executionLoading.set(false);
      });
  }
}
