import { Injectable, OnDestroy } from '@angular/core';
import { Observable, Subject } from 'rxjs';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

@Injectable({ providedIn: 'root' })
export class TauriEventBridgeService implements OnDestroy {
  private subjects = new Map<string, Subject<unknown>>();
  private unlistens = new Map<string, UnlistenFn>();
  private destroyed = false;

  getStream<T>(id: string): Observable<T> {
    if (!this.subjects.has(id)) {
      this.subjects.set(id, new Subject<unknown>());
      void this.connect(id);
    }
    return (this.subjects.get(id) as Subject<unknown>).asObservable() as Observable<T>;
  }

  private async connect(id: string): Promise<void> {
    if (this.destroyed || this.unlistens.has(id)) {
      return;
    }

    const unlisten = await listen<unknown>(id, ({ payload }) => {
      this.subjects.get(id)?.next(payload);
    });

    if (this.destroyed) {
      unlisten();
      return;
    }

    this.unlistens.set(id, unlisten);
  }

  ngOnDestroy(): void {
    this.destroyed = true;
    this.unlistens.forEach(unlisten => unlisten());
    this.unlistens.clear();
    this.subjects.forEach(s => s.complete());
    this.subjects.clear();
  }
}
