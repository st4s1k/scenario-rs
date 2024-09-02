import { Directive, HostListener } from '@angular/core';

@Directive({
  standalone: true,
  selector: '[appNoRightClick]'
})
export class NoRightClickDirective {

  @HostListener('contextmenu', ['$event'])
  onRightClick(event: Event) {
    event.preventDefault();
  }

  constructor() { }

}
