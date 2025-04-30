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
export class AutoScrollDirective {

  @Input('autoScroll') valueChangedSignal?: Signal<unknown>;

  private pending = false;
  private autoScrollEnabled = true;

  constructor(
    private host: ElementRef<HTMLTextAreaElement>,
    private zone: NgZone
  ) {
    effect(() => {
      if (this.valueChangedSignal) {
        this.valueChangedSignal();
        this.scheduleScroll();
      }
    });
    zone.runOutsideAngular(() => {
      host.nativeElement.addEventListener('scroll', () => this.onScroll());
    });
  }

  private onScroll() {
    const ta = this.host.nativeElement;
    const scrollThreshold = 32;
    const distanceFromBottom = ta.scrollHeight - ta.scrollTop - ta.clientHeight;
    this.autoScrollEnabled = distanceFromBottom < scrollThreshold;
  }
  
  private scheduleScroll() {
    if (this.pending || !this.autoScrollEnabled) return;

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
