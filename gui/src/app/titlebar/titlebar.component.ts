import { Component, signal } from '@angular/core';
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { TooltipComponent } from '../shared/tooltip/tooltip.component';
const appWindow = getCurrentWebviewWindow()

@Component({
  selector: 'titlebar',
  imports: [
    TooltipComponent
  ],
  templateUrl: './titlebar.component.html',
  styleUrl: './titlebar.component.scss'
})
export class TitlebarComponent {

  debugMode = signal(false);

  async ngOnInit(): Promise<void> {
    const debugMode = await invoke<boolean>('get_debug_mode');
    this.debugMode.set(debugMode);
  }

  toggleDebugMode(): void {
    const newValue = !this.debugMode();
    this.debugMode.set(newValue);
    invoke('set_debug_mode', { debugMode: newValue });
  }

  save(): void {
    invoke('save_state');
  }

  clearState(): void {
    invoke('clear_state');
  }

  minimize(): void {
    appWindow.minimize();
  }

  maximize(): void {
    appWindow.toggleMaximize();
  }

  close(): void {
    appWindow.close();
  }

}
