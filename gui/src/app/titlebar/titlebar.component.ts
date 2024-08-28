import { Component } from '@angular/core';
import { appWindow } from '@tauri-apps/api/window'

@Component({
  selector: 'app-titlebar',
  standalone: true,
  imports: [],
  templateUrl: './titlebar.component.html',
  styleUrl: './titlebar.component.scss'
})
export class TitlebarComponent {

  minimize(): void {
    appWindow.minimize();
  }

  maximize(): void {
    appWindow.maximize();
  }

  close(): void {
    appWindow.close();
  }

}
