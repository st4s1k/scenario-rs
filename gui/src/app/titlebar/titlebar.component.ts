import { Component } from '@angular/core';
import { invoke } from '@tauri-apps/api';
import { appWindow } from '@tauri-apps/api/window'

@Component({
  selector: 'app-titlebar',
  standalone: true,
  imports: [],
  templateUrl: './titlebar.component.html',
  styleUrl: './titlebar.component.scss'
})
export class TitlebarComponent {

  save(): void {
    invoke('save_state');
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
