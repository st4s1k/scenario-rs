import { CommonModule } from '@angular/common';
import { Component, signal } from '@angular/core';
import { FormsModule } from "@angular/forms";
import { RouterOutlet } from '@angular/router';
import { dialog } from "@tauri-apps/api"; // Import Tauri's dialog API
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [CommonModule, RouterOutlet, FormsModule],
  templateUrl: './app.component.html',
  styleUrl: './app.component.css'
})
export class AppComponent {
  outputLog = signal('');

  ngOnInit(): void {
    invoke<string>('get_log')
      .then((log) => {
        console.log(`output log: ${log}`);
        this.outputLog.set(log);
      });
  }

  unlisten = listen<string>('log-update', () => {
    invoke<string>('get_log')
      .then((log) => {
        console.log(`output log: ${log}`);
        this.outputLog.set(log);
      });
  });

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
