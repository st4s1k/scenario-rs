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

  dryRun = signal(false);

  async ngOnInit(): Promise<void> {
    const dryRun = await invoke<boolean>('get_dry_run');
    this.dryRun.set(dryRun);
  }

  toggleDryRun(): void {
    const newValue = !this.dryRun();
    this.dryRun.set(newValue);
    invoke('set_dry_run', { dryRun: newValue });
  }

  save(): void {
    invoke('save_state');
  }

  saveConfig(): void {
    invoke('save_config');
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
