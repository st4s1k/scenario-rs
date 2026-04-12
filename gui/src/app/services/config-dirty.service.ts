import { Injectable, computed, signal } from '@angular/core';
import { invoke } from '@tauri-apps/api/core';

export interface ConfigDiff {
  modified_tasks: string[];
  modified_variables: string[];
}

@Injectable({ providedIn: 'root' })
export class ConfigDirtyService {
  readonly modifiedTasks = signal<Set<string>>(new Set());
  readonly modifiedVariables = signal<Set<string>>(new Set());

  readonly isDirty = computed(
    () => this.modifiedTasks().size > 0 || this.modifiedVariables().size > 0
  );

  isTaskModified(name: string): boolean {
    return this.modifiedTasks().has(name);
  }

  isVariableModified(name: string): boolean {
    return this.modifiedVariables().has(name);
  }

  markTaskDirty(name: string): void {
    const next = new Set(this.modifiedTasks());
    next.add(name);
    this.modifiedTasks.set(next);
  }

  markVariableDirty(name: string): void {
    const next = new Set(this.modifiedVariables());
    next.add(name);
    this.modifiedVariables.set(next);
  }

  markClean(): void {
    this.modifiedTasks.set(new Set());
    this.modifiedVariables.set(new Set());
  }

  async syncFromBackend(): Promise<void> {
    const diff = await invoke<ConfigDiff>('get_config_diff');
    this.modifiedTasks.set(new Set(diff.modified_tasks));
    this.modifiedVariables.set(new Set(diff.modified_variables));
  }
}
