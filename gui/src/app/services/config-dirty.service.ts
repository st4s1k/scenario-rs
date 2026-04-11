import { Injectable, signal } from '@angular/core';
import { invoke } from '@tauri-apps/api/core';

@Injectable({ providedIn: 'root' })
export class ConfigDirtyService {
  readonly isDirty = signal(false);

  markDirty(): void {
    this.isDirty.set(true);
  }

  markClean(): void {
    this.isDirty.set(false);
  }

  async syncFromBackend(): Promise<void> {
    const dirty = await invoke<boolean>('has_unsaved_config_changes');
    this.isDirty.set(dirty);
  }
}
