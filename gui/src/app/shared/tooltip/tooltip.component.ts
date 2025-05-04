import { Component, ElementRef, HostBinding, Input, Renderer2 } from '@angular/core';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'tooltip',
  template: `<span>{{text}}</span>`,
  styleUrls: ['./tooltip.component.scss'],
  imports: [CommonModule]
})
export class TooltipComponent {
  @Input() text: string = '';
  @Input() arrow: 'top' | 'left' = 'top';

  @HostBinding('class.visible') private visible = false;
  @HostBinding('class.transition') private transition = true;
  @HostBinding('class.arrow-top') get arrowTop() { return this.arrow === 'top'; }
  @HostBinding('class.arrow-left') get arrowLeft() { return this.arrow === 'left'; }

  private parent!: HTMLElement;
  private unlistens: Array<() => void> = [];

  constructor(
    private elRef: ElementRef<HTMLElement>,
    private renderer: Renderer2
  ) { }

  ngAfterViewInit(): void {
    this.parent = this.elRef.nativeElement.parentElement as HTMLElement;
    this.unlistens.push(this.renderer.listen(this.parent, 'mouseenter', () => { this.visible = true; this.transition = true; }));
    this.unlistens.push(this.renderer.listen(this.parent, 'mouseleave', () => { this.visible = false; this.transition = true; }));
    this.unlistens.push(this.renderer.listen(this.parent, 'mousedown', () => { this.visible = false; this.transition = false; }));
  }

  ngOnDestroy(): void {
    this.unlistens.forEach(unlisten => unlisten());
  }
}
