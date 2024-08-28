import { CommonModule } from '@angular/common';
import { Component, signal } from '@angular/core';
import {
  FormsModule,
  FormGroup,
  FormControl,
  ReactiveFormsModule
} from "@angular/forms";
import { RouterOutlet } from '@angular/router';
import { dialog } from "@tauri-apps/api"; // Import Tauri's dialog API
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";
import { TitlebarComponent } from "./titlebar/titlebar.component";

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
  selectScenarioConfigForm = new FormGroup({
    scenarioConfigPath: new FormControl('')
  });

  unlisten = listen<string>('log-update', () => {
    invoke<string>('get_log')
      .then((log) => {
        console.log(`output log: ${log}`);
        this.executionLog.set(log);
      });
  });

  ngOnInit(): void {
    invoke<string>('get_log')
      .then((log) => {
        console.log(`output log: ${log}`);
        this.executionLog.set(log);
      });
  }

  clearLog(): void {
    invoke('clear_log')
      .then(() => {
        console.log('Log cleared');
      });
  }

  async selectConfigFile(): Promise<void> {
    const selectedFilePath = await dialog.open({
      multiple: false,
      filters: [{
        name: 'Configuration Files',
        extensions: ['json']
      }]
    });

    if (selectedFilePath && typeof selectedFilePath === 'string') {
      invoke('load_config', { configPath: selectedFilePath })
        .then(() => {
          console.log('Config loaded');
        });
    }
  }

  executeScenario(): void {
    invoke('execute_scenario')
      .then(() => {
        console.log('Scenario started');
      });
  }
}
