import { Component } from '@angular/core';
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
