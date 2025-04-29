import {
  Directive, ElementRef, Input, NgZone,
  OnDestroy,
  Signal, effect, isSignal
} from '@angular/core';
import { Observable, Subscription } from 'rxjs';

@Directive({
  selector: '[autoScroll]',
  standalone: true
})
export class AutoScrollDirective implements OnDestroy {

  @Input('autoScroll')
  set source(value: Signal<unknown> | Observable<unknown>) {
    this.sub?.unsubscribe();
    if (isSignal(value)) {
      this.currentSignal = value;
      this.sub = undefined;
    } else if (isObservable(value)) {
      this.currentSignal = undefined;
      this.sub = value.subscribe(() => this.scheduleScroll());
    } else {
      this.currentSignal = undefined;
      this.sub = undefined;
    }
  }

  private currentSignal?: Signal<unknown>;
  private sub?: Subscription;
  private pending = false;

  constructor(
    private host: ElementRef<HTMLTextAreaElement>,
    private zone: NgZone
  ) {
    effect(() => {
      if (this.currentSignal) {
        this.currentSignal();
        this.scheduleScroll();
      }
    });
  }

  ngOnDestroy() {
    this.sub?.unsubscribe();
  }

  private scheduleScroll() {
    if (this.pending) return;

    this.pending = true;
    this.zone.runOutsideAngular(() => {
      requestAnimationFrame(() => {
        const ta = this.host.nativeElement;
        if (ta.isConnected) {
          ta.scrollTop = ta.scrollHeight;
        }
        this.pending = false;
      });
    });
  }
}

function isObservable(o: unknown): o is Observable<unknown> {
  return !!o && typeof (o as any).subscribe === 'function';
}
